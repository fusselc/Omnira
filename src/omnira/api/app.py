"""FastAPI app skeleton for future backend API expansion."""

from __future__ import annotations

from fastapi import FastAPI

from .dependencies import build_container


def create_app() -> FastAPI:
    container = build_container()
    app = FastAPI(title=container.settings.app_name)

    @app.get("/health", tags=["system"])
    def health() -> dict[str, str]:
        return {"status": "ok", "mode": "local-first"}

    return app
