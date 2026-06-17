import unittest

from omnira.core.runtime_manager import RuntimeManager
from omnira.providers.base import BaseProvider, InferenceRequest, InferenceResponse
from omnira.providers.interfaces import ProviderMetadata
from omnira.providers.registry import ProviderRegistry


class LifecycleProvider(BaseProvider):
    def __init__(self, metadata: ProviderMetadata) -> None:
        super().__init__(metadata)
        self.started = False

    def initialize(self) -> None:
        self.started = True

    def shutdown(self) -> None:
        self.started = False

    def health_check(self) -> bool:
        return self.started

    def infer(self, request: InferenceRequest) -> InferenceResponse:
        return InferenceResponse(outputs={})


class RuntimeManagerTest(unittest.TestCase):
    def test_start_and_stop_all(self) -> None:
        registry = ProviderRegistry()
        manager = RuntimeManager(registry=registry)
        provider = LifecycleProvider(
            ProviderMetadata(name="runtime", version="0.1.0", description="runtime provider")
        )

        manager.add_provider(provider)
        manager.start_all()

        self.assertTrue(manager.health_snapshot()["runtime"])

        manager.stop_all()

        self.assertFalse(manager.health_snapshot()["runtime"])


if __name__ == "__main__":
    unittest.main()
