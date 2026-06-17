# Contributing to Omnira

Thanks for contributing.

## Principles

- Keep Omnira local-first.
- Avoid telemetry and cloud lock-in by default.
- Keep core orchestration separate from runtime providers.
- Prefer composable provider modules over monolithic implementations.

## Development flow

1. Create focused changes.
2. Add/adjust unit tests for behavior changes.
3. Run tests before submitting:
   - `PYTHONPATH=src python -m unittest discover -s tests/unit -v`
4. Keep architecture docs up to date when structure changes.

## Provider contribution guidelines

- Implement provider classes from `omnira.providers.base.BaseProvider`.
- Describe capabilities through `ProviderMetadata`.
- Register providers through `ProviderRegistry` (core) or entry points (plugins).
- Do not add runtime-specific inference logic to core orchestration modules.
