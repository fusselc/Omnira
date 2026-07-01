//! Shared IPC types. These structs define the typed command/event contract
//! with the frontend; src/lib/ipc.ts mirrors them one-to-one.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Ok,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub file_size_bytes: u64,
    /// Trained context length from GGUF metadata, if known.
    pub trained_context_length: Option<u64>,
    pub last_used_at: Option<String>,
    pub added_at: String,
    pub status: ModelStatus,
}

// ---------------------------------------------------------------------------
// Conversations and messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Complete,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub status: MessageStatus,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeVariant {
    Vulkan,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    Stopped,
    Starting,
    Ready,
    Error,
}

/// Snapshot of the managed runtime, for the status bar and Diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub state: RuntimeState,
    pub variant: Option<RuntimeVariant>,
    /// Human label for Diagnostics only, e.g. "NVIDIA GPU (Vulkan)" or "CPU".
    pub accelerator_label: Option<String>,
    /// Why the CPU fallback engaged, if it did. Diagnostics only.
    pub fallback_reason: Option<String>,
    pub model_id: Option<String>,
    pub port: Option<u16>,
    pub context_size: Option<u64>,
    pub last_error: Option<crate::errors::AppError>,
}

/// Connection info the frontend needs for the direct chat path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEndpoint {
    pub base_url: String,
    pub api_key: String,
    /// Conservative character budget for conversation history
    /// (docs/chat-provider.md section 5).
    pub context_chars_budget: u64,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub model_search_paths: Vec<String>,
    /// Optional custom runtime path override (Settings screen).
    pub runtime_path_override: Option<String>,
    /// Recorded working runtime variant from a previous run.
    pub preferred_runtime_variant: Option<RuntimeVariant>,
    /// Whether the first-run flow has completed.
    pub onboarding_complete: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            model_search_paths: Vec::new(),
            runtime_path_override: None,
            preferred_runtime_variant: None,
            onboarding_complete: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSnapshot {
    pub app_version: String,
    pub runtime: RuntimeStatus,
    pub data_dir: String,
    pub config_path: String,
    pub db_path: String,
    pub log_dir: String,
    pub recent_log_lines: Vec<String>,
    pub recent_errors: Vec<crate::errors::AppError>,
}
