# Phase 7 Implementation Plan -- Image Generation

Ordered, PR-sized steps for Phase 7. Status: **Next approved (design + headless
spike)**, in parallel with Phase 6 under the sequencing rule in
[roadmap.md](roadmap.md). The runtime decision is
[ADR 0002](adr/0002-image-provider-worker.md); the contract is
[image-provider.md](image-provider.md).

## Ground rules for every Phase 7 PR

- One step per PR, merged in order. A later step does not start until the
  previous step's acceptance tests pass on `main`.
- **Gate:** PRs 3 through 8 do not start until the spike's go/no-go decision on
  ADR 0002 D3 (per-job `sd-cli` transport) is recorded in
  `docs/evidence/phase7-spike.md`. A "no-go" sends the work back to a revised
  ADR before any further implementation.
- Create stays hidden in release builds until step 8, and step 8 merges only
  when every [Phase 7 done criterion](roadmap.md#phase-7-done-criteria) passes.
- No Python, no network calls, no downloads at runtime, no committed binaries
  or model files. Test fixtures are synthetic headers and text, never real
  model weights.
- Phase 7 PRs must not regress Chat or Phase 6. Changes to shared seams
  (`process/`, the packaging fetch script, the error taxonomy) are reviewed
  against both phases.
- Each PR updates the docs it changes (contract, storage, security, packaging)
  in the same change.

## Step 0 -- Docs authorization (this change)

ADR 0002, `image-provider.md`, this plan, and status updates in the roadmap,
capability map, and `AGENTS.md`.

**Acceptance:** docs-only diff (`*.md`); links resolve; roadmap, capability
map, and `AGENTS.md` agree on Phase 6 and Phase 7 status.

## Step 1 -- Headless worker spike (evidence only)

Pick a stable-diffusion.cpp release to pin and exercise `sd-cli` by hand on
Windows, outside the app. No source changes; the deliverable is
`docs/evidence/phase7-spike.md`.

Measure and record:

- Pinned tag, artifact names, and SHA-256 for the Vulkan and AVX2 Windows zips
  (and CUDA, for the Phase 6 distribution discussion only).
- For SD 1.5 at 512x512 and SDXL at 1024x1024, on Vulkan and on CPU, on at
  least one tier A or tier B machine and one CPU-only run:
  - model load time (spawn to first progress line),
  - per-step time and total job time,
  - peak memory where observable.
- The exact progress-line format, captured verbatim as future test fixtures
  (with the prompt removed).
- Cancel latency: time from terminating the process to the process being gone,
  during loading and during sampling.
- Whether `sd-cli` has an option to disable PNG metadata embedding, and whether
  it accepts the prompt from a file.
- Whether the run works with networking disconnected, and that no network
  activity or listening socket is observed (Resource Monitor / `netstat`).
- Spawned with `CREATE_NO_WINDOW`: confirm no console window appears.
- Re-check of `sd-server` on the pinned build: API-key support and CORS
  behavior (ADR 0002 D3 conditions).

**Acceptance:** the evidence file contains every item above with numbers, and
ends with an explicit **go / no-go on ADR 0002 D3** signed off by a maintainer,
plus the server-mode trigger evaluation below.

### Server-mode trigger

Per-job mode reloads the model on every generation. Moving to `sd-server`
becomes the next Phase 7 follow-up (still subject to ADR 0002 D3's auth and
CORS conditions) if the spike measures, on tier B hardware on the Vulkan
variant, either:

- a **median per-job model load time above 15 seconds** for SDXL at
  1024x1024, or
- model load time **above 50% of total job wall time** for SD 1.5 at 512x512.

If neither threshold is exceeded, per-job mode ships and the trigger is
re-evaluated when the pin is bumped.

## Step 2 -- Packaging: pin the worker

Uses the tag and checksums recorded in the step 1 evidence. Extend the
packaging fetch script to download the pinned `sd-cli` Vulkan and AVX2
artifacts, verify SHA-256 fail-closed, and extract into a gitignored
binaries directory bundled as Tauri resources. Add stable-diffusion.cpp (MIT)
and its bundled dependencies to `THIRD_PARTY_LICENSES`. Update
[packaging-process-model.md](packaging-process-model.md).

**Acceptance:**

- A tampered or wrong checksum aborts the script and removes the download.
- `git ls-files` shows no worker binaries.
- The NSIS installer size delta is measured and recorded.
- Chat runtime fetching and Phase 6 CUDA slot behavior are unchanged.

## Step 3 -- Supervisor and orphan check

*Requires the step 1 go decision.*

Add the image runtime module in the Rust core: variant probe (Vulkan, then
CPU, recorded in config), per-job spawn into the existing Job Object with
`CREATE_NO_WINDOW` and the Vulkan layer environment, argument-vector
construction (no shell), progress parser, cancel by termination, bounded
prompt-free stderr classification, and error normalization including the new
`ImageModelUnsupported` and `OutputWriteFailed` codes (friendly copy added to
`design-principles.md`).

**Acceptance:**

- `cargo test`: progress-parser tests against the spike's fixtures; error
  classification tests for missing runtime, load failure, out of memory, and
  unknown output; argument construction rejects out-of-range requests.
- Manual on Windows: no console window; cancel completes within 2 seconds;
  force-killing `omnira.exe` mid-job leaves no worker in `Get-Process`.
- Logs from a full job contain no prompt text.

## Step 4 -- Storage and image model registry

*Requires the step 1 go decision.*

Add `image_models` and `images` tables (`CREATE TABLE IF NOT EXISTS`), header
sniffing for safetensors and GGUF (architecture detection for SD 1.x, SD 2.x,
SDXL; `.ckpt` / `.pt` / `.pth` rejected), the `images\` and
`images\.staging\` directories, the completion write order with metadata
stripping, startup reconciliation, and deletion flows. Update
[data-ownership-and-storage.md](data-ownership-and-storage.md).

**Acceptance:**

- `cargo test`: opening an existing database with chat data adds the new
  tables without touching existing rows; sniffing tests with synthetic
  headers (each supported architecture, an unsupported architecture, corrupt
  header, pickle formats rejected); reconciliation removes staging and
  row-less files and flags missing files; delete and clear remove rows and
  files; removing an image model keeps its images and never touches the model
  file.
- A saved test PNG contains no prompt text chunk.

## Step 5 -- IPC

*Requires the step 1 go decision.*

Register the commands and the `image-job:{job_id}` event from
[image-provider.md](image-provider.md) section 9. Add TypeScript types and
wrappers in the frontend IPC module and the mock layer. Nothing in the visible
UI calls them yet.

**Acceptance:**

- `cargo test` covers command validation (active-job rejection, unknown ids,
  missing model).
- Vitest covers a job-event reducer that switches exhaustively on `kind` with
  a `never` default, and the mock implementations.
- TypeScript type-check and lint pass; no visible screen references the new
  commands.

## Step 6 -- Hidden dev-only Create route

*Requires the step 1 go decision.*

A minimal Create route (model picker, prompt, generate, progress, cancel,
gallery, delete) reachable only in development builds, gated by
`import.meta.env.DEV`, with no navigation entry and no settings flag.

**Acceptance:**

- A production frontend build contains no Create route: a search of the built
  bundle for the route path and Create screen strings finds nothing.
- In `tauri dev`, a maintainer can generate, cancel, view, and delete an image
  end to end.
- UI copy says "Running locally"; no variant names outside Diagnostics.

## Step 7 -- End-to-end test

*Requires the step 1 go decision.*

An opt-in integration test that runs a real generation when an environment
variable points at a local image model (skipped in CI, never downloads), plus
a manual run of every Phase 7 done criterion on a release-candidate build,
recorded in `docs/evidence/`. Add the Create checks to
[release-qa-checklist.md](release-qa-checklist.md).

**Acceptance:**

- The opt-in test passes on Vulkan and CPU on a maintainer machine.
- The evidence file lists each done criterion with its result.
- Offline run and orphan-process run are included.

## Step 8 -- Un-hide Create

*Requires the step 1 go decision and every done criterion passing.*

Add the Create navigation entry for release builds; Settings shows the images
location; Advanced Diagnostics shows the image runtime and variant. Update the
roadmap and capability map to **Shipped**, update `AGENTS.md` active screens,
release notes, and remove image generation and Create from the "not current
features" lists.

**Acceptance:**

- All 14 [Phase 7 done criteria](roadmap.md#phase-7-done-criteria) verified on
  an installed NSIS build with maintainer sign-off.
- Chat regression checklist passes; Windows CI is green.
