import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

from omnira.providers import (
    InferenceRequest,
    LlamaCppGenerationConfig,
    LlamaCppProvider,
    LlamaCppProviderConfig,
    LlamaCppProviderError,
    ProviderCapability,
    ProviderMetadata,
)


class TestLlamaCppProvider(unittest.TestCase):
    def setUp(self) -> None:
        self._tempdir = tempfile.TemporaryDirectory()
        self.addCleanup(self._tempdir.cleanup)

        self.model_a = Path(self._tempdir.name) / "tiny-a.gguf"
        self.model_b = Path(self._tempdir.name) / "tiny-b.gguf"
        self.not_gguf = Path(self._tempdir.name) / "not-a-model.txt"

        self.model_a.write_text("mock")
        self.model_b.write_text("mock")
        self.not_gguf.write_text("mock")

        self.metadata = ProviderMetadata(
            name="llama.cpp",
            version="0.1.0",
            description="llama.cpp provider",
            capabilities={ProviderCapability.TEXT},
        )

    def test_initialize_and_generate_text(self) -> None:
        mock_model = Mock()
        mock_model.create_completion.return_value = {"choices": [{"text": " hello world"}]}
        mock_llama_cls = Mock(return_value=mock_model)

        config = LlamaCppProviderConfig(
            models={"tiny": str(self.model_a)},
            default_model_id="tiny",
            generation=LlamaCppGenerationConfig(
                max_tokens=128,
                temperature=0.3,
                top_p=0.85,
                stop=["\nUser:"],
            ),
        )
        provider = LlamaCppProvider(metadata=self.metadata, config=config)

        with patch(
            "omnira.providers.llama_cpp_provider.import_module",
            return_value=SimpleNamespace(Llama=mock_llama_cls),
        ):
            provider.initialize()
            response = provider.infer(InferenceRequest(model_id="tiny", payload={"prompt": "Hi"}))

        self.assertTrue(provider.health_check())
        self.assertEqual(response.outputs["text"], " hello world")
        mock_llama_cls.assert_called_once()
        mock_model.create_completion.assert_called_once_with(
            prompt="Hi",
            max_tokens=128,
            temperature=0.3,
            top_p=0.85,
            stop=["\nUser:"],
        )

    def test_infer_switches_loaded_model(self) -> None:
        first_model = Mock()
        second_model = Mock()
        second_model.create_completion.return_value = {"choices": [{"text": " switched"}]}
        mock_llama_cls = Mock(side_effect=[first_model, second_model])

        config = LlamaCppProviderConfig(
            models={"a": str(self.model_a), "b": str(self.model_b)},
            default_model_id="a",
        )
        provider = LlamaCppProvider(metadata=self.metadata, config=config)

        with patch(
            "omnira.providers.llama_cpp_provider.import_module",
            return_value=SimpleNamespace(Llama=mock_llama_cls),
        ):
            provider.initialize()
            response = provider.infer(
                InferenceRequest(
                    model_id="b",
                    payload={
                        "prompt": "Switch model",
                        "max_tokens": 16,
                        "temperature": 0.1,
                        "top_p": 0.5,
                        "stop": ["END"],
                    },
                )
            )

        self.assertEqual(response.outputs["text"], " switched")
        self.assertEqual(mock_llama_cls.call_count, 2)
        first_call_kwargs = mock_llama_cls.call_args_list[0].kwargs
        second_call_kwargs = mock_llama_cls.call_args_list[1].kwargs
        self.assertEqual(first_call_kwargs["model_path"], str(self.model_a))
        self.assertEqual(second_call_kwargs["model_path"], str(self.model_b))
        second_model.create_completion.assert_called_once_with(
            prompt="Switch model",
            max_tokens=16,
            temperature=0.1,
            top_p=0.5,
            stop=["END"],
        )

    def test_initialize_raises_when_llama_cpp_missing(self) -> None:
        provider = LlamaCppProvider(metadata=self.metadata)

        with patch("omnira.providers.llama_cpp_provider.import_module", side_effect=ImportError):
            with self.assertRaises(LlamaCppProviderError):
                provider.initialize()

    def test_infer_requires_initialization(self) -> None:
        provider = LlamaCppProvider(metadata=self.metadata)

        with self.assertRaises(LlamaCppProviderError):
            provider.infer(InferenceRequest(model_id="any", payload={"prompt": "hello"}))

    def test_infer_requires_prompt(self) -> None:
        mock_llama_cls = Mock(return_value=Mock())
        provider = LlamaCppProvider(
            metadata=self.metadata,
            config=LlamaCppProviderConfig(models={"tiny": str(self.model_a)}, default_model_id="tiny"),
        )

        with patch(
            "omnira.providers.llama_cpp_provider.import_module",
            return_value=SimpleNamespace(Llama=mock_llama_cls),
        ):
            provider.initialize()
            with self.assertRaises(LlamaCppProviderError):
                provider.infer(InferenceRequest(model_id="tiny", payload={}))

    def test_infer_validates_numeric_generation_parameters(self) -> None:
        mock_llama_cls = Mock(return_value=Mock())
        provider = LlamaCppProvider(
            metadata=self.metadata,
            config=LlamaCppProviderConfig(models={"tiny": str(self.model_a)}, default_model_id="tiny"),
        )

        with patch(
            "omnira.providers.llama_cpp_provider.import_module",
            return_value=SimpleNamespace(Llama=mock_llama_cls),
        ):
            provider.initialize()
            with self.assertRaises(LlamaCppProviderError):
                provider.infer(
                    InferenceRequest(
                        model_id="tiny",
                        payload={"prompt": "hello", "max_tokens": "bad-value"},
                    )
                )

    def test_rejects_non_gguf_model_paths(self) -> None:
        mock_llama_cls = Mock(return_value=Mock())
        provider = LlamaCppProvider(
            metadata=self.metadata,
            config=LlamaCppProviderConfig(models={"invalid": str(self.not_gguf)}),
        )

        with self.assertRaises(LlamaCppProviderError):
            with patch(
                "omnira.providers.llama_cpp_provider.import_module",
                return_value=SimpleNamespace(Llama=mock_llama_cls),
            ):
                provider.initialize()

    def test_wraps_corrupted_model_load_errors(self) -> None:
        mock_llama_cls = Mock(side_effect=ValueError("invalid gguf format"))
        provider = LlamaCppProvider(
            metadata=self.metadata,
            config=LlamaCppProviderConfig(models={"tiny": str(self.model_a)}, default_model_id="tiny"),
        )

        with self.assertRaises(LlamaCppProviderError):
            with patch(
                "omnira.providers.llama_cpp_provider.import_module",
                return_value=SimpleNamespace(Llama=mock_llama_cls),
            ):
                provider.initialize()


if __name__ == "__main__":
    unittest.main()
