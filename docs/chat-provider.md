# ChatProvider -- MVP Provider Contract

The MVP defines exactly one provider abstraction: **ChatProvider**, with one
implementation: **LlamaServerChatProvider**. Do not design a universal provider
API in MVP. Long-term capability interfaces (`EmbeddingProvider`,
`ImageProvider`, `SpeechToTextProvider`, `TextToSpeechProvider`,
`VisionProvider`, `OnnxProvider`, `RagProvider`, `ToolAgentProvider`,
`WorkflowProvider`, `VideoProvider`, `MusicAudioProvider`,
`WebSearchProvider`) are documented in `docs/runtimes-and-routing.md` only.

## 1. Responsibilities

A ChatProvider describes:

- Provider name and version
- Health check
- Supported model formats (MVP: GGUF only)
- Selected model metadata
- Load/attach model behavior (start the managed runtime against a model file)
- Unload/stop runtime behavior
- Chat completion behavior (streaming)
- Cancellation behavior
- Structured error reporting against the taxonomy below

The provider owns spawn/supervise/stream/cancel/error-reporting for its
runtime worker. The Rust core owns registry, config, persistence, and IPC.

## 2. Lifecycle

```
Initialize -> Health Check -> Select GGUF Model -> Start llama-server
    -> Ready -> Stream Chat -> Cancel or Complete -> Stop Runtime
    -> Error State when needed
```

State transitions surface to the frontend as typed events so the UI can show
plain-language status ("Starting model...", "Ready", "Running locally").

## 3. LlamaServerChatProvider specifics

- Runtime variants: Vulkan first, CPU/AVX2 fallback (see
  `docs/architecture.md`). The working variant is recorded in config.
- Startup: reserve loopback port, spawn with
  `--host 127.0.0.1 --port <port> --api-key <session-secret> --ctx-size <n>`,
  assign to the Windows Job Object, poll `/health` until ready or timeout.
- Chat: OpenAI-compatible `/v1/chat/completions` with `stream: true`. The chat
  template embedded in GGUF metadata is applied by llama-server; Omnira never
  implements prompt templating.
- Before startup, the GGUF header is sanity-checked (magic bytes, version,
  basic metadata including trained context length). Invalid files fail fast
  with `ModelFormatInvalid`.

## 4. Streaming and cancellation

Primary path: the frontend consumes the SSE stream directly over loopback and
cancels via `AbortController`. Fallback path (if the Phase 3 CORS spike blocks
direct fetch): Rust `chat_stream` command forwarding chunks as Tauri events,
with `chat_cancel`. Both paths share one frontend contract: a stream of token
chunks plus a terminal status (completed, cancelled, failed).

**Cancellation verification is a Phase 3 acceptance criterion.** It must be
demonstrated against the pinned llama-server release that closing the SSE
connection halts generation (observable via slot state / processing activity),
mid-prefill and mid-decode. If disconnect-abort is unreliable in any observed
case, the escalation ladder applies in order:

1. Explicit cancellation via the server's slot/task cancellation API, if the
   pinned release exposes one.
2. Rust-side control path: route chat through `chat_stream` so the Rust core
   owns the connection and can terminate it authoritatively.
3. Last resort: restart the llama-server process on cancel (acceptable UX cost
   in a single-user, single-generation MVP, only if 1 and 2 fail).

**Spike outcome (recorded):** verified against the pinned release (b9859).
Closing the connection mid-generation frees the generation slot: after
abandoning a long streaming request following the first chunk, a subsequent
short request on the same single-slot server completed in ~6 seconds
(including prefill), far below the still-generating threshold. **Client
disconnect via AbortController ships as the cancellation mechanism.** The
Rust `chat_cancel` proxy path remains available as escalation step 2, and is
exercised automatically if the proxy chat path is in use.

## 5. Context budget

The Rust core exposes `context_chars_budget` via a Tauri command: a
conservative character budget derived from the runtime `--ctx-size` using a
fixed ~3 chars/token approximation, with headroom reserved for the response.
The frontend truncates conversation history oldest-first against this budget
before sending and shows a subtle "earlier messages not included" notice.
No tokenizer dependency in MVP; approximate-but-conservative is the accepted
tradeoff. If truncation still overflows, the llama-server error maps to a
friendly "conversation too long" message.

## 6. Error taxonomy

Every user-facing failure maps to exactly one of these codes. No ad hoc error
strings anywhere in the codebase.

| Code | Meaning | Normalized by |
|---|---|---|
| `RuntimeMissing` | Bundled/configured llama-server binary not found | Rust core |
| `RuntimeFailedToStart` | Spawn or health check failed (incl. port-race retries exhausted) | Rust core |
| `ModelFileMissing` | Registered model file moved or deleted | Rust core |
| `ModelFormatInvalid` | GGUF header sanity check failed | Rust core |
| `ModelLoadFailed` | Runtime started but could not load the model | Rust core |
| `InsufficientMemory` | Runtime failed due to memory exhaustion | Rust core |
| `GenerationCancelled` | User stopped generation | Streaming module |
| `GenerationFailed` | Stream failed mid-generation | Streaming module |
| `BackendUnavailable` | Rust core/runtime not reachable when expected | Rust core |
| `UnauthorizedLocalRequest` | Request rejected for missing/invalid session key | Streaming module |
| `UnknownRuntimeError` | Anything unclassified | Rust core |

Each error carries: internal code, friendly user message, suggested action,
technical detail (for Advanced Diagnostics), and a log entry. Friendly copy for
each code is defined in `docs/design-principles.md`.
