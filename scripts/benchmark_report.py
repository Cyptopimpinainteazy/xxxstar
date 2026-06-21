#!/usr/bin/env python3
"""
X3 Benchmark Report Generator

Reads Criterion JSON output, aggregates results, detects regressions
against a baseline file, and writes a structured report.

Usage:
    python scripts/benchmark_report.py                        # run all aggregation
    python scripts/benchmark_report.py --baseline <file>      # compare against baseline
    python scripts/benchmark_report.py --criterion-dir <dir>  # specify Criterion output dir
    python scripts/benchmark_report.py --output <file>        # custom output path

Output schema matches the X3 benchmark report JSON specification:
    reports/benchmarks/x3-benchmark-report-YYYY-MM-DD.json
"""

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ─── Constants ───────────────────────────────────────────────────────────────

REPORTS_DIR = Path("reports/benchmarks")
CRITERION_DIR = Path("target/criterion")
SCHEMA_VERSION = "1.0.0"

# Metric names that count as regressions if they degrade by >threshold
REGRESSION_THRESHOLDS: Dict[str, float] = {
    "atomic_swap_p99_ns": 20.0,       # 20% regression threshold
    "dex_route_p99_ns": 15.0,
    "bridge_proof_verify_ns": 20.0,
    "vm_dispatch_ns": 15.0,
    "rpc_encode_p99_ns": 20.0,
    "sig_verify_p99_ns": 10.0,
    "eth_call_p95_ms": 30.0,
    "eth_call_p99_ms": 30.0,
    "atomic_cross_vm_p99_ms": 25.0,
    "block_import_p99_ms": 20.0,
    "bridge_verify_per_sec": -15.0,   # negative = regression when value decreases
}

PASS_THRESHOLDS: Dict[str, Dict[str, float]] = {
    "eth_call_p95_ms": {"max": 500},
    "eth_call_p99_ms": {"max": 1000},
    "x3_quote_multivm_p95_ms": {"max": 2000},
    "rpc_error_rate": {"max": 0.05},
    "block_import_p99_ms": {"max": 2000},
    "finality_p99_ms": {"max": 12000},
}


# ─── Helpers ─────────────────────────────────────────────────────────────────

def detect_criterion_results(criterion_dir: Path) -> Dict[str, Dict[str, float]]:
    """Parse Criterion's JSON output (new/estimates.json per benchmark group)."""
    results: Dict[str, Dict[str, float]] = {}

    if not criterion_dir.exists():
        return results

    for bench_dir in criterion_dir.iterdir():
        if not bench_dir.is_dir():
            continue
        estimates_file = bench_dir / "new" / "estimates.json"
        if not estimates_file.exists():
            continue

        try:
            with open(estimates_file) as f:
                data = json.load(f)

            bench_name = bench_dir.name
            for group_name, group_data in data.items():
                full_name = f"{bench_name}/{group_name}"
                if "Slope" in group_data:
                    results[full_name] = {
                        "mean_ns": group_data["Slope"]["point_estimate"],
                        "p95_ns": group_data["Slope"]["confidence_interval"]["upper_bound"],
                        "stddev_ns": abs(
                            group_data["Slope"]["confidence_interval"]["upper_bound"]
                            - group_data["Slope"]["confidence_interval"]["lower_bound"]
                        ) / 4.0,  # approx stddev from CI width
                    }
        except (json.JSONDecodeError, KeyError) as e:
            print(f"  ⚠ Skipping {estimates_file}: {e}", file=sys.stderr)

    return results


def detect_regressions(
    current: Dict[str, Dict[str, float]],
    baseline: Dict[str, Dict[str, float]],
    thresholds: Dict[str, float],
) -> List[Dict[str, Any]]:
    """Compare current results against baseline; return list of regressions."""
    regressions: List[Dict[str, Any]] = []

    for metric, current_data in current.items():
        if metric not in baseline:
            continue

        prev_mean = baseline[metric]["mean_ns"]
        curr_mean = current_data["mean_ns"]

        if prev_mean == 0.0:
            continue

        delta_pct = ((curr_mean - prev_mean) / prev_mean) * 100.0
        threshold = thresholds.get(metric, 15.0)  # default 15%

        # Check for regression in either direction depending on threshold sign
        is_regression = False
        if threshold > 0 and delta_pct > threshold:
            is_regression = True
        elif threshold < 0 and delta_pct < threshold:
            is_regression = True

        if is_regression:
            regressions.append({
                "metric": metric,
                "previous_mean_ns": prev_mean,
                "current_mean_ns": curr_mean,
                "delta_percent": round(delta_pct, 2),
                "status": "fail",
            })

    return regressions


def get_git_info() -> Dict[str, str]:
    """Get git commit and branch info."""
    try:
        commit = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], stderr=subprocess.DEVNULL, text=True
        ).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        commit = "unknown"

    try:
        branch = subprocess.check_output(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            stderr=subprocess.DEVNULL, text=True,
        ).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        branch = "unknown"

    return {"commit": commit, "branch": branch}


def get_machine_info() -> Dict[str, Any]:
    """Detect machine specs."""
    info: Dict[str, Any] = {
        "hostname": platform.node(),
        "cpu": platform.processor() or "unknown",
        "os": f"{platform.system()} {platform.release()}",
        "python": platform.python_version(),
    }

    # Try to detect RAM
    try:
        import psutil
        mem = psutil.virtual_memory()
        info["ram_gb"] = round(mem.total / (1024**3), 1)
    except ImportError:
        info["ram_gb"] = "unknown"

    # CPU cores
    info["cpu_cores"] = os.cpu_count() or "unknown"

    return info


def check_pass_thresholds(metrics: Dict[str, float]) -> Dict[str, Any]:
    """Check metrics against pass/fail thresholds."""
    violations: List[Dict[str, Any]] = []
    for metric, thresholds in PASS_THRESHOLDS.items():
        value = metrics.get(metric)
        if value is None:
            violations.append({"metric": metric, "status": "missing", "message": "No data"})
            continue
        if "max" in thresholds and value > thresholds["max"]:
            violations.append({
                "metric": metric,
                "value": value,
                "threshold": thresholds["max"],
                "status": "fail",
                "message": f"Exceeds max {thresholds['max']}",
            })
        elif "min" in thresholds and value < thresholds["min"]:
            violations.append({
                "metric": metric,
                "value": value,
                "threshold": thresholds["min"],
                "status": "fail",
                "message": f"Below min {thresholds['min']}",
            })
    return {"violations": violations, "passed": len(violations) == 0}


# ─── Main Report Generation ─────────────────────────────────────────────────

def generate_report(
    criterion_dir: Path,
    baseline_path: Optional[Path] = None,
    output_path: Optional[Path] = None,
) -> Dict[str, Any]:
    """Generate complete benchmark report."""
    timestamp = datetime.now(timezone.utc).isoformat()

    # Collect Criterion results
    criterion_results = detect_criterion_results(criterion_dir)
    print(f"Found {len(criterion_results)} criterion benchmark results")

    # Git info
    git = get_git_info()

    # Machine info
    machine = get_machine_info()

    # Flatten criterion results into metrics dict
    flat_metrics: Dict[str, float] = {}
    for name, data in criterion_results.items():
        flat_metrics[f"{name}_mean_ns"] = data.get("mean_ns", 0)
        flat_metrics[f"{name}_p95_ns"] = data.get("p95_ns", 0)

    # Load k6 results if they exist
    k6_summary_path = REPORTS_DIR / "x3-k6-summary.json"
    if k6_summary_path.exists():
        try:
            with open(k6_summary_path) as f:
                k6_data = json.load(f)
                for k, v in k6_data.get("metrics", {}).items():
                    if isinstance(v, (int, float)):
                        flat_metrics[f"{k}_ms" if "_p" in k else k] = v
        except (json.JSONDecodeError, KeyError):
            pass

    # Detect regressions
    regressions: List[Dict[str, Any]] = []
    if baseline_path and baseline_path.exists():
        try:
            with open(baseline_path) as f:
                baseline = json.load(f)
            baseline_metrics = {}
            for name, data in baseline.get("criteria", {}).items():
                baseline_metrics[name] = {
                    "mean_ns": data.get("mean_ns", 0),
                    "p95_ns": data.get("p95_ns", 0),
                    "stddev_ns": data.get("stddev_ns", 0),
                }
            regressions = detect_regressions(
                criterion_results, baseline_metrics, REGRESSION_THRESHOLDS
            )
            print(f"Detected {len(regressions)} regressions against baseline")
        except (json.JSONDecodeError, KeyError) as e:
            print(f"  ⚠ Could not parse baseline: {e}", file=sys.stderr)

    # Pass/fail check
    threshold_check = check_pass_thresholds(flat_metrics)

    # Determine verdict
    verdict = "pass"
    if regressions:
        verdict = "fail"
    if not threshold_check["passed"]:
        verdict = "fail"

    report: Dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "commit": git["commit"],
        "branch": git["branch"],
        "timestamp": timestamp,
        "machine": machine,
        "runtime": {
            "wasm_size_mb": 0,  # TODO: detect from build
            "block_time_ms": 12000,
            "max_block_weight": 0,
        },
        "criteria": criterion_results,
        "k6": (
            json.loads(k6_summary_path.read_text())
            if k6_summary_path.exists()
            else {}
        ),
        "flat_metrics": flat_metrics,
        "regressions": regressions,
        "threshold_violations": threshold_check["violations"],
        "verdict": verdict,
    }

    # Write report
    if output_path is None:
        date_str = datetime.now().strftime("%Y-%m-%d")
        output_path = REPORTS_DIR / f"x3-benchmark-report-{date_str}.json"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"Report written to {output_path}")

    # Save as baseline for future runs
    baseline_out = output_path.parent / "x3-benchmark-baseline.json"
    with open(baseline_out, "w") as f:
        json.dump(report, f, indent=2)
    print(f"Baseline saved to {baseline_out}")

    return report


# ─── CLI ─────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="X3 Benchmark Report Generator"
    )
    parser.add_argument(
        "--criterion-dir",
        default="target/criterion",
        help="Path to Criterion output directory (default: target/criterion)",
    )
    parser.add_argument(
        "--baseline",
        default=None,
        help="Path to baseline JSON for regression detection",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Custom output path for the report",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output report as JSON to stdout",
    )
    args = parser.parse_args()

    criterion_dir = Path(args.criterion_dir)
    baseline_path = Path(args.baseline) if args.baseline else None
    output_path = Path(args.output) if args.output else None

    report = generate_report(criterion_dir, baseline_path, output_path)

    if args.json:
        print(json.dumps(report, indent=2))

    # Summary
    print(f"\n═══ X3 Benchmark Report ═══")
    print(f"Commit:   {report['commit'][:8]}")
    print(f"Branch:   {report['branch']}")
    print(f"Machine:  {report['machine']['cpu']} ({report['machine'].get('ram_gb', '?')} GB RAM)")
    print(f"Criteria: {len(report['criteria'])} groups")
    print(f"Regressions: {len(report['regressions'])}")
    if report["regressions"]:
        for r in report["regressions"]:
            arrow = "↑" if r["delta_percent"] > 0 else "↓"
            print(f"  {r['status'].upper()}  {r['metric']}: {r['delta_percent']:+.1f}% {arrow}")
    print(f"Verdict:  {report['verdict'].upper()}")
    print()


if __name__ == "__main__":
    main()