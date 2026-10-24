"""Dashboard benchmark panel, ported from tools/cross-chain-gpu-validator."""

from __future__ import annotations

import json
import os
import threading
import urllib.request
from functools import partial
from http.server import ThreadingHTTPServer
from unittest import mock

import pytest

from cross_chain_gpu_validator import cli
from cross_chain_gpu_validator.dashboard.server import (
    DashboardHandler,
    load_benchmarks,
)


def test_load_benchmarks_without_a_report_is_empty(tmp_path):
    assert load_benchmarks(None) == {}
    assert load_benchmarks(str(tmp_path / "absent.json")) == {}
    bad = tmp_path / "bad.json"
    bad.write_text("{not json", encoding="utf-8")
    assert load_benchmarks(str(bad)) == {}
    listed = tmp_path / "list.json"
    listed.write_text("[1, 2]", encoding="utf-8")
    assert load_benchmarks(str(listed)) == {}


def test_load_benchmarks_returns_the_report(tmp_path):
    report = {"our_chain_id": "solana", "chains": [{"chain_id": "solana"}]}
    path = tmp_path / "all_chains_tps.json"
    path.write_text(json.dumps(report), encoding="utf-8")
    assert load_benchmarks(str(path)) == report


@pytest.fixture
def served(tmp_path):
    (tmp_path / "index.html").write_text("<h1>validator</h1>", encoding="utf-8")
    report = {"chains": [{"chain_id": "solana", "max_tps": 1.0}]}
    handler = partial(
        DashboardHandler,
        metrics_provider=lambda: {"ok": True},
        benchmark_provider=lambda: report,
        directory=str(tmp_path),
    )
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_address[1]}", report
    finally:
        server.shutdown()
        server.server_close()


def _get(url: str) -> bytes:
    with urllib.request.urlopen(url, timeout=5) as response:
        return response.read()


def test_benchmark_endpoint_serves_the_report(served):
    base, report = served
    assert json.loads(_get(base + "/chain-benchmarks.json")) == report
    assert json.loads(_get(base + "/metrics.json")) == {"ok": True}


def test_static_files_come_from_the_dashboard_dir_without_chdir(served):
    base, _ = served
    before = os.getcwd()
    assert b"validator" in _get(base + "/index.html")
    assert os.getcwd() == before


def test_cli_serves_this_packages_dashboard_and_benchmark_path(monkeypatch):
    monkeypatch.delenv("CCGV_DASHBOARD_DIR", raising=False)
    monkeypatch.delenv("CCGV_BENCHMARK_PATH", raising=False)
    with mock.patch.object(cli, "run_dashboard") as run:
        cli._run_dashboard()
    _, _, _, static_dir = run.call_args.args
    benchmark_path = run.call_args.kwargs["benchmark_path"]
    package_root = os.path.dirname(static_dir)

    # The validator's own dashboard, not infra-structure/dashboard.
    assert os.path.basename(package_root) == "validator"
    with open(os.path.join(static_dir, "index.html"), encoding="utf-8") as page:
        assert "chain_tps_rows" in page.read()
    assert benchmark_path == os.path.join(
        package_root, "benchmarks", "all_chains_tps.json"
    )
