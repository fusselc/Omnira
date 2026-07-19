# Packaging and Process Model

Packaging is a first-class architecture decision. End users must never install
Python, run pip, use a terminal, start services manually, or edit config files.

## 1. Process model

One installed application, one process tree:

1. The user launches Omnira (a single Tauri executable; the Rust core is inside
   this process -- there is no sidecar backend).
2. On model selection, the Rust core reserves a free loopback port, generates a
   per-session api-key in memory, and spawns the bundled `llama-server` as a
   direct child process with `--host 127.0.0.1 --port <port> --api-key <key>`.
3. The child is assigned to a Windows Job Object with
   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` at spawn time, guaranteeing cleanup even
   if Omnira crashes or is force-killed.
4. The Rust core polls `/health` until the runtime is ready, then enables chat.
5. Selecting a different model (or quitting) stops the child; quitting Omnira
   shuts down the entire tree.

No fixed ports. No port files. No orphaned processes.

## 2. Bundled runtime

Two pinned llama.cpp `llama-server` Windows builds ship in the installer:

- **Vulkan x64** -- GPU acceleration on NVIDIA, AMD, and Intel GPUs.
- **CPU x64 (AVX2)** -- universal fallback.

Selection: try Vulkan first; on health-check failure (missing/old drivers),
fall back to CPU automatically and record the working variant in config.
The runtime manager treats variants as additive data so future backends (CUDA)
require no architectural change.

Current pin: llama.cpp release tag `b9859`
(commit `4fc4ec5541b243957ae5099edb67372f8f3b550e`). Artifact names and SHA-256
checksums are recorded in `THIRD_PARTY_LICENSES` and in the packaging script.

## 3. Runtime acquisition -- binaries are never committed

**Do not commit `llama-server` binaries or model files to git.** This is
enforced via `.gitignore` and review.

Runtime binaries are acquired exclusively through
`scripts/packaging/fetch-llama-server.ps1`, which:

1. Downloads the pinned release artifacts from the official llama.cpp GitHub
   release.
2. Verifies each artifact against its expected SHA-256 checksum.
3. **Fails closed on mismatch:** if verification fails, the script exits with
   an error and removes the unverified download. The build must refuse to
   bundle or distribute the runtime until a valid checksum matches the pinned
   artifact. No silent fallback, no partial packaging.
4. Extracts the binaries into `apps/desktop/src-tauri/binaries/` (gitignored)
   for bundling as Tauri resources.

Dev mode uses the same script, or a user-supplied runtime path via Settings.

## 4. Installer

- Tauri bundler producing an **NSIS installer** for MVP alpha.
  **MSI is deferred** until after alpha; NSIS is sufficient for early releases.
- NSIS installs per-machine under `Program Files\Omnira`; runtime/user data
  remains separate under `%LOCALAPPDATA%\Omnira\`.
- The installer includes: the Omnira executable, both llama-server variants and
  their DLLs, `THIRD_PARTY_LICENSES`, and LICENSE.
- No network access is required at install time or first run. The installer
  uses `webviewInstallMode: skip` and does not download WebView2. **Release
  notes must state that the WebView2 runtime is a prerequisite**; alpha targets
  current Windows systems where WebView2 is already present.
- Uninstall removes the application; user data under `%LOCALAPPDATA%\Omnira\`
  is preserved unless the user opts to remove it.
- The repeatable alpha validation sequence lives in
  [alpha-readiness-checklist.md](alpha-readiness-checklist.md), including NSIS
  artifact validation, offline-after-install testing, fresh install/relaunch
  validation, diagnostics redaction review, orphan-process testing, release
  notes preparation, and tag creation.
- Installer/offline validation should prove the installed app can launch, select
  a local GGUF in place, start the managed `llama-server`, produce a chat
  response, close and reopen with conversation history intact, and repeat the
  flow without network connectivity after installation.

## 5. No silent downloads

Omnira does not download llama-server, models, or runtime dependencies at
runtime in MVP. The runtime ships in the installer; models are files the user
already has.

## 6. Code signing (pre-alpha checklist item)

Before public alpha, evaluate Windows code signing for the installer and the
main executable: certificate type (EV vs. standard OV), cost, and CI signing
workflow. Unsigned installers trigger SmartScreen warnings. Code signing should
be evaluated before public alpha, but it does not block internal alpha testing
unless maintainers explicitly make it a release gate.

## 7. Orchestrator implementation

The MVP orchestrator is the **Rust core inside Tauri**, not a separate service.
See [ADR 0001](adr/0001-rust-tauri-core-orchestrator.md) for why an earlier
Python/FastAPI + PyInstaller sidecar was not implemented.
