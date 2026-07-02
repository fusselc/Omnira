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

## Security and Webview Hardening

- [ ] **Not yet verified: CSP production review**
  - Build a release bundle (`npm run tauri build` from `apps/desktop/`).
  - Inspect the effective CSP in the packaged app (Tauri config:
    `apps/desktop/src-tauri/tauri.conf.json`).
  - Confirm production `connect-src` allows only Tauri IPC and
    `http://127.0.0.1:*` (no `http://localhost:*`). Dev builds merge
    `tauri.dev.conf.json` for Vite/HMR; see [development.md](development.md).
  - Confirm no remote script, style, or connect sources.
  - Evidence needed: release bundle inspection notes.

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

- [ ] **Not yet verified: llama-server loopback + api-key verification**
  - From Advanced Diagnostics or logs, confirm runtime binds to `127.0.0.1`
    only.
  - Confirm chat requests without the session Bearer token return 401.
  - Confirm api-key is regenerated on each runtime start and is not written to
    disk or logs.
  - Evidence needed: diagnostics/log excerpt with prompts redacted and manual
    unauthorized-request test notes.

## Local-First and Privacy

- [x] **Verified: Local GGUF chat manual validation**
  - Evidence, 2026-07-02: app launch, local GGUF selection, managed
    `llama-server` start, prompt send, model response, and conversation
    persistence after close/reopen were manually validated.

- [ ] **Not yet verified: Offline-after-install verification**
  - On a machine with a completed install, bundled runtimes, and at least one
    registered local GGUF: disconnect all networking (Wi-Fi/Ethernet off or
    airplane mode).
  - Run through: launch -> select model -> send message -> stream response ->
    stop generation -> relaunch -> prior conversation still present.
  - Automated helper: `scripts/diagnostics/offline-smoke-test.ps1` (manual steps
    and optional checks).
  - Evidence needed: dated offline install smoke-test notes.

- [ ] **Not yet verified: No external network calls at runtime**
  - During the offline test above, monitor with Resource Monitor, Wireshark, or
    similar: Omnira and `llama-server` should not initiate outbound connections
    during normal chat (build-time fetch scripts are excluded).
  - Evidence needed: dated network monitor notes.

- [ ] **Not yet verified: Prompt-free logs verification**
  - Exercise chat, then inspect `%LOCALAPPDATA%\Omnira\logs\`.
  - Confirm no user prompts or assistant response text appear in log lines.
  - Evidence needed: dated log inspection notes with message content omitted.

- [ ] **Not yet verified: Diagnostics redaction verification**
  - Export diagnostics **without** "include paths".
  - Confirm Windows user profile segments are redacted (`<user>`) and no message
    content is included.
  - Evidence needed: redacted diagnostics export review notes.

## Repository and Packaging Hygiene

- [x] **Verified: No committed model/runtime binaries**
  - Review: no `*.gguf`, `llama-server.exe`, or runtime DLLs in the repository.
    `.gitignore` enforces this; CI/review confirms.
  - Evidence, 2026-07-02: `git ls-files` scan found no committed GGUF files,
    `llama-server` binaries, DLLs, build outputs, caches, `node_modules`, or
    `target` directories.

- [ ] **Not yet verified: THIRD_PARTY_LICENSES included in packaged app**
  - After install, verify `THIRD_PARTY_LICENSES` and `LICENSE` exist beside the
    application resources (bundled via `tauri.conf.json`).
  - llama.cpp MIT attribution and pinned release metadata must be present.
  - Optional for alpha: run `scripts/packaging/aggregate-licenses.ps1` and ship
    the generated dependency appendix with the installer.
  - Evidence needed: packaged app file listing and license attribution review.

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

- [x] **Verified: Rust tests**
  - Evidence, 2026-07-02: `cargo test` passed.

- [x] **Verified: Frontend build**
  - Evidence, 2026-07-02: `npm.cmd run build` passed.

## Install Lifecycle

- [ ] **Not yet verified: Fresh install / relaunch test**
  - Silent or interactive install to a clean directory.
  - First launch completes onboarding (or skip path).
  - Register model, chat, quit, relaunch: data persists under
    `%LOCALAPPDATA%\Omnira\`.
  - Evidence needed: dated install/relaunch notes.

- [ ] **Not yet verified: Uninstall / orphan-process test**
  - Run `scripts/dev/orphan-check.ps1` before release (Job Object verification).
  - After manual uninstall, confirm no orphaned `llama-server.exe` remains when
    Omnira is not running.
  - Evidence needed: orphan-check output and manual uninstall notes.

## Installer Scope (MVP Alpha)

- [ ] **Not yet verified: NSIS installer verified**
  - Release artifact is NSIS (see `tauri.conf.json` `bundle.targets`).
  - Evidence needed: release artifact file listing and install smoke-test.

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
