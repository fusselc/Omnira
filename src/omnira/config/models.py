"""Pydantic-based configuration models."""

from __future__ import annotations

from pydantic import BaseModel, Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class LoggingSettings(BaseModel):
    """Logging configuration for local-first operation."""

    level: str = Field(default="INFO")
    json_logs: bool = Field(default=False)


class ProviderSettings(BaseModel):
    """Provider configuration block."""

    enabled: bool = Field(default=True)
    priority: int = Field(default=100, ge=0)


class AppSettings(BaseSettings):
    """Top-level application settings loaded from environment or .env file."""

    model_config = SettingsConfigDict(env_prefix="OMNIRA_", env_file=".env", extra="ignore")

    app_name: str = Field(default="Omnira")
    environment: str = Field(default="development")
    telemetry_enabled: bool = Field(default=False)
    local_first: bool = Field(default=True)
    providers: dict[str, ProviderSettings] = Field(default_factory=dict)
    logging: LoggingSettings = Field(default_factory=LoggingSettings)
