//! Typed Tauri IPC command surface. This is the ONLY functionality the
//! webview can reach (security layer 1, docs/local-security-boundary.md).
//! The frontend mirror of this contract is src/lib/ipc.ts.

use tauri::{Manager, State};

use crate::chat_proxy::{ChatRequestMessage, ChatStreams};
use crate::config;
use crate::diagnostics;
use crate::errors::{AppError, ErrorCode};
use crate::gguf;
use crate::logging;
use crate::runtime::{self, RuntimeManager};
use crate::storage::Storage;
use crate::types::{
    ChatEndpoint, Conversation, DiagnosticsSnapshot, Message, MessageRole, MessageStatus,
    ModelEntry, RuntimeStatus, Settings,
};

pub struct AppState {
    pub storage: Storage,
    pub runtime: RuntimeManager,
    pub chat_streams: ChatStreams,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings() -> Settings {
    config::load()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), AppError> {
    config::save(&settings)
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Result<Vec<ModelEntry>, AppError> {
    state.storage.list_models()
}

/// Register a GGUF file in place. Sanity-checks the header (Decision 11)
/// before creating the registry entry.
#[tauri::command]
pub fn add_model(state: State<AppState>, path: String) -> Result<ModelEntry, AppError> {
    let file_path = std::path::PathBuf::from(&path);
    let info = gguf::inspect(&file_path)?;
    let size = std::fs::metadata(&file_path)?.len();
    let name = info
        .model_name
        .clone()
        .unwrap_or_else(|| {
            file_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Model".to_string())
        });
    logging::info("model.add", &format!("gguf v{} ctx={:?}", info.version, info.trained_context_length));
    state
        .storage
        .add_model(&name, &path, size, info.trained_context_length)
}

/// Removes only the registry entry; never deletes the model file.
#[tauri::command]
pub fn remove_model(state: State<AppState>, id: String) -> Result<(), AppError> {
    logging::info("model.remove", "registry entry only");
    state.storage.remove_model(&id)
}

/// Renames the registry display name only; never renames the file on disk.
#[tauri::command]
pub fn rename_model(state: State<AppState>, id: String, name: String) -> Result<(), AppError> {
    logging::info("model.rename", "display name only");
    state.storage.rename_model(&id, &name)
}

// ---------------------------------------------------------------------------
// Conversations and messages
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_conversations(state: State<AppState>) -> Result<Vec<Conversation>, AppError> {
    state.storage.list_conversations()
}

#[tauri::command]
pub fn create_conversation(
    state: State<AppState>,
    title: String,
    model_id: Option<String>,
) -> Result<Conversation, AppError> {
    state.storage.create_conversation(&title, model_id.as_deref())
}

#[tauri::command]
pub fn rename_conversation(
    state: State<AppState>,
    id: String,
    title: String,
) -> Result<(), AppError> {
    state.storage.rename_conversation(&id, &title)
}

#[tauri::command]
pub fn set_conversation_model(
    state: State<AppState>,
    id: String,
    model_id: String,
) -> Result<(), AppError> {
    state.storage.set_conversation_model(&id, &model_id)
}

#[tauri::command]
pub fn delete_conversation(state: State<AppState>, id: String) -> Result<(), AppError> {
    state.storage.delete_conversation(&id)
}

#[tauri::command]
pub fn clear_conversations(state: State<AppState>) -> Result<(), AppError> {
    logging::info("data.clear_conversations", "");
    state.storage.clear_conversations()
}

#[tauri::command]
pub fn list_messages(
    state: State<AppState>,
    conversation_id: String,
) -> Result<Vec<Message>, AppError> {
    state.storage.list_messages(&conversation_id)
}

/// Persist a message (stream-boundary contract: user messages BEFORE
/// streaming; assistant messages on completion or interruption).
#[tauri::command]
pub fn add_message(
    state: State<AppState>,
    conversation_id: String,
    role: MessageRole,
    content: String,
    status: MessageStatus,
) -> Result<Message, AppError> {
    state
        .storage
        .add_message(&conversation_id, role, &content, status)
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn runtime_status(state: State<AppState>) -> RuntimeStatus {
    state.runtime.status()
}

/// Connection info for the direct chat path (host, port, session api-key,
/// context character budget).
#[tauri::command]
pub fn chat_endpoint(state: State<AppState>) -> Result<ChatEndpoint, AppError> {
    state.runtime.endpoint()
}

/// Start (or switch) the managed llama-server for a registered model.
/// One loaded model at a time: a running runtime is stopped first.
#[tauri::command]
pub async fn start_runtime(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<RuntimeStatus, AppError> {
    let model = state
        .storage
        .get_model(&model_id)?
        .ok_or_else(|| AppError::new(ErrorCode::ModelFileMissing, Some("unknown model id".into())))?;

    // Re-verify the file at start time (it may have moved since registration).
    let model_path = std::path::PathBuf::from(&model.path);
    let info = gguf::inspect(&model_path)?;

    // Stop any running runtime first (concurrency policy, Decision 7).
    if let Some(rt) = state.runtime.take_runtime() {
        runtime::stop(rt).await;
    }

    state.runtime.set_starting();

    let settings = config::load();
    let resource_dir = app.path().resource_dir().ok();

    match runtime::start(
        resource_dir,
        settings.runtime_path_override.clone(),
        settings.preferred_runtime_variant,
        model.path.clone(),
        model_id.clone(),
        info.trained_context_length,
    )
    .await
    {
        Ok(rt) => {
            // Record the working variant for next launch.
            let mut settings = config::load();
            if settings.preferred_runtime_variant != Some(rt.variant) {
                settings.preferred_runtime_variant = Some(rt.variant);
                let _ = config::save(&settings);
            }
            let _ = state.storage.touch_model(&model_id);
            state.runtime.set_ready(rt);
            Ok(state.runtime.status())
        }
        Err(e) => {
            logging::error("runtime.start_failed", &format!("code={:?}", e.code));
            state.runtime.set_error(e.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_runtime(state: State<'_, AppState>) -> Result<RuntimeStatus, AppError> {
    if let Some(rt) = state.runtime.take_runtime() {
        runtime::stop(rt).await;
    }
    Ok(state.runtime.status())
}

// ---------------------------------------------------------------------------
// Chat proxy path (architecture-level CORS fallback, design addendum 1)
// ---------------------------------------------------------------------------

/// Start a proxied chat stream. Chunks and the terminal status arrive as
/// Tauri events named `chat-stream:{stream_id}`.
#[tauri::command]
pub async fn chat_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    stream_id: String,
    messages: Vec<ChatRequestMessage>,
) -> Result<(), AppError> {
    let endpoint = state.runtime.endpoint()?;
    crate::chat_proxy::run_stream(
        app.clone(),
        &state.chat_streams,
        stream_id,
        endpoint.base_url,
        endpoint.api_key,
        messages,
    )
    .await;
    Ok(())
}

#[tauri::command]
pub fn chat_cancel(state: State<AppState>, stream_id: String) {
    state.chat_streams.cancel(&stream_id);
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn diagnostics_snapshot(state: State<AppState>) -> DiagnosticsSnapshot {
    diagnostics::snapshot(&state.runtime)
}

/// Redacted by default; `include_paths = true` is the explicit opt-in.
#[tauri::command]
pub fn diagnostics_export(
    state: State<AppState>,
    include_paths: bool,
) -> Result<String, AppError> {
    diagnostics::export(&state.runtime, include_paths, None)
}
