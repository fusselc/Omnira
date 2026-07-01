# Privacy

Omnira's privacy position, in plain language: **nothing leaves your computer.**

## What Omnira does not do

- No telemetry or analytics of any kind.
- No user accounts or sign-in.
- No cloud sync.
- No crash uploads.
- No update checks or phone-home calls.
- No external network calls by default, for any reason.

Omnira is fully functional with the internet disconnected after installation,
assuming you already have a local GGUF model. This is a tested release
requirement, not a slogan.

## Where your data lives

Everything is on your machine under `%LOCALAPPDATA%\Omnira\`: conversations and
model registry in a local SQLite database, preferences in a readable JSON file,
and local log files. The Settings screen shows these locations. See
`docs/data-ownership-and-storage.md`.

Your model files are never copied, moved, or modified. Omnira references them
where they already are, and removing a model from Omnira never deletes the
file.

## Logs

Local logs never contain your prompts or the assistant's responses. They
record app lifecycle events, runtime start/stop, error codes, and minimal
timing metadata only.

## Diagnostics export

If you choose to export diagnostics (for a bug report, for example), the export
is redacted by default: user paths and content are masked. Including
unredacted details requires your explicit opt-in at export time, and the export
is a local file that only you decide to share.

## The future: network-using features

Post-MVP features (model download assistance, optional web search providers)
may involve the network. When they arrive:

- They will be off by default.
- They will ask clearly before any network access.
- The local-first defaults documented here will not change.

Any future network-capable provider will require an explicit user permission
model, designed and documented before it ships.
