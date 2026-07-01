# Design Principles

Omnira's default experience is calm, modern, private, and beginner-friendly.
Power exists, but it waits to be asked for.

## 1. Core principles

- **Design around tasks, not model internals.** The user opens an app, picks a
  model, and chats privately. They never feel like they are managing services.
- **Hide runtime complexity by default.** Ports, runtime flags, quantization,
  samplers, CUDA/Vulkan, context windows, and process details never appear in
  the default flow. They live in Advanced Diagnostics only.
- **Always show clear plain-language status.** "Starting model...", "Ready",
  "Running locally" -- never "spawning subprocess" or "binding port".
- **Friendly errors with suggested actions.** Every failure maps to the error
  taxonomy and shows a plain-language message plus what to try next.
- **First-class empty states.** The no-model-selected Chat state is the first
  thing most users see; it must feel intentional and guide the user to the
  Models screen, never feel broken.

## 2. Language rules

- Main UI copy says **"Running locally"**. It must **never** say
  "GPU accelerated" or otherwise imply CUDA-class performance -- the MVP's
  Vulkan path measurably underperforms CUDA on NVIDIA hardware for prompt
  processing, and copy must not overpromise.
- Advanced Diagnostics is the only place that names the accelerator:
  "Accelerator: NVIDIA GPU (Vulkan)", "Accelerator: AMD GPU (Vulkan)",
  "Accelerator: Intel GPU (Vulkan)", or "Accelerator: CPU", plus the fallback
  reason if any.
- Plain language over jargon everywhere. "Model file is missing" beats
  "registry dereference failed".
- Privacy language is concrete: "Nothing leaves your computer", not marketing
  abstractions.

## 3. Friendly error copy

Each taxonomy code (see `docs/chat-provider.md`) has canonical user-facing
copy. Baseline set (refined during Phase 2 copy review):

| Code | Friendly message | Suggested action |
|---|---|---|
| `RuntimeMissing` | Omnira's chat engine is missing. | Reinstall Omnira, or set a runtime path in Settings. |
| `RuntimeFailedToStart` | The chat engine could not start. | Try again; if it keeps failing, check Advanced Diagnostics. |
| `ModelFileMissing` | This model's file has moved or been deleted. | Locate the file again from the Models screen, or remove the entry. |
| `ModelFormatInvalid` | This file does not look like a valid GGUF model. | Choose a different .gguf file. |
| `ModelLoadFailed` | The model could not be loaded. | Try a smaller model, or check Advanced Diagnostics. |
| `InsufficientMemory` | There is not enough memory to run this model. | Close other apps or try a smaller model. |
| `GenerationCancelled` | Generation stopped. | (informational; partial response kept) |
| `GenerationFailed` | The response could not be completed. | Try sending your message again. |
| `BackendUnavailable` | Omnira's engine is not responding. | Restart Omnira. |
| `UnauthorizedLocalRequest` | A request was blocked for your security. | Restart Omnira if chat stops working. |
| `UnknownRuntimeError` | Something unexpected went wrong. | Try again; details are in Advanced Diagnostics. |

## 4. Screens

**Chat:** conversation list, current thread, composer, model selector,
streaming response, stop button, "Running locally" indicator, friendly
no-model empty state. A subtle notice appears when older messages were
truncated from context.

**Models:** add/select local `.gguf`; friendly name, file path, file size,
last used, status indicator; missing-file warning; "Remove from Omnira" action
with copy making clear the file itself is not deleted.

**Settings:** data location, model search paths, privacy defaults, theme,
optional runtime path override, clear-conversations action, and a short
local-first explanation.

**Advanced Diagnostics:** runtime status, selected accelerator and fallback
reason, local API port/binding status, selected model metadata, recent runtime
errors, local log viewer, redacted diagnostic export.

## 5. First-run flow

1. Welcome to Omnira.
2. Local-first, private-by-default explanation in one short screen.
3. Accept the recommended data location (or choose another).
4. Select a local GGUF model file -- or skip for now (the Chat empty state
   handles the skip path gracefully).
5. Enter Chat.

## 6. Accessibility and tone

- Keyboard navigable; visible focus states; readable contrast in both themes.
- Motion is subtle and reduced-motion aware.
- Tone is calm and direct. No exclamation marks in error messages. No blame.
