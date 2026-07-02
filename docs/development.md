# Development

How to build Omnira from source. End users never need any of this; installers
are self-contained.

## Prerequisites

- Windows 10/11 x64
- [Rust](https://rustup.rs/) (stable toolchain) with the MSVC target
- [Node.js](https://nodejs.org/) 20+ and npm
- Tauri 2 Windows prerequisites: Microsoft Visual Studio C++ Build Tools and
  the WebView2 runtime (preinstalled on current Windows)

The Rust/Tauri core is the orchestrator. No Python runtime is used or required.
See [ADR 0001](adr/0001-rust-tauri-core-orchestrator.md) for historical context.

## Repository layout

```
omnira/
  apps/desktop/          Tauri app
    src/                 React + TypeScript + Tailwind frontend
    src-tauri/           Rust core (supervision, storage, config, IPC)
      binaries/          fetched llama-server runtimes (gitignored)
  docs/                  canonical architecture and product docs
    adr/                 architecture decision records
  scripts/
    packaging/           runtime fetch, license aggregation
    diagnostics/         release verification helpers
    dev/                 developer utilities
  models/                placeholder only -- never commit model files
  data/                  placeholder only -- never commit runtime data
```

## Setup

```powershell
cd apps/desktop
npm install

# Fetch the pinned llama-server runtimes (checksum-verified, fails closed)
powershell -ExecutionPolicy Bypass -File ../../scripts/packaging/fetch-llama-server.ps1
```

## Run in development

```powershell
cd apps/desktop
npm run tauri:dev
```

This starts Vite for the frontend and builds/runs the Rust core with hot
reload for UI changes. The `tauri:dev` script merges `src-tauri/tauri.dev.conf.json`,
which widens CSP `connect-src` to include `http://localhost:*` and
`ws://localhost:*` for the Vite dev server and HMR. **Production builds do not
use this merge** -- see [local-security-boundary.md](local-security-boundary.md).

For UI-only work without a runtime, `npm run dev` runs the frontend in a
browser with the in-memory mock backend (`src/lib/mock.ts`).

## Build a release bundle

```powershell
cd apps/desktop
npm run tauri build
```

Produces the NSIS installer under `apps/desktop/src-tauri/target/release/bundle/`.
Packaging requires the fetched runtimes to be present and checksum-valid.

Before alpha, walk through the ordered release workflow in
[alpha-readiness-checklist.md](alpha-readiness-checklist.md). At minimum, record
evidence for:

```powershell
cd apps/desktop/src-tauri
cargo check
cargo test

cd ..
npm.cmd run build
npm.cmd audit --omit=dev
```

## Packaging helpers

```powershell
# Dependency license appendix (optional, for release artifacts)
powershell -ExecutionPolicy Bypass -File scripts/packaging/aggregate-licenses.ps1

# Offline-after-install smoke test (manual UI steps; post-install)
powershell -ExecutionPolicy Bypass -File scripts/diagnostics/offline-smoke-test.ps1

# Orphan llama-server process check (development harness)
powershell -ExecutionPolicy Bypass -File scripts/dev/orphan-check.ps1
```

## Useful checks

```powershell
# Rust
cd apps/desktop/src-tauri
cargo check
cargo test
cargo clippy

# Frontend
cd apps/desktop
npx tsc --noEmit

# Orphan llama-server check (requires fetched runtimes + optional test GGUF)
powershell -ExecutionPolicy Bypass -File ../../scripts/dev/orphan-check.ps1
```

## Rules that apply to every change

- Never commit `llama-server` binaries, GGUF files, or user data.
- Data written at runtime goes to `%LOCALAPPDATA%\Omnira\`, never the repo.
- Every user-facing error maps to the taxonomy in `docs/chat-provider.md`.
- Read `CONTRIBUTING.md` before opening a PR.
