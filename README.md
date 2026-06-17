# Omnira

> A unified, local-first AI workstation for language, vision, audio, image generation, video generation, agents, and workflows.

Omnira is an open source platform designed to bring modern AI tools together into a single, cohesive desktop experience.

Today, users often rely on multiple disconnected applications:

- LM Studio for local language models
- Ollama for model management
- ComfyUI for image and workflow pipelines
- Open WebUI for chat interfaces
- AnythingLLM for knowledge bases and RAG
- Various standalone tools for audio, video, and automation

Omnira aims to provide a unified experience while remaining modular, extensible, and fully local-first.

## Vision

The goal of Omnira is not to replace existing open source projects.

Instead, Omnira serves as a common platform that can integrate and orchestrate multiple AI runtimes, model formats, and workflows through a consistent interface.

Core principles:

- Local-first
- Open source
- Provider-based architecture
- Hardware acceleration where available
- Cross-platform support
- Extensible plugin ecosystem
- User ownership of data and models

## Current Status

🚧 Early Development

Omnira is currently in active development.

The initial focus is building a solid foundation before expanding into advanced functionality.

## MVP Goals

The first milestone is intentionally small:

- Load local GGUF models
- Chat with models locally
- Switch between installed models
- Manage model settings
- Build a provider-based architecture for future runtimes

The project will expand incrementally after the core foundation is complete.

## Planned Features

### Language Models

- GGUF support
- llama.cpp integration
- Multi-model management
- Local chat interface
- Conversation history
- Prompt templates

### Inference Providers

- llama.cpp
- ONNX Runtime
- Windows ML
- DirectML
- TensorRT
- OpenVINO
- Future providers

### Vision

- Image classification
- Object detection
- Segmentation
- OCR

### Image Generation

- Stable Diffusion support
- ComfyUI integration
- Workflow execution
- Model management

### Video Generation

- Local video generation workflows
- Model management
- Pipeline orchestration

### Audio

- Speech-to-text
- Text-to-speech
- Voice assistants
- Audio workflows

### Agents

- Local AI agents
- Tool calling
- Workflow automation
- Multi-agent systems

### Knowledge and RAG

- Document ingestion
- Local vector databases
- Retrieval pipelines
- Personal knowledge bases

## Architecture

Omnira is built around a provider architecture.

Every inference backend implements a common interface.

```python
class InferenceProvider:
    def load_model(self):
        pass

    def unload_model(self):
        pass

    def generate(self, prompt):
        pass
```

This allows new runtimes to be added without changing the rest of the application.

Example providers:

```
Providers

├── llama.cpp
├── ONNX Runtime
├── Windows ML
├── TensorRT
├── DirectML
└── OpenVINO
```

## Technology Stack

### Frontend

- React
- TypeScript
- Tailwind CSS
- Tauri

### Backend

- Python

### AI Frameworks

- llama.cpp
- ONNX Runtime
- Windows ML
- Hugging Face ecosystem

## Why Omnira?

Existing tools are excellent at solving individual problems.

Omnira aims to provide a unified platform that connects these capabilities without requiring users to learn and manage multiple disconnected applications.

The goal is to make local AI more accessible, more modular, and easier to extend.

## Roadmap

### Phase 1

- Repository setup
- Core architecture
- Provider interfaces
- GGUF model loading
- Local chat interface

### Phase 2

- Model management
- ONNX Runtime support
- Settings system
- Plugin framework

### Phase 3

- Image generation
- Audio capabilities
- Knowledge base support

### Phase 4

- Agents
- Workflow orchestration
- Advanced automation

### Phase 5

- Expanded provider ecosystem
- Community contributions
- Plugin marketplace

## Contributing

Contributions, feedback, ideas, and discussions are welcome.

As the project matures, contribution guidelines and development documentation will be expanded.

## License

Licensed under the Apache License 2.0.

## Screenshots

Coming soon.
