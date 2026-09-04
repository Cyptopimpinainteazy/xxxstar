"""Logging helpers for the cross-chain validator."""

from __future__ import annotations

import logging


def configure_logging(level: str) -> None:
    """Configure structured logging for the service."""

    logging.basicConfig(
        level=level.upper(),
        format=(
            "%(asctime)s %(levelname)s %(name)s "
            "trace_id=%(trace_id)s span_id=%(span_id)s %(message)s"
        ),
    )


class ContextLoggerAdapter(logging.LoggerAdapter):
    """Inject placeholder trace/span metadata into logs."""

    def process(self, msg: str, kwargs: dict) -> tuple[str, dict]:
        extra = kwargs.setdefault("extra", {})
        extra.setdefault("trace_id", "n/a")
        extra.setdefault("span_id", "n/a")
        return msg, kwargs


def get_logger(name: str) -> ContextLoggerAdapter:
    """Return a context-aware logger adapter."""

    return ContextLoggerAdapter(logging.getLogger(name), {})
