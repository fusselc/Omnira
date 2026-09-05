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
- The main UI **does** name the engine honestly: the status pill reads
  "Running locally · llama.cpp" so it never looks like nothing is running, and
  it never claims an engine Omnira does not ship (no ONNX, no CUDA in MVP).
- The main UI may say the engine is **on CPU** when it is (the dismissible
  "Running on CPU" notice), because hiding a slower mode would be dishonest.
  It does not otherwise name GPU vendors or backends.
- Advanced Diagnostics is the place that names the accelerator in full:
  "GPU acceleration (Vulkan)" or "CPU mode", plus the fallback reason
  (including the engine's startup output when a variant failed) and the
  "Try GPU acceleration again" control.
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
streaming response, stop button, "Running locally · llama.cpp" indicator, an
"Unload model" / "Cancel loading" control beside it, friendly no-model empty
state. Message text is selectable so users can copy replies. A subtle notice
appears when older messages were truncated from context. Chat stays mounted
while other screens are shown so an in-flight response keeps streaming.

**Models:** add/select local `.gguf`; friendly name, file path, file size,
last used, status indicator; missing-file warning; "Remove from Omnira" action
with copy making clear the file itself is not deleted.

**Settings:** a short local-first privacy explanation (no privacy toggles),
theme, read-only data paths (Omnira data directory, conversations database,
settings file, logs), optional runtime path override, and a clear-conversations
action. There is no data-location picker and no model search-path list.

**Advanced Diagnostics:** runtime status, selected accelerator and fallback
reason, local API port/binding status, selected model metadata, recent runtime
errors, local log viewer, redacted diagnostic export.

## 5. First-run flow

1. Welcome to Omnira.
2. Local-first, private-by-default explanation, plus a read-only display of
   the data directory under `%LOCALAPPDATA%\Omnira\`. There is no control to
   choose another location.
3. Select a local GGUF model file -- or skip for now (the Chat empty state
   handles the skip path gracefully).
4. Enter Chat.

## 6. Accessibility and tone

- Keyboard navigable; visible focus states; readable contrast in both themes.
- Motion is subtle and reduced-motion aware.
- Tone is calm and direct. No exclamation marks in error messages. No blame.
