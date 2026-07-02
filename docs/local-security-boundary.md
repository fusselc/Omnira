# Local Security Boundary

Omnira is local-first, but local services still need security boundaries.
This document defines them for MVP. Changes here require explicit maintainer
sign-off.

## 1. Threat model (MVP scope)

Threats considered:

- Another local process calling the managed llama-server API.
- The runtime accidentally binding to a public interface.
- Runtime logs containing private prompt content.
- The webview being made to run non-first-party code (which would expose the
  session api-key).
- Future providers making network calls without user understanding (post-MVP
  design topic, documented in `docs/privacy.md`).

Out of MVP scope: plugin sandboxing, remote trust systems, provider
permissions. Those belong to post-MVP design.

## 2. Security layers (both mandatory, independent)

### Layer 1: Tauri IPC boundary

The webview can only reach Rust functionality explicitly exposed as Tauri
`invoke` commands, enforced by the OS process boundary and Tauri's capability
system. The command surface is minimal and typed; no generic shell/fs/http
commands are exposed to the webview.

### Layer 2: llama-server api-key + loopback binding

`llama-server` is always started with:

- `--host 127.0.0.1` -- loopback only, never a public interface.
- `--api-key <session-secret>` -- a cryptographically random secret generated
  in memory by the Rust core at model start, never persisted, never sent over
  a network hop.

Any local process that finds the port still cannot use the runtime without the
key. Requests without the key are rejected by llama-server itself and map to
`UnauthorizedLocalRequest`.

## 3. Session api-key in the webview -- accepted, documented tradeoff

The frontend obtains host, port, and the session api-key via a Tauri command so
it can stream chat directly over loopback without a proxy hop. This is
acceptable because:

- The key is session-scoped and regenerated on every runtime start.
- It is never written to disk or logs.
- The runtime is loopback-only.
- The webview is first-party code -- an assumption *enforced* by the hardening
  in section 4, not merely assumed.

Documented alternative if the threat model changes: route chat through a Rust
`chat_stream` proxy command so the key never enters the webview. This is also
the approved CORS fallback (see `docs/architecture.md`).

## 4. Webview hardening (mandatory)

Configured in Phase 1 from the first scaffold; audited in
[alpha-readiness-checklist.md](alpha-readiness-checklist.md):

- **Strict Content Security Policy:** no remote script, style, or connect
  sources. Production `connect-src` is limited to Tauri IPC and
  `http://127.0.0.1:*` (the chat path). `default-src 'self'`.
  Development merges `tauri.dev.conf.json` to allow `http://localhost:*` and
  `ws://localhost:*` for Vite/HMR only (`npm run tauri:dev`).
- **No remote content loading** of any kind.
- **No arbitrary navigation:** the app is a single-page app; external links
  open in the system browser via the Tauri opener API, never inside the
  webview.
- **No raw HTML rendering:** assistant output is rendered as sanitized
  markdown with raw HTML disabled. User input is never interpreted as HTML.
- **Devtools disabled in production release builds.** Tauri exposes devtools
  only when the `devtools` Cargo feature is enabled on the `tauri` dependency.
  Omnira does not enable that feature (`apps/desktop/src-tauri/Cargo.toml`).
  Release builds therefore ship without devtools support; debug/`tauri dev`
  builds may still allow inspector access per Tauri defaults.

## 5. Network defaults

- Zero external network calls by default: no telemetry, accounts, analytics,
  cloud sync, crash upload, or update checks.
- The application must be fully functional with the internet disconnected
  after installation (MVP acceptance criterion 13, verified in Phase 5 by
  disconnecting networking entirely and exercising the full workflow).

## 6. Logging privacy

- Logs are prompt-free by default: lifecycle events, startup/shutdown, runtime
  failures, structured error codes, and minimal generation metadata (timings,
  token counts) -- never prompt or response content.
- Diagnostics export redacts user paths and content unless the user explicitly
  opts in. See `docs/data-ownership-and-storage.md`.
