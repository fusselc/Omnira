# ADR 0002: stable-diffusion.cpp as the Phase 7 image worker

- **Status:** Accepted (design; implementation gated by the Phase 7 spike)
- **Date:** 2026-10-23
- **Deciders:** Omnira maintainers

## Context

Phase 7 adds local image generation behind a new `ImageProvider` (contract:
[docs/image-provider.md](../image-provider.md)). The roadmap allows Phase 7
design, a headless spike, and Rust-core backend work to run in parallel with
Phase 6 CUDA, while the Create screen stays hidden until the done criteria in
[docs/roadmap.md](../roadmap.md) pass.

Constraints that any choice must satisfy:

- No Python runtime, FastAPI service, PyInstaller pipeline, or `backend/`
  directory ([ADR 0001](0001-rust-tauri-core-orchestrator.md), `AGENTS.md`).
- No default network calls, no silent downloads, models referenced in place
  ([docs/privacy.md](../privacy.md)).
- Every worker is supervised under the Windows Job Object, never left orphaned,
  and, if it exposes an API, is loopback-only and authenticated
  ([docs/runtimes-and-routing.md](../runtimes-and-routing.md) section 7,
  [docs/local-security-boundary.md](../local-security-boundary.md)).
- Runtimes ship as pinned, checksum-verified official artifacts; binaries and
  model files are never committed
  ([docs/packaging-process-model.md](../packaging-process-model.md)).
- Windows-first; hardware ranges from NVIDIA GPUs through AMD/Intel GPUs to
  CPU-only machines.

## Options considered

### (a) Managed stable-diffusion.cpp worker -- chosen

stable-diffusion.cpp (MIT license) is a C/C++ diffusion inference engine built
on ggml, the same tensor library as llama.cpp. Upstream publishes official
Windows x64 release artifacts per build (`master-<n>-<sha>` tags) for CPU
(AVX, AVX2, AVX512), Vulkan, CUDA 12, and ROCm. It ships two executables:
`sd-cli` (one generation per invocation) and `sd-server` (HTTP server).

- **Pros:** no Python; same operational model as `llama-server` (pinned
  official zip, SHA-256 verification, child process in the Job Object); CUDA,
  Vulkan, and CPU from one codebase; loads the formats users already hold
  (`.safetensors`, `.gguf`); broad model coverage (SD 1.x/2.x, SDXL, SD3,
  FLUX, Qwen-Image, and more); small CPU and Vulkan artifacts.
- **Cons:** fast-moving upstream ("API and command-line option may change
  frequently"); `sd-server` has no documented API-key authentication and
  echoes any request `Origin` in its CORS headers (see D3); progress from
  `sd-cli` is text output, not a structured protocol; multi-file models
  (FLUX, SD3) need a model-bundle concept Omnira does not have yet.

### (b) ONNX Runtime / Windows ML diffusion -- rejected for Phase 7

- **Pros:** Windows-native; DirectML and NPU acceleration on Copilot+ PCs;
  aligns with the Phase 8 Windows ML pillar.
- **Cons:** there is no turnkey native diffusion pipeline to supervise. Omnira
  would implement the tokenizer, scheduler, UNet/DiT denoising loop, and VAE
  decode in Rust. Models must be ONNX; users overwhelmingly hold
  `.safetensors`, and converting them needs Python tooling (Optimum / Olive),
  which conflicts with ADR 0001 for an end-user product. Prebuilt ONNX
  diffusion models are scarce and architecture-specific.
- **Disposition:** revisit in Phase 8 for NPU-accelerated components, not as
  the Phase 7 engine.

### (c) ComfyUI as a managed worker -- rejected for Phase 7

- **Pros:** the broadest model and workflow ecosystem; mature node graph.
- **Cons:** requires an embedded Python interpreter plus PyTorch and CUDA
  wheels (multi-GB), which is exactly the general-purpose Python sidecar ADR 0001
  rejected; custom nodes execute arbitrary Python; its HTTP/WebSocket server
  listens without authentication by default; its manager and many nodes
  perform network access; large installer and elevated antivirus/SmartScreen
  risk.
- **Disposition:** remains **Unscheduled** as a power-user managed worker,
  never the beginner default. Any future integration needs its own ADR that
  explicitly amends ADR 0001.

## Decisions

### D1. Runtime: stable-diffusion.cpp

The Phase 7 `ImageProvider` implementation is backed by stable-diffusion.cpp.

**Rationale:** it is the only option that satisfies the no-Python rule while
loading the model formats users already have, and it reuses the supervision,
pinning, and packaging model Omnira already operates for llama.cpp.

### D2. Acquisition: official pinned release artifacts only

Omnira consumes official upstream release zips, pinned by tag and SHA-256,
fetched by the packaging script and verified fail-closed, exactly like
`llama-server`. Omnira does **not** build stable-diffusion.cpp from source and
does **not** carry patches. Binaries are never committed.

**Rationale:** keeps the packaging pipeline free of a C++/CUDA/Vulkan SDK
toolchain and keeps the supply-chain story identical to the existing runtime.

### D3. Transport: per-job `sd-cli`, no listening socket

The shipping transport is **per-job `sd-cli`**: each generation is one
short-lived `sd-cli` process, spawned by the Rust core into the existing Job
Object, writing its output to a staging path the core chooses. The worker
opens **no listening socket**. Cancellation terminates the process.

`sd-server` may replace per-job mode **only** when an official pinned release
both (1) supports a per-session API key that rejects unauthenticated requests
and (2) does not reflect arbitrary `Origin` values in CORS responses. Until
then it is not shipped. Omnira may propose these options upstream. An
unauthenticated listener is never shipped, even on loopback.

**Rationale:** as documented upstream at the time of writing, `sd-server`
exposes generation endpoints and an embedded web UI with no authentication and
answers CORS preflights by echoing the caller's `Origin`. A page in the user's
browser that discovers the port could drive the worker. Per-job `sd-cli`
removes the network surface entirely, satisfies the "authenticated if it
exposes an API" guardrail vacuously, and gives the most reliable cancellation
available (process termination under the Job Object).

### D4. Data path: Rust core only; images as data URLs

The webview never talks to the image worker. All image operations go through
typed Tauri IPC; no worker port or key ever enters the webview. Finished images
are delivered to the UI as `data:` URLs by an IPC command.

**Rationale:** there is no token stream that would justify a direct webview
path, and keeping the worker behind IPC preserves Layer 1 of the security
boundary. Production CSP already allows `img-src 'self' data:`, so no CSP or
asset-protocol change is needed.

### D5. v1 model scope: single-file SD 1.x, SD 2.x, SDXL

v1 accepts single-file checkpoints for SD 1.x, SD 2.x, and SDXL (including
Turbo variants) in `.safetensors` or stable-diffusion.cpp-compatible `.gguf`
format, referenced in place. The Rust core sniffs the file header (safetensors
JSON header / GGUF metadata) to identify the architecture before any spawn.

**`.ckpt` (and `.pt` / `.pth`) is excluded** because these are pickle-based
files, and pickle deserialization can execute arbitrary code on load. Omnira
refuses the format outright rather than depending on any particular loader's
pickle handling being safe.

Multi-component models (FLUX, SD3, Qwen-Image, and similar, which need
separate diffusion, VAE, and text-encoder files) are a documented follow-up
that requires a model-bundle registry design. GGUF files must be
stable-diffusion.cpp-compatible builds; GGUF conversions made for other
loaders may fail and map to `ModelLoadFailed`.

**Rationale:** single-file models cover the most common local image models,
keep the registry one-row-per-file like the chat registry, and avoid designing
bundle UX before the pipeline is proven.

### D6. Variants and distribution

- The installer bundles the **Vulkan** and **CPU (AVX2)** `sd-cli` variants.
  As of upstream build `master-721` the zipped artifacts were about 40 MB and
  20 MB respectively (about +60 MB to the download before installer
  compression); the exact delta is re-measured when a release is pinned.
- Variant order is Vulkan, then CPU; the working variant is recorded in config,
  mirroring the chat runtime.
- The **CUDA** variant is about 336 MB zipped plus a separate CUDA runtime zip
  of about 537 MB. It is not bundled by default; it follows the Phase 6
  acceleration-pack and update-strategy decision in
  [docs/roadmap.md](../roadmap.md). Whether the CUDA runtime DLLs can be shared
  with a CUDA `llama-server` is an open question.
- No runtime is ever downloaded at runtime.

**Rationale:** Vulkan covers NVIDIA, AMD, and Intel GPUs with a small artifact;
CPU is the universal fallback; CUDA's size makes it a distribution decision,
not a Phase 7 default.

### D7. Hardware tiers (guidance; the spike measures real numbers)

| Tier | Hardware | Expected v1 experience |
|---|---|---|
| A | NVIDIA GPU with 8 GB+ VRAM | SDXL at 1024x1024 on Vulkan; CUDA later per D6 |
| B | AMD / Intel GPU, or NVIDIA with 4-6 GB VRAM | SD 1.5 / Turbo at 512x512 on Vulkan; SDXL via quantized GGUF and VAE tiling |
| C | CPU only (AVX2) | SD-Turbo / SD 1.5 at 512x512 with few steps; minutes per image; UI sets that expectation |

NPUs are not used in Phase 7 (see option b).

### D8. Output privacy: no prompt embedded in saved images

stable-diffusion.cpp embeds generation parameters (including the prompt) into
PNG output by default. Saved images must not carry the prompt: the core either
disables embedding via a CLI option (if the pinned release has one) or
re-encodes / strips PNG text chunks before moving the file out of staging.
Prompts live only in SQLite, alongside other user content.

**Rationale:** users share images; a shared file must not leak the prompt the
user typed.

## Consequences

### Positive

- No Python, no new language runtime, no listening socket.
- One supervision and packaging model across chat and image workers.
- Cancellation is authoritative (process termination).

### Tradeoffs

- **Per-job model reload.** Every generation reloads the model from disk.
  The spike measures this; see the server-mode trigger in
  [docs/phase7-implementation-plan.md](../phase7-implementation-plan.md).
- **Progress parsing** depends on `sd-cli` text output, which can change
  between upstream builds. It is pinned, covered by fixture tests, and
  re-verified on every version bump.
- **Prompt in the process command line.** Passing the prompt as an argument
  makes it visible to other processes running as the same user (for example,
  Task Manager's command-line column). This is accepted under the current threat
  model, where same-user processes can already read local data. The spike
  checks for a prompt-file option that would avoid it.
- **Upstream velocity.** Pins must be bumped deliberately and re-verified
  against the contract, not floated.

## References

- [docs/image-provider.md](../image-provider.md) -- ImageProvider contract
- [docs/phase7-implementation-plan.md](../phase7-implementation-plan.md) -- ordered PR plan
- [docs/roadmap.md](../roadmap.md) -- sequencing rule and Phase 7 done criteria
- [ADR 0001](0001-rust-tauri-core-orchestrator.md) -- no Python sidecar
- stable-diffusion.cpp: <https://github.com/leejet/stable-diffusion.cpp>
- `sd-server` API reference: <https://github.com/leejet/stable-diffusion.cpp/blob/master/examples/server/api.md>
