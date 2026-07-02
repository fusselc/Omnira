//! Omnira error taxonomy.
//!
//! Every user-facing failure maps to exactly one `ErrorCode`. Lifecycle and
//! process errors are normalized here in the Rust core; streaming-time errors
//! are mapped in the frontend's dedicated streaming module. No ad hoc error
//! strings anywhere else. See docs/chat-provider.md.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    RuntimeMissing,
    RuntimeFailedToStart,
    ModelFileMissing,
    ModelFormatInvalid,
    ModelLoadFailed,
    InsufficientMemory,
    GenerationCancelled,
    GenerationFailed,
    BackendUnavailable,
    UnauthorizedLocalRequest,
    UnknownRuntimeError,
}

/// The structured error surfaced to the frontend over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: ErrorCode,
    /// Friendly, plain-language message shown to the user.
    pub message: String,
    /// What the user can try next.
    pub suggested_action: String,
    /// Technical detail for Advanced Diagnostics only.
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, detail: impl Into<Option<String>>) -> Self {
        let (message, suggested_action) = friendly_copy(code);
        Self {
            code,
            message: message.to_string(),
            suggested_action: suggested_action.to_string(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)?;
        if let Some(d) = &self.detail {
            write!(f, " ({d})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

/// Canonical friendly copy per code (docs/design-principles.md).
fn friendly_copy(code: ErrorCode) -> (&'static str, &'static str) {
    match code {
        ErrorCode::RuntimeMissing => (
            "Omnira's chat engine is missing.",
            "Reinstall Omnira, or set a runtime path in Settings.",
        ),
        ErrorCode::RuntimeFailedToStart => (
            "The chat engine could not start.",
            "Try again; if it keeps failing, check Advanced Diagnostics.",
        ),
        ErrorCode::ModelFileMissing => (
            "This model's file has moved or been deleted.",
            "Locate the file again from the Models screen, or remove the entry.",
        ),
        ErrorCode::ModelFormatInvalid => (
            "This file does not look like a valid GGUF model.",
            "Choose a different .gguf file.",
        ),
        ErrorCode::ModelLoadFailed => (
            "The model could not be loaded.",
            "Try a smaller model, or check Advanced Diagnostics.",
        ),
        ErrorCode::InsufficientMemory => (
            "There is not enough memory to run this model.",
            "Close other apps or try a smaller model.",
        ),
        ErrorCode::GenerationCancelled => (
            "Generation stopped.",
            "Your partial response has been kept.",
        ),
        ErrorCode::GenerationFailed => (
            "The response could not be completed.",
            "Try sending your message again.",
        ),
        ErrorCode::BackendUnavailable => (
            "Omnira's engine is not responding.",
            "Restart Omnira.",
        ),
        ErrorCode::UnauthorizedLocalRequest => (
            "A request was blocked for your security.",
            "Restart Omnira if chat stops working.",
        ),
        ErrorCode::UnknownRuntimeError => (
            "Something unexpected went wrong.",
            "Try again; details are in Advanced Diagnostics.",
        ),
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::new(ErrorCode::BackendUnavailable, Some(format!("sqlite: {e}")))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::new(ErrorCode::UnknownRuntimeError, Some(format!("io: {e}")))
    }
}
