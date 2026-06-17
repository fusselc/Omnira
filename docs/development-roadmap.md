# Development Roadmap

## Phase 0: Foundation (current)

- Define monorepo layout
- Implement provider interfaces and orchestration skeleton
- Introduce configuration and logging systems
- Add unit test scaffolding and contributor docs

## Phase 1: Runtime integrations

- Add first local providers (llama.cpp, ONNX Runtime)
- Implement model loading lifecycle
- Add streaming token/event interfaces

## Phase 2: Modalities

- Add vision/audio/image/video capability contracts
- Add provider-specific capability negotiation

## Phase 3: Knowledge + automation

- RAG interfaces and vector store adapters
- Tool calling and agent orchestration
- Workflow execution graph support

## Phase 4: Desktop experience

- Tauri + React + TypeScript frontend scaffold
- IPC bridge to Python backend
- Local model management UI
