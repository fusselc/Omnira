# Alpha Readiness Checklist

Use this checklist before tagging an alpha release or publishing installers.
Each item should be marked with evidence: command output, screenshot, log
excerpt, dated maintainer validation, or dated release notes.

MVP scope remains **local GGUF chat via managed llama-server on Windows** only.

## Status Legend

- **Verified**: completed for the current MVP foundation with evidence below.
- **Not yet verified**: required before public alpha, but no current evidence is
  recorded yet.
- **Deferred**: explicitly not required for alpha or not applicable to the
  Windows MVP target.
- **Blocked**: cannot be completed until a named prerequisite is resolved.

Current blocked items: **none identified**.

## Current Evidence Snapshot

- **Verified, 2026-07-06**: MVP scope remains Windows-first local GGUF chat
  only; no ONNX conversion, model downloads, image generation, voice mode,
  cloud calls, accounts, telemetry, CUDA, plugins, RAG, agents, or provider
  expansion were added for this milestone.
- **Verified, 2026-07-06**: `cargo check` passed from
  `apps/desktop/src-tauri/`.
- **Verified, 2026-07-06**: `cargo test` passed from
  `apps/desktop/src-tauri/`, including runtime CORS/streaming,
  unauthorized-request, cancellation-on-disconnect, restart-cycle, GGUF
  inspection, storage, and conversation/message tests. The ignored manual
  orphan test remains exercised through `scripts/dev/orphan-check.ps1`.
- **Verified, 2026-07-06**: `npm.cmd run build` passed from `apps/desktop/`.
- **Verified, 2026-07-06**: `npm.cmd audit --omit=dev` completed against the
  npm registry and reported `found 0 vulnerabilities`.
- **Verified, 2026-07-06**: Packaged smoke testing found and fixed two NSIS
  packaging issues before alpha: WebView2 download-bootstrapper mode was
  disabled so the installer remains offline-safe, and NSIS install mode was
  changed to per-machine so app binaries install under `Program Files\Omnira`
  instead of colliding with `%LOCALAPPDATA%\Omnira\` runtime data.
- **Verified, 2026-07-06**: `npm.cmd run tauri build` produced the rebuilt
  NSIS installer at
  `apps/desktop/src-tauri/target/release/bundle/nsis/Omnira_0.1.0_x64-setup.exe`
  (17,715,441 bytes, SHA-256
  `742ABA99CBEA35A2ABB710882DCC22040734C5B3D3A04D83BC1FD0D2A12B2972`).
- **Verified, 2026-07-06**: Silent install of the rebuilt per-machine NSIS
  artifact populated `C:\Program Files\Omnira`, wrote HKLM uninstall metadata,
  and `C:\Program Files\Omnira\omnira.exe` launched successfully.
- **Verified, 2026-07-06**: Release resources under
  `apps/desktop/src-tauri/target/release/` include `runtimes/cpu`,
  `runtimes/vulkan`, `LICENSE`, and `THIRD_PARTY_LICENSES`; both runtime
  folders include `llama-server.exe`, `llama-server-impl.dll`, and required
  DLLs.
- **Verified, 2026-07-06**: `scripts/dev/orphan-check.ps1` passed after
  force-killing its runtime test harness; `llama-server.exe` died with the
  parent process. The script now uses a hidden `ProcessStartInfo` launch and a
  fallback kill path for Windows environments where `taskkill` is denied.
- **Verified, 2026-07-06**: Model registry status detection now marks
  existing-but-corrupt GGUF files as `Invalid` instead of presenting them as
  ready.
- **Verified, 2026-07-02**: Manual MVP validation confirmed the app launches, a
  local GGUF model can be selected, the managed `llama-server` runtime starts, a
  prompt can be sent from Chat, the model responds, and conversation history
  persists after close and reopen.
- **Verified, 2026-07-02**: `cargo check` passed.
- **Verified, 2026-07-02**: `cargo test` passed.
- **Verified, 2026-07-02**: `npm.cmd run build` passed.
- **Verified, 2026-07-02**: Dependabot `esbuild` alert
  GHSA-g7r4-m6w7-qqqr is fixed by resolving `esbuild` to `0.28.1`.
- **Verified, 2026-07-02**: `git ls-files` scan found no committed GGUF
  files, `llama-server` binaries, DLLs, `node_modules`, `target`, `dist`, or
  cache directories.
- **Deferred, 2026-07-02**: `glib` alert GHSA-wrw7-89jp-8q8g is not present in
  the Windows MVP target graph; revisit before Linux packaging.

## Repeatable Alpha Release Workflow

Use this ordered workflow for each internal or public alpha candidate. Record
evidence in the sections below before tagging or publishing installers.

1. **Confirm scope**
   - Confirm the release remains Windows-first local GGUF chat through managed
     `llama-server`.
   - Confirm no image generation, voice, RAG, agents, workflows, video, music,
     ONNX, CUDA, plugins, model downloads, cloud calls, telemetry, or
     architecture changes are included unless a separate milestone explicitly
     approves them.

2. **Start from a clean repository**
   - Run `git status --short --branch`.
   - Confirm there are no unintended local changes before building or tagging.

3. **Run required validation commands**
   - From `apps/desktop/src-tauri/`, run `cargo check`.
   - From `apps/desktop/src-tauri/`, run `cargo test`.
   - From `apps/desktop/`, run `npm.cmd run build`.
   - From `apps/desktop/`, run `npm.cmd audit --omit=dev`.

4. **Triage dependency alerts**
   - Review GitHub Dependabot alerts before release.
   - Fix supported-target runtime or production-impacting alerts before public
     alpha.
   - Document dev-only or unsupported-target decisions with rationale.
   - Keep `glib` GHSA-wrw7-89jp-8q8g deferred until Linux packaging becomes in
     scope; it is not present in the Windows MVP target graph.

5. **Verify repository hygiene**
   - Confirm no `*.gguf`, `llama-server` binaries, runtime DLLs, build outputs,
     caches, `node_modules`, or `target` directories are committed.
   - Confirm runtime data remains under `%LOCALAPPDATA%\Omnira\`.

6. **Prepare runtime binaries**
   - Run `scripts/packaging/fetch-llama-server.ps1`.
   - Confirm the script verifies pinned SHA-256 checksums and fails closed on
     mismatch.
   - Do not commit fetched runtime binaries.

7. **Verify license materials**
   - Confirm `THIRD_PARTY_LICENSES` includes llama.cpp attribution, pinned
     release metadata, artifact names, and checksums.
   - Confirm `LICENSE` and `THIRD_PARTY_LICENSES` are included in the packaged
     app resources.

8. **Build and validate the NSIS installer**
   - Run `npm run tauri build` from `apps/desktop/`.
   - Confirm the release artifact is NSIS under
     `apps/desktop/src-tauri/target/release/bundle/`.
   - Confirm NSIS is per-machine, installs under `Program Files\Omnira`, and
     does not use a WebView2 download-bootstrapper mode.
   - Confirm bundled `llama-server` resources are present in the packaged app
     and were produced by `scripts/packaging/fetch-llama-server.ps1`.
   - MSI remains deferred until after alpha and must not block alpha unless a
     later release decision changes installer scope.

9. **Run install and offline validation**
   - Install to a clean directory from the NSIS artifact.
   - Launch for the first time, complete local GGUF selection, and confirm the
     model file is referenced in place.
   - Confirm Omnira starts the managed `llama-server` runtime.
   - Send a prompt, receive a response, quit, relaunch, and confirm
     conversation persistence.
   - Disconnect networking and repeat the normal local chat flow.
   - Monitor for unexpected external network calls during normal chat.

10. **Verify diagnostics and process cleanup**
    - Export diagnostics without including paths and confirm user profile paths
      are redacted and prompt/response text is absent.
    - Inspect logs for prompt-free behavior.
    - Run `scripts/dev/orphan-check.ps1` or complete an equivalent manual
      harness to confirm no orphaned `llama-server.exe` remains after quit or
      uninstall.

11. **Evaluate release gates**
    - Evaluate Windows code signing before public alpha. Code signing does not
      block internal alpha testing unless maintainers explicitly decide it does.
    - Review remaining not-yet-verified checklist items and record any accepted
      risk.

12. **Prepare release notes and tag**
    - Prepare release notes summarizing validation evidence, known limitations,
      and deferred items.
    - Create the release tag only after the checklist evidence is recorded.
    - Do not create a GitHub Release automatically unless the release process
      explicitly calls for it.

## Security and Webview Hardening

- [x] **Verified: CSP production review**
  - Build a release bundle (`npm run tauri build` from `apps/desktop/`).
  - Inspect the effective CSP in the packaged app (Tauri config:
    `apps/desktop/src-tauri/tauri.conf.json`).
  - Confirm production `connect-src` allows only Tauri IPC and
    `http://127.0.0.1:*` (no `http://localhost:*`). Dev builds merge
    `tauri.dev.conf.json` for Vite/HMR; see [development.md](development.md).
  - Confirm no remote script, style, or connect sources.
  - Evidence, 2026-07-06: release bundle built with `npm.cmd run tauri build`.
    Production CSP in `tauri.conf.json` allows `connect-src` only for `ipc:`,
    `http://ipc.localhost`, and `http://127.0.0.1:*`; no remote script, style,
    frame, object, or form sources are configured.

- [ ] **Not yet verified: Devtools disabled/verified in production**
  - Confirm `apps/desktop/src-tauri/Cargo.toml` does **not** enable Tauri's
    `devtools` feature on the `tauri` dependency.
  - Release builds (`cargo build --release` / `tauri build`) must not expose
    WebView2 devtools. Tauri enables devtools only when the `devtools` Cargo
    feature is present; it is off by default in release when that feature is
    absent.
  - Smoke-check a release install: right-click -> Inspect / devtools entry
    should not be available (WebView2 behavior varies; absence of the `devtools`
    feature is the guarantee).
  - Partial evidence, 2026-07-02: `apps/desktop/src-tauri/Cargo.toml` sets
    `tauri = { version = "2", features = [] }`.
  - Evidence still needed: release install smoke-check.

- [x] **Verified: llama-server loopback + api-key verification**
  - From Advanced Diagnostics or logs, confirm runtime binds to `127.0.0.1`
    only.
  - Advanced Diagnostics should name Vulkan vs CPU and explain CPU fallback in
    friendly language when fallback data is present.
  - Confirm chat requests without the session Bearer token return 401.
  - Confirm api-key is regenerated on each runtime start and is not written to
    disk or logs.
  - Evidence, 2026-07-06: `cargo test --test runtime_spikes` exercised
    startup arguments using `--host 127.0.0.1`, verified CORS/streaming,
    verified missing Bearer token returns 401, and verified
    cancellation-on-disconnect. Runtime code generates a fresh in-memory
    48-character key per start and logs only variant/attempt/port/error code,
    not the key.

## Local-First and Privacy

- [x] **Verified: Local GGUF chat manual validation**
  - Evidence, 2026-07-02: app launch, local GGUF selection, managed
    `llama-server` start, prompt send, model response, and conversation
    persistence after close/reopen were manually validated.

- [ ] **Not yet verified: Offline-after-install verification**
  - Prerequisites: completed NSIS install, bundled runtimes present from the
    installer, and at least one valid local GGUF available on disk.
  - Disconnect all networking (Wi-Fi/Ethernet off or airplane mode).
  - Run through: first launch or relaunch -> select/confirm local GGUF -> managed
    `llama-server` reaches "Running locally" -> send message -> receive streamed
    response -> quit -> relaunch -> prior conversation still present.
  - Automated helper: `scripts/diagnostics/offline-smoke-test.ps1` (guided
    manual steps and optional process/network checks).
  - Evidence needed: dated offline install smoke-test notes. This remains not
    yet verified until a packaged install is exercised locally.

- [ ] **Not yet verified: No external network calls at runtime**
  - During the offline test above, monitor with Resource Monitor, Wireshark, or
    similar: Omnira and `llama-server` should not initiate outbound connections
    during normal chat (build-time fetch scripts are excluded).
  - Evidence needed: dated network monitor notes.

- [ ] **Not yet verified: Prompt-free logs verification**
  - Exercise chat, then inspect `%LOCALAPPDATA%\Omnira\logs\`.
  - Confirm no user prompts or assistant response text appear in log lines.
  - Evidence needed: dated log inspection notes with message content omitted.
  - Partial evidence, 2026-07-06: logging code records lifecycle events,
    runtime metadata, and error codes only. No local UI-session log files were
    present to inspect in this environment, so the manual verification remains
    open.

- [ ] **Not yet verified: Diagnostics redaction verification**
  - Export diagnostics **without** "include paths".
  - Confirm Windows user profile segments are redacted (`<user>`) and no message
    content is included.
  - Evidence needed: redacted diagnostics export review notes.
  - Partial evidence, 2026-07-06: diagnostics export serializes runtime status,
    paths, recent log lines, and recent errors only, then redacts
    `\Users\<name>\` segments unless paths are explicitly included. No local
    diagnostics export was present to inspect in this environment, so the
    manual verification remains open.

## Repository and Packaging Hygiene

- [x] **Verified: No committed model/runtime binaries**
  - Review: no `*.gguf`, `llama-server.exe`, or runtime DLLs in the repository.
    `.gitignore` enforces this; CI/review confirms.
  - Evidence, 2026-07-02: `git ls-files` scan found no committed GGUF files,
    `llama-server` binaries, DLLs, build outputs, caches, `node_modules`, or
    `target` directories.

- [x] **Verified: THIRD_PARTY_LICENSES included in packaged app**
  - After install, verify `THIRD_PARTY_LICENSES` and `LICENSE` exist beside the
    application resources (bundled via `tauri.conf.json`).
  - llama.cpp MIT attribution and pinned release metadata must be present.
  - Optional for alpha: run `scripts/packaging/aggregate-licenses.ps1` and ship
    the generated dependency appendix with the installer.
  - Evidence, 2026-07-06: release resources include
    `apps/desktop/src-tauri/target/release/LICENSE` and
    `apps/desktop/src-tauri/target/release/THIRD_PARTY_LICENSES`. The license
    file includes llama.cpp MIT attribution, pinned tag `b9859`, pinned commit
    `4fc4ec5541b243957ae5099edb67372f8f3b550e`, artifact names, and SHA-256
    checksums.
  - Evidence, 2026-07-06: installed Program Files payload includes
    `C:\Program Files\Omnira\LICENSE` and
    `C:\Program Files\Omnira\THIRD_PARTY_LICENSES`.

- [ ] **Not yet verified: Runtime fetch fail-closed**
  - `scripts/packaging/fetch-llama-server.ps1` verifies SHA-256; tamper a cached
    zip locally and confirm the script exits nonzero without bundling.
  - Evidence needed: dated tamper-test command output or release notes.

- [x] **Verified: esbuild Dependabot alert fixed**
  - Evidence, 2026-07-02: PR #9 resolved `esbuild` to `0.28.1`; Dependabot
    reported GHSA-g7r4-m6w7-qqqr as fixed.

- [x] **Deferred: glib advisory is not in the Windows MVP target graph**
  - Evidence, 2026-07-02: `glib` GHSA-wrw7-89jp-8q8g is not present in the
    Windows MVP target graph; revisit before Linux packaging.

## Build and Test Verification

- [x] **Verified: Rust check**
  - Evidence, 2026-07-02: `cargo check` passed.
  - Evidence, 2026-07-06: `cargo check` passed from
    `apps/desktop/src-tauri/`.

- [x] **Verified: Rust tests**
  - Evidence, 2026-07-02: `cargo test` passed.
  - Evidence, 2026-07-06: `cargo test` passed from
    `apps/desktop/src-tauri/`; runtime spike tests passed with the local
    `models/qwen2.5-0.5b-instruct-q4_k_m.gguf`.

- [x] **Verified: Frontend build**
  - Evidence, 2026-07-02: `npm.cmd run build` passed.
  - Evidence, 2026-07-06: `npm.cmd run build` passed from `apps/desktop/`.

## Install Lifecycle

- [ ] **Not yet verified: Fresh install / relaunch test**
  - Install the NSIS artifact to a clean directory.
  - First launch completes local GGUF model selection or the existing-data path.
  - Confirm Omnira starts the managed `llama-server` runtime.
  - Send a chat prompt and receive a response.
  - Quit and relaunch: conversation history persists under
    `%LOCALAPPDATA%\Omnira\`.
  - Evidence needed: dated install/relaunch notes. This remains not yet verified
    until a packaged install is exercised locally.
  - Partial evidence, 2026-07-06: rebuilt per-machine NSIS artifact installed
    to `C:\Program Files\Omnira`, installed payload and HKLM uninstall metadata
    were present, and `omnira.exe` launched from Program Files. Model
    selection, streamed chat, stop-generation, relaunch persistence, and
    offline UI workflow still require an interactive manual pass.

- [ ] **Not yet verified: Uninstall / orphan-process test**
  - Run `scripts/dev/orphan-check.ps1` before release (Job Object verification).
  - If the harness cannot run, manually start Omnira from an installed build,
    load a local GGUF, confirm `llama-server.exe` appears, force-close Omnira,
    and confirm `llama-server.exe` exits with it.
  - After manual uninstall, confirm no orphaned `llama-server.exe` remains when
    Omnira is not running.
  - Partial evidence, 2026-07-06: `scripts/dev/orphan-check.ps1` launched the ignored
    runtime harness, observed `llama-server.exe`, force-killed the parent
    harness, and reported `PASS: llama-server.exe died with its parent. No
    orphaned processes.` Manual uninstall validation remains open.
  - Partial evidence, 2026-07-06: no `omnira.exe` or `llama-server.exe`
    processes remained after closing the installed app. Silent Program Files
    uninstall/removal could not complete in this environment because true admin
    file removal was denied, so manual uninstall verification remains open.

## Installer Scope (MVP Alpha)

- [x] **Verified: NSIS installer verified**
  - Release artifact is NSIS (see `tauri.conf.json` `bundle.targets`).
  - Confirm the package includes the Omnira executable, bundled
    `llama-server` runtimes, `LICENSE`, and `THIRD_PARTY_LICENSES`.
  - Evidence, 2026-07-06: `npm.cmd run tauri build` produced
    `apps/desktop/src-tauri/target/release/bundle/nsis/Omnira_0.1.0_x64-setup.exe`
    (17,715,441 bytes, SHA-256
    `742ABA99CBEA35A2ABB710882DCC22040734C5B3D3A04D83BC1FD0D2A12B2972`).
    Generated NSIS config has `INSTALLMODE "perMachine"`,
    `RequestExecutionLevel admin`, Program Files install path, and no active
    WebView2 download-bootstrapper mode. Installed payload inspection confirmed
    bundled CPU and Vulkan runtime folders plus `LICENSE` and
    `THIRD_PARTY_LICENSES`. Full interactive chat and offline smoke testing
    remain tracked separately above.

- [x] **Deferred: MSI installer**
  - **MSI is deferred** post-alpha; do not block alpha on MSI.

## Pre-Public Alpha

- [ ] **Not yet verified: Windows code signing evaluation documented**
  - See [packaging-process-model.md](packaging-process-model.md) section 6.
  - Evidence needed: dated signing decision or tracked follow-up.

- [ ] **Not yet verified: 13 MVP acceptance criteria reviewed end-to-end**
  - Evidence needed: dated criteria review with any exceptions tracked.

## Sign-Off

| Role | Name | Date | Notes |
|------|------|------|-------|
| Maintainer | | | |
