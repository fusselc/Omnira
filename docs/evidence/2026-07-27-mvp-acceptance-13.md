# Evidence: mvp-acceptance-13

- **Date:** 2026-07-27
- **Maintainer:** Chris Fussel
- **Machine:** DESKTOP-QIODQGF / 5.1.26100.8894 / Microsoft Windows NT 10.0.26200.0
- **Recorded by:** scripts/diagnostics/run-alpha-human-qa.ps1

## Result
- Overall Pass / Fail: Pass
- Exceptions: 

## Criteria
- [x] Pass: 1. Install/launch without terminal/Python/Docker
- [x] Pass: 2. Select local .gguf in place
- [x] Pass: 3. Managed llama-server starts
- [x] Pass: 4. Streaming local chat
- [x] Pass: 5. Stop generation works
- [x] Pass: 6. Quit/relaunch persistence
- [x] Pass: 7. No telemetry/accounts/cloud sync
- [x] Pass: 8. Works offline / no default external network
- [x] Pass: 9. Loopback + session api-key
- [x] Pass: 10. Main UI says Running locally
- [x] Pass: 11. Remove model does not delete GGUF
- [x] Pass: 12. Diagnostics redaction / prompt-free export
- [x] Pass: 13. No orphaned llama-server on force-kill (Job Object)

