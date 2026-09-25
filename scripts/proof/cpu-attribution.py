#!/usr/bin/env python3
"""Where did the CPU go during a throughput run?

Every TPS number in this repository was produced by a JavaScript load generator
running on the same two cores as the nodes it is loading, which makes "the chain
does 119 TPS" an unattributable statement: the chain's own cost and the harness's
cost are added together and only their sum is visible. The runtime-call metrics
answer it from the node's side; this answers it from the machine's side.

It samples `/proc/<pid>/stat` for named processes over a window and reports
core-seconds, which is a measure of *CPU consumed*, not of wall time — so it
separates "the node was busy" from "the node was waiting".

Usage:
    scripts/proof/cpu-attribution.py --seconds 70 --pid author=PID --pid load=PID
        [--transactions N] [--json]

`--transactions` turns the result into a cost model: core-seconds per finalized
extrinsic, which is the number that decides how much hardware a target TPS needs.

Exit code 0 when the window was measured.
"""

from __future__ import annotations

import json
import os
import sys
import time

CLOCK_TICKS = os.sysconf("SC_CLK_TCK")


def cpu_seconds(pid: int) -> float | None:
    """utime + stime + children, in seconds, or None if the pid is gone."""
    try:
        with open(f"/proc/{pid}/stat", "r", encoding="utf-8") as handle:
            fields = handle.read().rsplit(") ", 1)[1].split()
    except (OSError, IndexError):
        return None
    # After the comm field, `stat` continues at field 3 (state); utime is field
    # 14 overall, which is index 11 here.
    utime, stime = int(fields[11]), int(fields[12])
    cutime, cstime = int(fields[13]), int(fields[14])
    return (utime + stime + cutime + cstime) / CLOCK_TICKS


def parse_args(argv: list[str]) -> tuple[float, list[tuple[str, int]], int | None, bool, float]:
    seconds = 60.0
    pids: list[tuple[str, int]] = []
    transactions = None
    as_json = False
    interval = 2.0
    index = 1
    while index < len(argv):
        arg = argv[index]
        if arg == "--json":
            as_json = True
        elif arg in ("--seconds", "--pid", "--transactions", "--interval"):
            index += 1
            if index >= len(argv):
                raise SystemExit(f"error: {arg} needs a value")
            value = argv[index]
            if arg == "--seconds":
                seconds = float(value)
            elif arg == "--transactions":
                transactions = int(value)
            elif arg == "--interval":
                interval = float(value)
            else:
                if "=" not in value:
                    raise SystemExit("error: --pid takes name=pid")
                name, raw_pid = value.split("=", 1)
                pids.append((name, int(raw_pid)))
        else:
            raise SystemExit(f"error: unexpected argument {arg}")
        index += 1
    if not pids:
        raise SystemExit("error: at least one --pid name=pid is required")
    return seconds, pids, transactions, as_json, interval


def main(argv: list[str]) -> int:
    seconds, pids, transactions, as_json, interval = parse_args(argv)

    before = {}
    for name, pid in pids:
        value = cpu_seconds(pid)
        if value is None:
            print(f"error: pid {pid} ({name}) is not running", file=sys.stderr)
            return 1
        before[name] = (pid, value)

    cores = os.cpu_count() or 1
    started = time.monotonic()
    # Sample rather than sleep in one go, so the report can say whether each
    # process was present for the whole window.
    alive = {name: True for name, _ in pids}
    while time.monotonic() - started < seconds:
        time.sleep(min(interval, max(0.05, seconds - (time.monotonic() - started))))
        for name, pid in pids:
            if alive[name] and cpu_seconds(pid) is None:
                alive[name] = False
    elapsed = time.monotonic() - started

    results = []
    total = 0.0
    for name, pid in pids:
        after = cpu_seconds(pid)
        start_value = before[name][1]
        if after is None:
            # The process exited during the window; the last reading is unknown,
            # so report the lower bound rather than pretending.
            results.append(
                {
                    "name": name,
                    "pid": pid,
                    "core_seconds": None,
                    "share_of_machine": None,
                    "present_for_whole_window": False,
                }
            )
            continue
        core_seconds = after - start_value
        total += core_seconds
        results.append(
            {
                "name": name,
                "pid": pid,
                "core_seconds": core_seconds,
                "share_of_machine": core_seconds / (elapsed * cores),
                "present_for_whole_window": alive[name],
            }
        )

    report = {
        "window_seconds": elapsed,
        "cores": cores,
        "processes": results,
        "core_seconds_total": total,
        "share_of_machine_total": total / (elapsed * cores),
    }
    if transactions:
        report["transactions"] = transactions
        report["cpu_seconds_per_transaction"] = total / transactions if transactions else None
        report["cpu_milliseconds_per_transaction"] = (
            (total / transactions) * 1000 if transactions else None
        )

    if as_json:
        print(json.dumps(report, indent=2))
        return 0

    print(f"CPU attribution over {elapsed:.1f}s on {cores} core(s)")
    print()
    print(f"{'process':<24}{'pid':>8}{'core-seconds':>15}{'share of box':>14}")
    print("-" * 61)
    for row in results:
        if row["core_seconds"] is None:
            print(f"{row['name']:<24}{row['pid']:>8}{'exited':>15}{'n/a':>14}")
            continue
        print(
            f"{row['name']:<24}{row['pid']:>8}{row['core_seconds']:>15.2f}"
            f"{row['share_of_machine']:>13.1%}"
        )
    print("-" * 61)
    print(f"{'total':<24}{'':>8}{total:>15.2f}{report['share_of_machine_total']:>13.1%}")

    if transactions:
        cpu_per_tx = total / transactions
        print()
        print(f"{transactions} finalized transactions in the window")
        print(
            f"{cpu_per_tx * 1000:.2f} ms of CPU per transaction across every process measured"
        )
        print(
            f"at {cores} core(s) that is a ceiling of about {cores / cpu_per_tx:.0f} transactions "
            "per second if every core were spent on this work and nothing else"
        )
    print()
    print(
        "core-seconds measure CPU consumed, so a process that is waiting (on the network, on "
        "the disk, on a lock) does not look busy here even though its wall clock is passing."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
