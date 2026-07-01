# Agent Guidance

This file is the canonical repository-wide instruction source for coding agents working on Omnira.

## Project Scope

- MVP scope is Windows-first local GGUF chat only.
- The runtime stack is Tauri 2, React, TypeScript, Tailwind CSS, and a Rust core.
- The Rust core owns process supervision, SQLite persistence, config, and typed IPC.
- Do not add a Python runtime, FastAPI orchestrator, PyInstaller pipeline, or `backend/` directory for MVP.
- Do not add telemetry, accounts, cloud sync, model downloads, or default external network calls.

## Runtime And Data Rules

- Do not commit `llama-server` binaries.
- Do not commit GGUF model files.
- Runtime data belongs under `%LOCALAPPDATA%\Omnira\`.
- Models are referenced in place by default.
- `llama-server` must be loopback-only and protected by a per-session API key.

## UI And Product Rules

- MVP screens are Chat, Models, Settings, and Advanced Diagnostics only.
- Keep post-MVP features out of the active UI unless explicitly requested.
- Main UI copy should say "Running locally", not "GPU accelerated".
- Advanced Diagnostics may name runtime variants such as Vulkan or CPU.

## Documentation Rules

- Keep docs canonical before implementation work.
- Update architecture, security, data, packaging, and roadmap docs when implementation decisions change.
- Avoid duplicating project rules across editor-specific files; those files should defer to this one.
