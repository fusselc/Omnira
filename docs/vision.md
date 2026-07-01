# Omnira Product Vision

Omnira is a Windows-first, local-first, open-source AI desktop app. Its purpose is
to make open-source AI feel as simple, polished, and approachable as the big cloud
assistants while preserving privacy, modularity, extensibility, and power-user
control.

**Mantra: Simple by default. Powerful when desired. Open for everyone.**

## 1. What Omnira is

- A beginner-friendly, local-first AI desktop environment.
- A polished desktop interface over open-source AI runtimes.
- An orchestration layer, not a replacement for every AI engine.
- A way to run, manage, and interact with local models without terminals,
  Docker, manual services, or runtime knowledge.

## 2. What Omnira is not

- Not a cloud AI service.
- Not a terminal-first developer dashboard.
- Not a model training platform.
- Not a node-graph-first workflow editor for beginners.
- Not trying to replace llama.cpp, Ollama, LM Studio, ComfyUI, Open WebUI,
  AnythingLLM, or similar tools.
- Not a tool that exposes overwhelming technical complexity by default.

## 3. Why Omnira exists

The open-source AI ecosystem is powerful but fragmented. Users need separate
tools for local chat, model management, image generation, RAG, voice, workflows,
and automation -- each with its own interface, model formats, setup steps, and
technical concepts. Omnira provides one friendly, local-first desktop workspace
that can eventually integrate and orchestrate multiple tools and runtimes
through one simple interface.

The differentiation: **the no-learning-curve desktop app for open-source AI.**
One installer, one interface, simple onboarding, local-first privacy, clean model
management, beginner-first UX, optional advanced diagnostics, and a modular
architecture underneath.

## 4. Target users

- **Privacy-first users:** people and organizations who require full network
  isolation, zero telemetry, and local-only data.
- **Non-technical users:** writers, students, and enthusiasts who want to use
  local GGUF models on consumer hardware without Docker, CLI tooling, or manual
  service management.
- **Power users and developers:** people who want a clean native desktop app for
  local models, with real diagnostics available when they ask for them.

## 5. Long-term direction (documented, not built)

The long-term goal is a unified local AI workstation: chat, model management,
image generation, voice, memory/RAG, agents, workflows, video, music, and
multimodal AI, orchestrated across multiple runtimes (llama.cpp/GGUF,
Windows ML/ONNX, CUDA/TensorRT). See `docs/runtimes-and-routing.md` for the
runtime strategy and `docs/roadmap.md` for phase ordering.

The MVP is intentionally narrow: local GGUF chat only. Public messaging must
always separate current MVP status from long-term vision.

## 6. Non-goals

- Replacing native runtimes. Omnira bundles or orchestrates them.
- Cloud account syncing, registration, telemetry, or remote backends by default.
- Implementing every AI capability in the first release.
