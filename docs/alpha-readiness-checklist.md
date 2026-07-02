# Alpha Readiness Checklist

Use this checklist before tagging an alpha release or publishing installers.
Each item should be marked pass/fail with evidence (command output, screenshot,
log excerpt, or dated note in the release notes).

MVP scope remains **local GGUF chat via managed llama-server on Windows** only.

## Security and webview hardening

- [ ] **CSP production review**
  - Build a release bundle (`npm run tauri build` from `apps/desktop/`).
  - Inspect the effective CSP in the packaged app (Tauri config:
    `apps/desktop/src-tauri/tauri.conf.json`).
  - Confirm production `connect-src` allows only Tauri IPC and
    `http://127.0.0.1:*` (no `http://localhost:*`). Dev builds merge
    `tauri.dev.conf.json` for Vite/HMR; see [development.md](development.md).
  - Confirm no remote script, style, or connect sources.

- [ ] **Devtools disabled/verified in production**
  - Confirm `apps/desktop/src-tauri/Cargo.toml` does **not** enable Tauri's
    `devtools` feature on the `tauri` dependency.
  - Release builds (`cargo build --release` / `tauri build`) must not expose
    WebView2 devtools. Tauri enables devtools only when the `devtools` Cargo
    feature is present; it is off by default in release when that feature is
    absent.
  - Smoke-check a release install: right-click -> Inspect / devtools entry
    should not be available (WebView2 behavior varies; absence of the `devtools`
    feature is the guarantee).

- [ ] **llama-server loopback + api-key verification**
  - From Advanced Diagnostics or logs, confirm runtime binds to `127.0.0.1` only.
  - Confirm chat requests without the session Bearer token return 401.
  - Confirm api-key is regenerated on each runtime start and is not written to
    disk or logs.

## Local-first and privacy

- [ ] **Offline-after-install verification**
  - On a machine with a completed install, bundled runtimes, and at least one
    registered local GGUF: disconnect all networking (Wi-Fi/Ethernet off or
    airplane mode).
  - Run through: launch -> select model -> send message -> stream response ->
    stop generation -> relaunch -> prior conversation still present.
  - Automated helper: `scripts/diagnostics/offline-smoke-test.ps1` (manual steps
    and optional checks).

- [ ] **No external network calls at runtime**
  - During the offline test above, monitor with Resource Monitor, Wireshark, or
    similar: Omnira and `llama-server` should not initiate outbound connections
    during normal chat (build-time fetch scripts are excluded).

- [ ] **Prompt-free logs verification**
  - Exercise chat, then inspect `%LOCALAPPDATA%\Omnira\logs\`.
  - Confirm no user prompts or assistant response text appear in log lines.

- [ ] **Diagnostics redaction verification**
  - Export diagnostics **without** "include paths".
  - Confirm Windows user profile segments are redacted (`<user>`) and no message
    content is included.

## Repository and packaging hygiene

- [ ] **No committed model/runtime binaries**
  - Review: no `*.gguf`, `llama-server.exe`, or runtime DLLs in the repository.
    `.gitignore` enforces this; CI/review confirms.

- [ ] **THIRD_PARTY_LICENSES included in packaged app**
  - After install, verify `THIRD_PARTY_LICENSES` and `LICENSE` exist beside the
    application resources (bundled via `tauri.conf.json`).
  - llama.cpp MIT attribution and pinned release metadata must be present.
  - Optional for alpha: run `scripts/packaging/aggregate-licenses.ps1` and ship
    the generated dependency appendix with the installer.

- [ ] **Runtime fetch fail-closed**
  - `scripts/packaging/fetch-llama-server.ps1` verifies SHA-256; tamper a cached
    zip locally and confirm the script exits nonzero without bundling.

- [ ] **Deferred non-Windows dependency alerts documented**
  - `glib` GHSA-wrw7-89jp-8q8g is not present in the Windows MVP target graph;
    revisit before Linux packaging.

## Install lifecycle

- [ ] **Fresh install / relaunch test**
  - Silent or interactive install to a clean directory.
  - First launch completes onboarding (or skip path).
  - Register model, chat, quit, relaunch: data persists under
    `%LOCALAPPDATA%\Omnira\`.

- [ ] **Uninstall / orphan-process test**
  - Run `scripts/dev/orphan-check.ps1` before release (Job Object verification).
  - After manual uninstall, confirm no orphaned `llama-server.exe` remains when
    Omnira is not running.

## Installer scope (MVP alpha)

- [ ] **NSIS installer verified**
  - Release artifact is NSIS (see `tauri.conf.json` `bundle.targets`).
  - **MSI is deferred** post-alpha; do not block alpha on MSI.

## Pre-public alpha (recommended, not blocking)

- [ ] **Windows code signing evaluation** documented (see
  [packaging-process-model.md](packaging-process-model.md) section 6).
- [ ] **13 MVP acceptance criteria** from the canonical plan reviewed end-to-end.

## Sign-off

| Role | Name | Date | Notes |
|------|------|------|-------|
| Maintainer | | | |
