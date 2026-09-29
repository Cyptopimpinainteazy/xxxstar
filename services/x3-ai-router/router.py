#!/usr/bin/env python3
"""Small OpenAI-compatible, budgeted model router. Standard library only."""
import argparse
import base64
import datetime as dt
import html
import json
import os
import sqlite3
import threading
import uuid
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
        self.db.execute("CREATE TABLE IF NOT EXISTS reservations (id TEXT PRIMARY KEY, day TEXT, agent TEXT, cost_usd REAL)")
        self.db.commit()

    def choose(self, request):
        text = " ".join(str(m.get("content", "")) for m in request.get("messages", [])).lower()
        tier = "critical" if any(term in text for term in CRITICAL) else "routine"
        return tier, self.config["routes"][tier]

    def reserve(self, agent, estimate):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            total = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=?", (day,)).fetchone()[0]
            total += self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM reservations WHERE day=?", (day,)).fetchone()[0]
            spent = self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM usage WHERE day=? AND agent=?", (day, agent)).fetchone()[0]
            spent += self.db.execute("SELECT COALESCE(SUM(cost_usd),0) FROM reservations WHERE day=? AND agent=?", (day, agent)).fetchone()[0]
            if total + estimate > self.config["daily_budget_usd"] or spent + estimate > self.config["agent_daily_budget_usd"]:
                self.db.commit()
                return None
            reservation = uuid.uuid4().hex
            self.db.execute("INSERT INTO reservations VALUES (?,?,?,?)", (reservation, day, agent, estimate))
            self.db.commit()
            return reservation

    def finish(self, reservation, agent, provider=None, model=None, usage=None, cost=0):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            self.db.execute("BEGIN IMMEDIATE")
            self.db.execute("DELETE FROM reservations WHERE id=?", (reservation,))
            if provider is not None:
                self.db.execute("INSERT INTO usage VALUES (?,?,?,?,?,?,?)", (day, agent, provider, model, usage.get("prompt_tokens", 0), usage.get("completion_tokens", 0), cost))
            self.db.commit()

    def stats(self):
        with self.lock:
            rows = self.db.execute("SELECT day,agent,provider,COUNT(*),ROUND(SUM(cost_usd),6) FROM usage GROUP BY day,agent,provider ORDER BY day DESC,agent").fetchall()
        return [{"day": d, "agent": a, "provider": p, "requests": n, "cost_usd": c} for d, a, p, n, c in rows]

    def snapshot(self):
        day = dt.datetime.now(dt.timezone.utc).date().isoformat()
        with self.lock:
            spent, requests, inputs, outputs = self.db.execute(
                "SELECT COALESCE(SUM(cost_usd),0),COUNT(*),COALESCE(SUM(input_tokens),0),COALESCE(SUM(output_tokens),0) FROM usage WHERE day=?", (day,)
            ).fetchone()
            reserved, inflight = self.db.execute(
                "SELECT COALESCE(SUM(cost_usd),0),COUNT(*) FROM reservations WHERE day=?", (day,)
            ).fetchone()
            rows = self.db.execute(
                "SELECT agent,provider,COUNT(*),SUM(cost_usd) FROM usage WHERE day=? GROUP BY agent,provider ORDER BY SUM(cost_usd) DESC", (day,)
            ).fetchall()
        return {"day": day, "spent_usd": spent, "reserved_usd": reserved, "requests": requests,
                "inflight": inflight, "input_tokens": inputs, "output_tokens": outputs,
                "daily_budget_usd": self.config["daily_budget_usd"],
                "breakdown": [{"agent": a, "provider": p, "requests": n, "cost_usd": c} for a, p, n, c in rows]}


    def complete(self, request, agent):
        # UTF-8 JSON bytes conservatively bound visible input tokens; reject
        # oversized requests instead of trusting a configured estimate.
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config["max_input_tokens"]:
            return 413, {"error": {"message": "Input exceeds configured budget bound"}}
        tier, chain = self.choose(request)
        failures = []
        for name in chain:
            provider = self.config["providers"][name]
            if tier == "critical" and not provider.get("critical_allowed", False):
                continue
            model = provider["model"]
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            if provider.get("api_key_env") and (price_in <= 0 or price_out <= 0):
                failures.append(name + ": configure positive token prices")
                continue
            # Reserve against an upper-bound configured for each request before making the call.
            estimate = (request.get("max_tokens", 4096) * price_out + self.config["max_input_tokens"] * price_in) / 1_000_000
            key = os.environ.get(provider.get("api_key_env", ""), "") if provider.get("api_key_env") else ""
            if provider.get("api_key_env") and not key:
                failures.append(name + ": credential unavailable")
                continue
            reservation = self.reserve(agent, estimate)
            if reservation is None:
                return 429, {"error": {"message": "Daily budget exhausted", "type": "budget_exceeded"}}
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
                if not isinstance(result, dict) or "choices" not in result:
                    raise ValueError("Provider response lacks choices")
                usage = result.get("usage", {})
                cost = (usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000 if usage else estimate
                self.finish(reservation, agent, name, model, usage, cost)
                return 200, result
            except (urllib.error.URLError, TimeoutError, ValueError) as exc:
                self.finish(reservation, agent)
                failures.append(name + ": " + type(exc).__name__)
            except Exception:
                self.finish(reservation, agent)
                raise
        return 502, {"error": {"message": "No provider succeeded", "attempts": failures}}

    def stream(self, request, agent, start, send):
        if len(json.dumps(request, ensure_ascii=False).encode("utf-8")) > self.config["max_input_tokens"]:
            return 413, {"error": "Input exceeds configured budget bound"}
        tier, chain = self.choose(request)
        failures = []
        for name in chain:
            provider = self.config["providers"][name]
            if tier == "critical" and not provider.get("critical_allowed", False):
                continue
            price_in = provider.get("input_usd_per_million", 0)
            price_out = provider.get("output_usd_per_million", 0)
            if provider.get("api_key_env") and (price_in <= 0 or price_out <= 0):
                failures.append(name + ": configure positive token prices")
                continue
            key = os.environ.get(provider.get("api_key_env", ""), "") if provider.get("api_key_env") else ""
            if provider.get("api_key_env") and not key:
                failures.append(name + ": credential unavailable")
                continue
            estimate = (request.get("max_tokens", 4096) * price_out + self.config["max_input_tokens"] * price_in) / 1_000_000
            reservation = self.reserve(agent, estimate)
            if reservation is None:
                return 429, {"error": "Daily budget exhausted"}
            payload = dict(request)
            payload["model"] = provider["model"]
            payload["stream"] = True
            payload["stream_options"] = {"include_usage": True}
            headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
            if key:
                headers["Authorization"] = "Bearer " + key
            emitted = False
            usage = None
            try:
                call = urllib.request.Request(provider["base_url"].rstrip("/") + "/chat/completions",
                                              json.dumps(payload).encode(), headers, method="POST")
                with urllib.request.urlopen(call, timeout=provider.get("timeout_seconds", 120)) as response:
                    if "text/event-stream" not in response.headers.get("Content-Type", ""):
                        raise ValueError("Provider did not return SSE")
                    for line in response:
                        if len(line) > 1_000_000:
                            raise ValueError("Oversized SSE line")
                        if not line.startswith(b"data: "):
                            if emitted:
                                send(line)
                            continue
                        data = line[6:].strip()
                        if data != b"[DONE]":
                            event = json.loads(data)
                            if event.get("usage"):
                                usage = event["usage"]
                        if not emitted:
                            start()
                            emitted = True
                        send(line)
                if not emitted:
                    raise ValueError("Empty SSE response")
                cost = ((usage.get("prompt_tokens", 0) * price_in + usage.get("completion_tokens", 0) * price_out) / 1_000_000) if usage else estimate
                self.finish(reservation, agent, name, provider["model"], usage or {}, cost)
                return None
            except (urllib.error.URLError, TimeoutError, ValueError, OSError) as exc:
                if emitted:
                    self.finish(reservation, agent, name, provider["model"], usage or {}, estimate)
                    return None  # A partial stream cannot be retried with another model.
                self.finish(reservation, agent)
                failures.append(name + ": " + type(exc).__name__)
            except Exception:
                self.finish(reservation, agent, name if emitted else None, provider["model"], usage or {}, estimate if emitted else 0)
                raise
        return 502, {"error": {"message": "No provider succeeded", "attempts": failures}}


def dashboard(snapshot):
    rows = "".join("<tr>" + "".join(f"<td>{html.escape(str(item[key]))}</td>" for key in ("agent", "provider", "requests", "cost_usd")) + "</tr>"
                   for item in snapshot["breakdown"])
    cells = "".join(f"<li><strong>{html.escape(key.replace('_', ' ').title())}:</strong> {html.escape(str(value))}</li>"
                    for key, value in snapshot.items() if key != "breakdown")
    return ("<!doctype html><html lang='en'><meta charset='utf-8'><meta http-equiv='refresh' content='15'>"
            "<meta name='viewport' content='width=device-width,initial-scale=1'><title>X3 AI router</title>"
            "<style>body{font:16px system-ui;background:#111827;color:#f9fafb;max-width:960px;margin:3rem auto;padding:1rem}"
            "table{border-collapse:collapse;width:100%}td,th{padding:.7rem;border-bottom:1px solid #4b5563;text-align:left}"
            "li{margin:.5rem 0}a{color:#fb923c}</style><h1>X3 AI router</h1><ul>" + cells +
            "</ul><h2>Today by agent and provider</h2><table><thead><tr><th>Agent</th><th>Provider</th>"
            "<th>Requests</th><th>USD</th></tr></thead><tbody>" + rows + "</tbody></table></html>")


def metrics(snapshot):
    fields = ("spent_usd", "reserved_usd", "requests", "inflight", "input_tokens", "output_tokens", "daily_budget_usd")
    return "".join(f"x3_ai_router_{name} {snapshot[name]}\n" for name in fields)


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
            auth = self.headers.get("Authorization", "")
            return not secret or auth in ("Bearer " + secret, "Basic " + base64.b64encode(("x3:" + secret).encode()).decode())

        def raw(self, status, body, content_type):
            data = body.encode()
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            if not self.authorized():
                self.send_response(401)
                self.send_header("WWW-Authenticate", 'Basic realm="X3 AI router"')
                self.end_headers()
                return
            if self.path == "/health":
                return self.reply(200, {"status": "ok"})
            if self.path == "/v1/usage":
                return self.reply(200, {"usage": router.stats()})
            if self.path == "/v1/dashboard":
                return self.raw(200, dashboard(router.snapshot()), "text/html; charset=utf-8")
            if self.path == "/metrics":
                return self.raw(200, metrics(router.snapshot()), "text/plain; version=0.0.4; charset=utf-8")
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
                if not isinstance(data.get("messages"), list) or not isinstance(data.get("stream", False), bool):
                    return self.reply(400, {"error": "Expected messages and boolean stream"})
                if not isinstance(data.get("max_tokens", 4096), int) or not 1 <= data.get("max_tokens", 4096) <= 32768:
                    return self.reply(400, {"error": "Invalid max_tokens"})
                agent = self.headers.get("X-X3-Agent", "default")[:80]
                if data.get("stream"):
                    def start():
                        self.send_response(200)
                        self.send_header("Content-Type", "text/event-stream")
                        self.send_header("Cache-Control", "no-cache")
                        self.send_header("Connection", "close")
                        self.end_headers()

                    def send(chunk):
                        self.wfile.write(chunk)
                        self.wfile.flush()

                    outcome = router.stream(data, agent, start, send)
                    if outcome is not None:
                        return self.reply(*outcome)
                    self.close_connection = True
                    return
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
