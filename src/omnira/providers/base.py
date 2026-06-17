"""Abstract provider base classes (no runtime-specific inference implementation)."""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

from .interfaces import ProviderMetadata


@dataclass(slots=True)
class InferenceRequest:
    """Generic inference request shape for future specialization."""

    model_id: str
    payload: dict[str, Any] = field(default_factory=dict)
    stream: bool = False


@dataclass(slots=True)
class InferenceResponse:
    """Generic inference response shape for future specialization."""

    outputs: dict[str, Any] = field(default_factory=dict)


class BaseProvider(ABC):
    """Abstract inference provider contract used by orchestration."""

    def __init__(self, metadata: ProviderMetadata) -> None:
        self._metadata = metadata

    @property
    def metadata(self) -> ProviderMetadata:
        return self._metadata

    @abstractmethod
    def initialize(self) -> None:
        """Prepare provider runtime resources."""

    @abstractmethod
    def shutdown(self) -> None:
        """Release provider runtime resources."""

    @abstractmethod
    def health_check(self) -> bool:
        """Return provider health status."""

    @abstractmethod
    def infer(self, request: InferenceRequest) -> InferenceResponse:
        """Execute inference (implementation intentionally deferred)."""
