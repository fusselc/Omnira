# Omnira Roadmap

MVP first, everything else after. Each phase completes before the next begins.

## MVP phases

### Phase 0 -- Planning and documentation (this phase)

- Finalize charter, MVP scope, and non-goals.
- Write README, CONTRIBUTING, LICENSE (Apache-2.0), THIRD_PARTY_LICENSES, and
  the full docs set encoding all locked architecture decisions.
- Pin the llama.cpp release for bundled runtimes.

### Phase 1 -- Architecture skeleton

- Monorepo scaffold (`apps/desktop/` with Rust core + React frontend).
- Rust core module boundaries: process supervision, runtime lifecycle, SQLite
  layer, config, logging, diagnostics, errors, IPC command surface.
- SQLite schema, config schema, typed command/event contract shared with the
  frontend.
- Hardened Tauri configuration (CSP, navigation restrictions,
  devtools-off-in-release) from the first scaffold, not retrofitted.

### Phase 2 -- Desktop shell with mock data

- Chat, Models, Settings, and Advanced Diagnostics screens against mocked
  Tauri commands.
- Validate first-run flow, empty states, and beginner-friendly copy before any
  runtime work. Assistant output rendered as sanitized markdown with raw HTML
  disabled from the start.

### Phase 3 -- Managed llama-server local chat

- Two verification spikes first: (a) CORS/webview-origin for the direct chat
  path, (b) cancellation-on-disconnect behavior. Outcomes select the shipping
  chat path and cancellation mechanism.
- Process supervisor (Windows Job Object), port reservation + spawn + health
  gating, Vulkan -> CPU runtime selection, SSE streaming, cancellation, GGUF
  header sanity check, context-budget command, error normalization.
- Exit criteria: cancellation demonstrably halts generation on the pinned
  release; no orphaned llama-server after force-kill; port-race retry works.

### Phase 4 -- Persistence and diagnostics

- SQLite stores wired in per the stream-boundary persistence contract.
- Model registry with missing-file and invalid-file detection.
- Settings, log viewer, redacted diagnostics export, deletion flows
  (delete conversation, clear all conversations, remove model entry).

### Phase 5 -- MVP hardening and packaging

- Tauri bundling (single Rust binary, no sidecar pipeline), NSIS/MSI installer.
- Fresh-install, first-launch, model-selection, chat, shutdown, relaunch, and
  uninstall testing.
- Offline verification: no external network calls by default; full workflow
  works with networking disconnected (acceptance criterion 13).
- CSP/devtools production audit.
- Evaluate Windows code signing (cost, EV vs. OV certificate, CI signing
  workflow) before public alpha -- a go/no-go checklist item.

## Post-MVP phases

Ordered by expected user impact and dependency on the MVP shell. Each phase
adds one provider plus minimal UI; no phase ships until the prior is stable.
See `docs/runtimes-and-routing.md` for the runtime strategy behind this order.

| Phase | Focus | Runtime pillar | New UI (minimum) |
|---|---|---|---|
| 6 | CUDA llama.cpp for LLMs | High-performance GPU (LLM) | None (same Chat; faster path visible in Diagnostics) |
| 7 | Image generation | CUDA/TensorRT (diffusion) or managed diffusion worker | Create (image) |
| 8 | Windows ML / ONNX | Windows-native (vision, audio, NPU) | Create (vision/audio tasks) |
| 9 | Video generation | CUDA/TensorRT | Create (video) |
| 10 | Agents, tool calling, RAG, memory | llama.cpp + embeddings | Memory/RAG, Agents screens |
| 11 | Voice mode (ASR, TTS) | Windows ML / ONNX + optional CUDA | Voice screen |
| 12 | Plugin / community provider ecosystem | All pillars | Provider registry, extensibility docs |

CUDA for LLMs (Phase 6) is the **first** post-MVP runtime addition -- it is the
single biggest expected performance gap for NVIDIA users on the MVP's Vulkan
path. Hardware-aware routing (prefer NPU on Copilot+ PCs for ONNX, prefer CUDA
on NVIDIA for heavy workloads, Vulkan/CPU fallback) is introduced incrementally
starting Phase 6.

Other post-MVP capabilities, unscheduled: hardware detection UI, model download
assistance, ComfyUI integration as a managed worker, local knowledge base,
music generation, community provider registry.

## Update strategy (placeholder -- post-MVP)

The MVP ships no self-update mechanism. Because each installer pins a specific
`llama-server` release, the Omnira version and runtime version are coupled.
Post-MVP, app update strategy, runtime upgrades, and optional CUDA
acceleration-pack distribution are one connected design topic and will be
specified together before any of them ship.
