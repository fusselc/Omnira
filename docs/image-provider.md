# ImageProvider -- Phase 7 Provider Contract

**Status:** Phase 7 is **Next approved (design + headless spike)**. Nothing in
this contract is shipped. The Create screen must not become an active visible
screen until the Phase 7 done criteria in [roadmap.md](roadmap.md) pass. The
runtime decision is recorded in
[ADR 0002](adr/0002-image-provider-worker.md); the ordered work is in
[phase7-implementation-plan.md](phase7-implementation-plan.md).

Phase 7 defines one new provider abstraction, **ImageProvider**, with one
implementation: **SdCliImageProvider** (stable-diffusion.cpp `sd-cli`, one
process per job). It does not change [ChatProvider](chat-provider.md), and it
does not add attachments or image understanding to Chat.

## 1. Responsibilities

An ImageProvider describes:

- Provider name and version (including the pinned stable-diffusion.cpp build)
- Runtime variant probe (the image equivalent of a health check)
- Supported model formats (v1: single-file `.safetensors` and
  stable-diffusion.cpp-compatible `.gguf`; architectures SD 1.x, SD 2.x, SDXL)
- Generate behavior (text-to-image, one job at a time)
- Progress reporting
- Cancellation behavior
- Structured error reporting against the taxonomy below

The provider owns spawn/supervise/progress/cancel/error-reporting for its
worker. The Rust core owns the image model registry, output storage, SQLite,
config, and IPC.

## 2. Lifecycle

```
Idle -> Probe Variant -> Queued -> Loading -> Sampling (progress) -> Encoding
    -> Completed | Cancelled | Failed -> Idle
```

- **Probe Variant** runs once per app session (and again after "Try GPU
  acceleration again" or a variant failure). It replaces `/health` polling,
  because per-job mode has no long-lived server.
- **Loading** starts at spawn and ends at the first parsed progress line.
- **Encoding** covers VAE decode, file write, metadata stripping, and the move
  out of staging.

State transitions surface to the frontend as typed events so the UI can show
plain-language status ("Loading model...", "Generating...", "Running locally").

## 3. SdCliImageProvider specifics

### Spawn and supervision

- The worker is a **direct child** of the Tauri process, spawned by the Rust
  core and assigned to the existing Windows Job Object
  (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) at spawn time. Force-killing Omnira
  kills the worker.
- Created with `CREATE_NO_WINDOW`, null stdin, and piped stdout/stderr. No
  console is ever visible. No `cmd.exe` wrapper.
- The child environment sets the same Vulkan loader layer filters as
  `llama-server` (see [packaging-process-model.md](packaging-process-model.md)
  section 1), never written to the global environment.
- Output path is a per-job staging directory chosen by the core (section 6).
- The worker opens **no listening socket**. There is no port and no API key in
  per-job mode.

### Variant probe ("health gating")

- Order: Vulkan, then CPU (AVX2). CUDA is added only after the Phase 6
  distribution decision (ADR 0002 D6).
- The probe runs the pinned binary with a version/info argument and checks the
  exit status and reported build. A variant that fails to start, or whose first
  real job fails before sampling with a device/backend error, falls back once to
  the next variant. The working variant is recorded in config, mirroring the
  chat runtime.
- Generation is not offered (`image_generate` rejects with `RuntimeMissing` or
  `RuntimeFailedToStart`) until a variant has probed successfully.

### Concurrency

- **One active image job at a time.** A second `image_generate` while a job is
  active is rejected; the UI disables the action.
- Image and chat may coexist. Starting an image job does **not** unload the
  chat model. If the worker fails with memory exhaustion, the error maps to
  `InsufficientMemory` and the suggested action is to unload the chat model and
  try again.

### Progress

- The core parses the pinned `sd-cli` progress line into `{step, total}` and
  emits a `progress` event, throttled to at most about 10 events per second.
- All other stdout is discarded unread. stderr is kept only as a bounded
  (4 KiB) tail for classifying failures, and it is never logged verbatim because
  it may echo the prompt (section 8).
- The progress parser is covered by fixture tests per pinned build; a format
  change fails CI, not users.

### Cancellation

- `image_cancel` terminates the worker process. The target is that the
  process is gone and the job is `cancelled` within **2 seconds**.
- The job's staging directory is removed. No partial image is kept.
- Cancellation during **Loading** behaves the same way.

### Future server mode (not shipped)

`sd-server` may replace per-job mode only under the conditions in ADR 0002 D3.
If it does, it inherits every `llama-server` rule: spawned with an explicit
`127.0.0.1` bind, a reserved loopback port (port-race retry up to 3 attempts),
a per-session secret generated in memory and never persisted or logged,
`/health`-style gating, and `UnauthorizedLocalRequest` for rejected requests.
The webview still never talks to the worker (ADR 0002 D4); the key stays in the
Rust core.

## 4. Generate request

v1 request fields (validated in the Rust core before spawn):

| Field | Notes |
|---|---|
| `model_id` | Image model registry id; must exist, be present on disk, and be a supported architecture |
| `prompt` | Required, non-empty |
| `negative_prompt` | Optional |
| `width`, `height` | Multiples of 64; bounded per architecture (defaults: 512 for SD 1.x/2.x, 1024 for SDXL) |
| `steps` | Bounded; architecture-specific defaults (Turbo variants use few steps) |
| `cfg_scale` | Bounded |
| `seed` | Optional; random when absent, and the used seed is recorded |
| `sampler` | Restricted to an allowlist supported by the pinned build |

Arguments are passed to `sd-cli` as a discrete argument vector, never through
a shell. The spike checks for a prompt-file option to keep the prompt out of
the process command line (ADR 0002 tradeoffs).

## 5. Error taxonomy

Image errors use the same shared taxonomy as [chat-provider.md](chat-provider.md)
section 6: every user-facing failure maps to exactly one code, carrying an
internal code, a friendly message, a suggested action, technical detail for
Advanced Diagnostics, and a log entry. Friendly copy for the new codes is added
to `docs/design-principles.md` in the PR that introduces them.

| Code | Meaning in image context | Normalized by |
|---|---|---|
| `RuntimeMissing` | Bundled `sd-cli` variant not found | Rust core |
| `RuntimeFailedToStart` | No variant probed successfully, or spawn failed | Rust core |
| `ModelFileMissing` | Registered image model file moved or deleted | Rust core |
| `ModelFormatInvalid` | Header sniff failed (not safetensors/GGUF, corrupt, or excluded format such as `.ckpt`) | Rust core |
| `ImageModelUnsupported` (new) | Valid file, but the architecture is outside v1 scope (for example FLUX, SD3) | Rust core |
| `ModelLoadFailed` | Worker started but could not load the model | Rust core |
| `InsufficientMemory` | Worker failed with GPU or system memory exhaustion | Rust core |
| `GenerationCancelled` | User cancelled the job | Rust core |
| `GenerationFailed` | Worker failed after sampling began | Rust core |
| `OutputWriteFailed` (new) | Output could not be written, stripped, or moved (disk full, permissions) | Rust core |
| `BackendUnavailable` | Rust core or image subsystem not reachable when expected | Rust core |
| `UnauthorizedLocalRequest` | Server mode only (not shipped) | Rust core |
| `UnknownRuntimeError` | Anything unclassified | Rust core |

Unlike chat, there is no frontend streaming module: all image errors are
normalized in the Rust core and delivered as typed command errors or
`failed` events.

## 6. Output storage

Under `%LOCALAPPDATA%\Omnira\` (see
[data-ownership-and-storage.md](data-ownership-and-storage.md)):

| Path | Contents |
|---|---|
| `images\<image_id>.png` | Finished images, owned by Omnira |
| `images\.staging\<job_id>\` | Per-job worker output; never shown to the user |

Write order for a completed job:

1. The worker writes into `images\.staging\<job_id>\`.
2. The core validates the PNG signature and dimensions, removes embedded
   generation metadata (ADR 0002 D8), and writes the final file to
   `images\<image_id>.png`.
3. The core inserts the `images` row in a transaction.
4. The staging directory is removed.

**Startup reconciliation** (before any image IPC is served):

- Delete everything under `images\.staging\`.
- Delete files in `images\` that have no `images` row.
- Rows whose file is missing are reported as missing in the gallery (shown,
  never silently dropped) and can be deleted by the user.

Image model files are **referenced in place**, never copied, moved, or
modified.

## 7. SQLite additions

Added to `data\omnira.db` with the same `CREATE TABLE IF NOT EXISTS` pattern
the existing schema uses, so opening an existing database is non-destructive.
Jobs are in-memory only; there is no persisted job table.

```sql
CREATE TABLE IF NOT EXISTS image_models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    file_size_bytes INTEGER NOT NULL,
    format TEXT NOT NULL CHECK (format IN ('safetensors', 'gguf')),
    architecture TEXT NOT NULL CHECK (architecture IN ('sd1', 'sd2', 'sdxl')),
    added_at TEXT NOT NULL,
    last_used_at TEXT
);

CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY,
    model_id TEXT,
    prompt TEXT NOT NULL,
    negative_prompt TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    steps INTEGER NOT NULL,
    cfg_scale REAL NOT NULL,
    seed INTEGER NOT NULL,
    sampler TEXT NOT NULL,
    runtime_variant TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_images_created ON images(created_at);
```

- The image registry is separate from the chat `models` table, so chat
  registry behavior is untouched.
- `images.model_id` deliberately has no foreign key: removing an image model
  keeps the images that were generated with it.
- `file_name` is relative to `images\`; absolute paths are never stored for
  outputs.

## 8. Deletion flows

| Action | Effect |
|---|---|
| Delete image | Delete the `images` row, then the file. If the file delete fails, the next startup reconciliation removes it. |
| Clear all images | Same as above for every row, then remove any remaining files under `images\`. Blocked while a job is active. |
| Remove image model | Remove the `image_models` row only. The model file is never deleted; images generated with it are kept. |
| Uninstall | Unchanged: user data under `%LOCALAPPDATA%\Omnira\` is preserved unless the user opts to remove it. |

Settings shows the images location read-only, alongside the existing data
paths, when Create is un-hidden.

## 9. IPC contract

Commands follow the existing snake_case naming; the job event follows the
existing `chat-stream:{stream_id}` pattern.

### Commands

| Command | Returns | Notes |
|---|---|---|
| `image_runtime_status` | Status, working variant, fallback reason | Variant shown only in Advanced Diagnostics |
| `list_image_models` | Image model entries with status (ok / missing / invalid / unsupported) | |
| `add_image_model(path)` | Image model entry | Header sniff before insert; in place only |
| `remove_image_model(id)` | `()` | Registry row only; cancels an active job using that model |
| `rename_image_model(id, name)` | `()` | Display name only |
| `image_generate(request)` | `job_id` | Rejects if a job is active or validation fails |
| `image_cancel(job_id)` | `()` | Idempotent |
| `list_images` | Image metadata, newest first | No pixel data |
| `image_read(id)` | `data:image/png;base64,...` | The only path pixels take to the webview |
| `delete_image(id)` | `()` | |
| `clear_images` | `()` | |

### Event: `image-job:{job_id}`

A discriminated union on `kind`:

| `kind` | Payload |
|---|---|
| `queued` | -- |
| `loading` | -- |
| `progress` | `step`, `total` |
| `completed` | `image_id` |
| `cancelled` | -- |
| `failed` | Normalized error (section 5) |

Every job ends with exactly one of `completed`, `cancelled`, or `failed`.
Frontend handlers switch exhaustively on `kind` with a `never` check in the
default branch.

## 10. Privacy and security

- **No network.** The worker makes no network calls and opens no listening
  socket; the full flow works with networking disconnected. Verified by the
  spike and again by the done criteria.
- **No downloads.** No runtime or model is ever downloaded; `sd-cli` variants
  ship in the installer from pinned, checksum-verified artifacts.
- **Models in place.** Never copied, moved, or modified; removal never deletes
  the file.
- **Prompt-free logs.** Logs record job lifecycle, variant, timings, sizes,
  and error codes only. Never prompts, negative prompts, or raw worker output.
- **Diagnostics export** includes image runtime status and recent error codes;
  prompts are never included, and paths are redacted by default.
- **Saved images carry no prompt** (ADR 0002 D8).
- **Webview isolation.** The webview reaches the image subsystem only through
  the typed commands above (Layer 1 in
  [local-security-boundary.md](local-security-boundary.md)). CSP is unchanged:
  images render from `data:` URLs, already allowed by `img-src 'self' data:`.
- **UI copy.** Main UI says "Running locally"; variant names (Vulkan, CPU,
  later CUDA) appear only in Advanced Diagnostics.
