#!/usr/bin/env python3
"""How fast does a chain's database actually grow?

The storage audit's §22-23 question is disk economics, and it had no number:
block contexts were measured (3.6 KB per block under load), but the *state*
behind them was not. Those are different quantities — a block body is written
once, while state accumulates — and only the growing side decides how long a
validator's disk lasts.

This samples the database directory while a node runs and turns the growth into
bytes per block, bytes per extrinsic and bytes per day. It measures the whole
database, not only the state trie, and says so: with ParityDB the trie, the
block bodies and the metadata live in the same directory, and separating them
needs a state-diff metric the node does not have.

Two caveats are built into the output rather than hidden in a footnote:

* ParityDB preallocates, so growth arrives in steps. The script reports the
  slope over the *second half* of the window as well as the total, and a run
  shorter than a minute should not be quoted.
* A dev chain is a floor, not a workload. Real activity writes far more state.

Usage:
    scripts/proof/state-growth.py --base-path <node base path> [--seconds 180]
        [--interval 15] [--chain <dir name>] [--transactions N] [--json]

Exit code 0 when at least three samples were taken, 1 when the database was not
found.
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path


def run(argv: list[str]) -> int:
    return subprocess.run(argv, check=False).returncode


def directory_bytes(path: Path) -> tuple[int, int]:
    """(allocated, apparent) bytes.

    The two are not the same thing, and the difference matters: ParityDB
    preallocates and extends its files sparsely, so `st_size` (apparent) can grow
    by megabytes while `st_blocks` (what the filesystem actually spent) lags
    behind. Disk economics is about the allocated number; write volume is closer
    to the apparent one.
    """
    allocated = 0
    apparent = 0
    for entry in path.rglob("*"):
        try:
            if entry.is_file():
                stat = entry.stat()
                apparent += stat.st_size
                allocated += stat.st_blocks * 512
        except OSError:
            # A file that vanished mid-walk is not a measurement error worth
            # aborting for; the next sample will pick the tree up again.
            continue
    return allocated, apparent


def find_database(base: Path, chain: str | None) -> Path:
    candidates = sorted((base / "chains").glob("*/paritydb")) if chain is None else [
        base / "chains" / chain / "paritydb"
    ]
    for candidate in candidates:
        if candidate.is_dir():
            return candidate
    raise SystemExit(
        f"error: no paritydb directory under {base}/chains"
        + (f" for chain {chain}" if chain else "")
    )


def parse_args(argv: list[str]) -> tuple[Path, float, float, str | None, int | None, bool]:
    base = None
    seconds = 180.0
    interval = 15.0
    chain = None
    transactions = None
    as_json = False
    index = 1
    while index < len(argv):
        arg = argv[index]
        if arg == "--json":
            as_json = True
        elif arg in ("--base-path", "--seconds", "--interval", "--chain", "--transactions"):
            index += 1
            if index >= len(argv):
                raise SystemExit(f"error: {arg} needs a value")
            value = argv[index]
            if arg == "--base-path":
                base = Path(value)
            elif arg == "--seconds":
                seconds = float(value)
            elif arg == "--interval":
                interval = float(value)
            elif arg == "--chain":
                chain = value
            else:
                transactions = int(value)
        else:
            raise SystemExit(f"error: unexpected argument {arg}")
        index += 1
    if base is None:
        raise SystemExit("error: --base-path is required")
    return base, seconds, interval, chain, transactions, as_json


def main(argv: list[str]) -> int:
    base, seconds, interval, chain, transactions, as_json = parse_args(argv)
    database = find_database(base, chain)

    samples = []
    started = time.monotonic()
    while True:
        elapsed = time.monotonic() - started
        samples.append((elapsed, directory_bytes(database)))
        if elapsed >= seconds:
            break
        time.sleep(min(interval, max(0.1, seconds - elapsed)))

    if len(samples) < 3:
        print("error: fewer than three samples; the window is too short", file=sys.stderr)
        return 1

    first_time, (first_alloc, first_apparent) = samples[0]
    last_time, (last_alloc, last_apparent) = samples[-1]
    window = last_time - first_time

    # A naive slope is wrong here, and visibly so: ParityDB compacts, so a window
    # that contains one compaction reports negative growth and a per-day figure of
    # minus tens of GiB. Three separate numbers are what the question needs:
    #
    #   writes      every byte written (the sum of the positive deltas), which is
    #               the I/O a disk has to sustain;
    #   net growth  end - start, which is only meaningful over a window long
    #               enough to contain a compaction cycle;
    #   live floor  the smallest size seen, i.e. what the database occupies just
    #               after compaction — the closest thing to "the size of the data".
    writes_alloc = 0
    writes_apparent = 0
    reclaims = 0
    previous_alloc, previous_apparent = first_alloc, first_apparent
    for _, (allocated, apparent) in samples[1:]:
        writes_alloc += max(0, allocated - previous_alloc)
        writes_apparent += max(0, apparent - previous_apparent)
        if allocated < previous_alloc:
            reclaims += 1
        previous_alloc, previous_apparent = allocated, apparent

    write_rate = writes_alloc / window if window > 0 else 0.0
    live_floor = min(size for _, (size, _) in samples)
    net_growth = last_alloc - first_alloc

    blocks_per_second = 5.0  # measured: 200 ms blocks
    per_block = write_rate / blocks_per_second
    report = {
        "database": str(database),
        "window_seconds": window,
        "samples": len(samples),
        "allocated_bytes_start": first_alloc,
        "allocated_bytes_end": last_alloc,
        "allocated_live_floor_bytes": live_floor,
        "allocated_written_bytes": writes_alloc,
        "allocated_net_growth_bytes": net_growth,
        "apparent_bytes_start": first_apparent,
        "apparent_bytes_end": last_apparent,
        "apparent_written_bytes": writes_apparent,
        "compactions_observed": reclaims,
        "write_bytes_per_second": write_rate,
        "write_apparent_bytes_per_second": writes_apparent / window if window > 0 else 0.0,
        "assumed_blocks_per_second": blocks_per_second,
        "bytes_per_block": per_block,
        "written_gib_per_day_at_this_rate": write_rate * 86_400 / 1024**3,
        "net_gib_per_day_from_this_window": (
            net_growth / window * 86_400 / 1024**3 if window > 0 else 0.0
        ),
    }
    if transactions:
        report["transactions"] = transactions
        report["bytes_written_per_transaction"] = writes_alloc / transactions

    if as_json:
        print(json.dumps(report, indent=2))
        return 0

    print(f"database: {database}")
    print(f"{'seconds':>9}{'allocated':>16}{'delta':>14}{'apparent':>16}")
    print("-" * 55)
    previous = first_alloc
    for elapsed, (allocated, apparent) in samples:
        print(f"{elapsed:>9.1f}{allocated:>16}{allocated - previous:>+14}{apparent:>16}")
        previous = allocated
    print("-" * 55)
    print(f"window              {window:.1f} s")
    print(f"written (allocated) {writes_alloc} bytes = {write_rate:.0f} B/s")
    print(f"written (apparent)  {writes_apparent} bytes = {writes_apparent / window:.0f} B/s")
    print(f"per block           {per_block:.0f} bytes written (at {blocks_per_second:.0f} blocks/s)")
    print(f"written per day     {report['written_gib_per_day_at_this_rate']:.2f} GiB at this rate")
    print(f"net growth          {net_growth:+} bytes over the window "
          f"({report['net_gib_per_day_from_this_window']:+.2f} GiB/day)")
    print(f"live floor          {live_floor} bytes (smallest size seen: post-compaction)")
    print(f"compactions seen    {reclaims}")
    if transactions:
        print(f"per transaction     {report['bytes_written_per_transaction']:.0f} bytes written")
    print()
    print(
        "'written' is every byte the database wrote (the I/O a disk must sustain); 'net growth' is "
        "what it kept, and is only meaningful over a window that contains a compaction cycle; "
        "'live floor' is the size just after compaction. This is the whole database — state trie, "
        "block bodies and metadata together — and a dev chain is a floor, not a workload"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
