#!/usr/bin/env python3
import argparse
import json
import math
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def parse_json_tail(text: str) -> dict:
    idx = text.rfind("{")
    if idx == -1:
      raise ValueError("No JSON object found in process output")
    return json.loads(text[idx:])


def main() -> int:
    parser = argparse.ArgumentParser(description="Run multiprocess X3 remark load test.")
    parser.add_argument("--rpc-ws", default="ws://127.0.0.1:9944")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--senders", type=int, default=120)
    parser.add_argument("--duration-sec", type=int, default=60)
    parser.add_argument("--finality-wait-sec", type=int, default=20)
    parser.add_argument("--concurrency-total", type=int, default=512)
    parser.add_argument("--prefund-amount-planck", default="1000000000000")
    parser.add_argument("--output", default="benchmarks/x3_chain_tps_multiprocess.json")
    parser.add_argument("--require-baseline", action="store_true")
    parser.add_argument("--min-duration-sec", type=int, default=1200)
    parser.add_argument("--min-finalized-tps", type=float, default=0.0)
    parser.add_argument("--max-error-rate", type=float, default=0.01)
    args = parser.parse_args()

    if args.workers < 1 or args.senders < 1 or args.concurrency_total < 1:
        raise ValueError("workers/senders/concurrency-total must be > 0")

    senders_per_worker = math.ceil(args.senders / args.workers)
    conc_per_worker = max(1, args.concurrency_total // args.workers)
    procs = []

    # Stage 1: one-shot prefund to avoid funder nonce collisions across workers.
    prefund_env = os.environ.copy()
    prefund_env["RPC_WS"] = args.rpc_ws
    prefund_env["SENDER_MODE"] = "derived"
    prefund_env["DERIVATION_BASE"] = "//Alice//load"
    prefund_env["SENDER_OFFSET"] = "0"
    prefund_env["SENDER_COUNT"] = str(args.senders)
    prefund_env["PRE_FUND"] = "true"
    prefund_env["ONLY_PREFUND"] = "true"
    prefund_env["PREFUND_AMOUNT_PLANCK"] = str(args.prefund_amount_planck)
    prefund = subprocess.run(
        ["node", "scripts/testnet/load-remarks-tps.js"],
        cwd=Path(__file__).resolve().parents[2],
        env=prefund_env,
        capture_output=True,
        text=True,
        check=False,
    )
    prefund_stage = {
        "returncode": prefund.returncode,
        "stdout_tail": (prefund.stdout or "")[-2000:],
        "stderr_tail": (prefund.stderr or "")[-2000:],
    }
    if prefund.returncode != 0:
        aggregate = {
            "timestamp_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
            "benchmark": "x3_chain_tps_multiprocess",
            "rpc_ws": args.rpc_ws,
            "workers": args.workers,
            "senders_total": args.senders,
            "prefund_stage": prefund_stage,
            "error": "prefund stage failed",
            "runs": [],
            "successful_workers": 0,
            "finalized_total": 0,
            "finalized_tps_submit_window": 0.0,
        }
        out = Path(args.output)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(aggregate, indent=2))
        print(str(out))
        print(json.dumps({"error": "prefund stage failed"}, indent=2))
        return 1

    for worker_id in range(args.workers):
        sender_offset = worker_id * senders_per_worker
        sender_count = min(senders_per_worker, args.senders - sender_offset)
        if sender_count <= 0:
            continue

        env = os.environ.copy()
        env["RPC_WS"] = args.rpc_ws
        env["SENDER_MODE"] = "derived"
        env["DERIVATION_BASE"] = "//Alice//load"
        env["SENDER_OFFSET"] = str(sender_offset)
        env["SENDER_COUNT"] = str(sender_count)
        env["PRE_FUND"] = "false"
        env["PREFUND_AMOUNT_PLANCK"] = str(args.prefund_amount_planck)
        env["DURATION_SEC"] = str(args.duration_sec)
        env["FINALITY_WAIT_SEC"] = str(args.finality_wait_sec)
        env["CONCURRENCY"] = str(conc_per_worker)

        cmd = ["node", "scripts/testnet/load-remarks-tps.js"]
        procs.append(
            (
                worker_id,
                sender_offset,
                sender_count,
                subprocess.Popen(
                    cmd,
                    cwd=Path(__file__).resolve().parents[2],
                    env=env,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                ),
            )
        )

    runs = []
    for worker_id, sender_offset, sender_count, proc in procs:
        stdout, stderr = proc.communicate()
        run = {
            "worker_id": worker_id,
            "sender_offset": sender_offset,
            "sender_count": sender_count,
            "returncode": proc.returncode,
        }
        if proc.returncode == 0:
            try:
                run["result"] = parse_json_tail(stdout or "")
            except Exception as exc:
                run["parse_error"] = str(exc)
                run["stdout_tail"] = (stdout or "")[-2000:]
        else:
            run["stderr_tail"] = (stderr or "")[-2000:]
            run["stdout_tail"] = (stdout or "")[-2000:]
        runs.append(run)

    ok = [r["result"] for r in runs if isinstance(r.get("result"), dict)]
    finalized = sum(r.get("finalized", 0) for r in ok)
    sent = sum(r.get("sent", 0) for r in ok)
    accepted = sum(r.get("accepted", 0) for r in ok)
    failed = sum(r.get("failed", 0) for r in ok)

    aggregate = {
        "timestamp_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "benchmark": "x3_chain_tps_multiprocess",
        "rpc_ws": args.rpc_ws,
        "workers": args.workers,
        "senders_total": args.senders,
        "duration_sec": args.duration_sec,
        "finality_wait_sec": args.finality_wait_sec,
        "concurrency_total": args.concurrency_total,
        "concurrency_per_worker": conc_per_worker,
        "prefund_amount_planck": str(args.prefund_amount_planck),
        "prefund_stage": prefund_stage,
        "sent_total": sent,
        "accepted_total": accepted,
        "failed_total": failed,
        "finalized_total": finalized,
        "finalized_tps_submit_window": round(finalized / args.duration_sec, 3) if args.duration_sec > 0 else 0.0,
        "error_rate": round((failed / sent), 4) if sent > 0 else 0.0,
        "successful_workers": len(ok),
        "runs": runs,
    }

    baseline_ok = (
        args.duration_sec >= args.min_duration_sec
        and (args.min_finalized_tps <= 0 or aggregate["finalized_tps_submit_window"] >= args.min_finalized_tps)
        and aggregate["error_rate"] <= args.max_error_rate
    )
    aggregate["baseline_requirements"] = {
        "require_baseline": args.require_baseline,
        "min_duration_sec": args.min_duration_sec,
        "min_finalized_tps": args.min_finalized_tps,
        "max_error_rate": args.max_error_rate,
        "baseline_ok": baseline_ok,
    }

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(aggregate, indent=2))
    print(str(out))
    print(json.dumps({k: aggregate[k] for k in ["workers", "senders_total", "concurrency_total", "finalized_total", "finalized_tps_submit_window", "successful_workers"]}, indent=2))
    if args.require_baseline and not baseline_ok:
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise
