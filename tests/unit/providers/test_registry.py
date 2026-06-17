import unittest

from omnira.providers.base import BaseProvider, InferenceRequest, InferenceResponse
from omnira.providers.interfaces import ProviderCapability, ProviderMetadata
from omnira.providers.registry import ProviderRegistry


class DummyProvider(BaseProvider):
    def initialize(self) -> None:
        return None

    def shutdown(self) -> None:
        return None

    def health_check(self) -> bool:
        return True

    def infer(self, request: InferenceRequest) -> InferenceResponse:
        return InferenceResponse(outputs={"echo": request.payload})


class ProviderRegistryTest(unittest.TestCase):
    def test_register_and_get_provider(self) -> None:
        registry = ProviderRegistry()
        provider = DummyProvider(
            ProviderMetadata(
                name="dummy",
                version="0.1.0",
                description="test provider",
                capabilities={ProviderCapability.TEXT},
            )
        )

        registry.register(provider)

        self.assertEqual(registry.get("dummy"), provider)
        self.assertEqual(registry.list_names(), ["dummy"])
        self.assertEqual(registry.find_by_capability(ProviderCapability.TEXT), [provider])


if __name__ == "__main__":
    unittest.main()
