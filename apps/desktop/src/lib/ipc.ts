/**
 * Typed IPC contract with the Rust core. Mirrors src-tauri/src/types.rs and
 * src-tauri/src/errors.rs one-to-one. All non-chat functionality goes through
 * these commands; chat streaming goes through lib/chat.ts.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { isMockMode, mockInvoke } from "./mock";

/** Dispatch to the real Rust core, or the in-memory mock outside Tauri. */
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isMockMode) return mockInvoke(cmd, args) as Promise<T>;
  return tauriInvoke<T>(cmd, args);
}

// ---------------------------------------------------------------------------
// Error taxonomy (docs/chat-provider.md section 6)
// ---------------------------------------------------------------------------

export type ErrorCode =
  | "RuntimeMissing"
  | "RuntimeFailedToStart"
  | "ModelFileMissing"
  | "ModelFormatInvalid"
  | "ModelLoadFailed"
  | "InsufficientMemory"
  | "GenerationCancelled"
  | "GenerationFailed"
  | "BackendUnavailable"
  | "UnauthorizedLocalRequest"
  | "UnknownRuntimeError";

export interface AppError {
  code: ErrorCode;
  message: string;
  suggested_action: string;
  detail: string | null;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "code" in e &&
    "message" in e &&
    "suggested_action" in e
  );
}

/** Normalize any thrown value into an AppError (streaming module ownership,
 *  docs/architecture.md section 8). */
export function toAppError(e: unknown): AppError {
  if (isAppError(e)) return e;
  return {
    code: "UnknownRuntimeError",
    message: "Something unexpected went wrong.",
    suggested_action: "Try again; details are in Advanced Diagnostics.",
    detail: e instanceof Error ? e.message : String(e),
  };
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ModelStatus = "ok" | "missing" | "invalid";

export interface ModelEntry {
  id: string;
  name: string;
  path: string;
  file_size_bytes: number;
  trained_context_length: number | null;
  last_used_at: string | null;
  added_at: string;
  status: ModelStatus;
}

export interface Conversation {
  id: string;
  title: string;
  model_id: string | null;
  created_at: string;
  updated_at: string;
}

export type MessageRole = "user" | "assistant";
export type MessageStatus = "complete" | "interrupted";

export interface Message {
  id: string;
  conversation_id: string;
  role: MessageRole;
  content: string;
  status: MessageStatus;
  created_at: string;
}

export type RuntimeVariant = "vulkan" | "cpu";
export type RuntimeState = "stopped" | "starting" | "ready" | "error";

export interface RuntimeStatus {
  state: RuntimeState;
  variant: RuntimeVariant | null;
  accelerator_label: string | null;
  fallback_reason: string | null;
  model_id: string | null;
  port: number | null;
  context_size: number | null;
  last_error: AppError | null;
}

export interface ChatEndpoint {
  base_url: string;
  api_key: string;
  context_chars_budget: number;
}

export interface Settings {
  theme: string;
  runtime_path_override: string | null;
  preferred_runtime_variant: RuntimeVariant | null;
  onboarding_complete: boolean;
}

export interface DiagnosticsSnapshot {
  app_version: string;
  runtime: RuntimeStatus;
  data_dir: string;
  config_path: string;
  db_path: string;
  log_dir: string;
  recent_log_lines: string[];
  recent_errors: AppError[];
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

export const ipc = {
  // Settings
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),

  // Models
  listModels: () => invoke<ModelEntry[]>("list_models"),
  addModel: (path: string) => invoke<ModelEntry>("add_model", { path }),
  removeModel: (id: string) => invoke<void>("remove_model", { id }),

  // Conversations
  listConversations: () => invoke<Conversation[]>("list_conversations"),
  createConversation: (title: string, modelId: string | null) =>
    invoke<Conversation>("create_conversation", { title, modelId }),
  renameConversation: (id: string, title: string) =>
    invoke<void>("rename_conversation", { id, title }),
  setConversationModel: (id: string, modelId: string) =>
    invoke<void>("set_conversation_model", { id, modelId }),
  deleteConversation: (id: string) => invoke<void>("delete_conversation", { id }),
  clearConversations: () => invoke<void>("clear_conversations"),

  // Messages (stream-boundary persistence contract)
  listMessages: (conversationId: string) =>
    invoke<Message[]>("list_messages", { conversationId }),
  addMessage: (
    conversationId: string,
    role: MessageRole,
    content: string,
    status: MessageStatus,
  ) => invoke<Message>("add_message", { conversationId, role, content, status }),

  // Runtime
  runtimeStatus: () => invoke<RuntimeStatus>("runtime_status"),
  chatEndpoint: () => invoke<ChatEndpoint>("chat_endpoint"),
  startRuntime: (modelId: string) =>
    invoke<RuntimeStatus>("start_runtime", { modelId }),
  stopRuntime: () => invoke<RuntimeStatus>("stop_runtime"),

  // Chat proxy fallback path
  chatStream: (streamId: string, messages: { role: string; content: string }[]) =>
    invoke<void>("chat_stream", { streamId, messages }),
  chatCancel: (streamId: string) => invoke<void>("chat_cancel", { streamId }),

  // Diagnostics
  diagnosticsSnapshot: () => invoke<DiagnosticsSnapshot>("diagnostics_snapshot"),
  diagnosticsExport: (includePaths: boolean) =>
    invoke<string>("diagnostics_export", { includePaths }),
};
