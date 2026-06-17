"""Provider registry and lookup APIs."""

from __future__ import annotations

from dataclasses import dataclass, field

from .base import BaseProvider
from .interfaces import ProviderCapability


@dataclass(slots=True)
class ProviderRegistry:
    """In-memory provider registry supporting capability-based lookup."""

    _providers: dict[str, BaseProvider] = field(default_factory=dict)

    def register(self, provider: BaseProvider) -> None:
        name = provider.metadata.name
        if name in self._providers:
            msg = f"Provider already registered: {name}"
            raise ValueError(msg)
        self._providers[name] = provider

    def get(self, name: str) -> BaseProvider:
        try:
            return self._providers[name]
        except KeyError as exc:
            raise KeyError(f"Provider not found: {name}") from exc

    def list_names(self) -> list[str]:
        return sorted(self._providers.keys())

    def find_by_capability(self, capability: ProviderCapability) -> list[BaseProvider]:
        return [
            provider
            for provider in self._providers.values()
            if capability in provider.metadata.capabilities
        ]
