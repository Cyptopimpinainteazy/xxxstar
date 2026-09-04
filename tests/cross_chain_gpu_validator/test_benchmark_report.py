"""Invariant: INFRA-CCGV-003
Benchmark reports include TPS metrics.
"""

from __future__ import annotations

import json
from pathlib import Path

from cross_chain_gpu_validator.benchmark import run_benchmark, write_report


def test_benchmark_report(tmp_path: Path) -> None:
    report = run_benchmark(svm_tps=100.0, evm_tps=200.0, duration_seconds=1)
    output = tmp_path / "report.json"
    write_report(report, str(output))

    data = json.loads(output.read_text())
    assert "combined_tps" in data
    assert data["combined_tps"] > 0
