"""Runtime manager skeleton for provider lifecycle orchestration."""

from __future__ import annotations

from dataclasses import dataclass

from omnira.providers.base import BaseProvider
from omnira.providers.registry import ProviderRegistry


@dataclass(slots=True)
class RuntimeManager:
    """Coordinates provider startup/shutdown without runtime-specific execution."""

    registry: ProviderRegistry

    def add_provider(self, provider: BaseProvider) -> None:
        self.registry.register(provider)

    def start_all(self) -> None:
        for name in self.registry.list_names():
            self.registry.get(name).initialize()

    def stop_all(self) -> None:
        for name in self.registry.list_names():
            self.registry.get(name).shutdown()

    def health_snapshot(self) -> dict[str, bool]:
        return {name: self.registry.get(name).health_check() for name in self.registry.list_names()}
