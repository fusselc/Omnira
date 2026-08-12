# Omnira Roadmap

MVP first, everything else after. Each phase completes before the next begins.

Status index for agents and maintainers:
[capability-map.md](capability-map.md) (navigation only; this roadmap and other
canonical docs remain authoritative on conflict).

## Status legend

| Status | Meaning |
|---|---|
| **Shipped** | On `main` as part of the current alpha product. |
| **Next approved** | Approved for the next implementation work. |
| **Deferred** | Planned but blocked until prior work and done-criteria are met. |
| **Later** | On this table after next approved / deferred work. |
| **Unscheduled** | Direction without a fixed phase number. |
| **Requires design decision** | Needs explicit UX and/or runtime design before implementation. |
| **Requires explicit product decision** | Not planned as default product behavior. |

**Current snapshot:** Phases 0–6 are **Shipped** (Phase 6 = CUDA for the
existing ChatProvider). Phase 7 image / Create is **Deferred** (no active Create
screen until end-to-end criteria are met). Phases 8–12 are **Later**. See
[capability-map.md](capability-map.md) for the full index, including multimodal
chat / file understanding (requires design decision; no phase number yet).

## Privacy defaults

No accounts, telemetry, cloud sync, default external network calls, or silent
downloads. Network-capable features (including web search and model-download
assistance) must be off by default and require explicit, understandable
permission before use. See [privacy.md](privacy.md).

## Agent / tool safety (future design guardrail)

This does **not** authorize implementing agents, tools, workflows, or plugins
now. When Phase 10 (or related) work is designed:

- By default, **no agent, workflow, plugin, or tool may cause an external side
  effect without explicit per-action user approval.**
- Examples: file writes/deletes; shell/process execution; browser or network
  actions; clipboard writes; settings, model, or runtime changes.
- Any future **persistent** permission must be **narrow, visible, revocable,
  and documented** before it ships.

## MVP phases

### Phase 0 -- Planning and documentation (**Shipped**)

- Finalize charter, MVP scope, and non-goals.
- Write LICENSE (Apache-2.0), THIRD_PARTY_LICENSES, README, CONTRIBUTING, and
  the full docs set encoding all locked architecture decisions.
- Pin the llama.cpp release for bundled runtimes.

### Phase 1 -- Architecture skeleton (**Shipped**)

- Monorepo scaffold (`apps/desktop/` with Rust core + React frontend).
- Rust core module boundaries: process supervision, runtime lifecycle, SQLite
  layer, config, logging, diagnostics, errors, IPC command surface.
- SQLite schema, config schema, typed command/event contract shared with the
  frontend.
- Hardened Tauri configuration (CSP, navigation restrictions,
  devtools-off-in-release) from the first scaffold, not retrofitted.

### Phase 2 -- Desktop shell with mock data (**Shipped**)

- Chat, Models, Settings, and Advanced Diagnostics screens against mocked
  Tauri commands.
- Validate first-run flow, empty states, and beginner-friendly copy before any
  runtime work. Assistant output rendered as sanitized markdown with raw HTML
  disabled from the start.

### Phase 3 -- Managed llama-server local chat (**Shipped**)

- Two verification spikes first: (a) CORS/webview-origin for the direct chat
  path, (b) cancellation-on-disconnect behavior. Outcomes select the shipping
  chat path and cancellation mechanism.
- Process supervisor (Windows Job Object), port reservation + spawn + health
  gating, Vulkan -> CPU runtime selection, SSE streaming, cancellation, GGUF
  header sanity check, context-budget command, error normalization.
- Exit criteria: cancellation demonstrably halts generation on the pinned
  release; no orphaned llama-server after force-kill; port-race retry works.

### Phase 4 -- Persistence and diagnostics (**Shipped**)

- SQLite stores wired in per the stream-boundary persistence contract.
- Model registry with missing-file and invalid-file detection.
- Settings, log viewer, redacted diagnostics export, deletion flows
  (delete conversation, clear all conversations, remove model entry).

### Phase 5 -- MVP hardening and packaging (**Shipped** / alpha)

- Tauri bundling (single Rust binary, no sidecar pipeline), **NSIS installer**
  (MSI deferred post-alpha).
- Fresh-install, first-launch, model-selection, chat, shutdown, relaunch, and
  uninstall testing.
- Offline verification: no external network calls by default; full workflow
  works with networking disconnected (acceptance criterion 13).
- CSP/devtools production audit ([alpha-readiness-checklist.md](alpha-readiness-checklist.md)).
- Evaluate Windows code signing (cost, EV vs. OV certificate, CI signing
  workflow) before public alpha -- a go/no-go checklist item.
- Internal alpha tagged `v0.1.0-alpha` with maintainer Sign-Off recorded.

## Post-MVP phases

Ordered by expected user impact and dependency on the MVP shell. Each phase
adds one provider plus minimal UI; no phase ships until the prior is stable.
See `docs/runtimes-and-routing.md` for the runtime strategy behind this order
and `docs/capability-map.md` for status.

| Phase | Focus | Runtime pillar | New UI (minimum) | Status |
|---|---|---|---|---|
| 6 | CUDA llama.cpp for LLMs (existing ChatProvider only) | High-performance GPU (LLM) | None (same Chat; faster path visible in Diagnostics) | **Shipped** |
| 7 | Image generation | CUDA/TensorRT (diffusion) or managed diffusion worker | Create (image) -- only after end-to-end done criteria | **Deferred** |
| 8 | Windows ML / ONNX | Windows-native (vision, audio, NPU) | Create (vision/audio tasks) | **Later** |
| 9 | Video generation | CUDA/TensorRT | Create (video) | **Later** |
| 10 | Agents, tool calling, RAG, memory, document chat | llama.cpp + embeddings | Memory/RAG, Agents screens | **Later** |
| 11 | Voice mode (ASR, TTS) | Windows ML / ONNX + optional CUDA | Voice screen | **Later** |
| 12 | Plugin / community provider ecosystem | All pillars | Provider registry, extensibility docs | **Later** |

Phase 6 is the **first** post-MVP runtime addition: a CUDA **llama-server**
variant for the existing ChatProvider on NVIDIA machines (CUDA -> Vulkan ->
CPU). It is **not** TensorRT diffusion and does **not** add a Create screen.
Hardware-aware routing for ONNX/NPU continues in later phases.

Phase 7 remains **Deferred**. Create must not become an active visible screen
until a user can generate, see, delete, and relaunch to view a local image
without a terminal, with Job Object supervision and no orphaned worker after
force-close.

**Multimodal chat and file understanding** is not a row in this phase table.
It requires a deliberate attachment/ingestion UX and provider/runtime decision,
and must not be silently added to the current ChatProvider. See
[capability-map.md](capability-map.md) (status: Requires design decision).

### Unscheduled / design-needed (not in the numbered table)

- Hardware detection UI -- **Unscheduled**
- Model download / import assistance -- **Unscheduled** (off by default;
  explicit permission; never silent downloads)
- ComfyUI integration as a managed worker -- **Unscheduled** (never
  beginner-default UX)
- Music / audio generation -- **Unscheduled** (`MusicAudioProvider`)
- Optional web search and other network providers -- **Requires design
  decision** (off by default; permission model before ship)
- Cloud providers, accounts, sync, or telemetry as product defaults --
  **Requires explicit product decision** (not planned as default behavior)

Model display-name renaming (H3) shipped in the Models screen (registry name
only; GGUF path unchanged).

## Update strategy (placeholder -- post-MVP)

The MVP ships no self-update mechanism. Because each installer pins a specific
`llama-server` release, the Omnira version and runtime version are coupled.
Post-MVP, app update strategy, runtime upgrades, and optional CUDA
acceleration-pack distribution are one connected design topic and will be
specified together before any of them ship.
