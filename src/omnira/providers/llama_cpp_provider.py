"""llama.cpp provider implementation."""

from __future__ import annotations

from dataclasses import dataclass, field
from importlib import import_module
from pathlib import Path
from typing import Any

from .base import BaseProvider, InferenceRequest, InferenceResponse
from .interfaces import ProviderMetadata


class LlamaCppProviderError(RuntimeError):
    """Raised when llama.cpp provider operations fail."""


@dataclass(slots=True)
class LlamaCppGenerationConfig:
    """Default generation parameters for llama.cpp inference calls."""

    max_tokens: int = 256
    temperature: float = 0.8
    top_p: float = 0.95
    stop: list[str] = field(default_factory=list)


@dataclass(slots=True)
class LlamaCppProviderConfig:
    """Runtime configuration for llama.cpp provider."""

    models: dict[str, str] = field(default_factory=dict)
    default_model_id: str | None = None
    n_ctx: int = 2048
    n_threads: int | None = None
    n_gpu_layers: int = 0
    seed: int | None = None
    verbose: bool = False
    generation: LlamaCppGenerationConfig = field(default_factory=LlamaCppGenerationConfig)


class LlamaCppProvider(BaseProvider):
    """Inference provider backed by llama.cpp GGUF models."""

    def __init__(
        self,
        metadata: ProviderMetadata,
        config: LlamaCppProviderConfig | None = None,
    ) -> None:
        super().__init__(metadata)
        self._config = config or LlamaCppProviderConfig()
        self._llama_cls: Any | None = None
        self._model: Any | None = None
        self._active_model_id: str | None = None
        self._active_model_path: str | None = None

    def initialize(self) -> None:
        self._llama_cls = self._load_llama_class()

        initial_model_id = self._config.default_model_id
        if initial_model_id is None and self._config.models:
            initial_model_id = next(iter(self._config.models.keys()))

        if initial_model_id is not None:
            self._load_model(initial_model_id)

    def shutdown(self) -> None:
        self._model = None
        self._active_model_id = None
        self._active_model_path = None

    def health_check(self) -> bool:
        return self._model is not None

    def infer(self, request: InferenceRequest) -> InferenceResponse:
        if self._llama_cls is None:
            msg = "Provider not initialized."
            raise LlamaCppProviderError(msg)

        if not request.model_id:
            msg = "Inference request must include a model_id."
            raise LlamaCppProviderError(msg)

        if self._active_model_id != request.model_id:
            self._load_model(request.model_id)

        prompt = request.payload.get("prompt")
        if not isinstance(prompt, str):
            msg = "Inference request payload must include a prompt string."
            raise LlamaCppProviderError(msg)
        if not prompt.strip():
            msg = "Inference request payload must include a non-empty prompt."
            raise LlamaCppProviderError(msg)

        try:
            max_tokens = int(request.payload.get("max_tokens", self._config.generation.max_tokens))
            temperature = float(request.payload.get("temperature", self._config.generation.temperature))
            top_p = float(request.payload.get("top_p", self._config.generation.top_p))
        except (TypeError, ValueError) as exc:
            msg = "Generation parameters max_tokens, temperature, and top_p must be numeric."
            raise LlamaCppProviderError(msg) from exc
        stop_sequences = request.payload.get("stop", self._config.generation.stop)

        try:
            result = self._model.create_completion(
                prompt=prompt,
                max_tokens=max_tokens,
                temperature=temperature,
                top_p=top_p,
                stop=stop_sequences,
            )
        except (RuntimeError, ValueError, OSError) as exc:
            msg = f"Text generation failed for model '{request.model_id}'."
            raise LlamaCppProviderError(msg) from exc

        text = ""
        choices = result.get("choices")
        if isinstance(choices, list) and choices:
            first_choice = choices[0]
            if isinstance(first_choice, dict):
                text = str(first_choice.get("text", ""))

        return InferenceResponse(
            outputs={
                "text": text,
                "model_id": request.model_id,
                "raw": result,
            }
        )

    def _load_llama_class(self) -> Any:
        try:
            module = import_module("llama_cpp")
        except ImportError as exc:
            msg = "llama_cpp dependency is unavailable. Install llama-cpp-python."
            raise LlamaCppProviderError(msg) from exc

        llama_cls = getattr(module, "Llama", None)
        if llama_cls is None:
            msg = "llama_cpp.Llama class is unavailable."
            raise LlamaCppProviderError(msg)
        return llama_cls

    def _resolve_model_path(self, model_id: str) -> Path:
        configured_path = self._config.models.get(model_id)
        raw_path = configured_path or model_id
        model_path = Path(raw_path)

        if model_path.suffix.lower() != ".gguf":
            msg = f"Model '{model_id}' must point to a .gguf file."
            raise LlamaCppProviderError(msg)
        if not model_path.exists():
            msg = f"GGUF model file not found for '{model_id}': {model_path}"
            raise LlamaCppProviderError(msg)
        return model_path

    def _load_model(self, model_id: str) -> None:
        if self._llama_cls is None:
            msg = "Provider not initialized."
            raise LlamaCppProviderError(msg)

        model_path = self._resolve_model_path(model_id)

        try:
            self._model = self._llama_cls(
                model_path=str(model_path),
                n_ctx=self._config.n_ctx,
                n_threads=self._config.n_threads,
                n_gpu_layers=self._config.n_gpu_layers,
                seed=self._config.seed,
                verbose=self._config.verbose,
            )
        except (RuntimeError, ValueError, OSError) as exc:
            msg = f"Failed to load GGUF model '{model_id}' from {model_path}"
            raise LlamaCppProviderError(msg) from exc

        self._active_model_id = model_id
        self._active_model_path = str(model_path)
