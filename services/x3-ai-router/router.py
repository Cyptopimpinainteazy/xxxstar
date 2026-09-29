#!/usr/bin/env python3
"""Small OpenAI-compatible, budgeted model router. Standard library only."""
import argparse
import datetime as dt
import json
import os
import sqlite3
import threading
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

CRITICAL = ("consensus", "finality", "settlement", "atomic", "cryptograph", "supply", "runtime upgrade", "slashing", "cross-vm")
MAX_BODY = 2_000_000


class Router:
    def __init__(self, config, db_path):
        self.config = config
        self.db = sqlite3.connect(db_path, check_same_thread=False)
        self.lock = threading.Lock()
        self.db.execute("CREATE TABLE IF NOT EXISTS usage (day TEXT, agent TEXT, provider TEXT, model TEXT, input_tokens INTEGER, output_tokens INTEGER, cost_usd REAL)")
        self.db.commit()

    def choose(self, request):
        text = " ".join(str(m.get("content", "")) for m in request.get("messages", [])).lower()
        tier = "critical" if any(term in text for term in CRITICAL) else "routine"
        return tier, self.config["routes"][tier]

    def remaining(self, agent, estimate):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            total = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=?", (day,)).fetchone()[0]
            spent = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=? AND agent=?", (day, agent)).fetchone()[0]
        return total + estimate <= self.config["daily_budget_usd"] and spent + estimate <= self.config["agent_daily_budget_usd"]

    def record(self, agent, provider, model, usage, cost):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            self.db.execute("INSERT INTO usage VALUES (?,?,?,?,?,?,?)", (day, agent, provider, model, usage.get("prompt_tokens", 0), usage.get("completion_tokens", 0), cost))
            self.db.commit()

    def stats(self):
        with self.lock:
            rows = self.db.execute("SELECT day,agent,provider,COUNT(*),ROUND(SUM(cost_usd),6) FROM usage GROUP BY day,agent,provider ORDER BY day DESC,agent").fetchall()
        return [{"day": d, "agent": a, "provider": p, "requests": n, "cost_usd": c} for d, a, p, n, c in rows]

    def complete(self, request, agent):
        tier, chain = self.choose(request)
        failures = []
        for name in chain:
            provider = self.config["providers"][name]
            if tier == "critical" and not provider.get("critical_allowed", False):
                continue
            model = provider["model"]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            # Reserve against an upper-bound configured for each request before making the call.
            estimate = (request.get("max_tokens", 4096) * price_out + self.config["max_input_tokens"] * price_in) / 1_000_000
            if not self.remaining(agent, estimate):
                return 429, {"error": {"message": "Daily budget exhausted", "type": "budget_exceeded"}}
            key = os.environ.get(provider.get("api_key_env", ""), "") if provider.get("api_key_env") else ""
            if provider.get("api_key_env") and not key:
                failures.append(name + ": credential unavailable")
                continue
            payload = dict(request)
            payload["model"] = model
            payload["stream"] = False
            headers = {"Content-Type": "application/json"}
            if key:
                headers["Authorization"] = "Bearer " + key
            url = provider["base_url"].rstrip("/") + "/chat/completions"
            try:
                call = urllib.request.Request(url, json.dumps(payload).encode(), headers, method="POST")
                with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                    result = json.load(response)
                usage = result.get("usage", {})
                cost = (usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000
                self.record(agent, name, model, usage, cost)
                return 200, result
            except (urllib.error.URLError, TimeoutError, ValueError) as exc:
                failures.append(name + ": " + type(exc).__name__)
        return 502, {"error": {"message": "No provider succeeded", "attempts": failures}}


def handler_for(router):
    class Handler(BaseHTTPRequestHandler):
        def reply(self, status, data):
            body = json.dumps(data).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def authorized(self):
            secret = os.environ.get("X3_ROUTER_TOKEN")
            return not secret or self.headers.get("Authorization") == "Bearer " + secret

        def do_GET(self):
            if not self.authorized():
                return self.reply(401, {"error": "Unauthorized"})
            if self.path == "/health":
                return self.reply(200, {"status": "ok"})
            if self.path == "/v1/usage":
                return self.reply(200, {"usage": router.stats()})
            if self.path == "/v1/models":
                return self.reply(200, {"object": "list", "data": [{"id": "x3-auto", "object": "model"}]})
            return self.reply(404, {"error": "Not found"})

        def do_POST(self):
            if not self.authorized():
                return self.reply(401, {"error": "Unauthorized"})
            if self.path != "/v1/chat/completions":
                return self.reply(404, {"error": "Not found"})
            try:
                size = int(self.headers.get("Content-Length", "0"))
                if size < 1 or size > MAX_BODY:
                    return self.reply(413, {"error": "Invalid request size"})
                data = json.loads(self.rfile.read(size))
                if not isinstance(data.get("messages"), list) or data.get("stream"):
                    return self.reply(400, {"error": "Expected messages and stream=false"})
                if not isinstance(data.get("max_tokens", 4096), int) or not 1 <= data.get("max_tokens", 4096) <= 32768:
                    return self.reply(400, {"error": "Invalid max_tokens"})
                agent = self.headers.get("X-X3-Agent", "default")[:80]
                status, result = router.complete(data, agent)
                return self.reply(status, result)
            except (ValueError, TypeError, KeyError):
                return self.reply(400, {"error": "Invalid request"})
    return Handler


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default=os.path.join(os.path.dirname(__file__), "config.json"))
    parser.add_argument("--db", default="x3-router.sqlite3")
    parser.add_argument("--port", type=int, default=11435)
    args = parser.parse_args()
    with open(args.config, encoding="utf-8") as source:
        config = json.load(source)
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler_for(Router(config, args.db)))
    server.serve_forever()


if __name__ == "__main__":
    main()
