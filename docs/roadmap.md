# Omnira Roadmap

MVP first, everything else after. Each phase completes before the next begins,
with one explicit exception: Phase 6 and Phase 7 may proceed in parallel under
the [sequencing rule](#sequencing-rule-phase-6-and-phase-7) below.

Status index for agents and maintainers:
[capability-map.md](capability-map.md) (navigation only; this roadmap and other
canonical docs remain authoritative on conflict).

## Status legend

| Status | Meaning |
|---|---|
| **Shipped** | On `main` as part of the current alpha product. |
| **Next approved** | Approved for the next implementation work. |
| **Next approved (design + headless spike)** | Scoped approval: design docs, a headless spike, Rust-core backend work, and a dev-build-only hidden route. No user-visible feature ships until the phase's done criteria pass. |
| **Deferred** | Planned but blocked until prior work and done-criteria are met. |
| **Later** | On this table after next approved / deferred work. |
| **Unscheduled** | Direction without a fixed phase number. |
| **Requires design decision** | Needs explicit UX and/or runtime design before implementation. |
| **Requires explicit product decision** | Not planned as default product behavior. |

**Current snapshot:** Phases 0–5 / packaged alpha are **Shipped**. Phase 6 CUDA
for the existing ChatProvider is **Next approved**. Phase 7 image generation is
**Next approved (design + headless spike)**, in parallel with Phase 6; Create
is not a visible screen until the [Phase 7 done
criteria](#phase-7-done-criteria) are met.
Phases 8–12 are **Later**. See [capability-map.md](capability-map.md) for the
full index, including multimodal chat / file understanding (requires design
decision; no phase number yet).

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
adds one provider plus minimal UI; no phase ships until the prior is stable
(Phase 6 / Phase 7 parallel work follows the sequencing rule below).
See `docs/runtimes-and-routing.md` for the runtime strategy behind this order
and `docs/capability-map.md` for status.

| Phase | Focus | Runtime pillar | New UI (minimum) | Status |
|---|---|---|---|---|
| 6 | CUDA llama.cpp for LLMs (existing ChatProvider only) | High-performance GPU (LLM) | None (same Chat; faster path visible in Diagnostics) | **Next approved** |
| 7 | Image generation | Managed stable-diffusion.cpp worker ([ADR 0002](adr/0002-image-provider-worker.md)) | Create (image) -- only after the Phase 7 done criteria | **Next approved (design + headless spike)** |
| 8 | Windows ML / ONNX | Windows-native (vision, audio, NPU) | Create (vision/audio tasks) | **Later** |
| 9 | Video generation | CUDA/TensorRT | Create (video) | **Later** |
| 10 | Agents, tool calling, RAG, memory, document chat | llama.cpp + embeddings | Memory/RAG, Agents screens | **Later** |
| 11 | Voice mode (ASR, TTS) | Windows ML / ONNX + optional CUDA | Voice screen | **Later** |
| 12 | Plugin / community provider ecosystem | All pillars | Provider registry, extensibility docs | **Later** |

Phase 6 is the **first** post-MVP runtime addition: a CUDA **llama-server**
variant for the existing ChatProvider on NVIDIA machines (CUDA -> Vulkan ->
CPU). It is **not** TensorRT diffusion and does **not** add a Create screen.
Hardware-aware routing for ONNX/NPU continues in later phases.

### Sequencing rule: Phase 6 and Phase 7

- Phase 6 (CUDA for the existing ChatProvider) and Phase 7 (image generation)
  may proceed **in parallel**.
- Until the Phase 7 done criteria below pass, Phase 7 work is limited to:
  design documents, the headless worker spike, pinning the worker in the
  packaging fetch script, Rust-core backend work (supervision, storage, IPC),
  and a **dev-build-only hidden** Create route
  with no navigation entry, excluded from release bundles.
- **Create is not a visible screen** in any release build until every Phase 7
  done criterion is met.
- Phase 7 PRs must not regress Chat or Phase 6. Changes to shared seams
  (`process/`, the packaging fetch script, the error taxonomy) are reviewed
  against both phases.
- Phase 7 follows the ordered steps in
  [phase7-implementation-plan.md](phase7-implementation-plan.md); the runtime
  decision is [ADR 0002](adr/0002-image-provider-worker.md) and the contract is
  [image-provider.md](image-provider.md).
- Phases 8 and later remain **Later**; this exception does not extend to them.

### Phase 7 done criteria

Create may become an active visible screen only when all of these are verified
and recorded with maintainer sign-off:

1. On an installed NSIS build, without a terminal, a user can add a local image
   model (referenced in place), enter a prompt, generate, and see the image in
   Create.
2. Progress is visible during generation, and Cancel stops the worker within
   2 seconds with no worker process remaining.
3. The image is stored under `%LOCALAPPDATA%\Omnira\images\` with a SQLite
   row, and it is still visible after quitting and relaunching Omnira.
4. Deleting one image and Clear all remove both the files and the rows.
   Removing an image model deletes neither the model file nor its images.
5. Force-killing `omnira.exe` mid-generation leaves no orphaned worker process
   (Job Object supervision), and the next launch cleans the staging directory.
6. Missing runtime, missing model file, invalid or unsupported model, memory
   exhaustion, and cancellation each map to exactly one error-taxonomy code
   with friendly copy.
7. The full flow works with networking disconnected, and no worker network
   activity is observed.
8. No listening socket exists during generation (per-job mode), or, if server
   mode has replaced it under ADR 0002 D3, only a loopback listener protected
   by a per-session key.
9. Logs and the redacted diagnostics export contain no prompt text and no
   unredacted user paths.
10. Saved images do not embed the prompt.
11. Main UI copy says "Running locally"; runtime variant names appear only in
    Advanced Diagnostics.
12. The Chat regression checklist and Phase 6 behavior pass unchanged.
13. CI is green, including Windows CI, with automated tests for image storage,
    startup reconciliation, IPC types, and error mapping.
14. THIRD_PARTY_LICENSES, release notes, the release QA checklist,
    the capability map, this roadmap, and `AGENTS.md` are updated.

**Multimodal chat and file understanding** is not a row in this phase table.
It requires a deliberate attachment/ingestion UX and provider/runtime decision,
and must not be silently added to the current ChatProvider. See
[capability-map.md](capability-map.md) (status: Requires design decision).

### Unscheduled / design-needed (not in the numbered table)

- Hardware detection UI -- **Unscheduled**
- Model download / import assistance -- **Unscheduled** (off by default;
  explicit permission; never silent downloads)
- ComfyUI integration as a managed worker -- **Unscheduled** (never
  beginner-default UX; rejected as the Phase 7 engine by
  [ADR 0002](adr/0002-image-provider-worker.md); would need its own ADR
  amending ADR 0001)
- Music / audio generation -- **Unscheduled** (`MusicAudioProvider`)
- Optional web search and other network providers -- **Requires design
  decision** (off by default; permission model before ship)
- Cloud providers, accounts, sync, or telemetry as product defaults --
  **Requires explicit product decision** (not planned as default behavior)
- Chat runtime `--ctx-size` is hardcoded to 8192 and only clamped *down* to a
  model's trained length when that is smaller. Models trained above 8192
  (for example 131072) are silently truncated. Follow-up: a Settings-exposed
  context size, capped by the model's trained length -- do not change the
  runtime default until that lands.

Model display-name renaming (H3) shipped in the Models screen (registry name
only; GGUF path unchanged).

## Update strategy (placeholder -- post-MVP)

The MVP ships no self-update mechanism. Because each installer pins a specific
`llama-server` release, the Omnira version and runtime version are coupled.
Post-MVP, app update strategy, runtime upgrades, and optional CUDA
acceleration-pack distribution are one connected design topic and will be
specified together before any of them ship.
