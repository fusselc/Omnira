//! Managed llama-server lifecycle (docs/architecture.md sections 2, 4, 5).
//!
//! Responsibilities: runtime variant selection (CUDA -> Vulkan -> CPU on NVIDIA
//! machines; otherwise Vulkan -> CPU), loopback port reservation, per-session
//! api-key generation, spawn under the Job Object, health gating, shutdown,
//! and status snapshots.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use rand::Rng;
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

pub struct ManagedRuntime {
    child: Child,
    pub variant: RuntimeVariant,
    pub port: u16,
    pub api_key: String,
    pub model_id: String,
    pub context_size: u64,
    pub fallback_reason: Option<String>,
}

#[derive(Default)]
pub struct RuntimeManager {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    runtime: Option<ManagedRuntime>,
    state: RuntimeState,
    last_error: Option<AppError>,
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

    let dir_name = match variant {
        RuntimeVariant::Cuda => "cuda",
        RuntimeVariant::Vulkan => "vulkan",
        RuntimeVariant::Cpu => "cpu",
    };

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(res) = resource_dir {
        candidates.push(res.join("runtimes").join(dir_name).join("llama-server.exe"));
    }
    // Optional acceleration pack / user drop-in (LocalAppData), Phase 6.
    if let Some(local) = dirs::data_local_dir() {
        candidates.push(
            local
                .join("Omnira")
                .join("runtimes")
                .join(dir_name)
                .join("llama-server.exe"),
        );
    }
    // Dev layout (populated by scripts/packaging/fetch-llama-server.ps1).
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(dir_name)
            .join("llama-server.exe"),
    );

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| {
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
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| AppError::new(ErrorCode::RuntimeFailedToStart, Some(format!("port reservation: {e}"))))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::new(ErrorCode::RuntimeFailedToStart, Some(format!("port reservation: {e}"))))?
        .port();
    drop(listener);
    Ok(port)
}

fn generate_api_key() -> String {
    let mut rng = rand::rng();
    (0..48)
        .map(|_| {
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            CHARSET[rng.random_range(0..CHARSET.len())] as char
        })
        .collect()
}

async fn wait_healthy(port: u16, child: &mut Child) -> Result<(), AppError> {
    let url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + HEALTH_TIMEOUT;

    loop {
        // Detect early exit (bind race, model load failure, OOM...).
        if let Some(status) = child.try_wait().map_err(|e| {
            AppError::new(ErrorCode::RuntimeFailedToStart, Some(format!("try_wait: {e}")))
        })? {
            return Err(AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(format!("llama-server exited during startup: {status}")),
            ));
        }

        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some("health check timed out".to_string()),
            ));
        }
        tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
    }
}

async fn spawn_variant(
    binary: &PathBuf,
    variant: RuntimeVariant,
    model_path: &str,
    model_id: &str,
    ctx_size: u64,
) -> Result<ManagedRuntime, AppError> {
    let mut last_err: Option<AppError> = None;

    for attempt in 1..=SPAWN_ATTEMPTS {
        let port = reserve_port()?;
        let api_key = generate_api_key();

        logging::info(
            "runtime.spawn",
            &format!("variant={variant:?} attempt={attempt} port={port}"),
        );

        let mut child = Command::new(binary)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--api-key")
            .arg(&api_key)
            .arg("--model")
            .arg(model_path)
            .arg("--ctx-size")
            .arg(ctx_size.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                AppError::new(ErrorCode::RuntimeFailedToStart, Some(format!("spawn: {e}")))
            })?;

        // Assign to the kill-on-close Job Object immediately after spawn.
        if let Some(pid) = child.id() {
            if let Err(e) = process::supervise(pid) {
                let _ = child.start_kill();
                return Err(AppError::new(
                    ErrorCode::RuntimeFailedToStart,
                    Some(format!("job object assignment failed: {e}")),
                ));
            }
        }

        match wait_healthy(port, &mut child).await {
            Ok(()) => {
                logging::info("runtime.ready", &format!("variant={variant:?} port={port}"));
                return Ok(ManagedRuntime {
                    child,
                    variant,
                    port,
                    api_key,
                    model_id: model_id.to_string(),
                    context_size: ctx_size,
                    fallback_reason: None,
                });
            }
            Err(e) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                logging::error("runtime.spawn_failed", &format!("variant={variant:?} attempt={attempt} code={:?}", e.code));
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        AppError::new(ErrorCode::RuntimeFailedToStart, Some("spawn attempts exhausted".into()))
    }))
}

impl RuntimeManager {
    pub fn status(&self) -> RuntimeStatus {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let rt = inner.runtime.as_ref();
        RuntimeStatus {
            state: inner.state,
            variant: rt.map(|r| r.variant),
            accelerator_label: rt.map(|r| match r.variant {
                RuntimeVariant::Cuda => "NVIDIA GPU (CUDA)".to_string(),
                RuntimeVariant::Vulkan => "GPU (Vulkan)".to_string(),
                RuntimeVariant::Cpu => "CPU".to_string(),
            }),
            fallback_reason: rt.and_then(|r| r.fallback_reason.clone()),
            model_id: rt.map(|r| r.model_id.clone()),
            port: rt.map(|r| r.port),
            context_size: rt.map(|r| r.context_size),
            last_error: inner.last_error.clone(),
        }
    }

    pub fn endpoint(&self) -> Result<crate::types::ChatEndpoint, AppError> {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let rt = inner.runtime.as_ref().ok_or_else(|| {
            AppError::new(ErrorCode::BackendUnavailable, Some("no runtime running".into()))
        })?;
        Ok(crate::types::ChatEndpoint {
            base_url: format!("http://127.0.0.1:{}", rt.port),
            api_key: rt.api_key.clone(),
            context_chars_budget: context_chars_budget(rt.context_size),
        })
    }

    pub fn set_starting(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.state = RuntimeState::Starting;
        inner.last_error = None;
    }

    pub fn set_error(&self, err: AppError) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.state = RuntimeState::Error;
        inner.last_error = Some(err);
        inner.runtime = None;
    }

    pub fn set_ready(&self, runtime: ManagedRuntime) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.state = RuntimeState::Ready;
        inner.last_error = None;
        inner.runtime = Some(runtime);
    }

    pub fn take_runtime(&self) -> Option<ManagedRuntime> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.state = RuntimeState::Stopped;
        inner.runtime.take()
    }

}

pub fn context_chars_budget(ctx_size: u64) -> u64 {
    let usable = ctx_size - ctx_size / RESPONSE_HEADROOM_FRACTION;
    usable * CHARS_PER_TOKEN
}

/// Start llama-server for the given model. Preference order:
/// recorded preferred variant first when set, else CUDA -> Vulkan -> CPU when
/// an NVIDIA GPU is detected, else Vulkan -> CPU (Phase 6).
pub async fn start(
    resource_dir: Option<PathBuf>,
    override_path: Option<String>,
    preferred: Option<RuntimeVariant>,
    model_path: String,
    model_id: String,
    trained_context_length: Option<u64>,
) -> Result<ManagedRuntime, AppError> {
    let ctx_size = match trained_context_length {
        Some(trained) if trained > 0 => DEFAULT_CTX_SIZE.min(trained),
        _ => DEFAULT_CTX_SIZE,
    };

    let order = variant_order(preferred);

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

        match spawn_variant(&binary, variant, &model_path, &model_id, ctx_size).await {
            Ok(mut rt) => {
                if let Some((failed_variant, err)) = &first_failure {
                    rt.fallback_reason = Some(format!(
                        "{failed_variant:?} unavailable: {}",
                        err.detail.clone().unwrap_or_default()
                    ));
                }
                return Ok(rt);
            }
            Err(e) => {
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

/// Build the attempt order for runtime variants.
fn variant_order(preferred: Option<RuntimeVariant>) -> Vec<RuntimeVariant> {
    let nvidia = nvidia_gpu_present();
    let default_order = if nvidia {
        vec![
            RuntimeVariant::Cuda,
            RuntimeVariant::Vulkan,
            RuntimeVariant::Cpu,
        ]
    } else {
        vec![RuntimeVariant::Vulkan, RuntimeVariant::Cpu]
    };

    match preferred {
        Some(pref) => {
            let mut order = vec![pref];
            for v in default_order {
                if v != pref {
                    order.push(v);
                }
            }
            order
        }
        None => default_order,
    }
}

/// Best-effort local NVIDIA detection. No network; looks for nvidia-smi on
/// PATH, System32, or the NVSMI install folder. Absence skips CUDA preference.
fn nvidia_gpu_present() -> bool {
    which_nvidia_smi().is_some()
}

fn which_nvidia_smi() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            candidates.push(dir.join("nvidia-smi.exe"));
        }
    }
    candidates.push(PathBuf::from(r"C:\Windows\System32\nvidia-smi.exe"));
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        candidates.push(
            PathBuf::from(pf)
                .join("NVIDIA Corporation")
                .join("NVSMI")
                .join("nvidia-smi.exe"),
        );
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Stop a managed runtime gracefully-ish: kill the child and reap it.
pub async fn stop(mut rt: ManagedRuntime) {
    logging::info("runtime.stop", &format!("port={}", rt.port));
    let _ = rt.child.start_kill();
    let _ = rt.child.wait().await;
}

#[cfg(test)]
mod tests {
    use super::variant_order;
    use crate::types::RuntimeVariant;

    #[test]
    fn preferred_variant_is_tried_first() {
        let order = variant_order(Some(RuntimeVariant::Cpu));
        assert_eq!(order[0], RuntimeVariant::Cpu);
        assert!(order.contains(&RuntimeVariant::Vulkan));
    }

    #[test]
    fn default_order_includes_vulkan_and_cpu() {
        let order = variant_order(None);
        assert!(order.contains(&RuntimeVariant::Vulkan));
        assert!(order.contains(&RuntimeVariant::Cpu));
        // CUDA is included only when nvidia-smi is present on this machine.
        if order.contains(&RuntimeVariant::Cuda) {
            assert_eq!(order[0], RuntimeVariant::Cuda);
        } else {
            assert_eq!(order[0], RuntimeVariant::Vulkan);
        }
    }
}
