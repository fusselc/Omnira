"""Core provider interfaces and capability models."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import StrEnum


class ProviderCapability(StrEnum):
    """Capabilities supported by an inference provider."""

    TEXT = "text"
    VISION = "vision"
    AUDIO = "audio"
    IMAGE_GENERATION = "image_generation"
    VIDEO_GENERATION = "video_generation"
    AGENTS = "agents"
    WORKFLOWS = "workflows"
    STREAMING = "streaming"
    RAG = "rag"


@dataclass(frozen=True, slots=True)
class ProviderMetadata:
    """Metadata used by registry and discovery mechanisms."""

    name: str
    version: str
    description: str
    capabilities: set[ProviderCapability] = field(default_factory=set)
    runtime_family: str = "generic"
