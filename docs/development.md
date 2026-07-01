# Development

How to build Omnira from source. End users never need any of this; installers
are self-contained.

## Prerequisites

- Windows 10/11 x64
- [Rust](https://rustup.rs/) (stable toolchain) with the MSVC target
- [Node.js](https://nodejs.org/) 20+ and npm
- Tauri 2 Windows prerequisites: Microsoft Visual Studio C++ Build Tools and
  the WebView2 runtime (preinstalled on current Windows)

No Python is required, anywhere.

## Repository layout

```
omnira/
  apps/desktop/          Tauri app
    src/                 React + TypeScript + Tailwind frontend
    src-tauri/           Rust core (supervision, storage, config, IPC)
      binaries/          fetched llama-server runtimes (gitignored)
  docs/                  canonical architecture and product docs
  scripts/
    packaging/           runtime fetch + packaging scripts
    dev/                 developer utilities
  tests/                 integration / e2e tests
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
npm run tauri dev
```

This starts Vite for the frontend and builds/runs the Rust core with hot
reload for UI changes.

## Build a release bundle

```powershell
cd apps/desktop
npm run tauri build
```

Produces the NSIS installer under `apps/desktop/src-tauri/target/release/bundle/`.
Packaging requires the fetched runtimes to be present and checksum-valid.

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
```

## Rules that apply to every change

- Never commit `llama-server` binaries, GGUF files, or user data.
- Data written at runtime goes to `%LOCALAPPDATA%\Omnira\`, never the repo.
- Every user-facing error maps to the taxonomy in `docs/chat-provider.md`.
- Read `CONTRIBUTING.md` before opening a PR.
