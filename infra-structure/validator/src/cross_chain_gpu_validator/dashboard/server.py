"""Serve dashboard assets and live metrics."""

from __future__ import annotations

import json
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
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
        if self.path == "/chain-benchmarks.json" and self._benchmark_provider:
            payload = json.dumps(self._benchmark_provider()).encode("utf-8")
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        return super().do_GET()


def load_benchmarks(path: str | None) -> dict:
    """Read a benchmark report; no report yet means an empty panel, not an error.

    The dashboard shows only what scripts/benchmark_all_chains_tps.py wrote on
    this host. A missing or unreadable file yields {} so the panel stays empty
    rather than showing stale or invented numbers.
    """
    if not path:
        return {}
    try:
        with open(path, "r", encoding="utf-8") as handle:
            data = json.load(handle)
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def run_dashboard(
    host: str,
    port: int,
    metrics: MetricsStore,
    static_dir: str,
    benchmark_path: str | None = None,
) -> None:
    """Start the dashboard server."""

    def _handler(*args, **kwargs):
        # directory= instead of os.chdir(): changing the process cwd would
        # silently re-root every relative path the process later opens.
        return DashboardHandler(
            *args,
            metrics_provider=metrics.snapshot_dict,
            benchmark_provider=lambda: load_benchmarks(benchmark_path),
            directory=static_dir,
            **kwargs,
        )

    server = ThreadingHTTPServer((host, port), _handler)
    server.serve_forever()
