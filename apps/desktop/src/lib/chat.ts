/**
 * The dedicated chat streaming module (docs/architecture.md section 8).
 *
 * Owns:
 * - the direct loopback SSE path (primary) with AbortController cancellation
 * - the Rust chat_stream proxy path (approved CORS fallback), same contract
 * - mapping of all streaming-time failures to the error taxonomy
 * - context truncation against the Rust-provided character budget
 *
 * No other module invents chat transport or error strings.
 */
import { listen } from "@tauri-apps/api/event";
import { ipc, toAppError, type AppError, type Message } from "./ipc";
import { isMockMode, mockStream } from "./mock";

export type FinishReason = "stop" | "length" | "cancelled";

export interface StreamCallbacks {
  onChunk: (delta: string) => void;
  onDone: (finishReason: FinishReason) => void;
  onError: (error: AppError) => void;
}

export interface StreamHandle {
  cancel: () => void;
}

interface WireMessage {
  role: string;
  content: string;
}

/**
 * Oldest-first truncation against the conservative character budget
 * (docs/chat-provider.md section 5). Returns the wire messages and whether
 * older messages were dropped.
 */
export function truncateToBudget(
  history: Pick<Message, "role" | "content">[],
  budgetChars: number,
): { messages: WireMessage[]; truncated: boolean } {
  const kept: WireMessage[] = [];
  let used = 0;
  for (let i = history.length - 1; i >= 0; i--) {
    const m = history[i];
    const cost = m.content.length + 16; // small per-message overhead
    if (used + cost > budgetChars && kept.length > 0) {
      return { messages: kept, truncated: true };
    }
    kept.unshift({ role: m.role, content: m.content });
    used += cost;
  }
  return { messages: kept, truncated: false };
}

/** Once the proxy fallback engages (CORS block), stick with it for the session. */
let useProxyPath = false;

export function startStream(
  messages: WireMessage[],
  callbacks: StreamCallbacks,
): StreamHandle {
  if (isMockMode) {
    let cancelled = false;
    const cancel = mockStream(callbacks.onChunk, () =>
      callbacks.onDone(cancelled ? "cancelled" : "stop"),
    );
    return {
      cancel: () => {
        cancelled = true;
        cancel();
      },
    };
  }
  if (useProxyPath) {
    return startProxyStream(messages, callbacks);
  }
  return startDirectStream(messages, callbacks);
}

// ---------------------------------------------------------------------------
// Direct loopback SSE path (primary)
// ---------------------------------------------------------------------------

function startDirectStream(
  messages: WireMessage[],
  callbacks: StreamCallbacks,
): StreamHandle {
  const controller = new AbortController();
  let cancelled = false;

  (async () => {
    let endpoint;
    try {
      endpoint = await ipc.chatEndpoint();
    } catch (e) {
      callbacks.onError(toAppError(e));
      return;
    }

    let response: Response;
    try {
      response = await fetch(`${endpoint.base_url}/v1/chat/completions`, {
        method: "POST",
        signal: controller.signal,
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${endpoint.api_key}`,
        },
        body: JSON.stringify({ messages, stream: true }),
      });
    } catch (e) {
      if (cancelled) {
        callbacks.onDone("cancelled");
        return;
      }
      // A network-layer TypeError before any response is the CORS/webview-origin
      // block signature: engage the approved proxy fallback for this session.
      useProxyPath = true;
      const proxy = startProxyStream(messages, callbacks);
      // Re-point cancellation at the proxy stream.
      cancelProxy = proxy.cancel;
      return;
    }

    if (response.status === 401) {
      callbacks.onError({
        code: "UnauthorizedLocalRequest",
        message: "A request was blocked for your security.",
        suggested_action: "Restart Omnira if chat stops working.",
        detail: "llama-server rejected the session key",
      });
      return;
    }
    if (!response.ok || !response.body) {
      const text = await response.text().catch(() => "");
      callbacks.onError({
        code: "GenerationFailed",
        message:
          response.status === 400 && text.includes("context")
            ? "This conversation is too long for the model."
            : "The response could not be completed.",
        suggested_action: "Try sending your message again.",
        detail: `${response.status}: ${text.slice(0, 500)}`,
      });
      return;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let finishReason: FinishReason = "stop";

    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        let nl: number;
        while ((nl = buffer.indexOf("\n")) >= 0) {
          const line = buffer.slice(0, nl).trim();
          buffer = buffer.slice(nl + 1);
          if (!line.startsWith("data: ")) continue;
          const data = line.slice(6);
          if (data === "[DONE]") {
            callbacks.onDone(finishReason);
            return;
          }
          try {
            const json = JSON.parse(data);
            const choice = json.choices?.[0];
            const delta = choice?.delta?.content;
            if (typeof delta === "string" && delta.length > 0) {
              callbacks.onChunk(delta);
            }
            if (choice?.finish_reason === "length") finishReason = "length";
          } catch {
            // Ignore malformed keep-alive lines.
          }
        }
      }
      callbacks.onDone(finishReason);
    } catch (e) {
      if (cancelled) {
        callbacks.onDone("cancelled");
      } else {
        callbacks.onError(toAppError(e));
      }
    }
  })();

  let cancelProxy: (() => void) | null = null;

  return {
    cancel: () => {
      cancelled = true;
      controller.abort();
      cancelProxy?.();
    },
  };
}

// ---------------------------------------------------------------------------
// Rust chat_stream proxy path (approved fallback, same contract)
// ---------------------------------------------------------------------------

type ProxyEvent =
  | { type: "chunk"; delta: string }
  | { type: "done"; finish_reason: string }
  | { type: "failed"; error: AppError };

function startProxyStream(
  messages: WireMessage[],
  callbacks: StreamCallbacks,
): StreamHandle {
  const streamId = crypto.randomUUID();
  let unlisten: (() => void) | null = null;
  let finished = false;

  const finish = () => {
    finished = true;
    unlisten?.();
  };

  (async () => {
    unlisten = await listen<ProxyEvent>(`chat-stream:${streamId}`, (event) => {
      if (finished) return;
      const payload = event.payload;
      if (payload.type === "chunk") {
        callbacks.onChunk(payload.delta);
      } else if (payload.type === "done") {
        finish();
        callbacks.onDone(
          payload.finish_reason === "cancelled"
            ? "cancelled"
            : payload.finish_reason === "length"
              ? "length"
              : "stop",
        );
      } else {
        finish();
        callbacks.onError(payload.error);
      }
    });

    try {
      await ipc.chatStream(streamId, messages);
    } catch (e) {
      if (!finished) {
        finish();
        callbacks.onError(toAppError(e));
      }
    }
  })();

  return {
    cancel: () => {
      void ipc.chatCancel(streamId);
    },
  };
}
