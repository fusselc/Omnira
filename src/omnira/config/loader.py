"""Configuration loading entrypoint."""

from __future__ import annotations

from .models import AppSettings


def load_settings() -> AppSettings:
    """Load and return application settings."""

    return AppSettings()
