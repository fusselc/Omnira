"""Model manager skeleton for model/provider association."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True, slots=True)
class ModelRecord:
    """Model metadata tracked by orchestration."""

    model_id: str
    provider_name: str
    modality: str
    local_path: str | None = None


@dataclass(slots=True)
class ModelManager:
    """Simple in-memory model registry for future persistence extension."""

    _models: dict[str, ModelRecord] = field(default_factory=dict)

    def register_model(self, record: ModelRecord) -> None:
        if record.model_id in self._models:
            msg = f"Model already registered: {record.model_id}"
            raise ValueError(msg)
        self._models[record.model_id] = record

    def get(self, model_id: str) -> ModelRecord:
        try:
            return self._models[model_id]
        except KeyError as exc:
            raise KeyError(f"Model not found: {model_id}") from exc

    def list_models(self) -> list[ModelRecord]:
        return sorted(self._models.values(), key=lambda item: item.model_id)
