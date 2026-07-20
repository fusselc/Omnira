# Alpha Manual Verification Runbook

Maintainer procedure for closing Phase 5 alpha readiness gates that require a
human on a Windows machine. Record evidence under `docs/evidence/` and only then
mark items **Verified** in [alpha-readiness-checklist.md](alpha-readiness-checklist.md).

Do not mark a gate Verified from code audit prose alone.

## Evidence conventions

- Path: `docs/evidence/YYYY-MM-DD-<gate-slug>.md` or `.txt`
- Optional screenshots: `docs/evidence/YYYY-MM-DD-<gate-slug>/`
- Each evidence file must include: date, maintainer name, machine notes, and the
  exact command or UI steps taken
- Template stubs live in `docs/evidence/templates/`

## Ordered session

1. **Build NSIS** from `apps/desktop/`: `npm.cmd run tauri build`
2. Walk [release-qa-checklist.md](release-qa-checklist.md) sections 1–8
3. Run `scripts/diagnostics/offline-smoke-test.ps1` with networking **actually
   disconnected** (airplane mode or adapters disabled)
4. During one chat generation, monitor with **Resource Monitor** or **Wireshark**;
   note tool name and whether `omnira.exe` / `llama-server.exe` opened outbound
   connections
5. Inspect `%LOCALAPPDATA%\Omnira\logs\omnira-*.log` after that chat for
   prompt-free behavior
6. On the release install: confirm WebView2 Inspect / remote debugging is not
   available (devtools smoke-check)
7. Map each result to the matching checkbox in
   [alpha-readiness-checklist.md](alpha-readiness-checklist.md)

## Checklist gate mapping

| Gate | Primary procedure | Evidence slug |
|------|-------------------|---------------|
| Devtools disabled in production | Release-QA install + Inspect check | `devtools-smoke` |
| Offline-after-install | Airplane mode + offline-smoke + chat | `offline-after-install` |
| No external network calls | Resource Monitor / Wireshark during chat | `network-monitor` |
| Prompt-free logs | Chat then log excerpt | `prompt-free-logs` |
| Fresh install / relaunch | Release-QA §1–3 and §6 | `fresh-install-relaunch` |
| Uninstall / orphan | Release-QA §7–8; orphan-check.ps1 for Job Object | `uninstall-orphan` |
| Runtime fetch fail-closed | Tamper cached zip; see packaging script | `fetch-fail-closed` |
| Code signing evaluation | Documented decision in packaging-process-model | `code-signing-decision` |
| 13 MVP acceptance criteria | List below; one evidence file | `mvp-acceptance-13` |

## Canonical 13 MVP acceptance criteria

Derived from the README MVP workflow and Phase 5 packaging/security requirements.

1. Install and launch Omnira with no terminal, Python, Docker, or manual services.
2. Select an existing local `.gguf` from the Models screen (referenced in place).
3. Omnira starts and manages `llama-server` automatically.
4. Local chat streams assistant tokens into the UI.
5. Stop generation cancels in-flight output and preserves partial content.
6. Quit and relaunch; conversations and settings persist under `%LOCALAPPDATA%\Omnira\`.
7. No telemetry, accounts, or cloud sync in the MVP path.
8. Default runtime path makes no external network calls; works offline after install.
9. `llama-server` binds loopback-only and requires a per-session API key.
10. Main UI says "Running locally"; Vulkan/CPU details stay in Advanced Diagnostics.
11. Removing a model from Omnira does not delete the GGUF file on disk.
12. Diagnostics export redacts user profile paths by default and stays prompt-free.
13. Force-killing Omnira does not leave an orphaned `llama-server.exe` (Job Object).

Record a dated pass/fail for each in
`docs/evidence/YYYY-MM-DD-mvp-acceptance-13.md`.

## Agent vs maintainer split

| Automatable (agent or CI) | Maintainer-only |
|---------------------------|-----------------|
| Fetch SHA fail-closed tamper | Airplane-mode offline chat |
| orphan-check.ps1 capture | Resource Monitor / Wireshark |
| Cargo/npm verification | Devtools Inspect on release UI |
| Code-signing decision docs | Full uninstall + reinstall recognition |
| Model rename (H3) feature | Sign-Off row attestation |

## After the session

1. Link every new evidence file from alpha-readiness-checklist.md
2. Complete the Sign-Off table (human maintainer only)
3. Do not start Phase 6 (CUDA) or image generation until Sign-Off is filled
