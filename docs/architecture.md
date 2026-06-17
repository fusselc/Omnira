# Omnira Architecture (Initial)

## Goals

- Local-first by default
- No telemetry by default
- No cloud dependency by default
- Provider-based, vendor-neutral inference architecture
- Clean separation of orchestration and runtime providers

## Monorepo Structure

```text
Omnira/
├── apps/
│   └── desktop/               # future Tauri + React + TypeScript UI
├── docs/
│   ├── architecture.md
│   ├── development-roadmap.md
│   └── environment-setup.md
├── src/omnira/
│   ├── api/                   # FastAPI app skeleton and DI wiring
│   ├── config/                # Pydantic settings and configuration loading
│   ├── core/                  # orchestration (runtime/model managers)
│   ├── observability/         # logging setup
│   └── providers/             # provider interfaces, bases, registry, plugin loading
└── tests/unit/
    ├── core/
    └── providers/
```

## Core Components

- `ProviderMetadata` + `ProviderCapability`: provider description and feature indexing.
- `BaseProvider`: abstract provider lifecycle + inference contract.
- `ProviderRegistry`: central provider registration and capability lookup.
- `RuntimeManager`: lifecycle orchestration across providers.
- `ModelManager`: model inventory and provider association.
- `AppSettings`: pydantic configuration with local-first defaults.
- `configure_logging`: centralized logging setup.
- `discover_provider_factories`: entry-point plugin discovery skeleton.

## Planned Provider Families

- llama.cpp
- ONNX Runtime
- Windows ML
- TensorRT
- DirectML
- OpenVINO
- ComfyUI
- CUDA-based runtimes

## Future Features This Architecture Supports

- Local LLMs
- Vision, audio, image, and video models
- Voice assistants
- RAG and vector database integrations
- Agents and tool calling
- Workflow automation
- Web search
- Streaming inference

## Plugin Strategy (Initial)

External providers will load through Python entry points in the `omnira.providers` group.
Core providers can be statically registered first; external plugins can be dynamically discovered later.

Potential future hardening:
- signed plugin manifests
- provider capability compatibility validation
- sandbox policy and permission scopes
