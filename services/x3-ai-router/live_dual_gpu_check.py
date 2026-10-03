"""Live check and benchmark of the dual-GPU router stack on real hardware.

Runs against the running services (router :11435, worker A :11434 on the
RTX 2060 SUPER, worker B :11436 on the GTX 1070) and writes JSON evidence to
audit-artifacts/gpu-ai/<timestamp>.json. Exits nonzero if a routing or
failover expectation fails.

    python3 services/x3-ai-router/live_dual_gpu_check.py [--iterations 5] [--no-failover]

The failover step stops and restarts the `ollama-worker-b` user service.
"""
import argparse
import datetime as dt
import json
import os
import statistics
import subprocess
import threading
import time
import urllib.request
import uuid
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROUTER = "http://127.0.0.1:11435"
WORKERS = {"rtx": "http://127.0.0.1:11434", "gtx": "http://127.0.0.1:11436"}
REPO = Path(__file__).resolve().parents[2]
PROMPT = ("Write a Rust function `fn checked_sum(xs: &[u64]) -> Option<u64>` that returns None on overflow, "
          "then a unit test for it. Code only.")


def http(url, body=None, headers=None, timeout=300):
    data = json.dumps(body).encode() if body is not None else None
    request = urllib.request.Request(url, data, {"Content-Type": "application/json", **(headers or {})},
                                     method="POST" if data else "GET")
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.load(response)


def gpus():
    # No UUIDs: evidence may be published, and index + name identify the card.
    out = subprocess.run(["nvidia-smi", "--query-gpu=index,name,memory.used,memory.total,utilization.gpu,"
                          "temperature.gpu,power.draw,power.limit,pcie.link.gen.current,pcie.link.width.current,"
                          "driver_version", "--format=csv,noheader,nounits"],
                         capture_output=True, text=True, check=True).stdout
    keys = ["index", "name", "mem_used_mib", "mem_total_mib", "util_pct", "temp_c", "power_w",
            "power_limit_w", "pcie_gen", "pcie_width", "driver"]
    return [dict(zip(keys, (v.strip() for v in line.split(",")))) for line in out.strip().splitlines()]


class Sampler:
    """Polls nvidia-smi while a benchmark runs, for peak VRAM/power/util."""

    def __init__(self):
        self.samples, self.stop = [], threading.Event()
        self.thread = threading.Thread(target=self.run, daemon=True)

    def run(self):
        while not self.stop.is_set():
            self.samples.append(gpus())
            self.stop.wait(0.5)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *exc):
        self.stop.set()
        self.thread.join()

    def peaks(self):
        peak = {}
        for sample in self.samples:
            for gpu in sample:
                p = peak.setdefault(gpu["name"], {"peak_mem_mib": 0, "peak_util_pct": 0, "peak_power_w": 0.0,
                                                  "peak_temp_c": 0})
                p["peak_mem_mib"] = max(p["peak_mem_mib"], int(gpu["mem_used_mib"]))
                p["peak_util_pct"] = max(p["peak_util_pct"], int(gpu["util_pct"]))
                p["peak_power_w"] = max(p["peak_power_w"], float(gpu["power_w"]))
                p["peak_temp_c"] = max(p["peak_temp_c"], int(gpu["temp_c"]))
        return peak


def p95(values):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, round(0.95 * (len(ordered) - 1)))]


def bench_worker(base, model, iterations):
    runs, failures = [], 0
    http(base + "/api/generate", {"model": model, "prompt": "hi", "stream": False, "options": {"num_predict": 1}})
    with Sampler() as sampler:
        for _ in range(iterations):
            started = time.monotonic()
            try:
                r = http(base + "/api/generate", {"model": model, "prompt": PROMPT, "stream": False,
                                                  "options": {"num_predict": 256, "temperature": 0}})
            except Exception:  # noqa: BLE001 - counted, not hidden
                failures += 1
                continue
            runs.append({"gen_tok_s": r["eval_count"] / r["eval_duration"] * 1e9,
                         "prompt_tok_s": r["prompt_eval_count"] / max(r["prompt_eval_duration"], 1) * 1e9,
                         "first_token_ms": (r.get("load_duration", 0) + r["prompt_eval_duration"]) / 1e6,
                         "total_ms": (time.monotonic() - started) * 1000, "prompt_tokens": r["prompt_eval_count"],
                         "gen_tokens": r["eval_count"]})
    ps = http(base + "/api/ps")
    summary = {"model": model, "iterations": iterations, "failures": failures, "gpu": sampler.peaks(),
               "loaded": [{"name": m["name"], "size_vram": m.get("size_vram"), "context": m.get("context_length")}
                          for m in ps.get("models", [])]}
    for key in ("gen_tok_s", "prompt_tok_s", "first_token_ms", "total_ms"):
        values = [run[key] for run in runs]
        if values:
            summary[key] = {"median": round(statistics.median(values), 2), "p95": round(p95(values), 2)}
    return summary


def routed(policy, agent=None):
    agent = agent or "live-" + uuid.uuid4().hex[:10]
    started = time.monotonic()
    http(ROUTER + "/v1/chat/completions",
         {"model": policy, "messages": [{"role": "user", "content": "Reply with the single word OK."}],
          "max_tokens": 16}, {"X-X3-Agent": agent})
    providers = [row["provider"] for row in http(ROUTER + "/v1/usage")["usage"] if row["agent"] == agent]
    return {"policy": policy, "agent": agent, "providers": providers, "ms": round((time.monotonic() - started) * 1000)}


def worker_of(provider):
    return "gtx" if provider.startswith("ollama_gtx") else "rtx" if provider.startswith("ollama") else "cloud"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=int, default=5)
    parser.add_argument("--no-failover", action="store_true")
    args = parser.parse_args()
    checks, evidence = [], {"started": dt.datetime.now(dt.timezone.utc).isoformat()}

    def check(name, ok, detail):
        checks.append({"check": name, "pass": bool(ok), "detail": detail})
        print(("PASS " if ok else "FAIL ") + name + " " + json.dumps(detail)[:200], flush=True)

    head = subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    dirty = bool(subprocess.run(["git", "-C", str(REPO), "status", "--porcelain"], capture_output=True,
                                text=True).stdout.strip())
    evidence.update({"commit": head, "dirty": dirty, "host": os.uname().nodename, "gpus": gpus()})

    health = http(ROUTER + "/health")
    check("router_health", health.get("status") == "ok", health)
    models = [m["id"] for m in http(ROUTER + "/v1/models")["data"]]
    check("router_models", {"x3-code", "x3-review", "x3-local"} <= set(models), models)

    evidence["bench"] = {}
    for label, base, model in (("rtx", WORKERS["rtx"], "qwen2.5-coder:7b"),
                               ("gtx", WORKERS["gtx"], "qwen2.5-coder:7b")):
        result = bench_worker(base, model, args.iterations)
        evidence["bench"][label] = result
        check(f"bench_{label}", result["failures"] == 0 and "gen_tok_s" in result,
              {k: result.get(k) for k in ("gen_tok_s", "prompt_tok_s", "first_token_ms")})

    # Both cards at once: one long request per worker, concurrently.
    with Sampler() as sampler, ThreadPoolExecutor(2) as pool:
        started = time.monotonic()
        jobs = [pool.submit(http, base + "/api/generate",
                            {"model": "qwen2.5-coder:7b", "prompt": PROMPT, "stream": False,
                             "options": {"num_predict": 256, "temperature": 0}}) for base in WORKERS.values()]
        results = [job.result() for job in jobs]
        wall = time.monotonic() - started
    tokens = sum(r["eval_count"] for r in results)
    evidence["bench"]["dual_concurrent"] = {"wall_s": round(wall, 2), "aggregate_gen_tok_s": round(tokens / wall, 2),
                                            "per_worker_gen_tok_s": [round(r["eval_count"] / r["eval_duration"] * 1e9, 2)
                                                                     for r in results], "gpu": sampler.peaks()}
    check("dual_concurrent", len(results) == 2, evidence["bench"]["dual_concurrent"])

    review = routed("x3-review")
    check("route_review_to_gtx", review["providers"] and worker_of(review["providers"][0]) == "gtx", review)
    code = routed("x3-code")
    check("route_code_to_rtx", code["providers"] and worker_of(code["providers"][0]) == "rtx", code)

    with ThreadPoolExecutor(4) as pool:
        spread = list(pool.map(lambda _: routed("x3-local"), range(4)))
    served = {worker_of(p) for r in spread for p in r["providers"]}
    check("router_concurrent_uses_both_gpus", served == {"rtx", "gtx"}, spread)

    if not args.no_failover:
        subprocess.run(["systemctl", "--user", "stop", "ollama-worker-b"], check=True)
        try:
            failed_over = routed("x3-review")
            check("gtx_down_review_fails_over_to_rtx",
                  failed_over["providers"] and worker_of(failed_over["providers"][0]) == "rtx", failed_over)
        finally:
            subprocess.run(["systemctl", "--user", "start", "ollama-worker-b"], check=True)
            for _ in range(60):
                try:
                    http(WORKERS["gtx"] + "/api/version")
                    break
                except Exception:  # noqa: BLE001
                    time.sleep(1)

    evidence["checks"] = checks
    evidence["finished"] = dt.datetime.now(dt.timezone.utc).isoformat()
    out = REPO / "audit-artifacts" / "gpu-ai" / (dt.datetime.now().strftime("%Y%m%dT%H%M%S") + ".json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(evidence, indent=2) + "\n")
    print("evidence:", out)
    raise SystemExit(0 if all(c["pass"] for c in checks) else 1)


if __name__ == "__main__":
    main()
