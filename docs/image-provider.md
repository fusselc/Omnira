# Image Provider (Phase 7)

Phase 7 adds local image generation behind a narrow `ImageProvider` contract.
Chat remains unchanged. The main UI gains a **Create** screen for image work.

## Status

- **UI:** Create screen (prompt, size, generate, gallery of local results).
- **Persistence:** generation metadata under SQLite; image files under
  `%LOCALAPPDATA%\Omnira\generations\`.
- **Worker:** optional managed diffusion worker. When no worker binary is
  present, Generate fails closed with a friendly error and never makes network
  calls. Bundling a specific diffusion runtime is additive (same packaging
  pattern as llama-server variants).

## Contract

```text
start_image_runtime(model_path) -> status
stop_image_runtime()
generate_image({ prompt, width, height, seed? }) -> GenerationRecord
list_generations() -> GenerationRecord[]
delete_generation(id)
```

Rules (same as ChatProvider privacy/security posture):

- Loopback-only if the worker exposes an HTTP API.
- Job Object supervision; no orphaned workers after quit/force-kill.
- Prompt-free logs (no prompt text in log lines).
- Models referenced in place; Omnira does not download weights.
- Main UI says "Running locally"; accelerator detail stays in Diagnostics.

## Storage

| Path | Contents |
|---|---|
| `%LOCALAPPDATA%\Omnira\generations\<id>.png` | Output image bytes |
| SQLite `generations` table | id, prompt, width, height, path, status, created_at |

## Out of scope for this phase slice

Video, ComfyUI workflow UI, cloud image APIs, model download marketplaces.
