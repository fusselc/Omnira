# Since v0.1.0-alpha

- **Baseline tag:** `v0.1.0-alpha` (2026-07-27, `0558aec`)
- **Notes as of:** `origin/main` at `ecdb4cf` (21 commits after the tag)
- **Audience:** maintainer and invited testers preparing the next internal tag
- **App versions on `main`:** still `0.1.0` in `apps/desktop/package.json`,
  `apps/desktop/src-tauri/tauri.conf.json`, and
  `apps/desktop/src-tauri/Cargo.toml`. This document does **not** bump them.

This is a working summary of user-visible work already on `main` since the
internal alpha. It is not a tagged release and does not replace
[release-notes-0.1.0-alpha.md](release-notes-0.1.0-alpha.md).

## Proposed next tag

**`v0.1.1-alpha`**

Same product line as the internal alpha (Windows local GGUF chat, Vulkan/CPU
`llama-server`). The increment is polish and engine-control honesty, not a
new phase. Leave the `0.1.0` version fields as-is until someone cuts the tag
and rebuilds the installer; bump them in that tagging change if the artifact
name should match (`Omnira_0.1.1_x64-setup.exe` vs the current
`Omnira_0.1.0_x64-setup.exe` in the QA checklist).

Do not treat this as a public alpha. The unsigned-installer / SmartScreen
limitation from `v0.1.0-alpha` still applies.

## Lockfile

On this checkout, `apps/desktop/package-lock.json` matches `origin/main` and
the working tree is clean. No lockfile regeneration or dependency bump was
made for this notes PR. If another machine shows libc metadata churn in the
lockfile, restore it to `origin/main` rather than committing incidental npm
metadata.

## User-visible changes on `main`

These items are already merged. They are the reason to re-run QA before a new
tag.

### Engine control and honesty

- **Unload / Cancel:** Chat header and the in-use Models row can unload a ready
  model, cancel a load that is still starting, or stop the engine after a
  start error.
- **No console window** when `llama-server.exe` starts (`CREATE_NO_WINDOW`).
- **Startup stderr** (bounded) is captured while the engine is starting and
  folded into failure / fallback reasons in Advanced Diagnostics.
- Status copy is **“Running locally · llama.cpp”**; CPU vs GPU (Vulkan) is in
  the tooltip, not marketed as “GPU accelerated” in the main pill.
- CPU fallback notice copy on `main` is: *Running on CPU. Responses may be
  slower than with GPU acceleration.* See open PR #29 below.
- Sending is blocked when the loaded model is gone or no engine is running
  (no “Thinking…” without a model). Removing the in-use model unloads the
  engine and shows a “no longer available” path.
- In-flight generation is kept when you leave Chat and come back (Chat stays
  mounted).

### Vulkan overlay isolation

`llama-server` is spawned with `VK_LOADER_LAYERS_DISABLE=~implicit~` and
`VK_LOADER_LAYERS_ENABLE=*optimus*` so implicit layers from overlays
(Discord, Steam, RTSS, OBS, Overwolf, and similar) are not injected into the
child process. Scoped to that process only. Needs Vulkan loader ≥ 1.3.234;
older loaders treat the variables as a no-op.

### Branding and theme

- Window, taskbar, Start menu, and installer use the Omnira ring icon instead
  of the stock Tauri icon.
- Light theme applies immediately from Settings (no restart) and persists
  across relaunch. Contrast and focus treatment were tightened for light mode.

### Chat and a11y polish

- Assistant and user messages are selectable; Ctrl+C copies the selection.
- Code blocks have a keyboard-accessible copy control.
- Escape stops generation; a length-cutoff notice can appear; chat scroll
  sticks to the bottom while you are following the stream.
- Delete conversation uses an in-app confirm dialog.
- Composer labels, reduced motion, and focus-visible row actions.

### Docs (not installer-facing)

- [capability-map.md](capability-map.md) is the navigation / status index
  (shipped vs next vs deferred). Phase 6 CUDA is still next approved, not
  shipped.

### Not user-visible (on `main`, for completeness)

- Dependabot: `postcss` and `browserslist` bumps.
- Vitest coverage for truncation, markdown safety, theme, and mount policy.
- Browser mock backend: Models list refreshes after add (dev/mock only).

## Follow-up PRs

[#29](https://github.com/fusselc/Omnira/pull/29),
[#30](https://github.com/fusselc/Omnira/pull/30), and
[#31](https://github.com/fusselc/Omnira/pull/31) are small copy/docs PRs
landing together with these notes. They are not lingering open follow-ups.

The remaining follow-ups are:

- [PR #32](https://github.com/fusselc/Omnira/pull/32) — additive CUDA
  `RuntimeVariant` scaffolding for ChatProvider. Separate review; not part of
  this notes land. Scaffolding only; it is not CUDA shipping.
- The Phase 6 work #32 defers — official llama.cpp CUDA artifact pin +
  SHA-256, NVIDIA detection, and cudart DLL merge (the pieces worth
  salvaging from older [PR #23](https://github.com/fusselc/Omnira/pull/23)).

## Re-run before tagging

Full list: [release-qa-checklist.md](release-qa-checklist.md). Priority
re-runs for work landed since `v0.1.0-alpha`:

- **Clean install / uninstall** (sections 1 and 8) on a new NSIS artifact
  built from the tag commit. Confirm the installer still uses the Omnira
  icon (section 4c).
- **Offline first run + local GGUF** (section 2). If #30 is still open,
  watch the first-run “Add a model” copy for Hugging Face / download wording.
- **Model start and chat** (section 3), including:
  - no console / Windows Terminal window
  - “Running locally · llama.cpp” pill and tooltip
  - Stop during generation; stream survives Chat → Models → Chat
  - selectable copy (Ctrl+C)
  - **Unload model** and **Cancel loading**
  - remove the loaded model from Models, then return to Chat
  - Vulkan load with Discord/Steam/RTSS (or similar) running — no
    ErrorDeviceLost
- **CPU fallback notice** (section 4) and “Try GPU acceleration again” in
  Advanced Diagnostics. Re-check copy if #29 has merged.
- **Theme** (section 4b): switch Light/Dark without restart; persist across
  relaunch.
- **Rename / delete** (section 5): in-app delete confirm.
- **Persistence** (section 6) and **orphan `llama-server.exe`** on normal
  and forced close (section 7).

No performance numbers or pass/fail counts are recorded here. Record evidence
when the tag build is actually QA’d.
