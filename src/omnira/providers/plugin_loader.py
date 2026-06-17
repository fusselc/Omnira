"""Future plugin/provider loading strategy skeleton."""

from __future__ import annotations

from importlib.metadata import entry_points
from typing import Iterable

from .base import BaseProvider

ENTRYPOINT_GROUP = "omnira.providers"


def discover_provider_factories() -> Iterable[type[BaseProvider]]:
    """Discover provider classes from Python entry points.

    Strategy:
    - Core providers can register directly in code.
    - External plugins will expose entry points under `omnira.providers`.
    - Optional signed-manifest validation can be added later for security.
    """

    for entry_point in entry_points(group=ENTRYPOINT_GROUP):
        loaded = entry_point.load()
        if isinstance(loaded, type) and issubclass(loaded, BaseProvider):
            yield loaded
