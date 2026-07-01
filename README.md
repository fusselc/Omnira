# Omnira

Omnira is a Windows-first, local-first, open-source AI desktop app designed to make
open-source AI feel as simple, polished, and approachable as the big cloud assistants --
while preserving privacy, modularity, extensibility, and power-user control.

**Simple by default. Powerful when desired. Open for everyone.**

## Current status

Omnira is in early development. The current focus is the MVP described below.
Nothing in the long-term vision section is implemented yet.

## What the MVP is (current scope)

The MVP does exactly one workflow, and does it well:

1. Install and launch Omnira -- no terminal, no Python, no Docker, no manual services.
2. Select an existing local `.gguf` model file from the Models screen.
3. Omnira launches and manages a local `llama-server` process automatically.
4. Chat locally; assistant responses stream into the UI.
5. Stop generation at any time.
6. Close and reopen Omnira; previous conversations are still there.
7. Everything runs locally. No telemetry, accounts, cloud sync, or external
   network calls by default. Omnira works with the internet disconnected.

MVP screens: **Chat**, **Models**, **Settings**, and **Advanced Diagnostics**.

The main UI says "Running locally". Technical details -- the selected accelerator
(Vulkan or CPU), ports, process state, logs -- live in Advanced Diagnostics only.

### Explicitly not in the MVP

Model downloads, Hugging Face browsing, image generation, voice, speech-to-text,
text-to-speech, memory/RAG, document ingestion, agents, tool calling, workflow
automation, video, music, third-party plugins, CUDA builds, training, fine-tuning,
and cloud providers. These are documented as future direction only (see below).

## How it works

Omnira is an orchestration layer over open-source AI runtimes -- not a replacement
for llama.cpp, Ollama, LM Studio, ComfyUI, or similar tools.

- **Tauri 2 desktop shell** with a **Rust core** that owns process supervision,
  SQLite persistence, config, and typed IPC. There is no separate backend process
  and no Python runtime.
- **React + TypeScript + Tailwind CSS** frontend.
- A bundled, pinned **llama-server** (llama.cpp) runtime in two variants:
  Vulkan (GPU acceleration on NVIDIA/AMD/Intel) with automatic CPU fallback.
- `llama-server` binds to loopback only and requires a per-session API key.
- Conversations, settings, and the model registry are stored locally under
  `%LOCALAPPDATA%\Omnira\`. Models are referenced in place -- Omnira never copies
  your multi-gigabyte model files, and removing a model from Omnira never deletes
  the file.

See [docs/architecture.md](docs/architecture.md) for the full design and
[ADR 0001](docs/adr/0001-rust-tauri-core-orchestrator.md) for the orchestrator
decision. Alpha release verification: [alpha-readiness-checklist.md](docs/alpha-readiness-checklist.md).

## Long-term vision (not current features)

Omnira's long-term goal is a unified local AI workstation: one app that routes
each model and task to the best-fit open-source runtime -- llama.cpp/GGUF for
LLM chat and agents, Windows ML/ONNX for vision, audio, and NPU acceleration on
Copilot+ PCs, and CUDA/TensorRT for heavy GPU workloads like large LLMs,
diffusion, and video generation -- all behind one calm interface.

That is a direction, not a promise of current functionality. The roadmap and the
runtime strategy live in [docs/roadmap.md](docs/roadmap.md) and
[docs/runtimes-and-routing.md](docs/runtimes-and-routing.md).

## Documentation

- [Vision](docs/vision.md) -- what Omnira is, who it is for, and what it is not
- [Architecture](docs/architecture.md) -- desktop shell, Rust core, managed runtime
- [ADR 0001](docs/adr/0001-rust-tauri-core-orchestrator.md) -- Rust core vs Python sidecar
- [Alpha readiness checklist](docs/alpha-readiness-checklist.md) -- pre-release verification
- [Roadmap](docs/roadmap.md) -- MVP phases and post-MVP capabilities
- [ChatProvider](docs/chat-provider.md) -- the MVP provider contract
- [Packaging and process model](docs/packaging-process-model.md)
- [Local security boundary](docs/local-security-boundary.md)
- [Data ownership and storage](docs/data-ownership-and-storage.md)
- [Design principles](docs/design-principles.md)
- [Runtimes and routing](docs/runtimes-and-routing.md) -- long-term runtime strategy
- [Development](docs/development.md) -- building Omnira from source
- [Privacy](docs/privacy.md) -- the local-first promise, in plain language
- [Contributing](CONTRIBUTING.md)

## License

Omnira is licensed under the [Apache License 2.0](LICENSE). Bundled third-party
components (including llama.cpp, MIT licensed) are attributed in
[THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).
