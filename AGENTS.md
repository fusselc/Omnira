# Agent Guidance

This file is the canonical repository-wide instruction source for coding agents working on Omnira.

## Project Scope

- Shipped scope on `main` is Windows-first local GGUF chat only (Vulkan/CPU
  managed `llama-server`), plus model registry, local persistence, Settings, and
  Advanced Diagnostics.
- The runtime stack is Tauri 2, React, TypeScript, Tailwind CSS, and a Rust core.
- The Rust core owns process supervision, SQLite persistence, config, and typed IPC.
- Do not add a Python runtime, FastAPI orchestrator, PyInstaller pipeline, or `backend/` directory. See `docs/adr/0001-rust-tauri-core-orchestrator.md`.
- Do not add telemetry, accounts, cloud sync, model downloads, or default external network calls.

## Capability map and phase order

Before starting post-alpha work, read:

- [docs/capability-map.md](docs/capability-map.md) -- navigation and status index
- [docs/roadmap.md](docs/roadmap.md) -- authoritative phase order
- [docs/runtimes-and-routing.md](docs/runtimes-and-routing.md) -- runtime pillars and providers
- [docs/privacy.md](docs/privacy.md) -- local-first and network defaults

The capability map is an index only. If it conflicts with ADRs, provider
contracts, privacy rules, packaging docs, or the roadmap, those documents win.

**Next approved engineering:** Phase 6 CUDA acceleration for the existing
ChatProvider only. Do not implement or expose Phase 7+ UI until that phase's
done criteria are met.

**Do not** claim or ship as current features: image generation, Create screen,
multimodal chat / file understanding, document chat, RAG, agents, voice, video,
music, plugins, web search, model downloading, or cloud capabilities.

**Do not** silently extend the current text `ChatProvider` with attachments or
multimodal understanding. That capability requires an explicit design decision
(see the capability map).

## Runtime And Data Rules

- Do not commit `llama-server` binaries.
- Do not commit GGUF model files.
- Runtime data belongs under `%LOCALAPPDATA%\Omnira\`.
- Models are referenced in place by default.
- `llama-server` must be loopback-only and protected by a per-session API key.
- No silent downloads of runtimes or models.

## UI And Product Rules

- Active screens on `main`: Chat, Models, Settings, and Advanced Diagnostics only.
- Keep deferred and later features out of the active UI until their phase is
  complete by its documented criteria (Phase 7 Create is deferred).
- Main UI copy should say "Running locally", not "GPU accelerated".
- Advanced Diagnostics may name runtime variants such as Vulkan or CPU today;
  CUDA only after Phase 6 lands on `main`.

## Agent / tool safety (future guardrail only)

This does not authorize implementing agents, tools, workflows, or plugins now.
When those features are designed: by default, no agent, workflow, plugin, or
tool may cause an external side effect without explicit per-action user
approval. Side effects include file writes/deletes, shell/process execution,
browser/network actions, clipboard writes, and settings/model/runtime changes.
Any future persistent permission must be narrow, visible, revocable, and
documented. See [docs/capability-map.md](docs/capability-map.md) and
[docs/roadmap.md](docs/roadmap.md).

## Documentation Rules

- Keep docs canonical before implementation work.
- Update architecture, security, data, packaging, and roadmap docs when implementation decisions change.
- Avoid duplicating project rules across editor-specific files; those files should defer to this one.
- Prefer updating the capability map status row when a phase ships, but keep
  detailed contracts in their authoritative documents.
