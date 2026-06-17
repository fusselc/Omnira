"""Logging setup helpers."""

from __future__ import annotations

import logging

from omnira.config.models import LoggingSettings


def configure_logging(settings: LoggingSettings) -> None:
    """Configure basic logging for local execution."""

    logging.basicConfig(
        level=getattr(logging, settings.level.upper(), logging.INFO),
        format="%(asctime)s | %(levelname)s | %(name)s | %(message)s",
    )
