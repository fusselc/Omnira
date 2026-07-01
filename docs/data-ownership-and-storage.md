# Data Ownership and Storage

Omnira makes data ownership obvious: conversations, settings, model references,
logs, and diagnostics stay on the user's device by default, in locations the
Settings screen displays plainly.

## 1. Data locations (Windows)

Everything lives under `%LOCALAPPDATA%\Omnira\`:

| Path | Contents | Format |
|---|---|---|
| `config\settings.json` | User preferences | Human-readable JSON |
| `data\omnira.db` | Structured state | SQLite |
| `logs\` | Local, prompt-free logs | Rotated text files |
| `diagnostics\` | Optional export reports | JSON |

`%LOCALAPPDATA%` (not `%APPDATA%`) is deliberate: a local-first app with a
SQLite database should not ride along with roaming profile sync.

## 2. SQLite (structured state)

`data\omnira.db` stores:

- **conversations** -- id, title, created/updated timestamps, model reference.
- **messages** -- id, conversation id, role, content, created timestamp,
  status (`complete`, `interrupted`).
- **models** -- registry entries: id, friendly name, absolute file path, file
  size, GGUF metadata (trained context length), last-used timestamp, status.

### Message persistence contract (stream boundaries)

- The user message is persisted via Tauri command **before** the streaming
  request starts.
- On cancellation, the partial assistant content received so far is persisted
  and marked `interrupted`.
- On completion, the final assistant message is persisted as `complete`.
- A crash mid-stream must never lose the user's message.

## 3. Config (human-readable preferences)

`config\settings.json` stores user-editable preferences:

- Model search paths
- Data location display
- Privacy settings
- Theme preference
- Advanced runtime preferences (custom runtime path override, recorded working
  runtime variant)
- Feature flags

## 4. Logs

Local and prompt-free by default: app lifecycle events, runtime
startup/shutdown and failures, structured error codes, minimal generation
metadata (timings, token counts). Never prompt or response content. See
`docs/local-security-boundary.md`.

## 5. Model file strategy

- Import **references GGUF files in place**. Omnira does not copy model files
  by default (an optional copy-to-managed-library may come later).
- If a referenced file is moved or deleted, the Models screen shows a clear
  missing-file warning (`ModelFileMissing`); chat with that model is blocked
  until resolved.
- **Removing a model from Omnira removes only the registry entry.** The
  underlying model file is never deleted by default.

## 6. Deletion and retention (MVP)

- Delete a single conversation.
- Clear all conversations.
- Remove a model registry entry (file untouched).
- Settings shows exactly where Omnira stores local data.

## 7. Diagnostics export

The Advanced Diagnostics screen offers a diagnostic export (a JSON report)
containing runtime status, recent logs, and configuration. By default the export is
**redacted**: user account names in paths, model file paths, and any content
fields are masked. Full unredacted export requires an explicit opt-in at
export time.

## 8. What is never stored or collected

No telemetry, no analytics, no crash uploads, no cloud copies of any user
data. Nothing leaves the machine unless the user manually shares an export
file themselves.
