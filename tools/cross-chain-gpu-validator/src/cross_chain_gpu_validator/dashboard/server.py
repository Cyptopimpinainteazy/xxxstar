"""Serve dashboard assets and live metrics."""

from __future__ import annotations

from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from typing import Callable

from cross_chain_gpu_validator.metrics import MetricsStore


class DashboardHandler(SimpleHTTPRequestHandler):
    """Handler for dashboard assets and metrics."""

    def __init__(
        self,
        *args,
        metrics_provider: Callable[[], dict],
        benchmark_provider: Callable[[], dict] | None = None,
        **kwargs,
    ):
        self._metrics_provider = metrics_provider
        self._benchmark_provider = benchmark_provider
        super().__init__(*args, **kwargs)

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/metrics.json":
            payload = json.dumps(self._metrics_provider()).encode("utf-8")
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        if self.path == "/chain-benchmarks.json" and self._benchmark_provider is not None:
            payload = json.dumps(self._benchmark_provider()).encode("utf-8")
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        return super().do_GET()


def run_dashboard(
    host: str,
    port: int,
    metrics: MetricsStore,
    static_dir: str,
    benchmark_path: str | None = None,
) -> None:
    """Start the dashboard server."""

    os.chdir(static_dir)

    def _benchmarks() -> dict:
        if not benchmark_path:
            return {}
        try:
            with open(benchmark_path, "r", encoding="utf-8") as handle:
                return json.load(handle)
        except FileNotFoundError:
            return {}
        except Exception:
            return {}

    def _handler(*args, **kwargs):
        return DashboardHandler(
            *args,
            metrics_provider=metrics.snapshot_dict,
            benchmark_provider=_benchmarks,
            **kwargs,
        )

    server = ThreadingHTTPServer((host, port), _handler)
    server.serve_forever()
