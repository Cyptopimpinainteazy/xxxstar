#!/usr/bin/env python3
"""Attribute a node's block-import time to runtime execution or to the client.

The storage audit's §3 question — which stage is the bottleneck? — needs the
import split into "the runtime ran" and "everything around the runtime". The
node exports both halves now:

* `x3_runtime_call_seconds{method}` / `x3_runtime_calls_total{method}`, from the
  timed executor wrapper (`node/src/timed_executor.rs`);
* `substrate_block_verification_and_import_time`, from the SDK, for the whole
  import.

This script reads a live `/metrics` endpoint and prints the split. What the
remainder contains is stated rather than guessed: verification, the state root,
the database commit and the import notification all happen after the executor
returns, and none of them has its own metric yet.

Usage:
    scripts/proof/runtime-attribution.py [prometheus-url] [--window SECONDS] [--json]

Without `--window` this reports the node's cumulative counters since it started,
which is a trap: the first runtime call on a fresh node has to decompress and
instantiate the runtime, so a cumulative mean over a short uptime is dominated by
that one call. Measured on a 2-core dev box, `x3_runtime_version_seconds` had a
cumulative mean of 2.3 ms per call five minutes after start and a *steady-state*
mean of 1.2 us — the difference was entirely startup. Use `--window` (two scrapes
N seconds apart) for anything you intend to quote, which is what this script
reports as deltas plus the share of wall time.

Exit code 0 means the split was computed; 1 means the endpoint did not answer.
"""

from __future__ import annotations

import json
import re
import sys
import urllib.error
import urllib.request
from collections import defaultdict

DEFAULT_URL = "http://127.0.0.1:9615/metrics"

SAMPLE = re.compile(r"^(?P<name>[a-zA-Z_:][a-zA-Z0-9_:]*)(?P<labels>\{[^}]*\})?\s+(?P<value>[-+0-9.eE]+)$")
LABEL = re.compile(r'(?P<key>[a-zA-Z_][a-zA-Z0-9_]*)="(?P<value>(?:[^"\\]|\\.)*)"')


def fetch(url: str) -> str:
    try:
        with urllib.request.urlopen(url, timeout=15) as response:
            return response.read().decode("utf-8", errors="replace")
    except (urllib.error.URLError, TimeoutError, OSError) as err:
        print(f"error: cannot read {url}: {err}", file=sys.stderr)
        raise SystemExit(1)


def parse(text: str) -> dict[str, list[dict[str, object]]]:
    """{metric name: [{"labels": {...}, "value": float}, ...]}"""
    samples: dict[str, list[dict[str, object]]] = defaultdict(list)
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = SAMPLE.match(line)
        if match is None:
            continue
        labels = {}
        if match.group("labels"):
            labels = {m.group("key"): m.group("value") for m in LABEL.finditer(match.group("labels"))}
        samples[match.group("name")].append(
            {"labels": labels, "value": float(match.group("value"))}
        )
    return samples


def scalar(samples: dict[str, list[dict[str, object]]], name: str) -> float | None:
    entries = samples.get(name)
    if not entries:
        return None
    return float(entries[0]["value"])


def by_method(samples: dict[str, list[dict[str, object]]], name: str) -> dict[str, float]:
    out: dict[str, float] = {}
    for entry in samples.get(name, []):
        labels = entry["labels"]
        method = labels.get("method") if isinstance(labels, dict) else None
        if method:
            out[str(method)] = float(entry["value"])
    return out


def snapshot(
    samples: dict[str, list[dict[str, object]]]
) -> dict[str, object]:
    """Extract the numbers this report needs out of one scrape."""
    calls = by_method(samples, "x3_runtime_calls_total")
    call_sum = by_method(samples, "x3_runtime_call_seconds_sum")
    call_count = by_method(samples, "x3_runtime_call_seconds_count")
    errors = by_method(samples, "x3_runtime_call_errors_total")

    if not calls:
        raise LookupError(
            "no x3_runtime_calls_total samples: either this node predates the timed "
            "executor, or it was started without a Prometheus registry"
        )

    import_sum = scalar(samples, "substrate_block_verification_and_import_time_sum")
    import_count = scalar(samples, "substrate_block_verification_and_import_time_count")
    version_sum = scalar(samples, "x3_runtime_version_seconds_sum")
    version_count = scalar(samples, "x3_runtime_version_seconds_count")

    methods = sorted(set(calls) | set(call_sum) | set(call_count) | set(errors))
    rows = [
        {
            "method": method,
            "calls": int(calls.get(method, 0.0)),
            "seconds_total": call_sum.get(method, 0.0),
            "seconds_mean": (
                call_sum.get(method, 0.0) / call_count[method]
                if call_count.get(method, 0.0)
                else None
            ),
            "errors": int(errors.get(method, 0.0)),
        }
        for method in methods
    ]
    rows.sort(key=lambda row: -row["seconds_total"])

    exec_total = call_sum.get("Core_execute_block", 0.0)
    exec_calls = call_count.get("Core_execute_block", 0.0)
    attributed = sum(call_sum.values()) + (version_sum or 0.0)

    return {
        "runtime_calls": rows,
        "runtime_call_seconds_total": attributed,
        "runtime_version": {
            "calls": int(version_count or 0),
            "seconds_total": version_sum or 0.0,
            "seconds_mean": ((version_sum or 0.0) / version_count) if version_count else None,
        },
        "import": {
            "blocks": int(import_count or 0),
            "seconds_total": import_sum or 0.0,
            "seconds_mean": (import_sum / import_count) if import_count else None,
        },
        "execute_block": {
            "calls": int(exec_calls),
            "seconds_total": exec_total,
            "seconds_mean": (exec_total / exec_calls) if exec_calls else None,
        },
    }


def difference(
    before: dict[str, object], after: dict[str, object], seconds: float
) -> dict[str, object]:
    """The counters that moved between two scrapes, plus derived shares."""
    before_by_method = {row["method"]: row for row in before["runtime_calls"]}
    rows = []
    for row in after["runtime_calls"]:
        previous = before_by_method.get(row["method"])
        calls = row["calls"] - (previous["calls"] if previous else 0)
        total = row["seconds_total"] - (previous["seconds_total"] if previous else 0.0)
        errors = row["errors"] - (previous["errors"] if previous else 0)
        rows.append(
            {
                "method": row["method"],
                "calls": calls,
                "seconds_total": total,
                "seconds_mean": (total / calls) if calls else None,
                "errors": errors,
            }
        )
    rows.sort(key=lambda row: -row["seconds_total"])

    def delta(section: str, field: str = "seconds_total") -> float:
        return float(after[section][field]) - float(before[section][field])

    import_seconds = delta("import", "seconds_total")
    import_blocks = int(after["import"]["blocks"]) - int(before["import"]["blocks"])
    exec_seconds = delta("execute_block", "seconds_total")
    exec_calls = int(after["execute_block"]["calls"]) - int(before["execute_block"]["calls"])

    report: dict[str, object] = {
        "window_seconds": seconds,
        "runtime_calls": rows,
        "runtime_call_seconds_total": sum(row["seconds_total"] for row in rows)
        + delta("runtime_version", "seconds_total"),
        "runtime_version": {
            "calls": int(after["runtime_version"]["calls"]) - int(before["runtime_version"]["calls"]),
            "seconds_total": delta("runtime_version", "seconds_total"),
            "seconds_mean": None,
        },
        "import": {
            "blocks": import_blocks,
            "seconds_total": import_seconds,
            "seconds_mean": (import_seconds / import_blocks) if import_blocks else None,
        },
        "execute_block": {
            "calls": exec_calls,
            "seconds_total": exec_seconds,
            "seconds_mean": (exec_seconds / exec_calls) if exec_calls else None,
        },
        "share_of_wall_time": {
            "import": import_seconds / seconds,
            "runtime_calls": (
                sum(row["seconds_total"] for row in rows) + delta("runtime_version", "seconds_total")
            )
            / seconds,
        },
    }
    if report["runtime_version"]["calls"]:
        report["runtime_version"]["seconds_mean"] = (
            report["runtime_version"]["seconds_total"] / report["runtime_version"]["calls"]
        )
    if import_seconds > 0:
        report["execution_share_of_import"] = exec_seconds / import_seconds
        report["unattributed_seconds"] = max(0.0, import_seconds - exec_seconds)
    return report


def render(report: dict[str, object], url: str, window: float | None) -> None:
    rows = report["runtime_calls"]
    if window:
        print(f"runtime call attribution from {url} over a {window:.0f}s window")
    else:
        print(f"runtime call attribution from {url} (cumulative since start)")
        print(
            "note: cumulative means include the first runtime call's instantiation cost; "
            "use --window for a steady-state number"
        )
    print()
    print(f"{'method':<56}{'calls':>10}{'mean ms':>12}{'total s':>12}{'errors':>8}")
    print("-" * 98)
    for row in rows[:20]:
        mean = row["seconds_mean"]
        mean_ms = f"{mean * 1000:.3f}" if mean is not None else "n/a"
        print(
            f"{row['method']:<56}{row['calls']:>10}{mean_ms:>12}"
            f"{row['seconds_total']:>12.4f}{row['errors']:>8}"
        )
    if len(rows) > 20:
        print(f"... {len(rows) - 20} more method(s)")

    version = report["runtime_version"]
    if version["calls"]:
        print()
        print(
            f"runtime version reads: {version['calls']} calls, "
            f"mean {(version['seconds_mean'] or 0) * 1e6:.1f} us, "
            f"total {version['seconds_total']:.4f} s"
        )

    print()
    import_seconds = report["import"]["seconds_total"]
    if import_seconds:
        mean_exec = report["execute_block"]["seconds_mean"] or 0.0
        mean_import = report["import"]["seconds_mean"] or 0.0
        print(
            f"Core_execute_block: {report['execute_block']['calls']} calls over "
            f"{report['import']['blocks']} imported blocks "
            f"(mean {mean_exec * 1000:.3f} ms per block)"
        )
        print(
            f"import (substrate_block_verification_and_import_time): "
            f"total {import_seconds:.3f} s over {report['import']['blocks']} blocks "
            f"(mean {mean_import * 1000:.3f} ms per block)"
        )
        print(f"execution accounts for {report.get('execution_share_of_import', 0):.1%} of import time")
        print(
            f"the remaining {report.get('unattributed_seconds', 0.0):.3f} s is verification + "
            "state root + database commit + notification; none of those has its own metric yet"
        )
        if window:
            share = report["share_of_wall_time"]
            print(
                f"of the {window:.0f}s of wall time: {share['import']:.1%} was block import, "
                f"{share['runtime_calls']:.1%} was inside the runtime"
            )
    else:
        print(
            "no substrate_block_verification_and_import_time in this window: this node did not "
            "import a block (an authoring node does not re-execute its own blocks)"
        )


def main(argv: list[str]) -> int:
    url = None
    window = None
    as_json = False
    index = 1
    while index < len(argv):
        arg = argv[index]
        if arg == "--json":
            as_json = True
        elif arg == "--window":
            index += 1
            if index >= len(argv):
                print("error: --window needs a number of seconds", file=sys.stderr)
                return 2
            try:
                window = float(argv[index])
            except ValueError:
                print(f"error: --window {argv[index]!r} is not a number", file=sys.stderr)
                return 2
        elif arg.startswith("--"):
            print(f"error: unknown flag {arg}", file=sys.stderr)
            return 2
        elif url is None:
            url = arg
        else:
            print(f"error: unexpected argument {arg}", file=sys.stderr)
            return 2
        index += 1
    url = url or DEFAULT_URL

    try:
        before = snapshot(parse(fetch(url)))
    except LookupError as err:
        print(f"error: {err}", file=sys.stderr)
        return 1

    if window:
        import time

        time.sleep(window)
        after = snapshot(parse(fetch(url)))
        report = difference(before, after, window)
    else:
        report = before
        if report["import"]["seconds_total"] > 0:
            report["execution_share_of_import"] = (
                report["execute_block"]["seconds_total"] / report["import"]["seconds_total"]
            )
            report["unattributed_seconds"] = max(
                0.0, report["import"]["seconds_total"] - report["execute_block"]["seconds_total"]
            )

    if as_json:
        report["url"] = url
        print(json.dumps(report, indent=2))
        return 0

    render(report, url, window)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
