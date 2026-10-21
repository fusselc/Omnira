//! Managed llama-server lifecycle (docs/architecture.md sections 2, 4, 5).
//!
//! Responsibilities: runtime variant selection (CUDA -> Vulkan -> CPU when a
//! CUDA binary is present; otherwise Vulkan -> CPU),
//! loopback port reservation, per-session api-key generation, spawn under the
//! Job Object, health gating, shutdown, and status snapshots.

use std::collections::HashMap;
use std::future::Future;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rand::Rng;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

use crate::errors::{AppError, ErrorCode};
use crate::logging;
use crate::process;
use crate::types::{RuntimeState, RuntimeStatus, RuntimeVariant};

/// Default context size Omnira requests from llama-server. Clamped to the
/// model's trained context length when that is smaller.
const DEFAULT_CTX_SIZE: u64 = 8192;
/// Conservative chars-per-token approximation (docs/chat-provider.md sec. 5).
const CHARS_PER_TOKEN: u64 = 3;
/// Fraction of the context reserved for the response.
const RESPONSE_HEADROOM_FRACTION: u64 = 4; // reserve 1/4
/// Port-race retry bound (Decision 12).
const SPAWN_ATTEMPTS: u32 = 3;
/// Health polling.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(120);
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(300);
/// How much of llama-server's startup stderr is kept for the fallback reason
/// / Advanced Diagnostics when a variant fails to come up. Capture stops the
/// moment the runtime is healthy so request-time logs are never retained.
const STDERR_TAIL_BYTES: usize = 4096;
/// The engine behind every managed runtime in MVP (docs/runtimes-and-routing.md
/// pillar 1). Shown honestly in the UI so users can see what is running.
pub const ENGINE_LABEL: &str = "llama.cpp";

/// Win32 `CREATE_NO_WINDOW`: the child gets no console of its own. Without it a
/// console-subsystem child (llama-server.exe) spawned from a GUI process opens
/// a visible console window (Windows Terminal on Windows 11) for every attempt.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Child-only Vulkan loader filters. Overlay tools (Discord, Steam, RTSS,
/// OBS, Overwolf) register implicit layers globally; the loader injects them
/// into llama-server and `vkCreateDevice` can then fail with ErrorDeviceLost.
/// Disable filters are evaluated before enable filters, so Optimus remains
/// available on dual-GPU laptops. Layer filtering requires Vulkan loader
/// >= 1.3.234 and is silently ignored by older loaders (harmless no-op).
const VK_LOADER_LAYERS_DISABLE_VALUE: &str = "~implicit~";
const VK_LOADER_LAYERS_ENABLE_VALUE: &str = "*optimus*";

pub struct ManagedRuntime {
    /// `None` only in unit tests that inject an inert handle (no OS child).
    child: Option<Child>,
    pub variant: RuntimeVariant,
    pub port: u16,
    pub api_key: String,
    pub model_id: String,
    pub context_size: u64,
    pub fallback_reason: Option<String>,
}

/// Cooperative cancellation for an in-progress `start`. Cloned into the
/// health-wait loop; `RuntimeManager` keeps the current one so `stop_runtime`
/// can abort a load that has not become ready yet.
#[derive(Clone, Default)]
pub struct StartCancel(Arc<AtomicBool>);

impl StartCancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    fn same_as(&self, other: &StartCancel) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Result of a cancellable start.
pub enum StartOutcome {
    Ready(ManagedRuntime),
    /// `StartCancel::cancel` was called before the runtime became healthy. Any
    /// spawned child has already been killed and reaped.
    Cancelled,
}

enum StartFailure {
    Cancelled,
    Error(AppError),
}

impl From<AppError> for StartFailure {
    fn from(e: AppError) -> Self {
        StartFailure::Error(e)
    }
}

pub struct RuntimeManager {
    shared: Arc<Shared>,
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self {
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner::default()),
                progress: tokio::sync::Notify::new(),
                #[cfg(test)]
                waited: tokio::sync::Notify::new(),
                #[cfg(test)]
                waiters: AtomicUsize::new(0),
            }),
        }
    }
}

struct Shared {
    inner: Mutex<Inner>,
    progress: tokio::sync::Notify,
    #[cfg(test)]
    waited: tokio::sync::Notify,
    #[cfg(test)]
    waiters: AtomicUsize,
}

/// `Idle` means no llama-server is running or being killed. A new spawn is
/// legal only from `Idle`. `Faulted` means shutdown exit was not confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Draining,
    Starting,
    Faulted,
}

struct Claim {
    seq: u64,
    model_id: String,
    stop_epoch: u64,
    #[allow(dead_code)]
    model_epoch: u64,
    cancelled: bool,
}

/// Ordering reservation for one `start_runtime` command. Minted at IPC-entry
/// (see `reserve_start_intent`), before slow pre-work, and consumed by
/// `start_with_ticket` after pre-work. Fields stay private: callers hold the
/// ticket opaquely and must not mint a fresh one after inspection.
pub struct StartTicket {
    seq: u64,
    stop_epoch: u64,
    model_epoch: u64,
}

struct SharedFailure {
    model_id: String,
    error: AppError,
    attempt_seq: u64,
}

#[derive(Default)]
struct Inner {
    runtime: Option<ManagedRuntime>,
    state: RuntimeState,
    last_error: Option<AppError>,
    pending_start: Option<StartCancel>,
    pending_model_id: Option<String>,
    /// Bumped by an explicit stop. An internal switch does not bump it.
    stop_epoch: u64,
    model_epoch: HashMap<String, u64>,
    /// Removal calls in flight. The id is unavailable while the count is > 0.
    removing: HashMap<String, u32>,
    phase: Phase,
    /// Monotonic allocator for `StartTicket.seq`. Bumped at reservation
    /// (IPC entry); `latest_*` is only published at state-machine arrival, so
    /// a reservation that never arrives (failed pre-work) cannot overwrite a
    /// newer request.
    next_seq: u64,
    latest_seq: u64,
    latest_model: Option<String>,
    claim: Option<Claim>,
    drain_generation: u64,
    completed_drain_generation: u64,
    drain_confirmed: bool,
    reaper: Option<tokio::task::JoinHandle<()>>,
    shared_start_failure: Option<SharedFailure>,
    #[cfg(test)]
    shutdown_hold: Option<Arc<ShutdownHold>>,
}

#[cfg(test)]
pub(crate) struct ShutdownHold {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

struct ReaperFinish {
    shared: Arc<Shared>,
    generation: u64,
    confirmed: bool,
}

impl Drop for ReaperFinish {
    fn drop(&mut self) {
        if self.confirmed {
            return;
        }
        let mut inner = self.shared.inner.lock().unwrap_or_else(|p| p.into_inner());
        if inner.drain_generation != self.generation || inner.phase != Phase::Draining {
            return;
        }
        inner.phase = Phase::Faulted;
        inner.drain_confirmed = false;
        inner.state = RuntimeState::Error;
        inner.last_error = Some(unconfirmed_shutdown_error());
        inner.claim = None;
        inner.pending_start = None;
        inner.pending_model_id = None;
        inner.reaper = None;
        drop(inner);
        self.shared.progress.notify_waiters();
    }
}

fn unconfirmed_shutdown_error() -> AppError {
    AppError::new(
        ErrorCode::RuntimeFailedToStart,
        Some("llama-server shutdown was not confirmed; another server was not started".into()),
    )
}

/// One in-flight `remove_model`. Drop releases that call's hold on the id.
pub struct Removal {
    shared: Arc<Shared>,
    id: String,
    generation: Option<u64>,
    start_seq: Option<u64>,
}

impl Removal {
    pub async fn wait_for_owned_shutdown(&self) -> Result<(), AppError> {
        if self.generation.is_none() && self.start_seq.is_none() {
            return Ok(());
        }
        loop {
            let waiter = {
                let inner = self.shared.inner.lock().unwrap_or_else(|p| p.into_inner());
                if self
                    .generation
                    .is_some_and(|generation| inner.completed_drain_generation >= generation)
                {
                    return Ok(());
                }
                if inner.phase == Phase::Faulted {
                    return Err(inner
                        .last_error
                        .clone()
                        .unwrap_or_else(unconfirmed_shutdown_error));
                }
                if self
                    .start_seq
                    .is_some_and(|seq| inner.claim.as_ref().is_none_or(|claim| claim.seq != seq))
                {
                    return Ok(());
                }
                self.shared.progress.notified()
            };
            waiter.await;
        }
    }
}

impl Drop for Removal {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(count) = inner.removing.get_mut(&self.id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                inner.removing.remove(&self.id);
            }
        }
        drop(inner);
        self.shared.progress.notify_waiters();
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        RuntimeState::Stopped
    }
}

/// Where the llama-server binaries live for a given variant.
/// Dev builds use `src-tauri/binaries/<variant>/`; production builds use the
/// bundled `runtimes/<variant>/` resource directory. A user override from
/// Settings takes precedence over both.
fn runtime_binary(
    resource_dir: Option<&PathBuf>,
    override_path: Option<&str>,
    variant: RuntimeVariant,
) -> Result<PathBuf, AppError> {
    if let Some(p) = override_path {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::new(
            ErrorCode::RuntimeMissing,
            Some(format!("configured runtime path not found: {p}")),
        ));
    }

    let dir_name = variant_dir_name(variant);

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = resource_dir {
        candidates.push(res.join("runtimes").join(dir_name).join("llama-server.exe"));
    }
    // Dev layout (populated by scripts/packaging/fetch-llama-server.ps1).
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(dir_name)
            .join("llama-server.exe"),
    );

    candidates.into_iter().find(|p| p.is_file()).ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeMissing,
            Some(format!("no {dir_name} llama-server.exe found")),
        )
    })
}

/// Reserve a free loopback port (Decision 12): bind port 0, read the assigned
/// port, release the socket. The release-to-spawn race is handled by bounded
/// retries in `start`.
fn reserve_port() -> Result<u16, AppError> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| {
        AppError::new(
            ErrorCode::RuntimeFailedToStart,
            Some(format!("port reservation: {e}")),
        )
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| {
            AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(format!("port reservation: {e}")),
            )
        })?
        .port();
    drop(listener);
    Ok(port)
}

fn generate_api_key() -> String {
    let mut rng = rand::rng();
    (0..48)
        .map(|_| {
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            CHARSET[rng.random_range(0..CHARSET.len())] as char
        })
        .collect()
}

/// Bounded tail of the child's stderr, kept only while the runtime is
/// starting. The reader task drains the pipe for the child's whole lifetime
/// (a full pipe would block llama-server) but discards everything once
/// `stop_capturing` is called, so prompts and responses never land here.
#[derive(Clone, Default)]
struct StderrTail {
    bytes: Arc<Mutex<Vec<u8>>>,
    capturing: Arc<AtomicBool>,
}

impl StderrTail {
    fn attach(child: &mut Child) -> Self {
        let tail = Self {
            bytes: Arc::new(Mutex::new(Vec::new())),
            capturing: Arc::new(AtomicBool::new(true)),
        };
        if let Some(mut stderr) = child.stderr.take() {
            let sink = tail.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match stderr.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => sink.push(&buf[..n]),
                    }
                }
            });
        }
        tail
    }

    fn push(&self, chunk: &[u8]) {
        if !self.capturing.load(Ordering::Relaxed) {
            return;
        }
        let mut bytes = self.bytes.lock().unwrap_or_else(|p| p.into_inner());
        bytes.extend_from_slice(chunk);
        if bytes.len() > STDERR_TAIL_BYTES {
            let excess = bytes.len() - STDERR_TAIL_BYTES;
            bytes.drain(..excess);
        }
    }

    fn stop_capturing(&self) {
        self.capturing.store(false, Ordering::Relaxed);
        self.bytes.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }

    /// Last few non-empty lines, single-spaced, for an error detail string.
    fn summary(&self) -> Option<String> {
        let bytes = self.bytes.lock().unwrap_or_else(|p| p.into_inner());
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            return None;
        }
        let start = lines.len().saturating_sub(8);
        Some(lines[start..].join(" | "))
    }
}

fn with_stderr(detail: String, tail: &StderrTail) -> String {
    match tail.summary() {
        Some(stderr) => format!("{detail}; stderr: {stderr}"),
        None => detail,
    }
}

async fn wait_healthy(
    port: u16,
    child: &mut Child,
    stderr: &StderrTail,
    cancel: &StartCancel,
) -> Result<(), StartFailure> {
    let url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + HEALTH_TIMEOUT;

    loop {
        if cancel.is_cancelled() {
            return Err(StartFailure::Cancelled);
        }

        // Detect early exit (bind race, model load failure, OOM...).
        if let Some(status) = child.try_wait().map_err(|e| {
            AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(format!("try_wait: {e}")),
            )
        })? {
            // Give the reader task a moment to flush the final stderr lines.
            tokio::time::sleep(Duration::from_millis(50)).await;
            return Err(AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(with_stderr(
                    format!("llama-server exited during startup: {status}"),
                    stderr,
                )),
            )
            .into());
        }

        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(with_stderr("health check timed out".to_string(), stderr)),
            )
            .into());
        }
        tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
    }
}

/// Build the llama-server command line. Kept separate so the flags can be
/// unit-tested without spawning anything.
fn server_args(port: u16, api_key: &str, model_path: &str, ctx_size: u64) -> Vec<String> {
    vec![
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "--api-key".into(),
        api_key.to_string(),
        "--model".into(),
        model_path.to_string(),
        "--ctx-size".into(),
        ctx_size.to_string(),
    ]
}

fn build_command(binary: &PathBuf, args: &[String]) -> Command {
    let mut cmd = Command::new(binary);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("VK_LOADER_LAYERS_DISABLE", VK_LOADER_LAYERS_DISABLE_VALUE)
        .env("VK_LOADER_LAYERS_ENABLE", VK_LOADER_LAYERS_ENABLE_VALUE);
    logging::info(
        "runtime.vulkan_layer_isolation",
        "VK_LOADER_LAYERS_DISABLE=~implicit~ VK_LOADER_LAYERS_ENABLE=*optimus*",
    );
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

async fn kill_and_reap(child: &mut Child) {
    let _ = child.start_kill();
    let _ = child.wait().await;
}

async fn spawn_variant(
    binary: &PathBuf,
    variant: RuntimeVariant,
    model_path: &str,
    model_id: &str,
    ctx_size: u64,
    cancel: &StartCancel,
) -> Result<ManagedRuntime, StartFailure> {
    let mut last_err: Option<AppError> = None;

    for attempt in 1..=SPAWN_ATTEMPTS {
        if cancel.is_cancelled() {
            return Err(StartFailure::Cancelled);
        }

        let port = reserve_port()?;
        let api_key = generate_api_key();

        logging::info(
            "runtime.spawn",
            &format!("variant={variant:?} attempt={attempt} port={port}"),
        );

        let args = server_args(port, &api_key, model_path, ctx_size);
        let mut child = build_command(binary, &args).spawn().map_err(|e| {
            AppError::new(ErrorCode::RuntimeFailedToStart, Some(format!("spawn: {e}")))
        })?;
        let stderr = StderrTail::attach(&mut child);

        // Assign to the kill-on-close Job Object immediately after spawn.
        if let Some(pid) = child.id() {
            if let Err(e) = process::supervise(pid) {
                kill_and_reap(&mut child).await;
                return Err(AppError::new(
                    ErrorCode::RuntimeFailedToStart,
                    Some(format!("job object assignment failed: {e}")),
                )
                .into());
            }
        }

        match wait_healthy(port, &mut child, &stderr, cancel).await {
            Ok(()) => {
                stderr.stop_capturing();
                logging::info("runtime.ready", &format!("variant={variant:?} port={port}"));
                return Ok(ManagedRuntime {
                    child: Some(child),
                    variant,
                    port,
                    api_key,
                    model_id: model_id.to_string(),
                    context_size: ctx_size,
                    fallback_reason: None,
                });
            }
            Err(StartFailure::Cancelled) => {
                kill_and_reap(&mut child).await;
                logging::info(
                    "runtime.start_cancelled",
                    &format!("variant={variant:?} port={port}"),
                );
                return Err(StartFailure::Cancelled);
            }
            Err(StartFailure::Error(e)) => {
                kill_and_reap(&mut child).await;
                logging::error(
                    "runtime.spawn_failed",
                    &format!(
                        "variant={variant:?} attempt={attempt} code={:?} detail={}",
                        e.code,
                        e.detail.as_deref().unwrap_or("")
                    ),
                );
                last_err = Some(e);
            }
        }
    }

    Err(last_err
        .unwrap_or_else(|| {
            AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some("spawn attempts exhausted".into()),
            )
        })
        .into())
}

impl RuntimeManager {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.shared.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// If the managed child has exited after ready, move to `error` with
    /// `BackendUnavailable`. Cheap `try_wait` only; no watchdog thread.
    fn reap_if_dead(inner: &mut Inner) {
        if inner.state != RuntimeState::Ready {
            return;
        }
        if inner.runtime.is_none() {
            Self::mark_unavailable(inner, "runtime handle missing while ready".into());
            return;
        }
        let waited = inner.runtime.as_mut().map(|rt| match rt.child.as_mut() {
            Some(child) => child.try_wait(),
            None => Ok(None),
        });
        let detail = match waited {
            Some(Ok(None)) => return,
            Some(Ok(Some(status))) => format!("llama-server exited: {status}"),
            Some(Err(e)) => format!("try_wait: {e}"),
            None => "runtime handle missing while ready".into(),
        };
        Self::mark_unavailable(inner, detail);
    }

    fn mark_unavailable(inner: &mut Inner, detail: String) {
        logging::error("runtime.unavailable", &detail);
        inner.state = RuntimeState::Error;
        inner.last_error = Some(AppError::new(ErrorCode::BackendUnavailable, Some(detail)));
        inner.runtime = None;
    }

    pub fn status(&self) -> RuntimeStatus {
        let mut inner = self.lock();
        Self::reap_if_dead(&mut inner);
        let rt = inner.runtime.as_ref();
        let engine_active = rt.is_some() || inner.state == RuntimeState::Starting;
        RuntimeStatus {
            state: inner.state,
            engine_label: engine_active.then(|| ENGINE_LABEL.to_string()),
            variant: rt.map(|r| r.variant),
            accelerator_label: rt.map(|r| accelerator_label(r.variant).to_string()),
            fallback_reason: rt.and_then(|r| r.fallback_reason.clone()),
            model_id: rt
                .map(|r| r.model_id.clone())
                .or_else(|| inner.pending_model_id.clone()),
            port: rt.map(|r| r.port),
            context_size: rt.map(|r| r.context_size),
            last_error: inner.last_error.clone(),
        }
    }

    pub fn endpoint(&self) -> Result<crate::types::ChatEndpoint, AppError> {
        let mut inner = self.lock();
        Self::reap_if_dead(&mut inner);
        let rt = inner.runtime.as_ref().ok_or_else(|| {
            inner.last_error.clone().unwrap_or_else(|| {
                AppError::new(
                    ErrorCode::BackendUnavailable,
                    Some("no runtime running".into()),
                )
            })
        })?;
        Ok(crate::types::ChatEndpoint {
            base_url: format!("http://127.0.0.1:{}", rt.port),
            api_key: rt.api_key.clone(),
            context_chars_budget: context_chars_budget(rt.context_size),
        })
    }

    /// Enter `starting` and hand back the cancellation token for this attempt.
    /// Any older in-flight start is cancelled so only the newest one can win.
    pub fn begin_start(&self, model_id: impl Into<String>) -> StartCancel {
        let mut inner = self.lock();
        if let Some(previous) = inner.pending_start.take() {
            previous.cancel();
        }
        let token = StartCancel::default();
        inner.pending_start = Some(token.clone());
        inner.pending_model_id = Some(model_id.into());
        inner.state = RuntimeState::Starting;
        inner.last_error = None;
        token
    }

    /// Record a start failure, unless a newer start has superseded `token`.
    pub fn finish_start_error(&self, token: &StartCancel, err: AppError) {
        let mut inner = self.lock();
        if !Self::is_current(&inner, token) {
            return;
        }
        inner.pending_start = None;
        inner.pending_model_id = None;
        inner.state = RuntimeState::Error;
        inner.last_error = Some(err);
        inner.runtime = None;
    }

    /// Install a healthy runtime. Returns the runtime back to the caller (who
    /// must stop it) when `token` was cancelled or superseded meanwhile.
    pub fn finish_start_ready(
        &self,
        token: &StartCancel,
        runtime: ManagedRuntime,
    ) -> Result<(), ManagedRuntime> {
        let mut inner = self.lock();
        if !Self::is_current(&inner, token) || token.is_cancelled() {
            return Err(runtime);
        }
        inner.pending_start = None;
        inner.pending_model_id = None;
        inner.state = RuntimeState::Ready;
        inner.last_error = None;
        inner.runtime = Some(runtime);
        Ok(())
    }

    fn is_current(inner: &Inner, token: &StartCancel) -> bool {
        inner
            .pending_start
            .as_ref()
            .is_some_and(|current| current.same_as(token))
    }

    /// Cancel an in-flight start and hand back a ready child. Does not bump
    /// `stop_epoch` when it only takes a ready runtime; that would abort the
    /// switch that called it. Cancelling a pending start does bump, so a
    /// queued caller observes the cancel.
    pub fn take_runtime(&self) -> Option<ManagedRuntime> {
        let mut inner = self.lock();
        let cancelled_inflight = if let Some(pending) = inner.pending_start.take() {
            pending.cancel();
            true
        } else {
            false
        };
        if cancelled_inflight {
            inner.stop_epoch = inner.stop_epoch.wrapping_add(1);
        }
        if let Some(claim) = inner.claim.as_mut() {
            claim.cancelled = true;
        }
        inner.pending_model_id = None;
        inner.state = RuntimeState::Stopped;
        inner.last_error = None;
        let runtime = inner.runtime.take();
        drop(inner);
        self.shared.progress.notify_waiters();
        runtime
    }

    fn begin_start_locked(inner: &mut Inner, model_id: &str) -> StartCancel {
        if let Some(previous) = inner.pending_start.take() {
            previous.cancel();
        }
        let token = StartCancel::default();
        inner.pending_start = Some(token.clone());
        inner.pending_model_id = Some(model_id.to_string());
        inner.state = RuntimeState::Starting;
        inner.last_error = None;
        token
    }

    fn begin_drain(shared: &Arc<Shared>, inner: &mut Inner, rt: ManagedRuntime) {
        inner.drain_generation = inner.drain_generation.wrapping_add(1);
        inner.drain_confirmed = false;
        inner.phase = Phase::Draining;
        inner.runtime = None;
        let generation = inner.drain_generation;
        let shared = shared.clone();
        let handle = tokio::spawn(async move {
            let mut finish = ReaperFinish {
                shared: shared.clone(),
                generation,
                confirmed: false,
            };
            #[cfg(test)]
            {
                let hold = {
                    let mut guard = shared.inner.lock().unwrap_or_else(|p| p.into_inner());
                    guard.shutdown_hold.take()
                };
                if let Some(hold) = hold {
                    hold.entered.notify_waiters();
                    hold.release.notified().await;
                }
            }
            stop(rt).await;
            finish.confirmed = true;
            {
                let mut guard = shared.inner.lock().unwrap_or_else(|p| p.into_inner());
                if guard.drain_generation == generation && guard.phase == Phase::Draining {
                    guard.drain_confirmed = true;
                    guard.completed_drain_generation = generation;
                    let stale = guard.claim.as_ref().is_none_or(|claim| {
                        claim.cancelled || claim.stop_epoch != guard.stop_epoch
                    });
                    if stale {
                        guard.phase = Phase::Idle;
                        guard.claim = None;
                        guard.pending_start = None;
                        guard.pending_model_id = None;
                        guard.state = RuntimeState::Stopped;
                        guard.last_error = None;
                    }
                    guard.reaper = None;
                }
            }
            shared.progress.notify_waiters();
        });
        inner.reaper = Some(handle);
    }

    /// Explicit unload. Always bumps `stop_epoch`. If a reaper already owns
    /// the child, this waits for confirmed exit. A newer Use may supersede Stop.
    pub async fn stop_and_wait(&self) -> Result<RuntimeStatus, AppError> {
        let mut stopped_epoch = None;
        loop {
            let waiter = {
                let mut inner = self.lock();
                if stopped_epoch.is_none() {
                    inner.stop_epoch = inner.stop_epoch.wrapping_add(1);
                    if let Some(token) = inner.pending_start.take() {
                        token.cancel();
                    }
                    if let Some(claim) = inner.claim.as_mut() {
                        claim.cancelled = true;
                    }
                    inner.pending_model_id = None;
                    if let Some(rt) = inner.runtime.take() {
                        Self::begin_drain(&self.shared, &mut inner, rt);
                    }
                    stopped_epoch = Some(inner.stop_epoch);
                    self.shared.progress.notify_waiters();
                }
                if inner.phase == Phase::Faulted {
                    return Err(inner
                        .last_error
                        .clone()
                        .unwrap_or_else(unconfirmed_shutdown_error));
                }
                let newer_start = inner.claim.as_ref().is_some_and(|claim| {
                    claim.stop_epoch >= stopped_epoch.expect("stop reserved") && !claim.cancelled
                });
                if inner.phase == Phase::Idle || newer_start {
                    if inner.phase == Phase::Idle && inner.runtime.is_none() {
                        inner.state = RuntimeState::Stopped;
                        inner.last_error = None;
                    }
                    drop(inner);
                    return Ok(self.status());
                }
                self.shared.progress.notified()
            };
            waiter.await;
        }
    }

    /// Mark `id` unavailable for the whole removal, including the gap before
    /// the registry row is deleted. The returned guard clears that hold.
    pub fn begin_removal(&self, id: &str) -> Removal {
        let mut inner = self.lock();
        let start_seq = inner
            .claim
            .as_ref()
            .filter(|claim| claim.model_id == id)
            .map(|claim| claim.seq);
        *inner.removing.entry(id.to_string()).or_insert(0) += 1;
        *inner.model_epoch.entry(id.to_string()).or_insert(0) += 1;
        if let Some(claim) = inner.claim.as_mut() {
            if claim.model_id == id {
                claim.cancelled = true;
            }
        }
        if inner.pending_model_id.as_deref() == Some(id) {
            if let Some(token) = inner.pending_start.take() {
                token.cancel();
            }
            inner.pending_model_id = None;
        }
        let generation = if inner.runtime.as_ref().is_some_and(|rt| rt.model_id == id) {
            let rt = inner.runtime.take().expect("checked");
            Self::begin_drain(&self.shared, &mut inner, rt);
            Some(inner.drain_generation)
        } else {
            None
        };
        drop(inner);
        self.shared.progress.notify_waiters();
        Removal {
            shared: self.shared.clone(),
            id: id.to_string(),
            generation,
            start_seq,
        }
    }

    #[cfg(test)]
    pub fn start_is_waiting(&self) -> tokio::sync::futures::Notified<'_> {
        self.shared.waited.notified()
    }

    #[cfg(test)]
    pub fn waiter_count(&self) -> usize {
        self.shared.waiters.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn arm_shutdown_hold(&self) -> Arc<ShutdownHold> {
        let hold = Arc::new(ShutdownHold {
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        });
        self.lock().shutdown_hold = Some(hold.clone());
        hold
    }

    #[cfg(test)]
    pub fn abort_reaper(&self) {
        if let Some(handle) = self.lock().reaper.take() {
            handle.abort();
        }
    }

    #[cfg(test)]
    pub fn subscribe(&self) -> tokio::sync::futures::Notified<'_> {
        self.shared.progress.notified()
    }

    /// Reserve start ordering at IPC-command entry, before slow pre-work
    /// (`storage.get_model`, `gguf::inspect`). Brief lock, never held across
    /// file I/O, and publishes nothing to `latest_*`/phase/claim — so dropping
    /// the ticket on pre-work failure abandons the intent without touching a
    /// newer request. A `stop_and_wait` processed after this reservation bumps
    /// `stop_epoch` past `ticket.stop_epoch`, which invalidates the ticket.
    pub fn reserve_start_intent(&self, model_id: &str) -> StartTicket {
        let mut inner = self.lock();
        inner.next_seq = inner.next_seq.wrapping_add(1);
        StartTicket {
            seq: inner.next_seq,
            stop_epoch: inner.stop_epoch,
            model_epoch: inner.model_epoch.get(model_id).copied().unwrap_or(0),
        }
    }

    /// Start a runtime, or join one already ready for `model_id`.
    ///
    /// The phase flag is the single-flight queue. The mutex is not held across
    /// process shutdown or spawn. A new spawn happens only from `Idle` after
    /// any previous llama-server is confirmed gone.
    pub async fn start_serialized<F, Fut>(
        &self,
        model_id: &str,
        spawn: F,
    ) -> Result<RuntimeStatus, AppError>
    where
        F: FnOnce(StartCancel) -> Fut,
        Fut: Future<Output = Result<StartOutcome, AppError>>,
    {
        let ticket = self.reserve_start_intent(model_id);
        self.start_with_ticket(model_id, ticket, spawn).await
    }

    /// Drive a previously reserved intent through the lifecycle state machine.
    /// Callers must pass the ticket minted before pre-work; minting a fresh
    /// ticket here would forget a Stop that landed during inspection.
    pub async fn start_with_ticket<F, Fut>(
        &self,
        model_id: &str,
        ticket: StartTicket,
        spawn: F,
    ) -> Result<RuntimeStatus, AppError>
    where
        F: FnOnce(StartCancel) -> Fut,
        Fut: Future<Output = Result<StartOutcome, AppError>>,
    {
        {
            let inner = self.lock();
            if inner.phase == Phase::Faulted {
                return Err(inner
                    .last_error
                    .clone()
                    .unwrap_or_else(unconfirmed_shutdown_error));
            }
        }
        // Publish arrival: the newest reserved ticket wins even if an older
        // one arrives later (slow pre-work). Older arrivals keep their ticket
        // and are superseded below when they target a different model.
        {
            let mut inner = self.lock();
            if ticket.seq > inner.latest_seq {
                inner.latest_seq = ticket.seq;
                inner.latest_model = Some(model_id.to_string());
            }
        }

        let mut ticket_waited = false;

        let token = loop {
            enum Step {
                Wait,
                Spawn(StartCancel),
            }
            let step = {
                let mut inner = self.lock();
                if inner.phase == Phase::Idle {
                    Self::reap_if_dead(&mut inner);
                }
                if inner.phase == Phase::Faulted {
                    return Err(inner
                        .last_error
                        .clone()
                        .unwrap_or_else(unconfirmed_shutdown_error));
                }
                if inner.stop_epoch != ticket.stop_epoch {
                    logging::info("runtime.start_skipped", "cancelled while queued");
                    drop(inner);
                    return Ok(self.status());
                }
                let model_epoch_now = inner.model_epoch.get(model_id).copied().unwrap_or(0);
                let removing = inner.removing.get(model_id).copied().unwrap_or(0) > 0;
                if removing || model_epoch_now != ticket.model_epoch {
                    return Err(AppError::new(
                        ErrorCode::ModelFileMissing,
                        Some("model is not available to start".into()),
                    ));
                }
                if self.ready_locked(&inner, model_id) {
                    logging::info("runtime.start_reuse", "already ready after wait");
                    drop(inner);
                    return Ok(self.status());
                }
                let newest_is_other = inner.latest_seq != ticket.seq
                    && inner.latest_model.as_deref() != Some(model_id);
                if newest_is_other
                    && inner
                        .claim
                        .as_ref()
                        .is_none_or(|claim| claim.seq != ticket.seq)
                {
                    logging::info("runtime.start_skipped", "superseded");
                    drop(inner);
                    return Ok(self.status());
                }
                if ticket_waited {
                    if let Some(failure) = inner.shared_start_failure.as_ref() {
                        if failure.model_id == model_id && failure.attempt_seq < ticket.seq {
                            logging::info(
                                "runtime.start_joined_failure",
                                "sharing in-flight failure",
                            );
                            return Err(failure.error.clone());
                        }
                    }
                }
                let owns_claim = inner
                    .claim
                    .as_ref()
                    .is_some_and(|claim| claim.seq == ticket.seq && !claim.cancelled);
                if inner.phase == Phase::Draining && inner.drain_confirmed && owns_claim {
                    inner.phase = Phase::Starting;
                    Step::Spawn(Self::begin_start_locked(&mut inner, model_id))
                } else if inner.phase == Phase::Idle && !newest_is_other {
                    if !ticket_waited {
                        inner.shared_start_failure = None;
                    }
                    inner.claim = Some(Claim {
                        seq: ticket.seq,
                        model_id: model_id.to_string(),
                        stop_epoch: ticket.stop_epoch,
                        model_epoch: ticket.model_epoch,
                        cancelled: false,
                    });
                    if let Some(previous) = inner.runtime.take() {
                        Self::begin_drain(&self.shared, &mut inner, previous);
                        self.shared.progress.notify_waiters();
                        Step::Wait
                    } else {
                        inner.phase = Phase::Starting;
                        Step::Spawn(Self::begin_start_locked(&mut inner, model_id))
                    }
                } else {
                    Step::Wait
                }
            };
            match step {
                Step::Wait => {
                    let waiter = {
                        let inner = self.lock();
                        let owns_claim = inner
                            .claim
                            .as_ref()
                            .is_some_and(|claim| claim.seq == ticket.seq && !claim.cancelled);
                        let ready_to_recheck = inner.phase == Phase::Idle
                            || inner.phase == Phase::Faulted
                            || (inner.phase == Phase::Draining
                                && inner.drain_confirmed
                                && owns_claim);
                        if ready_to_recheck {
                            None
                        } else {
                            Some(self.shared.progress.notified())
                        }
                    };
                    if let Some(waiter) = waiter {
                        ticket_waited = true;
                        #[cfg(test)]
                        {
                            self.shared.waiters.fetch_add(1, Ordering::SeqCst);
                            self.shared.waited.notify_waiters();
                        }
                        waiter.await;
                    }
                }
                Step::Spawn(token) => break token,
            }
        };

        let outcome = spawn(token.clone()).await;
        self.settle_spawn(model_id, &ticket, &token, outcome).await
    }

    fn ready_locked(&self, inner: &Inner, model_id: &str) -> bool {
        inner.state == RuntimeState::Ready
            && inner
                .runtime
                .as_ref()
                .is_some_and(|rt| rt.model_id == model_id)
    }

    async fn settle_spawn(
        &self,
        model_id: &str,
        ticket: &StartTicket,
        token: &StartCancel,
        outcome: Result<StartOutcome, AppError>,
    ) -> Result<RuntimeStatus, AppError> {
        enum Settled {
            Install(ManagedRuntime),
            Discard(ManagedRuntime),
            Cancelled,
            Failed(AppError),
        }
        let settled = {
            let mut inner = self.lock();
            let stop_moved = inner.stop_epoch != ticket.stop_epoch;
            let model_epoch_now = inner.model_epoch.get(model_id).copied().unwrap_or(0);
            let removing = inner.removing.get(model_id).copied().unwrap_or(0) > 0;
            let newer_removed = inner
                .latest_model
                .as_deref()
                .is_some_and(|id| inner.removing.get(id).copied().unwrap_or(0) > 0);
            let superseded = inner.latest_model.as_deref() != Some(model_id) && !newer_removed;
            let rejected = stop_moved
                || removing
                || model_epoch_now != ticket.model_epoch
                || superseded
                || token.is_cancelled();
            match outcome {
                Ok(StartOutcome::Ready(rt)) if rejected => Settled::Discard(rt),
                Ok(StartOutcome::Ready(rt)) => Settled::Install(rt),
                Ok(StartOutcome::Cancelled) | Err(_) if rejected || token.is_cancelled() => {
                    Settled::Cancelled
                }
                Err(err) => {
                    logging::error("runtime.start_failed", &format!("code={:?}", err.code));
                    inner.shared_start_failure = Some(SharedFailure {
                        model_id: model_id.to_string(),
                        error: err.clone(),
                        attempt_seq: ticket.seq,
                    });
                    inner.phase = Phase::Idle;
                    inner.claim = None;
                    inner.pending_start = None;
                    inner.pending_model_id = None;
                    inner.state = RuntimeState::Error;
                    inner.last_error = Some(err.clone());
                    Settled::Failed(err)
                }
                Ok(StartOutcome::Cancelled) => Settled::Cancelled,
            }
        };
        match settled {
            Settled::Install(rt) => {
                if let Err(stale) = self.finish_start_ready(token, rt) {
                    self.discard_runtime(stale).await?;
                } else {
                    let mut inner = self.lock();
                    if inner.phase == Phase::Starting {
                        inner.phase = Phase::Idle;
                        inner.claim = None;
                    }
                }
                self.shared.progress.notify_waiters();
                Ok(self.status())
            }
            Settled::Discard(rt) => {
                self.discard_runtime(rt).await?;
                self.shared.progress.notify_waiters();
                let inner = self.lock();
                if inner.phase == Phase::Faulted {
                    return Err(inner
                        .last_error
                        .clone()
                        .unwrap_or_else(unconfirmed_shutdown_error));
                }
                Ok(self.status())
            }
            Settled::Cancelled => {
                logging::info("runtime.start_cancelled", "by user");
                let mut inner = self.lock();
                if inner.phase == Phase::Starting {
                    inner.phase = Phase::Idle;
                    inner.claim = None;
                    inner.pending_start = None;
                    inner.pending_model_id = None;
                    if inner.state != RuntimeState::Ready {
                        inner.state = RuntimeState::Stopped;
                    }
                }
                drop(inner);
                self.shared.progress.notify_waiters();
                Ok(self.status())
            }
            Settled::Failed(err) => {
                self.shared.progress.notify_waiters();
                Err(err)
            }
        }
    }

    async fn discard_runtime(&self, rt: ManagedRuntime) -> Result<(), AppError> {
        let generation = {
            let mut inner = self.lock();
            if let Some(claim) = inner.claim.as_mut() {
                claim.cancelled = true;
            }
            Self::begin_drain(&self.shared, &mut inner, rt);
            inner.drain_generation
        };
        self.shared.progress.notify_waiters();
        loop {
            let waiter = {
                let inner = self.lock();
                if inner.completed_drain_generation >= generation {
                    return Ok(());
                }
                if inner.phase == Phase::Faulted && inner.drain_generation == generation {
                    return Err(inner
                        .last_error
                        .clone()
                        .unwrap_or_else(unconfirmed_shutdown_error));
                }
                self.shared.progress.notified()
            };
            waiter.await;
        }
    }
}

pub fn variant_dir_name(variant: RuntimeVariant) -> &'static str {
    match variant {
        RuntimeVariant::Cuda => "cuda",
        RuntimeVariant::Vulkan => "vulkan",
        RuntimeVariant::Cpu => "cpu",
    }
}

pub fn accelerator_label(variant: RuntimeVariant) -> &'static str {
    match variant {
        RuntimeVariant::Cuda => "GPU (CUDA)",
        RuntimeVariant::Vulkan => "GPU (Vulkan)",
        RuntimeVariant::Cpu => "CPU",
    }
}

/// Selection order for managed llama-server.
///
/// When a CUDA binary is on disk (`binaries/cuda` / `runtimes/cuda`), try
/// CUDA then Vulkan then CPU. Otherwise keep today's Vulkan then CPU order.
/// A recorded CPU preference starts on CPU; clearing it (Diagnostics
/// "Try GPU acceleration again") retries the higher-priority GPU variants.
/// NVIDIA device detection is not wired yet — binary presence is the only gate.
pub fn variant_selection_order(
    preferred: Option<RuntimeVariant>,
    cuda_binary_present: bool,
) -> Vec<RuntimeVariant> {
    let mut gpu = if cuda_binary_present {
        vec![RuntimeVariant::Cuda, RuntimeVariant::Vulkan]
    } else {
        vec![RuntimeVariant::Vulkan]
    };

    match preferred {
        Some(RuntimeVariant::Cpu) => {
            let mut order = vec![RuntimeVariant::Cpu];
            order.append(&mut gpu);
            order
        }
        Some(RuntimeVariant::Cuda) | Some(RuntimeVariant::Vulkan) | None => {
            gpu.push(RuntimeVariant::Cpu);
            gpu
        }
    }
}

pub fn context_chars_budget(ctx_size: u64) -> u64 {
    let usable = ctx_size - ctx_size / RESPONSE_HEADROOM_FRACTION;
    usable * CHARS_PER_TOKEN
}

/// Start llama-server for the given model: CUDA (if present) then Vulkan,
/// then CPU (Decision 9 / Phase 6 prep). The working variant and any
/// CPU-path fallback reason are recorded.
pub async fn start(
    resource_dir: Option<PathBuf>,
    override_path: Option<String>,
    preferred: Option<RuntimeVariant>,
    model_path: String,
    model_id: String,
    trained_context_length: Option<u64>,
) -> Result<ManagedRuntime, AppError> {
    match start_cancellable(
        resource_dir,
        override_path,
        preferred,
        model_path,
        model_id,
        trained_context_length,
        &StartCancel::default(),
    )
    .await?
    {
        StartOutcome::Ready(rt) => Ok(rt),
        StartOutcome::Cancelled => Err(AppError::new(
            ErrorCode::RuntimeFailedToStart,
            Some("start cancelled".into()),
        )),
    }
}

/// `start`, but abortable through `cancel` while a variant is still loading.
pub async fn start_cancellable(
    resource_dir: Option<PathBuf>,
    override_path: Option<String>,
    preferred: Option<RuntimeVariant>,
    model_path: String,
    model_id: String,
    trained_context_length: Option<u64>,
    cancel: &StartCancel,
) -> Result<StartOutcome, AppError> {
    let ctx_size = match trained_context_length {
        Some(trained) if trained > 0 => DEFAULT_CTX_SIZE.min(trained),
        _ => DEFAULT_CTX_SIZE,
    };

    // Binary presence only — do not treat a Settings override path as CUDA.
    let cuda_binary_present =
        runtime_binary(resource_dir.as_ref(), None, RuntimeVariant::Cuda).is_ok();
    let order = variant_selection_order(preferred, cuda_binary_present);

    let mut first_failure: Option<(RuntimeVariant, AppError)> = None;

    for variant in order {
        let binary = match runtime_binary(resource_dir.as_ref(), override_path.as_deref(), variant)
        {
            Ok(b) => b,
            Err(e) => {
                if first_failure.is_none() {
                    first_failure = Some((variant, e));
                }
                continue;
            }
        };

        match spawn_variant(&binary, variant, &model_path, &model_id, ctx_size, cancel).await {
            Ok(mut rt) => {
                rt.fallback_reason = fallback_reason(variant, first_failure.as_ref());
                return Ok(StartOutcome::Ready(rt));
            }
            Err(StartFailure::Cancelled) => return Ok(StartOutcome::Cancelled),
            Err(StartFailure::Error(e)) => {
                if first_failure.is_none() {
                    first_failure = Some((variant, e));
                }
            }
        }
    }

    Err(first_failure
        .map(|(_, e)| e)
        .unwrap_or_else(|| AppError::new(ErrorCode::RuntimeMissing, None)))
}

/// Why the runtime is not on a GPU path, if it is not. Always populated for
/// a CPU runtime so the UI can say "running on CPU" honestly -- including when
/// GPU variants were never attempted because CPU was the recorded working
/// variant. GPU-to-GPU fallback (CUDA -> Vulkan) does not set this; that
/// detail is not a CPU fallback.
fn fallback_reason(
    variant: RuntimeVariant,
    first_failure: Option<&(RuntimeVariant, AppError)>,
) -> Option<String> {
    match variant {
        RuntimeVariant::Cpu => match first_failure {
            Some((failed_variant, err)) => Some(format!(
                "{failed_variant:?} unavailable: {}",
                err.detail.clone().unwrap_or_default()
            )),
            None => Some(
                "Vulkan skipped: CPU was recorded as the working runtime on an earlier launch. \
                 Use \"Try GPU acceleration again\" in Advanced Diagnostics to retry \
                 higher-priority GPU variants."
                    .to_string(),
            ),
        },
        RuntimeVariant::Cuda | RuntimeVariant::Vulkan => None,
    }
}

/// Stop a managed runtime gracefully-ish: kill the child and reap it.
pub async fn stop(mut rt: ManagedRuntime) {
    logging::info("runtime.stop", &format!("port={}", rt.port));
    if let Some(ref mut child) = rt.child {
        kill_and_reap(child).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn cuda_variant_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&RuntimeVariant::Cuda).unwrap(),
            "\"cuda\""
        );
        assert_eq!(
            serde_json::from_str::<RuntimeVariant>("\"cuda\"").unwrap(),
            RuntimeVariant::Cuda
        );
    }

    #[test]
    fn build_command_isolates_vulkan_implicit_layers() {
        let cmd = build_command(&PathBuf::from("llama-server"), &[]);
        let envs: Vec<(String, Option<String>)> = cmd
            .as_std()
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        assert!(
            envs.iter().any(|(k, v)| {
                k == "VK_LOADER_LAYERS_DISABLE" && v.as_deref() == Some("~implicit~")
            }),
            "expected VK_LOADER_LAYERS_DISABLE=~implicit~, got {envs:?}"
        );
        assert!(
            envs.iter().any(|(k, v)| {
                k == "VK_LOADER_LAYERS_ENABLE" && v.as_deref() == Some("*optimus*")
            }),
            "expected VK_LOADER_LAYERS_ENABLE=*optimus*, got {envs:?}"
        );
    }

    #[test]
    fn server_args_bind_loopback_only_with_session_key() {
        let args = server_args(4242, "secret", r"C:\models\m.gguf", 8192);
        let joined = args.join(" ");
        assert!(joined.contains("--host 127.0.0.1"));
        assert!(joined.contains("--port 4242"));
        assert!(joined.contains("--api-key secret"));
        assert!(joined.contains(r"--model C:\models\m.gguf"));
        assert!(joined.contains("--ctx-size 8192"));
    }

    #[test]
    fn stderr_tail_is_bounded_and_stops_after_ready() {
        let tail = StderrTail::default();
        tail.capturing.store(true, Ordering::Relaxed);
        tail.push(b"ggml_vulkan: No devices found\n");
        tail.push(&vec![b'x'; STDERR_TAIL_BYTES * 2]);
        assert!(tail.bytes.lock().unwrap().len() <= STDERR_TAIL_BYTES);

        tail.stop_capturing();
        tail.push(b"request: POST /v1/chat/completions\n");
        assert!(
            tail.summary().is_none(),
            "nothing may be retained after ready"
        );
    }

    #[test]
    fn stderr_summary_keeps_last_lines() {
        let tail = StderrTail::default();
        tail.capturing.store(true, Ordering::Relaxed);
        tail.push(b"line one\n\nline two\nerror: failed to load model\n");
        let summary = tail.summary().unwrap();
        assert!(summary.ends_with("error: failed to load model"));
        assert!(summary.contains("line one | line two"));
    }

    #[test]
    fn cpu_runtime_always_carries_a_fallback_reason() {
        let failure = (
            RuntimeVariant::Vulkan,
            AppError::new(ErrorCode::RuntimeFailedToStart, Some("no device".into())),
        );
        let after_failure = fallback_reason(RuntimeVariant::Cpu, Some(&failure)).unwrap();
        assert!(after_failure.contains("Vulkan unavailable: no device"));

        let skipped = fallback_reason(RuntimeVariant::Cpu, None).unwrap();
        assert!(skipped.contains("Vulkan skipped"));
        assert!(skipped.contains("Try GPU acceleration again"));

        assert!(fallback_reason(RuntimeVariant::Vulkan, None).is_none());
        assert!(fallback_reason(RuntimeVariant::Cuda, None).is_none());
        // CUDA -> Vulkan is still a GPU path; do not surface a CPU fallback notice.
        let cuda_failed = (
            RuntimeVariant::Cuda,
            AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some("no nvcc device".into()),
            ),
        );
        assert!(fallback_reason(RuntimeVariant::Vulkan, Some(&cuda_failed)).is_none());
    }

    #[test]
    fn variant_selection_order_is_cuda_then_vulkan_then_cpu_when_cuda_binary_exists() {
        assert_eq!(
            variant_selection_order(None, true),
            vec![
                RuntimeVariant::Cuda,
                RuntimeVariant::Vulkan,
                RuntimeVariant::Cpu
            ]
        );
        assert_eq!(
            variant_selection_order(Some(RuntimeVariant::Vulkan), true),
            vec![
                RuntimeVariant::Cuda,
                RuntimeVariant::Vulkan,
                RuntimeVariant::Cpu
            ]
        );
        assert_eq!(
            variant_selection_order(Some(RuntimeVariant::Cuda), true),
            vec![
                RuntimeVariant::Cuda,
                RuntimeVariant::Vulkan,
                RuntimeVariant::Cpu
            ]
        );
    }

    #[test]
    fn variant_selection_order_stays_vulkan_then_cpu_without_cuda_binary() {
        assert_eq!(
            variant_selection_order(None, false),
            vec![RuntimeVariant::Vulkan, RuntimeVariant::Cpu]
        );
        assert_eq!(
            variant_selection_order(Some(RuntimeVariant::Cuda), false),
            vec![RuntimeVariant::Vulkan, RuntimeVariant::Cpu]
        );
    }

    #[test]
    fn clearing_cpu_preference_retries_higher_priority_gpus() {
        assert_eq!(
            variant_selection_order(Some(RuntimeVariant::Cpu), false),
            vec![RuntimeVariant::Cpu, RuntimeVariant::Vulkan]
        );
        assert_eq!(
            variant_selection_order(Some(RuntimeVariant::Cpu), true),
            vec![
                RuntimeVariant::Cpu,
                RuntimeVariant::Cuda,
                RuntimeVariant::Vulkan
            ]
        );
        // Diagnostics "Try GPU acceleration again" sets preferred to None.
        assert_eq!(
            variant_selection_order(None, true).first().copied(),
            Some(RuntimeVariant::Cuda)
        );
        assert_eq!(
            variant_selection_order(None, false).first().copied(),
            Some(RuntimeVariant::Vulkan)
        );
    }

    #[test]
    fn accelerator_label_names_cuda_for_diagnostics() {
        assert_eq!(accelerator_label(RuntimeVariant::Cuda), "GPU (CUDA)");
        assert_eq!(accelerator_label(RuntimeVariant::Vulkan), "GPU (Vulkan)");
        assert_eq!(accelerator_label(RuntimeVariant::Cpu), "CPU");
    }

    #[test]
    fn take_runtime_cancels_pending_start() {
        let manager = RuntimeManager::default();
        let token = manager.begin_start("pending-model");
        assert_eq!(manager.status().state, RuntimeState::Starting);
        assert_eq!(manager.status().engine_label.as_deref(), Some(ENGINE_LABEL));
        assert_eq!(manager.status().model_id.as_deref(), Some("pending-model"));

        assert!(manager.take_runtime().is_none());
        assert!(token.is_cancelled());
        let status = manager.status();
        assert_eq!(status.state, RuntimeState::Stopped);
        assert!(status.engine_label.is_none());
        assert!(status.model_id.is_none());
        assert!(status.last_error.is_none());
    }

    #[test]
    fn starting_status_includes_pending_model_id() {
        let manager = RuntimeManager::default();
        let _token = manager.begin_start("model-a");
        let status = manager.status();
        assert_eq!(status.state, RuntimeState::Starting);
        assert_eq!(status.model_id.as_deref(), Some("model-a"));
    }

    #[test]
    fn newer_start_supersedes_older_one() {
        let manager = RuntimeManager::default();
        let first = manager.begin_start("model-a");
        let second = manager.begin_start("model-b");
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());

        // A stale failure must not clobber the newer attempt's state.
        manager.finish_start_error(&first, AppError::new(ErrorCode::RuntimeFailedToStart, None));
        assert_eq!(manager.status().state, RuntimeState::Starting);

        manager.finish_start_error(
            &second,
            AppError::new(ErrorCode::RuntimeFailedToStart, None),
        );
        assert_eq!(manager.status().state, RuntimeState::Error);
    }

    impl ManagedRuntime {
        fn inert(model_id: &str) -> Self {
            use std::sync::atomic::{AtomicU16, Ordering as AtomicOrdering};
            static NEXT_PORT: AtomicU16 = AtomicU16::new(40_000);
            Self {
                child: None,
                variant: RuntimeVariant::Cpu,
                port: NEXT_PORT.fetch_add(1, AtomicOrdering::SeqCst),
                api_key: "test-key".into(),
                model_id: model_id.to_string(),
                context_size: 8192,
                fallback_reason: None,
            }
        }
    }

    #[tokio::test]
    async fn concurrent_starts_for_same_model_spawn_one_child() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        let spawns = Arc::new(AtomicU32::new(0));

        let launch = |manager: Arc<RuntimeManager>, spawns: Arc<AtomicU32>| {
            tokio::spawn(async move {
                manager
                    .start_serialized("model-a", |_token| {
                        let spawns = spawns.clone();
                        async move {
                            spawns.fetch_add(1, AtomicOrdering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(80)).await;
                            Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                        }
                    })
                    .await
            })
        };

        let first = launch(manager.clone(), spawns.clone());
        let second = launch(manager.clone(), spawns.clone());
        let first_status = first.await.unwrap().unwrap();
        let second_status = second.await.unwrap().unwrap();

        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(first_status.state, RuntimeState::Ready);
        assert_eq!(second_status.state, RuntimeState::Ready);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
        assert_eq!(manager.status().state, RuntimeState::Ready);
    }

    #[tokio::test]
    async fn start_when_already_ready_for_same_model_is_noop() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = RuntimeManager::default();
        let spawns = Arc::new(AtomicU32::new(0));

        let spawn_once = |spawns: Arc<AtomicU32>| {
            move |_token: StartCancel| {
                let spawns = spawns.clone();
                async move {
                    spawns.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                }
            }
        };

        manager
            .start_serialized("model-a", spawn_once(spawns.clone()))
            .await
            .unwrap();
        let port_after_first = manager.status().port;
        manager
            .start_serialized("model-a", spawn_once(spawns.clone()))
            .await
            .unwrap();

        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(manager.status().port, port_after_first);
        assert_eq!(manager.status().state, RuntimeState::Ready);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
    }

    #[tokio::test]
    async fn queued_start_behind_a_cancelled_one_does_not_spawn() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        let spawns = Arc::new(AtomicU32::new(0));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());

        let first_manager = manager.clone();
        let first_spawns = spawns.clone();
        let first_entered = entered.clone();
        let first_release = release.clone();
        let entered_wait = entered.notified();
        let first = tokio::spawn(async move {
            first_manager
                .start_serialized("model-a", |token| {
                    let spawns = first_spawns.clone();
                    let entered = first_entered.clone();
                    let release = first_release.clone();
                    async move {
                        spawns.fetch_add(1, AtomicOrdering::SeqCst);
                        entered.notify_waiters();
                        release.notified().await;
                        if token.is_cancelled() {
                            Ok(StartOutcome::Cancelled)
                        } else {
                            Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                        }
                    }
                })
                .await
        });

        entered_wait.await;
        let second_manager = manager.clone();
        let second_spawns = spawns.clone();
        let second = tokio::spawn(async move {
            second_manager
                .start_serialized("model-a", |_token| {
                    let spawns = second_spawns.clone();
                    async move {
                        spawns.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                    }
                })
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(manager.take_runtime().is_none());
        release.notify_waiters();

        let first_status = first.await.unwrap().unwrap();
        let second_status = second.await.unwrap().unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(first_status.state, RuntimeState::Stopped);
        assert_eq!(second_status.state, RuntimeState::Stopped);
    }

    #[tokio::test]
    async fn queued_start_shares_the_inflight_failure() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        let spawns = Arc::new(AtomicU32::new(0));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());

        let first_manager = manager.clone();
        let first_spawns = spawns.clone();
        let first_entered = entered.clone();
        let first_release = release.clone();
        let entered_wait = entered.notified();
        let first = tokio::spawn(async move {
            first_manager
                .start_serialized("model-a", |_token| {
                    let spawns = first_spawns.clone();
                    let entered = first_entered.clone();
                    let release = first_release.clone();
                    async move {
                        spawns.fetch_add(1, AtomicOrdering::SeqCst);
                        entered.notify_waiters();
                        release.notified().await;
                        Err(AppError::new(
                            ErrorCode::RuntimeFailedToStart,
                            Some("first attempt failed".into()),
                        ))
                    }
                })
                .await
        });

        entered_wait.await;
        let second_manager = manager.clone();
        let second_spawns = spawns.clone();
        let second = tokio::spawn(async move {
            second_manager
                .start_serialized("model-a", |_token| {
                    let spawns = second_spawns.clone();
                    async move {
                        spawns.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                    }
                })
                .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        release.notify_waiters();

        let first_err = first.await.unwrap().unwrap_err();
        let second_err = second.await.unwrap().unwrap_err();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(first_err.code, ErrorCode::RuntimeFailedToStart);
        assert_eq!(second_err.code, ErrorCode::RuntimeFailedToStart);
        assert_eq!(second_err.detail.as_deref(), Some("first attempt failed"));
    }

    async fn reach_ready(manager: &RuntimeManager, model_id: &str) {
        manager
            .start_serialized(model_id, {
                let model_id = model_id.to_string();
                move |_token| {
                    let model_id = model_id.clone();
                    async move { Ok(StartOutcome::Ready(ManagedRuntime::inert(&model_id))) }
                }
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ready_reuse_supersedes_an_older_start_still_in_prework() {
        let manager = RuntimeManager::default();
        reach_ready(&manager, "model-a").await;
        let older_b = manager.reserve_start_intent("model-b");
        let port = manager.status().port;
        manager
            .start_serialized("model-a", |_| async {
                panic!("ready A must be reused");
            })
            .await
            .unwrap();
        manager
            .start_with_ticket("model-b", older_b, |_| async {
                panic!("older B must not replace the latest selection A");
            })
            .await
            .unwrap();
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
        assert_eq!(manager.status().port, port);
    }

    #[tokio::test]
    async fn ready_reuse_does_not_accept_a_ticket_from_before_removal_and_readd() {
        let manager = RuntimeManager::default();
        reach_ready(&manager, "model-a").await;
        let old = manager.reserve_start_intent("model-a");
        let removal = manager.begin_removal("model-a");
        removal.wait_for_owned_shutdown().await.unwrap();
        drop(removal);
        reach_ready(&manager, "model-a").await;
        let err = manager
            .start_with_ticket("model-a", old, |_| async {
                panic!("a pre-removal ticket must not spawn");
            })
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ModelFileMissing);
    }

    #[tokio::test]
    async fn completed_removal_survives_later_runtime_shutdowns() {
        let manager = RuntimeManager::default();
        reach_ready(&manager, "model-a").await;
        let removal = manager.begin_removal("model-a");
        reach_ready(&manager, "model-b").await;
        reach_ready(&manager, "model-c").await;
        tokio::time::timeout(
            Duration::from_millis(200),
            removal.wait_for_owned_shutdown(),
        )
        .await
        .expect("completed A shutdown must remain observable after B shutdown")
        .unwrap();
        assert_eq!(manager.status().model_id.as_deref(), Some("model-c"));
    }

    #[tokio::test]
    async fn removing_a_starting_model_waits_for_cancelled_child_cleanup() {
        let manager = RuntimeManager::default();
        let entered = tokio::sync::Notify::new();
        let release = tokio::sync::Notify::new();
        let mut starting = Box::pin(manager.start_serialized("model-a", |token| {
            let entered = &entered;
            let release = &release;
            async move {
                entered.notify_waiters();
                release.notified().await;
                assert!(token.is_cancelled());
                Ok(StartOutcome::Cancelled)
            }
        }));
        let entered_wait = entered.notified();
        assert!(futures_util::poll!(starting.as_mut()).is_pending());
        entered_wait.await;
        let removal = manager.begin_removal("model-a");
        let mut removing = Box::pin(removal.wait_for_owned_shutdown());
        assert!(
            futures_util::poll!(removing.as_mut()).is_pending(),
            "registry deletion must wait until the in-flight child is gone"
        );
        release.notify_waiters();
        starting.await.unwrap();
        removing.await.unwrap();
        assert_eq!(manager.status().state, RuntimeState::Stopped);
    }

    #[tokio::test]
    async fn old_stop_does_not_hide_a_newly_ready_runtime() {
        let manager = RuntimeManager::default();
        reach_ready(&manager, "model-a").await;
        let hold = manager.arm_shutdown_hold();
        let entered = hold.entered.notified();
        let mut stopping = Box::pin(manager.stop_and_wait());
        assert!(futures_util::poll!(stopping.as_mut()).is_pending());
        entered.await;
        let drained = manager.subscribe();
        hold.release.notify_waiters();
        drained.await;
        reach_ready(&manager, "model-b").await;
        let status = stopping.await.unwrap();
        assert_eq!(status.state, RuntimeState::Ready);
        assert_eq!(status.model_id.as_deref(), Some("model-b"));
        assert_eq!(manager.status().state, RuntimeState::Ready);
    }

    #[tokio::test]
    async fn stop_during_runtime_handoff_prevents_new_spawn() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        reach_ready(&manager, "model-a").await;
        let hold = manager.arm_shutdown_hold();
        let entered = hold.entered.notified();
        let spawns = Arc::new(AtomicU32::new(0));
        let manager_b = manager.clone();
        let spawns_b = spawns.clone();
        let starting_b = tokio::spawn(async move {
            manager_b
                .start_serialized("model-b", |_| {
                    let spawns_b = spawns_b.clone();
                    async move {
                        spawns_b.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                    }
                })
                .await
        });
        entered.await;
        let manager_stop = manager.clone();
        let stopping = tokio::spawn(async move { manager_stop.stop_and_wait().await });
        hold.release.notify_waiters();
        let started = starting_b.await.unwrap().unwrap();
        let stopped = stopping.await.unwrap().unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(started.state, RuntimeState::Stopped);
        assert_eq!(stopped.state, RuntimeState::Stopped);
    }

    #[tokio::test]
    async fn reserved_start_before_stop_is_cancelled_after_prework() {
        // IPC-entry reservation, slow pre-work (inspect) paused, explicit
        // Stop lands mid-inspection, old Start resumes: zero new spawns.
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = RuntimeManager::default();
        reach_ready(&manager, "model-a").await;
        let ticket = manager.reserve_start_intent("model-b");
        manager.stop_and_wait().await.unwrap();
        assert_eq!(manager.status().state, RuntimeState::Stopped);
        let spawns = Arc::new(AtomicU32::new(0));
        let spawns_run = spawns.clone();
        let status = manager
            .start_with_ticket("model-b", ticket, move |_token| {
                let spawns_run = spawns_run.clone();
                async move {
                    spawns_run.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                }
            })
            .await
            .unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(status.state, RuntimeState::Stopped);
        assert_eq!(manager.status().state, RuntimeState::Stopped);
        assert!(manager.status().model_id.is_none());
    }

    #[tokio::test]
    async fn abandoned_prework_intent_does_not_block_newer_start() {
        // Failed lookup/invalid GGUF drops its ticket: the manager must not
        // be stuck and the abandoned seq must not overwrite a newer request.
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = RuntimeManager::default();
        let stale = manager.reserve_start_intent("model-a");
        let newer = manager.reserve_start_intent("model-b");
        drop(stale);
        let spawns = Arc::new(AtomicU32::new(0));
        let spawns_run = spawns.clone();
        let status = manager
            .start_with_ticket("model-b", newer, move |_token| {
                let spawns_run = spawns_run.clone();
                async move {
                    spawns_run.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                }
            })
            .await
            .unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(status.state, RuntimeState::Ready);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-b"));
        // A retry after failure is a fresh reservation and also works.
        manager.stop_and_wait().await.unwrap();
        let retry = manager.reserve_start_intent("model-a");
        let status = manager
            .start_with_ticket("model-a", retry, |_token| async {
                Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
            })
            .await
            .unwrap();
        assert_eq!(status.state, RuntimeState::Ready);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
    }

    #[tokio::test]
    async fn fresh_use_waits_for_inflight_unload() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        reach_ready(&manager, "model-a").await;
        let hold = manager.arm_shutdown_hold();
        let entered = hold.entered.notified();
        let manager_b = manager.clone();
        let switching = tokio::spawn(async move {
            manager_b
                .start_serialized("model-b", |_token| async {
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                })
                .await
        });
        entered.await;
        let stop_progress = manager.subscribe();
        let manager_stop = manager.clone();
        let stopping = tokio::spawn(async move { manager_stop.stop_and_wait().await });
        stop_progress.await;
        let waiting = manager.start_is_waiting();
        let spawns = Arc::new(AtomicU32::new(0));
        let manager_c = manager.clone();
        let spawns_c = spawns.clone();
        let starting_c = tokio::spawn(async move {
            manager_c
                .start_serialized("model-c", |_| {
                    let spawns_c = spawns_c.clone();
                    async move {
                        spawns_c.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-c")))
                    }
                })
                .await
        });
        waiting.await;
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 0);
        hold.release.notify_waiters();
        switching.await.unwrap().unwrap();
        stopping.await.unwrap().unwrap();
        let started = starting_c.await.unwrap().unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(started.state, RuntimeState::Ready);
        assert_eq!(started.model_id.as_deref(), Some("model-c"));
    }

    #[tokio::test]
    async fn cancelled_reaper_does_not_allow_another_spawn() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        reach_ready(&manager, "model-a").await;
        let hold = manager.arm_shutdown_hold();
        let entered = hold.entered.notified();
        let manager_b = manager.clone();
        let switching = tokio::spawn(async move {
            manager_b
                .start_serialized("model-b", |_token| async {
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                })
                .await
        });
        entered.await;
        let faulted = manager.subscribe();
        manager.abort_reaper();
        faulted.await;
        assert_eq!(manager.status().state, RuntimeState::Error);
        let spawns = Arc::new(AtomicU32::new(0));
        let err = manager
            .start_serialized("model-c", |_| {
                let spawns = spawns.clone();
                async move {
                    spawns.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-c")))
                }
            })
            .await
            .unwrap_err();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(err.code, ErrorCode::RuntimeFailedToStart);
        let _ = switching.await;
    }

    #[tokio::test]
    async fn remove_target_while_start_is_queued_cannot_leave_orphan_runtime() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let entered_wait = entered.notified();
        let manager_a = manager.clone();
        let entered_a = entered.clone();
        let release_a = release.clone();
        let starting_a = tokio::spawn(async move {
            manager_a
                .start_serialized("model-a", |_token| {
                    let entered_a = entered_a.clone();
                    let release_a = release_a.clone();
                    async move {
                        entered_a.notify_waiters();
                        release_a.notified().await;
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                    }
                })
                .await
        });
        entered_wait.await;
        let waiting_before = manager.waiter_count();
        let spawns_b = Arc::new(AtomicU32::new(0));
        let manager_b = manager.clone();
        let spawns_b2 = spawns_b.clone();
        let starting_b = tokio::spawn(async move {
            manager_b
                .start_serialized("model-b", |_| {
                    let spawns_b2 = spawns_b2.clone();
                    async move {
                        spawns_b2.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                    }
                })
                .await
        });
        while manager.waiter_count() == waiting_before {
            tokio::task::yield_now().await;
        }
        let removal = manager.begin_removal("model-b");
        release.notify_waiters();
        starting_a.await.unwrap().unwrap();
        let started_b = starting_b.await.unwrap();
        drop(removal);
        assert!(started_b.is_err());
        assert_eq!(spawns_b.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
    }

    #[tokio::test]
    async fn fresh_start_after_explicit_stop_spawns() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = RuntimeManager::default();
        let spawns = Arc::new(AtomicU32::new(0));
        let spawn_once = |spawns: Arc<AtomicU32>| {
            move |_token: StartCancel| {
                let spawns = spawns.clone();
                async move {
                    spawns.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                }
            }
        };
        manager
            .start_serialized("model-a", spawn_once(spawns.clone()))
            .await
            .unwrap();
        manager.stop_and_wait().await.unwrap();
        manager
            .start_serialized("model-a", spawn_once(spawns.clone()))
            .await
            .unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 2);
        assert_eq!(manager.status().state, RuntimeState::Ready);
    }

    #[tokio::test]
    async fn later_request_for_a_supersedes_queued_b() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = Arc::new(RuntimeManager::default());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let entered_wait = entered.notified();
        let spawns = Arc::new(AtomicU32::new(0));
        let manager_a = manager.clone();
        let entered_a = entered.clone();
        let release_a = release.clone();
        let spawns_a = spawns.clone();
        let starting_a = tokio::spawn(async move {
            manager_a
                .start_serialized("model-a", |_token| {
                    let entered_a = entered_a.clone();
                    let release_a = release_a.clone();
                    let spawns_a = spawns_a.clone();
                    async move {
                        spawns_a.fetch_add(1, AtomicOrdering::SeqCst);
                        entered_a.notify_waiters();
                        release_a.notified().await;
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                    }
                })
                .await
        });
        entered_wait.await;
        let waiting_b = manager.start_is_waiting();
        let manager_b = manager.clone();
        let spawns_b = spawns.clone();
        let starting_b = tokio::spawn(async move {
            manager_b
                .start_serialized("model-b", |_| {
                    let spawns_b = spawns_b.clone();
                    async move {
                        spawns_b.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                    }
                })
                .await
        });
        waiting_b.await;
        let waiting_a = manager.start_is_waiting();
        let manager_a2 = manager.clone();
        let spawns_a2 = spawns.clone();
        let starting_a2 = tokio::spawn(async move {
            manager_a2
                .start_serialized("model-a", |_| {
                    let spawns_a2 = spawns_a2.clone();
                    async move {
                        spawns_a2.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(StartOutcome::Ready(ManagedRuntime::inert("model-a")))
                    }
                })
                .await
        });
        waiting_a.await;
        release.notify_waiters();
        starting_a.await.unwrap().unwrap();
        starting_b.await.unwrap().unwrap();
        starting_a2.await.unwrap().unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(manager.status().state, RuntimeState::Ready);
        assert_eq!(manager.status().model_id.as_deref(), Some("model-a"));
    }

    #[tokio::test]
    async fn failed_removal_clears_unavailable() {
        use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
        let manager = RuntimeManager::default();
        let removal = manager.begin_removal("model-b");
        let blocked = manager
            .start_serialized("model-b", |_token| async {
                Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
            })
            .await;
        assert!(blocked.is_err());
        drop(removal);
        let spawns = Arc::new(AtomicU32::new(0));
        let spawns_run = spawns.clone();
        manager
            .start_serialized("model-b", move |_token| {
                let spawns_run = spawns_run.clone();
                async move {
                    spawns_run.fetch_add(1, AtomicOrdering::SeqCst);
                    Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
                }
            })
            .await
            .unwrap();
        assert_eq!(spawns.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn overlapping_removals_keep_model_unavailable_until_both_finish() {
        let manager = RuntimeManager::default();
        let first = manager.begin_removal("model-b");
        let second = manager.begin_removal("model-b");
        drop(first);
        let still_blocked = manager
            .start_serialized("model-b", |_token| async {
                Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
            })
            .await;
        assert!(still_blocked.is_err());
        drop(second);
        manager
            .start_serialized("model-b", |_token| async {
                Ok(StartOutcome::Ready(ManagedRuntime::inert("model-b")))
            })
            .await
            .unwrap();
        assert_eq!(manager.status().model_id.as_deref(), Some("model-b"));
    }
}
