//! Phase 7 local image generation worker seam (docs/image-provider.md).
//!
//! When a diffusion worker binary is present under the standard runtimes
//! layout, Generate will invoke it. Until then, Generate fails closed with a
//! friendly RuntimeMissing error and never opens a network connection.

use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::Command;

use crate::errors::{AppError, ErrorCode};
use crate::logging;
use crate::paths;
use crate::types::{Generation, GenerationStatus, ImageRuntimeStatus};
use crate::storage::Storage;

fn worker_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        out.push(
            local
                .join("Omnira")
                .join("runtimes")
                .join("diffusion")
                .join("omnira-diffusion.exe"),
        );
    }
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join("diffusion")
            .join("omnira-diffusion.exe"),
    );
    out
}

pub fn find_worker() -> Option<PathBuf> {
    worker_candidates().into_iter().find(|p| p.is_file())
}

pub fn status() -> ImageRuntimeStatus {
    match find_worker() {
        Some(path) => ImageRuntimeStatus {
            available: true,
            detail: format!("diffusion worker: {}", path.display()),
        },
        None => ImageRuntimeStatus {
            available: false,
            detail: "No local diffusion worker installed. Place omnira-diffusion.exe under %LOCALAPPDATA%\\Omnira\\runtimes\\diffusion\\ when a Phase 7 worker build is available.".into(),
        },
    }
}

pub async fn generate(
    storage: &Storage,
    prompt: String,
    width: u32,
    height: u32,
) -> Result<Generation, AppError> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::new(
            ErrorCode::GenerationFailed,
            Some("empty prompt".into()),
        ));
    }
    let width = width.clamp(256, 2048);
    let height = height.clamp(256, 2048);

    let worker = find_worker().ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeMissing,
            Some(status().detail),
        )
    })?;

    let id = uuid::Uuid::new_v4().to_string();
    let out_path = paths::generations_dir().join(format!("{id}.png"));
    std::fs::create_dir_all(paths::generations_dir())?;

    logging::info(
        "image.generate.start",
        &format!("width={width} height={height} worker={}", worker.display()),
    );

    let mut child = Command::new(&worker)
        .arg("--prompt")
        .arg(&prompt)
        .arg("--width")
        .arg(width.to_string())
        .arg("--height")
        .arg(height.to_string())
        .arg("--output")
        .arg(&out_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AppError::new(
                ErrorCode::RuntimeFailedToStart,
                Some(format!("failed to start diffusion worker: {e}")),
            )
        })?;

    let exit = child.wait().await.map_err(|e| {
        AppError::new(
            ErrorCode::GenerationFailed,
            Some(format!("diffusion worker wait failed: {e}")),
        )
    })?;

    if !exit.success() || !out_path.is_file() {
        let _ = storage.add_generation(
            &prompt,
            width,
            height,
            &out_path.display().to_string(),
            GenerationStatus::Failed,
        );
        return Err(AppError::new(
            ErrorCode::GenerationFailed,
            Some(format!("diffusion worker exited: {exit}")),
        ));
    }

    storage.add_generation(
        &prompt,
        width,
        height,
        &out_path.display().to_string(),
        GenerationStatus::Complete,
    )
}
