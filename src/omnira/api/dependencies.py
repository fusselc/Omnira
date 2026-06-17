"""Simple dependency injection container for API wiring."""

from __future__ import annotations

from dataclasses import dataclass

from omnira.config.loader import load_settings
from omnira.config.models import AppSettings
from omnira.core.model_manager import ModelManager
from omnira.core.runtime_manager import RuntimeManager
from omnira.providers.registry import ProviderRegistry


@dataclass(slots=True)
class ServiceContainer:
    """Container holding app-wide services for dependency injection."""

    settings: AppSettings
    runtime_manager: RuntimeManager
    model_manager: ModelManager


def build_container() -> ServiceContainer:
    settings = load_settings()
    registry = ProviderRegistry()
    return ServiceContainer(
        settings=settings,
        runtime_manager=RuntimeManager(registry=registry),
        model_manager=ModelManager(),
    )
