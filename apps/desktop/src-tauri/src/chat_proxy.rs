//! Rust-side streaming chat path (design addendum 1 / escalation ladder step b).
//!
//! The primary chat path is the frontend fetching llama-server directly over
//! loopback SSE. This module is the approved architecture-level fallback: a
//! `chat_stream` command performs the HTTP request and forwards chunks to the
//! frontend as Tauri events, and `chat_cancel` terminates the connection
//! authoritatively from the Rust side. Both paths share one frontend contract
//! (a stream of token chunks plus a terminal status).

use std::collections::HashMap;
use std::sync::Mutex;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio_util_shim::CancellationFlag;

use crate::errors::{AppError, ErrorCode};

/// Minimal cancellation primitive so we do not pull in tokio-util.
mod tokio_util_shim {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Default)]
    pub struct CancellationFlag(Arc<AtomicBool>);

    impl CancellationFlag {
        pub fn cancel(&self) {
            self.0.store(true, Ordering::SeqCst);
        }
        pub fn is_cancelled(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
    }
}

#[derive(Default)]
pub struct ChatStreams {
    active: Mutex<HashMap<String, CancellationFlag>>,
}

impl ChatStreams {
    fn register(&self, stream_id: &str) -> CancellationFlag {
        let flag = CancellationFlag::default();
        self.active
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(stream_id.to_string(), flag.clone());
        flag
    }

    fn unregister(&self, stream_id: &str) {
        self.active
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(stream_id);
    }

    pub fn cancel(&self, stream_id: &str) {
        if let Some(flag) = self
            .active
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(stream_id)
        {
            flag.cancel();
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatRequestMessage {
    pub role: String,
    pub content: String,
}

/// Event payload shared with the frontend (mirrors ipc.ts ChatStreamEvent).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    Chunk { delta: String },
    Done { finish_reason: String },
    Failed { error: AppError },
}

fn event_name(stream_id: &str) -> String {
    format!("chat-stream:{stream_id}")
}

/// Perform the streaming request against llama-server and forward chunks as
/// Tauri events. Terminates with exactly one Done or Failed event.
pub async fn run_stream(
    app: tauri::AppHandle,
    streams: &ChatStreams,
    stream_id: String,
    base_url: String,
    api_key: String,
    messages: Vec<ChatRequestMessage>,
) {
    let flag = streams.register(&stream_id);
    let event = event_name(&stream_id);

    let result = stream_inner(&app, &event, &flag, &base_url, &api_key, messages).await;

    let terminal = match result {
        Ok(finish_reason) => ChatStreamEvent::Done { finish_reason },
        Err(e) => {
            if flag.is_cancelled() {
                ChatStreamEvent::Done {
                    finish_reason: "cancelled".to_string(),
                }
            } else {
                ChatStreamEvent::Failed { error: e }
            }
        }
    };
    let _ = app.emit(&event, terminal);
    streams.unregister(&stream_id);
}

async fn stream_inner(
    app: &tauri::AppHandle,
    event: &str,
    flag: &CancellationFlag,
    base_url: &str,
    api_key: &str,
    messages: Vec<ChatRequestMessage>,
) -> Result<String, AppError> {
    let body = serde_json::json!({
        "messages": messages
            .iter()
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect::<Vec<_>>(),
        "stream": true,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/v1/chat/completions"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::GenerationFailed, Some(format!("request: {e}"))))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::new(ErrorCode::UnauthorizedLocalRequest, None));
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let code = if text.contains("context") || status.as_u16() == 400 {
            ErrorCode::GenerationFailed
        } else {
            ErrorCode::UnknownRuntimeError
        };
        return Err(AppError::new(code, Some(format!("{status}: {text}"))));
    }

    let mut finish_reason = "stop".to_string();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if flag.is_cancelled() {
            // Dropping the stream closes the connection; llama-server halts
            // the slot on disconnect (verified in the Phase 3 spike).
            return Err(AppError::new(ErrorCode::GenerationCancelled, None));
        }
        let bytes = chunk
            .map_err(|e| AppError::new(ErrorCode::GenerationFailed, Some(format!("stream: {e}"))))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Parse complete SSE lines from the buffer.
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer.drain(..=pos);

            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data == "[DONE]" {
                return Ok(finish_reason);
            }
            let Ok(json) = serde_json::from_str::<serde_json::Value>(data) else {
                continue;
            };
            if let Some(choice) = json["choices"].get(0) {
                if let Some(delta) = choice["delta"]["content"].as_str() {
                    if !delta.is_empty() {
                        let _ = app.emit(
                            event,
                            ChatStreamEvent::Chunk {
                                delta: delta.to_string(),
                            },
                        );
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    finish_reason = reason.to_string();
                }
            }
        }
    }

    Ok(finish_reason)
}
