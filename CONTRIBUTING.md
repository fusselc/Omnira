# Contributing to Omnira

Thanks for your interest in Omnira. This document explains how the project is
run and what we expect from contributions.

## Project principles

Contributions are evaluated against these principles, in this order:

1. **Scope discipline.** The MVP is local GGUF chat on Windows, full stop.
   Features outside the MVP (image, voice, RAG, agents, downloads, CUDA, plugins)
   are documented in the roadmap but not accepted as code until their phase begins.
   PRs that expand active scope will be declined, even if well written.
2. **Local-first, private by default.** No telemetry, accounts, cloud sync,
   crash upload, update checks, or external network calls by default. Any PR that
   introduces a default network call will be declined.
3. **Beginner-first UX.** The default experience never exposes ports, runtime
   flags, quantization, samplers, or process details. Technical depth belongs in
   Advanced Diagnostics.
4. **Docs are canonical.** Architecture decisions live in `docs/`. If your change
   contradicts a documented decision, the PR must update the doc and explain why,
   and the decision change must be agreed in an issue first.

## Ground rules

- Do not commit `llama-server` binaries, GGUF model files, user conversations,
  or any runtime data. Binaries are fetched by the packaging script with pinned
  versions and SHA-256 verification.
- The Rust core owns process supervision, SQLite, config, and IPC. Do not add a
  separate backend process or a Python runtime; that architecture was evaluated
  and deliberately removed (see `docs/architecture.md`).
- All user-facing errors must map to the error taxonomy in `docs/chat-provider.md`.
  Do not invent ad hoc error strings.
- Documentation and UI copy use plain language and ASCII-safe punctuation.
- Main UI copy says "Running locally", never "GPU accelerated". Accelerator
  details (Vulkan/CPU) appear only in Advanced Diagnostics.

## How to contribute

1. **Open an issue first** for anything beyond a small fix. Describe the problem,
   the proposed approach, and which documented decisions it touches.
2. **Keep PRs small and focused.** One concern per PR. Include what you tested.
3. **Match the surrounding code.** Naming, module boundaries, and conventions are
   established in `apps/desktop/`; new code should read as if written by the same
   author.
4. **Update docs in the same PR** when behavior or architecture changes.

## Development setup

See [docs/development.md](docs/development.md) for prerequisites and build steps.

## Review expectations

- Maintainers review for scope, security boundaries (loopback-only, session key,
  CSP), and UX language before code style.
- Changes to the security boundary (`docs/local-security-boundary.md`) or the
  process model (`docs/packaging-process-model.md`) require explicit maintainer
  sign-off.

## License

By contributing, you agree that your contributions are licensed under the
Apache License 2.0 (see [LICENSE](LICENSE)).
