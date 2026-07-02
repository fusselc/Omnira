/**
 * In-memory mock backend, used automatically when the app runs outside Tauri
 * (plain `vite dev` in a browser). Exists for UI development and for
 * validating screens, first-run flow, empty states, and copy without a
 * runtime (roadmap Phase 2). Never active inside the packaged app.
 */
import type {
  Conversation,
  DiagnosticsSnapshot,
  Message,
  ModelEntry,
  RuntimeStatus,
  Settings,
} from "./ipc";

export const isMockMode =
  typeof window !== "undefined" &&
  !("__TAURI_INTERNALS__" in window);

let settings: Settings = {
  theme: "dark",
  runtime_path_override: null,
  preferred_runtime_variant: null,
  onboarding_complete: false,
};

const models: ModelEntry[] = [];
const conversations: Conversation[] = [];
const messages: Message[] = [];
let runtime: RuntimeStatus = {
  state: "stopped",
  variant: null,
  accelerator_label: null,
  fallback_reason: null,
  model_id: null,
  port: null,
  context_size: null,
  last_error: null,
};

const now = () => new Date().toISOString();
const id = () => crypto.randomUUID();

export async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "get_settings":
      return settings;
    case "save_settings":
      settings = args!.settings as Settings;
      return;

    case "list_models":
      return models;
    case "add_model": {
      const path = args!.path as string;
      const m: ModelEntry = {
        id: id(),
        name: path.split(/[\\/]/).pop()?.replace(/\.gguf$/, "") ?? "Model",
        path,
        file_size_bytes: 491_400_032,
        trained_context_length: 32768,
        last_used_at: null,
        added_at: now(),
        status: "ok",
      };
      models.unshift(m);
      return m;
    }
    case "remove_model": {
      const idx = models.findIndex((m) => m.id === args!.id);
      if (idx >= 0) models.splice(idx, 1);
      return;
    }

    case "list_conversations":
      return [...conversations].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    case "create_conversation": {
      const c: Conversation = {
        id: id(),
        title: args!.title as string,
        model_id: (args!.modelId as string) ?? null,
        created_at: now(),
        updated_at: now(),
      };
      conversations.push(c);
      return c;
    }
    case "rename_conversation": {
      const c = conversations.find((c) => c.id === args!.id);
      if (c) c.title = args!.title as string;
      return;
    }
    case "set_conversation_model":
      return;
    case "delete_conversation": {
      const idx = conversations.findIndex((c) => c.id === args!.id);
      if (idx >= 0) conversations.splice(idx, 1);
      return;
    }
    case "clear_conversations":
      conversations.length = 0;
      messages.length = 0;
      return;

    case "list_messages":
      return messages.filter((m) => m.conversation_id === args!.conversationId);
    case "add_message": {
      const m: Message = {
        id: id(),
        conversation_id: args!.conversationId as string,
        role: args!.role as Message["role"],
        content: args!.content as string,
        status: args!.status as Message["status"],
        created_at: now(),
      };
      messages.push(m);
      const c = conversations.find((c) => c.id === m.conversation_id);
      if (c) c.updated_at = now();
      return m;
    }

    case "runtime_status":
      return runtime;
    case "start_runtime": {
      runtime = {
        state: "ready",
        variant: "vulkan",
        accelerator_label: "GPU (Vulkan)",
        fallback_reason: null,
        model_id: args!.modelId as string,
        port: 12345,
        context_size: 8192,
        last_error: null,
      };
      const m = models.find((m) => m.id === args!.modelId);
      if (m) m.last_used_at = now();
      return runtime;
    }
    case "stop_runtime":
      runtime = { ...runtime, state: "stopped", model_id: null, port: null };
      return runtime;
    case "chat_endpoint":
      return { base_url: "http://127.0.0.1:12345", api_key: "mock", context_chars_budget: 18000 };

    case "diagnostics_snapshot": {
      const snap: DiagnosticsSnapshot = {
        app_version: "0.1.0-mock",
        runtime,
        data_dir: "C:\\Users\\you\\AppData\\Local\\Omnira",
        config_path: "C:\\Users\\you\\AppData\\Local\\Omnira\\config\\settings.json",
        db_path: "C:\\Users\\you\\AppData\\Local\\Omnira\\data\\omnira.db",
        log_dir: "C:\\Users\\you\\AppData\\Local\\Omnira\\logs",
        recent_log_lines: [
          "2026-07-01 12:00:00.000 [INFO] app.start 0.1.0",
          "2026-07-01 12:00:05.132 [INFO] runtime.spawn variant=Vulkan attempt=1 port=12345",
          "2026-07-01 12:00:09.410 [INFO] runtime.ready variant=Vulkan port=12345",
        ],
        recent_errors: [],
      };
      return snap;
    }
    case "diagnostics_export":
      return "C:\\Users\\you\\AppData\\Local\\Omnira\\diagnostics\\omnira-diagnostics-mock.json";

    default:
      throw new Error(`mock backend: unknown command ${cmd}`);
  }
}

/** Canned streamed response for browser-mode chat validation. */
export function mockStream(
  onChunk: (delta: string) => void,
  onDone: () => void,
): () => void {
  const text =
    "This is a **mock response** streamed by the in-browser backend. " +
    "In the packaged app, this text is generated locally by your model.";
  const words = text.split(" ");
  let i = 0;
  const t = setInterval(() => {
    if (i >= words.length) {
      clearInterval(t);
      onDone();
      return;
    }
    onChunk(words[i] + " ");
    i++;
  }, 60);
  return () => {
    clearInterval(t);
    onDone();
  };
}
