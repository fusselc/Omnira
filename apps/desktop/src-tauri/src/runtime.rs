//! Managed llama-server lifecycle (docs/architecture.md sections 2, 4, 5).
//!
//! Responsibilities: runtime variant selection (CUDA -> Vulkan -> CPU when a
//! CUDA binary is present; otherwise Vulkan -> CPU),
//! loopback port reservation, per-session api-key generation, spawn under the
//! Job Object, health gating, shutdown, and status snapshots.

use std::future::Future;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
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

#[derive(Default)]
pub struct RuntimeManager {
    inner: Mutex<Inner>,
    /// Serializes start attempts so overlapping IPC calls cannot spawn two
    /// llama-server children. The second caller waits, then either joins a
    /// ready runtime for the same model or starts after the previous attempt.
    start_gate: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct Inner {
    runtime: Option<ManagedRuntime>,
    state: RuntimeState,
    last_error: Option<AppError>,
    /// Cancellation handle for the start currently in flight, if any.
    pending_start: Option<StartCancel>,
    /// Model being started; surfaced on `RuntimeStatus.model_id` while
    /// `state == Starting` so the UI does not treat the load as idle.
    pending_model_id: Option<String>,
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
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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

    /// Stop everything: cancel a start that is still loading and hand back the
    /// running runtime (if any) for the caller to kill. State becomes `stopped`.
    pub fn take_runtime(&self) -> Option<ManagedRuntime> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(pending) = inner.pending_start.take() {
            pending.cancel();
        }
        inner.pending_model_id = None;
        inner.state = RuntimeState::Stopped;
        inner.last_error = None;
        inner.runtime.take()
    }

    fn is_ready_for(&self, model_id: &str) -> bool {
        let status = self.status();
        status.state == RuntimeState::Ready && status.model_id.as_deref() == Some(model_id)
    }

    /// Start a runtime, or join an in-flight/already-ready one for `model_id`.
    ///
    /// Overlapping callers share a single-flight gate: only one spawn runs at
    /// a time. A second call for the same model that is already ready or whose
    /// start is in flight does not stop the current child or spawn another.
    pub async fn start_serialized<F, Fut>(
        &self,
        model_id: &str,
        spawn: F,
    ) -> Result<RuntimeStatus, AppError>
    where
        F: FnOnce(StartCancel) -> Fut,
        Fut: Future<Output = Result<StartOutcome, AppError>>,
    {
        if self.is_ready_for(model_id) {
            logging::info("runtime.start_reuse", "already ready");
            return Ok(self.status());
        }

        let _gate = self.start_gate.lock().await;

        if self.is_ready_for(model_id) {
            logging::info("runtime.start_reuse", "already ready after wait");
            return Ok(self.status());
        }

        if let Some(rt) = self.take_runtime() {
            stop(rt).await;
        }

        let token = self.begin_start(model_id.to_string());
        match spawn(token.clone()).await {
            Ok(StartOutcome::Ready(rt)) => {
                if let Err(stale) = self.finish_start_ready(&token, rt) {
                    stop(stale).await;
                }
                Ok(self.status())
            }
            Ok(StartOutcome::Cancelled) => {
                logging::info("runtime.start_cancelled", "by user");
                Ok(self.status())
            }
            Err(e) => {
                if token.is_cancelled() {
                    return Ok(self.status());
                }
                logging::error("runtime.start_failed", &format!("code={:?}", e.code));
                self.finish_start_error(&token, e.clone());
                Err(e)
            }
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
}
