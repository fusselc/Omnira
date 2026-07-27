# PENDING: Human evidence — offline / network / install / logs / devtools

These gates require a maintainer session on Windows.

## Easiest way

In PowerShell from the repo root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\diagnostics\run-alpha-human-qa.ps1
```

That script asks simple y/n questions and writes dated files under `docs/evidence/`.
When it finishes, tell the agent: **Human QA done — update the checklist**.

## Status (until the helper is run)

| Gate | Status | Evidence file (when done) |
|------|--------|---------------------------|
| Devtools release smoke-check | PENDING | `YYYY-MM-DD-devtools-smoke.md` |
| Offline-after-install | PENDING | `YYYY-MM-DD-offline-after-install.md` |
| No external network calls | PENDING | `YYYY-MM-DD-network-monitor.md` |
| Prompt-free logs | PENDING | `YYYY-MM-DD-prompt-free-logs.md` |
| Fresh install / relaunch | PENDING | `YYYY-MM-DD-fresh-install-relaunch.md` |
| Uninstall (beyond orphan-check) | PENDING | `YYYY-MM-DD-uninstall-orphan.md` |
| 13 MVP acceptance criteria | PENDING | `YYYY-MM-DD-mvp-acceptance-13.md` |

Do not mark the matching checklist items Verified until the dated evidence files
exist and are linked from `docs/alpha-readiness-checklist.md`.
