# Evidence: uninstall-orphan

- **Date:** 2026-07-27
- **Maintainer:** Chris Fussel
- **Machine:** DESKTOP-QIODQGF / 5.1.26100.8894 / Microsoft Windows NT 10.0.26200.0
- **Recorded by:** scripts/diagnostics/run-alpha-human-qa.ps1

## Steps taken
1. Closed Omnira
2. Ran uninstaller
3. Checked Program Files\Omnira removed
4. Checked %LOCALAPPDATA%\Omnira preserved
5. Checked GGUF file still on disk
6. Job Object orphan-check previously PASS (see 2026-07-19-orphan-check.txt)

## Result
- Pass / Fail: Pass
- Notes: 

