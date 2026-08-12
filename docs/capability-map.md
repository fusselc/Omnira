# Omnira capability map

This document is a **navigation and status index** for Omnira's shipped product
and long-term direction. It helps maintainers and coding agents find the right
phase and avoid implementing future work out of order.

**It is not a standalone canonical authority.** If anything here conflicts with
detailed architecture, ADRs, provider contracts, privacy rules, packaging
process, or the roadmap, those documents win:

| Topic | Authoritative source |
|---|---|
| Phase order and milestones | [roadmap.md](roadmap.md) |
| Runtime pillars and providers | [runtimes-and-routing.md](runtimes-and-routing.md) |
| Chat contract (MVP) | [chat-provider.md](chat-provider.md) |
| Privacy and network defaults | [privacy.md](privacy.md) |
| Security / process boundaries | [local-security-boundary.md](local-security-boundary.md) |
| Orchestrator decision | [adr/0001-rust-tauri-core-orchestrator.md](adr/0001-rust-tauri-core-orchestrator.md) |
| Product vision | [vision.md](vision.md) |
| Agent working rules | [../AGENTS.md](../AGENTS.md) |

## Status legend

Use these exact categories:

| Status | Meaning |
|---|---|
| **Shipped** | On `main` and part of the current alpha product. |
| **Next approved** | Approved for implementation next; not shipped yet. |
| **Deferred** | Planned, but blocked until prior work and done-criteria are met. |
| **Later** | On the roadmap after the next approved / deferred work. |
| **Unscheduled** | Intended direction without a fixed phase number. |
| **Requires design decision** | Must not be implemented until an explicit design chooses UX and/or runtime. |
| **Requires explicit product decision** | Not planned as default product behavior; would need a deliberate product + privacy decision before any work. |

## Current product snapshot

- **Shipped on `main`:** Windows local GGUF chat; model registry (in-place
  references); local persistence; settings; Advanced Diagnostics; Vulkan/CPU/CUDA
  managed `llama-server`; packaged alpha (`v0.1.0-alpha`).
- **Next approved engineering:** none beyond completing Phase 6 on `main`. Phase 7
  Create remains deferred.
- **Deferred:** Phase 7 `ImageProvider` / Create. Create must **not** become an
  active visible screen until end-to-end criteria are met: generate, see,
  delete, and relaunch to view a local image without a terminal; worker under
  Job Object supervision; no orphaned worker after force-close.
- **Not current features:** image generation, video, voice, music, RAG,
  document chat, multimodal chat / file understanding, agents, workflows,
  plugins, web search, cloud providers, or model downloading.

## Privacy defaults (non-negotiable)

- No accounts, telemetry, cloud sync, default external network calls, or silent
  downloads.
- Network-capable features—including web search and model-download
  assistance—must be **off by default** and require **explicit, understandable
  permission** before use. See [privacy.md](privacy.md).

## Agent / tool safety (future design guardrail)

This does **not** authorize implementing agents, tools, workflows, or plugins
now. When those features are designed:

- By default, **no agent, workflow, plugin, or tool may cause an external side
  effect without explicit per-action user approval.**
- Side effects include, at minimum: file writes and deletes; shell or process
  execution; browser or network actions; clipboard writes; and changes to
  settings, models, or runtimes.
- Any future **persistent** permission must be **narrow, visible, revocable,
  and documented** before it ships.

## Capability index

| Capability | Status | Phase / timing | Provider / runtime (planned) | Notes for agents |
|---|---|---|---|---|
| Local GGUF chat (stream, stop, persist) | Shipped | MVP Phases 0–5 / alpha | `ChatProvider` / llama-server Vulkan+CPU | Current product core |
| Model registry (in-place GGUF; rename; remove without deleting file) | Shipped | MVP | Registry in Rust / SQLite | No model downloads |
| Local persistence, settings, diagnostics | Shipped | MVP | Rust core | Prompt-free logs; redacted export |
| Packaged Windows alpha (NSIS, offline-capable) | Shipped | Phase 5 / `v0.1.0-alpha` | Bundled Vulkan+CPU runtimes | WebView2 prerequisite; unsigned internal alpha |
| CUDA acceleration for existing ChatProvider | Shipped | Phase 6 | Same `ChatProvider`; CUDA llama-server variant | Same Chat UI; Diagnostics names accelerator; not TensorRT diffusion |
| Image generation (Create) | Deferred | Phase 7 | `ImageProvider`; CUDA/TensorRT or managed diffusion worker | No active Create screen until end-to-end criteria above are met |
| Windows ML / ONNX / NPU tasks | Later | Phase 8 | `OnnxProvider`; Create vision/audio tasks | Computer-vision / ONNX tasks — not multimodal chat |
| Video generation | Later | Phase 9 | `VideoProvider`; CUDA/TensorRT | After image path is stable |
| Memory, embeddings, RAG, document chat | Later | Phase 10 | `EmbeddingProvider`, `RagProvider` | Distinct from multimodal file-understanding UX |
| Agents, tool calling, workflows | Later | Phase 10 | `ToolAgentProvider`, `WorkflowProvider` | Subject to agent/tool safety rule; not authorized now |
| Voice conversation / ASR / TTS | Later | Phase 11 | `SpeechToTextProvider`, `TextToSpeechProvider` | Voice screen |
| Plugin / community provider ecosystem | Later | Phase 12 | Provider registry | Sandbox and permissions require design |
| Multimodal chat and file understanding | Requires design decision | No fixed phase | TBD — must not silently extend current `ChatProvider` | **Not** image generation, **not** Phase 8 CV tasks, **not** RAG. Needs deliberate attachment/ingestion UX and an explicit provider/runtime decision before any Chat changes |
| Music / audio generation | Unscheduled | — | `MusicAudioProvider` | Direction only; no phase row |
| Model download / import assistance | Unscheduled | — | Network-capable assistance | Off by default; explicit permission; never silent downloads |
| Hardware detection UI | Unscheduled | — | Diagnostics / settings UX | Hardware-aware routing grows per phase; dedicated UI unscheduled |
| ComfyUI as managed worker | Unscheduled | — | Under image / workflow workers | Never the beginner-default UX |
| Optional web search / other network providers | Requires design decision | — | `WebSearchProvider` et al. | Off by default; permission model designed before ship |
| Cloud providers, accounts, sync, telemetry | Requires explicit product decision | — | — | Not planned as default product behavior. Omnira must retain no accounts, no telemetry, no cloud sync, and no required external network calls by default. Any optional cloud capability would require an explicit future product decision, privacy design, and user permission model |
| Agent / tool approval and safety UX | Requires design decision | Before any agent/tool ship (ties to Phase 10) | UX and policy | Guardrail above; does not authorize agents now |
| Persistent grants for tools / plugins | Requires design decision | With plugin / agent design | Permission model | Must be narrow, visible, revocable, and documented |

### Multimodal chat and file understanding

**Multimodal chat and file understanding** means attaching or ingesting user
files or images into a **conversational** understanding flow. It is **not**
image generation (Phase 7), **not** computer-vision Create tasks (Phase 8), and
**not** RAG or document memory (Phase 10). It is unstarted. Do not assign it a
roadmap phase number until a design decision exists. Do not silently extend the
current text `ChatProvider` or Chat UI to add attachments or multimodal
understanding.

## Related reading

- [roadmap.md](roadmap.md) — phase table and ordering
- [runtimes-and-routing.md](runtimes-and-routing.md) — pillars and provider list
- [privacy.md](privacy.md) — local-first promise
- [chat-provider.md](chat-provider.md) — shipped Chat contract only
- [vision.md](vision.md) — product vision vs shipped scope
