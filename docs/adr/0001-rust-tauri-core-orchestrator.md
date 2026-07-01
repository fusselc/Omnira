# ADR 0001: Rust/Tauri core orchestrator instead of Python/FastAPI sidecar

- **Status:** Accepted (implemented)
- **Date:** 2026-07-01
- **Deciders:** Omnira maintainers

## Context

Early Omnira planning considered a **Python/FastAPI orchestrator** packaged with
PyInstaller and launched as a **Tauri sidecar**. That process would have owned:

- spawning and supervising `llama-server`
- SQLite persistence and settings
- typed HTTP endpoints for the frontend (health, models, conversations, settings)
- optional proxying of chat SSE

The MVP product scope was always the same: install, select a local `.gguf`, run a
managed `llama-server`, stream local chat, persist conversations, and expose
diagnostics -- all local-first with no default network access.

Before implementation, the team re-evaluated whether a separate Python runtime was
necessary for that scope.

## Decision

The MVP **does not ship a Python runtime, FastAPI service, PyInstaller bundle, or
`backend/` directory**. Instead, a **Rust core inside the Tauri desktop process**
owns process supervision, SQLite, configuration, logging, diagnostics, and typed
Tauri IPC. The React frontend talks to `llama-server` over loopback HTTP/SSE for
chat (with an approved Rust proxy fallback if the webview blocks direct fetch).

## Rationale

### Packaging and distribution

- **One primary native binary** (Tauri + Rust core) plus bundled `llama-server`
  runtimes, instead of Tauri + a second interpreted runtime + PyInstaller artifacts.
- Removes PyInstaller hidden-import fragility, larger install footprint, and elevated
  antivirus / SmartScreen false-positive risk associated with packed Python
  executables on Windows.

### Process and sidecar complexity

- **One process hop** (Tauri/Rust parent -> `llama-server` child) instead of
  Tauri -> Python orchestrator -> `llama-server`.
- Windows Job Object supervision (`KILL_ON_JOB_CLOSE`) attaches at a single,
  well-defined spawn site.
- Port reservation, health gating, api-key generation, and shutdown logic live in
  one language/runtime with direct access to OS APIs.

### Functional fit

Everything the MVP orchestrator must do maps cleanly to Rust inside Tauri:

| Responsibility | Rust/Tauri approach |
|----------------|---------------------|
| Spawn/supervise `llama-server` | `tokio::process`, Windows Job Object |
| SQLite persistence | `rusqlite` |
| Settings | JSON file under `%LOCALAPPDATA%\Omnira\` |
| Frontend state/commands | Typed Tauri `invoke` commands |
| Chat streaming | Direct loopback SSE or Rust `chat_stream` proxy |
| GGUF sanity check | Header/metadata parser in Rust |

No Python-specific ML library was required for MVP GGUF chat via `llama-server`.

### Architecture principle preserved

The **layering principle is unchanged**; only the orchestrator implementation
changed:

```text
UI (React)
  -> local orchestrator (Rust core in Tauri)
    -> ChatProvider (LlamaServerChatProvider for MVP)
      -> managed llama-server (Vulkan or CPU)
        -> local GGUF file (referenced in place)
```

Non-chat concerns (conversations, settings, model registry, diagnostics) flow
through Tauri IPC to the Rust core. Chat uses the OpenAI-compatible
`/v1/chat/completions` endpoint on loopback, with session api-key protection.

## Consequences

### Positive

- Simpler install tree and fewer moving parts at runtime.
- Lower packaging/AV risk than a PyInstaller sidecar.
- Stronger alignment with Tauri's security model (IPC boundary + capabilities).
- Clear module seams (`process`, `runtime`, `storage`, `config`, `commands`) for
  post-MVP providers without redesigning the shell.

### Tradeoffs

- Rust expertise is required for core orchestration changes (acceptable for a
  Tauri-first project).
- If a **future feature** genuinely needs Python's ML ecosystem (e.g. a specialized
  training or conversion pipeline), that should be a **deliberate, feature-scoped**
  addition -- not a restoration of a general-purpose sidecar by default.

## Future alternatives

**FastAPI (or another Python HTTP service) may be reconsidered later** if there is
a clear, documented need that Rust cannot reasonably own -- for example, a
post-MVP provider that depends on Python-only libraries and is isolated behind a
narrow provider interface. Any such addition must:

- justify why it cannot live in the Rust core or a native worker
- preserve loopback-only defaults and explicit user consent for any network use
- not reintroduce a general "backend for everything" sidecar

Until then, the Rust/Tauri core remains the single orchestrator for Omnira MVP
and alpha releases.

## References

- [docs/architecture.md](../architecture.md) -- current MVP architecture
- [docs/packaging-process-model.md](../packaging-process-model.md) -- process and installer model
- [docs/local-security-boundary.md](../local-security-boundary.md) -- IPC and webview hardening
- [docs/runtimes-and-routing.md](../runtimes-and-routing.md) -- post-MVP provider growth (not MVP scope)
