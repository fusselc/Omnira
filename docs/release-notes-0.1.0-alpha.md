# Omnira 0.1.0 (internal alpha)

- **Date:** 2026-07-27
- **Artifact:** `Omnira_0.1.0_x64-setup.exe` (NSIS, per-machine)
- **Audience:** maintainer and invited testers only

Omnira runs open-source language models on your own Windows PC. Install it,
point it at a `.gguf` file you already have, and chat. Nothing leaves the
machine.

## Requirements

- Windows 10 or 11, 64-bit.
- **Microsoft Edge WebView2 Runtime must already be installed.** The installer
  is offline-safe and deliberately does not download it
  (`webviewInstallMode: skip`, see
  [packaging-process-model.md](packaging-process-model.md)). Most up-to-date
  Windows 11 systems already have it.
- At least one GGUF model file on disk. Omnira does not download models.
- Administrator rights for install (per-machine install to `Program Files`).

## The installer is unsigned

This internal alpha ships without a code signing certificate, so Windows
SmartScreen will warn on first run. Choose "More info" then "Run anyway" if you
trust the build. OV/EV certificate purchase is deferred until public alpha
(decision recorded in [packaging-process-model.md](packaging-process-model.md)
section 6).

## What this release does

1. Install and launch with no terminal, Python, Docker, or manual services.
2. Add a local `.gguf` from the Models screen; the file is referenced in place
   and never copied or moved.
3. Omnira starts and supervises `llama-server` for you.
4. Chat locally with streamed responses.
5. Stop generation at any time; partial responses are kept.
6. Quit and relaunch; conversations persist and the thread you had open reopens.
7. Rename or delete conversations; rename model display names; clear all
   conversations from Settings.
8. Advanced Diagnostics shows runtime state, accelerator (Vulkan or CPU), local
   port, logs, and a redacted diagnostics export.

## Privacy and security posture

- No telemetry, accounts, cloud sync, or model downloads.
- No external network calls on the default runtime path; the full workflow works
  with networking disconnected.
- `llama-server` binds to `127.0.0.1` only and requires a per-session API key
  that is regenerated on every start and never written to disk or logs.
- Logs record lifecycle and error events, not prompt or response text.
- Diagnostics export redacts Windows user profile paths by default.
- Model output is rendered as sanitized markdown; raw HTML is never executed.

## Where your data lives

All runtime data is under `%LOCALAPPDATA%\Omnira\` (database, settings, logs).
Uninstalling removes `C:\Program Files\Omnira` and leaves your conversations,
settings, and GGUF files untouched. Reinstalling picks the data back up.

## Validation evidence

Every gate in [alpha-readiness-checklist.md](alpha-readiness-checklist.md) is
recorded as Verified, with maintainer Sign-Off dated 2026-07-27. Human QA
evidence for this build lives in `docs/evidence/2026-07-27-*.md` and covers
offline-after-install, network monitoring during generation, prompt-free logs,
devtools absence on the release install, fresh install and relaunch, uninstall
behavior, and all 13 MVP acceptance criteria from
[alpha-manual-verification.md](alpha-manual-verification.md).

## Known limitations

- Windows only. No macOS or Linux packaging.
- Vulkan or CPU inference only. CUDA is the next planned runtime (Phase 6), so
  NVIDIA users will see slower generation than their hardware allows.
- One model and one generation at a time.
- Context handling uses a character-budget approximation rather than a real
  tokenizer; a notice appears when older messages fall out of context.
- Markdown rendering is deliberately minimal (code blocks, emphasis, headings,
  lists). No tables or clickable links yet.
- NSIS installer only. MSI is deferred post-alpha.
- No self-update. Each installer pins one `llama-server` release, so upgrading
  means installing a newer Omnira build.
- Unsigned build, as described above.

## Not included in this release

Image generation, video, music, voice or speech, ONNX and Windows ML providers,
RAG and document memory, agents and tool calling, plugins, cloud providers, and
model download assistance are all post-MVP. See [roadmap.md](roadmap.md) for the
order in which they are planned.

## Reporting problems

Open a GitHub issue with the redacted diagnostics export from Advanced
Diagnostics attached. For suspected security issues, follow
[SECURITY.md](../SECURITY.md) instead of filing a public issue.
