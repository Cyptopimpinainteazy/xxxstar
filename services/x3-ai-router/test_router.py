import contextlib
import importlib.util
import itertools
import io
import json
import os
import tempfile
import threading
import time
import unittest
import unittest.mock
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

spec = importlib.util.spec_from_file_location("x3_router", Path(__file__).with_name("router.py"))
router_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(router_module)


class Provider(BaseHTTPRequestHandler):
    requests = []

    def do_POST(self):
        data = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.requests.append(data)
        if data.get("stream"):
            chunks = [b'data: {"choices":[{"delta":{"content":"ok"}}]}\n\n',
                      b'data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":5}}\n\n',
                      b'data: [DONE]\n\n']
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for chunk in chunks:
                self.wfile.write(chunk)
                self.wfile.flush()
            return
        body = json.dumps({"choices": [{"message": {"role": "assistant", "content": "ok"}}], "usage": {"prompt_tokens": 10, "completion_tokens": 5}}).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


class RouterTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        Provider.requests = []
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), Provider)
        self.upstream_thread = threading.Thread(target=self.upstream.serve_forever, daemon=True)
        self.upstream_thread.start()
        self.config = {
            "daily_budget_usd": 0.01, "agent_daily_budget_usd": 0.01,
            "max_input_tokens": 1000, "max_request_bytes": 1000, "routes": {"routine": ["down", "up"], "critical": ["up"]},
            "providers": {
                "down": {"base_url": "http://127.0.0.1:1/v1", "model": "down", "input_usd_per_million": 1,
                         "output_usd_per_million": 1, "supports_tools": True},
                "up": {"base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "up", "critical_allowed": True,
                       "input_usd_per_million": 1, "output_usd_per_million": 1, "supports_tools": True}
            }
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.tmp.cleanup()

    def test_fallback_and_accounting(self):
        status, response = self.router.complete({"messages": [{"role": "user", "content": "format this"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 200)
        self.assertEqual(response["choices"][0]["message"]["content"], "ok")
        self.assertEqual(Provider.requests[0]["model"], "up")
        self.assertEqual(self.router.stats()[0]["cost_usd"], 0.000015)

    def test_critical_skips_unapproved_provider(self):
        self.assertEqual(self.router.choose({"messages": [{"content": "atomic settlement"}]})[0], "critical")
        status, _ = self.router.complete({"messages": [{"content": "atomic settlement"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 200)
        self.assertEqual(len(Provider.requests), 1)

    def test_concurrent_reservations_and_input_bound(self):
        self.config["daily_budget_usd"] = 0.00101
        self.config["agent_daily_budget_usd"] = 0.00101
        with ThreadPoolExecutor(max_workers=8) as pool:
            ids = list(pool.map(lambda _: self.router.reserve("alice", 0.001), range(8)))
        self.assertEqual(sum(x is not None for x in ids), 1)
        self.router.finish(next(x for x in ids if x), "alice")
        self.assertIsNotNone(self.router.reserve("alice", 0.001))
        status, _ = self.router.complete({"messages": [{"content": "z" * 1100}]}, "alice")
        self.assertEqual(status, 413)

    def test_stream_fallback_and_usage(self):
        chunks = []
        started = []
        result = self.router.stream({"messages": [{"role": "user", "content": "format"}], "max_tokens": 10},
                                    "alice", lambda: started.append(True), chunks.append)
        self.assertIsNone(result)
        self.assertEqual(started, [True])
        self.assertIn(b"data: [DONE]", b"".join(chunks))
        self.assertTrue(Provider.requests[0]["stream_options"]["include_usage"])
        self.assertEqual(self.router.stats()[0]["cost_usd"], 0.000015)

    def test_http_stream_endpoint(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            data = json.dumps({"model": "x3-auto", "messages": [{"role": "user", "content": "format"}],
                               "max_tokens": 10, "stream": True}).encode()
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                                             data, {"Content-Type": "application/json", "X-X3-Agent": "alice"})
            with urllib.request.urlopen(request) as response:
                self.assertEqual(response.headers["Content-Type"], "text/event-stream")
                self.assertIn(b"data: [DONE]", response.read())
            self.assertEqual(self.router.stats()[0]["cost_usd"], 0.000015)
        finally:
            server.shutdown()
            server.server_close()

    def test_dashboard_metrics_and_auth(self):
        self.router.finish(self.router.reserve("<script>", 0.001), "<script>", "up", "up",
                           {"prompt_tokens": 10, "completion_tokens": 5}, 0.000015)
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        previous = os.environ.get("X3_ROUTER_TOKEN")
        os.environ["X3_ROUTER_TOKEN"] = "test-secret"
        try:
            url = f"http://127.0.0.1:{server.server_port}"
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(url + "/v1/dashboard")
            self.assertEqual(rejected.exception.code, 401)
            headers = {"Authorization": "Bearer test-secret"}
            with urllib.request.urlopen(urllib.request.Request(url + "/v1/dashboard", headers=headers)) as response:
                page = response.read().decode()
                self.assertIn("&lt;script&gt;", page)
                self.assertNotIn("<script>", page)
            with urllib.request.urlopen(urllib.request.Request(url + "/metrics", headers=headers)) as response:
                self.assertIn("x3_ai_router_spent_usd 1.5e-05", response.read().decode())
        finally:
            if previous is None:
                os.environ.pop("X3_ROUTER_TOKEN", None)
            else:
                os.environ["X3_ROUTER_TOKEN"] = previous
            server.shutdown()
            server.server_close()

    def test_paid_price_freshness_and_free_id(self):
        paid = {"api_key_env": "TEST_KEY", "model": "paid", "input_usd_per_million": 1,
                "output_usd_per_million": 2, "pricing_checked_on": "2020-01-01"}
        self.assertEqual(router_module.pricing_error(paid), "refresh provider pricing")
        free = {"api_key_env": "TEST_KEY", "model": "nvidia/example:free", "free_model": True,
                "pricing_checked_on": router_module.dt.datetime.now(router_module.dt.timezone.utc).date().isoformat()}
        self.assertIsNone(router_module.pricing_error(free))
        free["model"] = "nvidia/example"
        self.assertIn("must use a :free ID", router_module.pricing_error(free))

    def test_task_binding_cost_and_outcome(self):
        revision = "a" * 40
        self.router.begin_task("task-1", "alice", revision, "router")
        status, _ = self.router.complete({"messages": [{"content": "format"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 200)
        evidence = {"task_id": "task-1", "revision": revision, "scope": "router",
                    "checks": [{"name": "router-tests", "exit_code": 0, "output_sha256": "b" * 64}]}
        with self.assertRaises(ValueError):
            self.router.task_outcome(evidence)  # Cannot finalize in-flight work.
        self.router.end_task_request(123)
        self.assertEqual(self.router.task_stats()[0]["cost_usd"], 0.000015)
        self.assertEqual(self.router.task_stats()[0]["elapsed_ms"], 123)
        with self.assertRaises(ValueError):
            self.router.task_outcome(dict(evidence, revision="c" * 40))
        self.assertEqual(self.router.task_outcome(evidence)["outcome"], "checks_passed")
        self.assertEqual(self.router.learning_stats()[0]["pass_rate"], 1)
        self.assertEqual(self.router.learning_stats()[0]["cost_per_passed_task_usd"], 0.000015)
        with self.assertRaises(ValueError):
            self.router.begin_task("task-1", "alice", revision, "router")

    def test_builder_cannot_submit_verification(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        previous = {k: os.environ.get(k) for k in ("X3_ROUTER_TOKEN", "X3_VERIFIER_TOKEN")}
        os.environ["X3_ROUTER_TOKEN"], os.environ["X3_VERIFIER_TOKEN"] = "builder", "verifier"
        try:
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/tasks/outcome", b"{}",
                                             {"Authorization": "Bearer builder"})
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(request)
            self.assertEqual(rejected.exception.code, 403)
        finally:
            for k, value in previous.items():
                if value is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = value
            server.shutdown()
            server.server_close()

    def test_free_provider_requires_opt_in(self):
        self.config["routes"]["routine"] = ["free", "up"]
        self.config["providers"]["free"] = {"base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1",
            "model": "nvidia/example:free", "free_model": True, "enabled_env": "X3_ENABLE_FREE_CLOUD_TEST",
            "api_key_env": "OPENROUTER_TEST_KEY", "pricing_checked_on": router_module.dt.datetime.now(router_module.dt.timezone.utc).date().isoformat()}
        request = {"messages": [{"role": "user", "content": "format this"}], "max_tokens": 10}
        os.environ.pop("X3_ENABLE_FREE_CLOUD_TEST", None)
        self.router.complete(request, "alice")
        self.assertEqual(Provider.requests[-1]["model"], "up")
        os.environ["X3_ENABLE_FREE_CLOUD_TEST"] = "1"
        os.environ["OPENROUTER_TEST_KEY"] = "test"
        try:
            self.router.complete(request, "alice")
            self.assertEqual(Provider.requests[-1]["model"], "nvidia/example:free")
        finally:
            os.environ.pop("X3_ENABLE_FREE_CLOUD_TEST", None)
            os.environ.pop("OPENROUTER_TEST_KEY", None)

    # ── Budget validation ────────────────────────────────────────────────

    def test_multiple_completions_are_refused(self):
        """`n` multiplies the bill; the reservation only covers one completion."""
        status, body = self.router.complete({"messages": [{"content": "format"}], "max_tokens": 10, "n": 2}, "alice")
        self.assertEqual(status, 400)
        self.assertIn("n must be 1", body["error"]["message"])
        self.assertEqual(Provider.requests, [], "a refused request must not reach a provider")

        status, _ = self.router.complete({"messages": [{"content": "format"}], "max_tokens": 10, "n": 1}, "alice")
        self.assertEqual(status, 200)

    def test_max_completion_tokens_is_bounded_and_reserved(self):
        status, body = self.router.complete({"messages": [{"content": "format"}], "max_completion_tokens": 999999}, "alice")
        self.assertEqual(status, 400)
        self.assertIn("max_completion_tokens", body["error"]["message"])

        # 32768 output tokens at $1/M plus the 1000-byte input bound is more than
        # the 0.01 daily budget. The estimate used to read only `max_tokens` and
        # fall back to 4096, so this request was served and billed afterwards.
        status, _ = self.router.complete({"messages": [{"content": "format"}], "max_completion_tokens": 32768}, "alice")
        self.assertEqual(status, 429)
        self.assertEqual(Provider.requests, [])

    def test_stream_refuses_multiple_completions(self):
        chunks = []
        status, body = self.router.stream({"messages": [{"content": "format"}], "max_tokens": 10, "n": 4},
                                          "alice", lambda: None, chunks.append)
        self.assertEqual(status, 400)
        self.assertIn("n must be 1", body["error"])
        self.assertEqual(chunks, [])

    # ── Crash recovery ───────────────────────────────────────────────────

    def test_orphaned_reservations_are_reclaimed_on_startup(self):
        day = router_module.dt.datetime.now(router_module.dt.timezone.utc).date().isoformat()
        self.router.db.execute("INSERT INTO reservations VALUES (?,?,?,?,?)",
                               ("orphan", day, "alice", 0.009, router_module.time.time() - 100_000))
        self.router.db.commit()
        self.assertGreater(self.router.snapshot()["reserved_usd"], 0)

        restarted = router_module.Router(self.config, self.tmp.name + "/usage.db")

        self.assertEqual(restarted.snapshot()["reserved_usd"], 0)
        self.assertEqual(restarted.reconciled_orphans, 1)
        self.assertIsNotNone(restarted.reserve("alice", 0.001), "the reclaimed budget must be usable again")

    def test_a_live_reservation_is_not_reclaimed(self):
        self.assertIsNotNone(self.router.reserve("alice", 0.009))
        restarted = router_module.Router(self.config, self.tmp.name + "/usage.db")
        self.assertEqual(restarted.reconciled_orphans, 0)
        self.assertGreater(restarted.snapshot()["reserved_usd"], 0)

    # ── Provider cooldowns ───────────────────────────────────────────────

    def test_budget_exhaustion_falls_through_to_a_free_provider(self):
        """Running out of money must stop the spending, not stop the work."""
        self.config["providers"]["local"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1",
            "model": "local", "critical_allowed": True}
        self.config["routes"]["critical"] = ["up"]
        self.config["budget_fallback"] = ["local"]
        self.config["daily_budget_usd"] = 0.000001
        self.config["agent_daily_budget_usd"] = 0.000001

        status, response = self.router.complete(
            {"messages": [{"content": "review this atomic settlement path"}], "max_tokens": 10}, "alice")

        self.assertEqual(status, 200, "the fallback provider must still answer")
        self.assertEqual(response["choices"][0]["message"]["content"], "ok")
        self.assertEqual(Provider.requests[-1]["model"], "local")

    def test_budget_exhaustion_still_refuses_paid_providers(self):
        self.config["routes"]["critical"] = ["up"]
        self.config["budget_fallback"] = []
        self.config["daily_budget_usd"] = 0.000001
        self.config["agent_daily_budget_usd"] = 0.000001

        status, body = self.router.complete(
            {"messages": [{"content": "review this atomic settlement path"}], "max_tokens": 10}, "alice")

        self.assertEqual(status, 429)
        self.assertEqual(body["error"]["type"], "budget_exceeded")
        self.assertEqual(Provider.requests, [], "an over-budget paid provider must not be called")

    def test_critical_request_never_leaks_to_a_free_cloud_provider(self):
        """A critical request must not leave this machine through the fallback.

        `budget_fallback` lists the free OpenRouter models, and their operator
        logs prompts. Critical requests carry consensus and settlement code, so
        a paid provider being unavailable or over budget has to fail the request
        closed rather than quietly downgrade it onto a third party.
        """
        checked = router_module.dt.datetime.now(router_module.dt.timezone.utc).date().isoformat()
        self.config["providers"]["paid"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "paid",
            "api_key_env": "X3_UNSET_TEST_KEY", "input_usd_per_million": 1, "output_usd_per_million": 1,
            "pricing_checked_on": checked}
        self.config["providers"]["free"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "nvidia/example:free",
            "free_model": True, "api_key_env": "OPENROUTER_TEST_KEY", "enabled_env": "X3_ENABLE_FREE_CLOUD_TEST",
            "pricing_checked_on": checked}
        self.config["routes"]["critical"] = ["paid"]
        self.config["budget_fallback"] = ["free"]
        os.environ["X3_ENABLE_FREE_CLOUD_TEST"] = "1"
        os.environ["OPENROUTER_TEST_KEY"] = "test"
        os.environ.pop("X3_UNSET_TEST_KEY", None)
        request = {"messages": [{"content": "review this atomic settlement path"}], "max_tokens": 10}
        chunks, started = [], []
        try:
            status, body = self.router.complete(request, "alice")
            streamed = self.router.stream(request, "alice", lambda: started.append(True), chunks.append)
        finally:
            os.environ.pop("X3_ENABLE_FREE_CLOUD_TEST", None)
            os.environ.pop("OPENROUTER_TEST_KEY", None)
        self.assertNotEqual(status, 200)
        self.assertEqual(Provider.requests, [], "critical code must never reach a free cloud model")
        self.assertIn("not cleared for critical work", " ".join(body["error"]["attempts"]))
        # The refusal has to hold on both entry points; a fix that lands in one
        # and not the other is the failure mode this router already shipped once.
        self.assertIsNotNone(streamed, "the streaming path must fail closed too")
        self.assertNotEqual(streamed[0], 200)
        self.assertIn("not cleared for critical work", " ".join(streamed[1]["error"]["attempts"]))
        self.assertEqual((chunks, started), ([], []), "nothing may be streamed to the client")

    def test_critical_falls_back_to_a_local_model_that_cannot_leak(self):
        """Refusing third parties must not stop a critical request working locally."""
        checked = router_module.dt.datetime.now(router_module.dt.timezone.utc).date().isoformat()
        self.config["providers"]["paid"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "paid",
            "api_key_env": "X3_UNSET_TEST_KEY", "input_usd_per_million": 1, "output_usd_per_million": 1,
            "pricing_checked_on": checked}
        self.config["providers"]["local"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "local"}
        self.config["routes"]["critical"] = ["paid"]
        self.config["budget_fallback"] = ["local"]
        os.environ.pop("X3_UNSET_TEST_KEY", None)
        status, _ = self.router.complete(
            {"messages": [{"content": "review this atomic settlement path"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 200)
        self.assertEqual(Provider.requests[-1]["model"], "local",
                         "a model on this machine may still answer a critical request")

    def test_failing_provider_is_cooled_down_and_skipped(self):
        status, _ = self.router.complete({"messages": [{"content": "format"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 200, "the second provider still serves the request")
        self.assertGreater(self.router.provider_cooldown("down"), 0)
        health = {row["provider"]: row for row in self.router.provider_health()}
        self.assertEqual(health["down"]["failures"], 1)

        # A route holding only the cooled-down provider fails without calling it.
        self.config["routes"]["routine"] = ["down"]
        status, body = self.router.complete({"messages": [{"content": "format"}], "max_tokens": 10}, "alice")
        self.assertEqual(status, 502)
        self.assertIn("cooling down", " ".join(body["error"]["attempts"]))
        self.assertEqual(len(Provider.requests), 1, "the cooled-down endpoint must not be retried")

    def test_retry_after_is_honoured_and_success_clears_it(self):
        self.router.note_provider_failure("down", "HTTP 429", 120)
        self.assertGreater(self.router.provider_cooldown("down"), 110)
        self.router.note_provider_success("down")
        self.assertEqual(self.router.provider_cooldown("down"), 0)

    def test_cooldown_backs_off_across_consecutive_failures(self):
        self.router.note_provider_failure("down", "timeout")
        first = self.router.provider_cooldown("down")
        self.router.note_provider_failure("down", "timeout")
        second = self.router.provider_cooldown("down")
        self.assertGreater(second, first)

    # ── Client compatibility ─────────────────────────────────────────────

    def test_tool_call_requests_are_forwarded_unchanged(self):
        tools = [{"type": "function", "function": {"name": "read_file", "parameters": {"type": "object"}}}]
        status, _ = self.router.complete({"messages": [{"role": "user", "content": "read a file"}],
                                          "max_tokens": 10, "tools": tools, "tool_choice": "auto"}, "alice")
        self.assertEqual(status, 200)
        self.assertEqual(Provider.requests[0]["tools"], tools)
        self.assertEqual(Provider.requests[0]["tool_choice"], "auto")
        self.assertEqual(Provider.requests[0]["model"], "up", "the router picks the model, the client's is ignored")

    def test_models_and_unsupported_endpoints(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            url = f"http://127.0.0.1:{server.server_port}"
            with urllib.request.urlopen(url + "/v1/models/x3-auto") as response:
                self.assertEqual(json.loads(response.read())["id"], "x3-auto")
            with self.assertRaises(urllib.error.HTTPError) as unknown:
                urllib.request.urlopen(url + "/v1/models/gpt-9")
            self.assertEqual(unknown.exception.code, 404)

            # A client that reaches for the Responses API must be told plainly
            # rather than handed a 404 that looks like a wrong base URL.
            request = urllib.request.Request(url + "/v1/responses", b"{}", {"Content-Type": "application/json"})
            with self.assertRaises(urllib.error.HTTPError) as malformed:
                urllib.request.urlopen(request)
            self.assertEqual(malformed.exception.code, 400, "the Responses endpoint exists and validates input")

            embeddings = urllib.request.Request(url + "/v1/embeddings", b"{}", {"Content-Type": "application/json"})
            with self.assertRaises(urllib.error.HTTPError) as unsupported:
                urllib.request.urlopen(embeddings)
            self.assertEqual(unsupported.exception.code, 501)
            self.assertIn("Chat Completions", unsupported.exception.read().decode())

            bad = json.dumps({"model": "x3-auto", "messages": [{"role": "user", "content": "hi"}], "max_tokens": 10, "n": 3}).encode()
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(urllib.request.Request(url + "/v1/chat/completions", bad, {"Content-Type": "application/json"}))
            self.assertEqual(rejected.exception.code, 400)
        finally:
            server.shutdown()
            server.server_close()


    # ── Responses API (the wire protocol Codex actually speaks) ──────────

    def responses_body(self, **overrides):
        body = {"model": "x3-auto", "stream": True, "instructions": "be brief",
                "input": [{"type": "message", "role": "user",
                           "content": [{"type": "input_text", "text": "hi"}]}]}
        body.update(overrides)
        return body

    def serve(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        return server

    def test_responses_request_translation(self):
        chat = router_module.responses_request_to_chat({
            "instructions": "sys",
            "input": [
                {"type": "message", "role": "developer", "content": [{"type": "input_text", "text": "dev"}]},
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]},
                {"type": "function_call", "call_id": "c1", "name": "exec_command",
                 "arguments": "{\"cmd\":\"ls\"}"},
                {"type": "function_call_output", "call_id": "c1", "output": "file.txt"},
            ],
            "tools": [
                {"type": "function", "name": "exec_command", "description": "run it",
                 "parameters": {"type": "object", "properties": {}}},
                {"type": "namespace", "name": "ns", "tools": [
                    {"type": "function", "name": "inner", "parameters": {"type": "object"}}]},
                {"type": "web_search"},
            ],
            "tool_choice": "auto", "max_output_tokens": 64,
        })
        self.assertEqual([m["role"] for m in chat["messages"]],
                         ["system", "system", "user", "assistant", "tool"])
        self.assertEqual(chat["messages"][3]["tool_calls"][0]["function"]["name"], "exec_command")
        self.assertEqual(chat["messages"][4]["tool_call_id"], "c1")
        self.assertEqual([t["function"]["name"] for t in chat["tools"]], ["exec_command", "inner"],
                         "namespaced tools flatten; web_search has no chat equivalent")
        self.assertEqual(chat["tool_choice"], "auto")
        self.assertEqual(chat["max_tokens"], 64)

    def test_chat_message_maps_to_responses_output(self):
        output = router_module.chat_message_to_response_output(
            {"content": "hello", "tool_calls": [{"id": "c1", "function": {"name": "f", "arguments": "{}"}}]},
            "resp_")
        self.assertEqual([item["type"] for item in output], ["message", "function_call"])
        self.assertEqual(output[0]["content"][0]["text"], "hello")
        self.assertEqual(output[1]["call_id"], "c1")
        self.assertEqual(output[1]["name"], "f")

    def test_responses_endpoint_non_streaming(self):
        server = self.serve()
        try:
            data = json.dumps(self.responses_body(stream=False, tools=[
                {"type": "function", "name": "exec_command", "description": "run it",
                 "parameters": {"type": "object", "properties": {}}}])).encode()
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                             data, {"Content-Type": "application/json", "X-X3-Agent": "codex"})
            with urllib.request.urlopen(request) as response:
                body = json.loads(response.read())
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(body["object"], "response")
        self.assertEqual(body["status"], "completed")
        self.assertEqual(body["output"][0]["type"], "message")
        self.assertEqual(body["output"][0]["content"][0]["text"], "ok")
        self.assertIn("usage", body)
        self.assertEqual(Provider.requests[0]["messages"][0], {"role": "system", "content": "be brief"})
        self.assertEqual(Provider.requests[0]["tools"][0]["function"]["name"], "exec_command")
        self.assertEqual(self.router.stats()[0]["provider"], "up", "budget accounting still applies")

    def test_responses_endpoint_omitted_stream_returns_a_json_envelope(self):
        # The Responses API defaults `stream` to false. A client that omits it
        # must get one JSON envelope, never an SSE body it cannot parse.
        server = self.serve()
        try:
            body = self.responses_body()
            del body["stream"]
            data = json.dumps(body).encode()
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                             data, {"Content-Type": "application/json", "X-X3-Agent": "codex"})
            with urllib.request.urlopen(request) as response:
                content_type = response.headers.get("Content-Type", "")
                payload = json.loads(response.read())
        finally:
            server.shutdown()
            server.server_close()

        self.assertIn("application/json", content_type)
        self.assertEqual(payload["object"], "response")
        self.assertEqual(payload["status"], "completed")
        self.assertEqual(payload["output"][0]["content"][0]["text"], "ok")

    def test_responses_endpoint_rejects_a_non_boolean_stream(self):
        server = self.serve()
        try:
            data = json.dumps(self.responses_body(stream="yes")).encode()
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                             data, {"Content-Type": "application/json", "X-X3-Agent": "codex"})
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(request)
            self.assertEqual(rejected.exception.code, 400)
        finally:
            server.shutdown()
            server.server_close()

    def test_responses_endpoint_streams_the_responses_event_sequence(self):
        server = self.serve()
        try:
            data = json.dumps(self.responses_body()).encode()
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                             data, {"Content-Type": "application/json", "X-X3-Agent": "codex"})
            with urllib.request.urlopen(request) as response:
                raw = response.read().decode()
        finally:
            server.shutdown()
            server.server_close()

        order = ["response.created", "response.output_item.added", "response.output_text.delta",
                 "response.output_text.done", "response.output_item.done", "response.completed"]
        positions = [raw.find('"type": "' + name + '"') for name in order]
        self.assertNotIn(-1, positions, f"missing event; got {raw[:400]}")
        self.assertEqual(positions, sorted(positions), "events must arrive in the order Codex expects")
        self.assertIn('"text": "ok"', raw)


class ScriptedProvider(BaseHTTPRequestHandler):
    """Upstream that behaves exactly as one test tells it to.

    Real providers differ in ways the default `Provider` cannot express: one
    sends fragments across many chunks, one dies mid-answer, one holds the
    socket open after `[DONE]`. Each script is a small function over the
    handler so the wire bytes stay visible in the test.
    """

    script = None
    requests = []
    daemon_threads = True

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        ScriptedProvider.requests.append(body)
        ScriptedProvider.script(self, body)

    def log_message(self, *_):
        pass


def sse(handler, chunks):
    handler.send_response(200)
    handler.send_header("Content-Type", "text/event-stream")
    handler.end_headers()
    for chunk in chunks:
        handler.wfile.write(chunk)
        handler.wfile.flush()


class ProtocolTests(unittest.TestCase):
    """Responses <-> Chat translation, with a controllable upstream."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        ScriptedProvider.requests = []
        ScriptedProvider.script = None
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), ScriptedProvider)
        self.upstream.daemon_threads = True
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 100000,
            "max_request_bytes": 2000000, "routes": {"routine": ["up"], "critical": ["up"]},
            "providers": {"up": {"base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1",
                                 "model": "up-model", "critical_allowed": True,
                                 "input_usd_per_million": 1, "output_usd_per_million": 1,
                                 "supports_tools": True}},
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.tmp.cleanup()

    def serve(self, timeout=30):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        return server

    def post(self, server, body, timeout=30):
        request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                         json.dumps(body).encode(),
                                         {"Content-Type": "application/json", "X-X3-Agent": "codex"})
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                return response.status, response.read().decode()
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read().decode()

    def events(self, raw):
        return [json.loads(line[6:]) for line in raw.splitlines()
                if line.startswith("data: ") and line[6:].strip() not in ("", "[DONE]")]

    def script_json(self, content="ok"):
        """Upstream answers once with a plain Chat Completions body."""
        def respond(handler, body):
            payload = json.dumps({"model": "up-model",
                                  "choices": [{"message": {"role": "assistant", "content": content}}],
                                  "usage": {"prompt_tokens": 3, "completion_tokens": 1}}).encode()
            handler.send_response(200)
            handler.send_header("Content-Type", "application/json")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)
        ScriptedProvider.script = respond

    def responses_body(self, **overrides):
        body = {"model": "x3-auto", "stream": True, "instructions": "be brief",
                "input": [{"type": "message", "role": "user",
                           "content": [{"type": "input_text", "text": "hi"}]}]}
        body.update(overrides)
        return body

    # ── Named tool_choice ────────────────────────────────────────────────

    def test_named_tool_choice_reaches_the_provider_as_a_named_choice(self):
        """The object form used to be dropped, so the model picked the tool."""
        chat = router_module.responses_request_to_chat({
            "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
            "tools": [{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}],
            "tool_choice": {"type": "function", "name": "exec_command"}})

        self.assertEqual(chat["tool_choice"],
                         {"type": "function", "function": {"name": "exec_command"}})

    def test_string_tool_choices_still_pass_through(self):
        for choice in ("auto", "none", "required"):
            chat = router_module.responses_request_to_chat({
                "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
                "tools": [{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}],
                "tool_choice": choice})
            self.assertEqual(chat["tool_choice"], choice)

    def test_unknown_tool_choice_fails_closed(self):
        with self.assertRaises(router_module.UnsupportedFeature):
            router_module.responses_request_to_chat({
                "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
                "tools": [{"type": "function", "name": "exec_command", "parameters": {"type": "object"}}],
                "tool_choice": {"type": "mystery", "name": "whatever"}})

    def test_forced_tool_choice_disables_thinking_for_a_provider_that_requires_it(self):
        """DeepSeek answers HTTP 400 for a forced tool_choice in thinking mode."""
        provider = dict(self.config["providers"]["up"], thinking={
            "parameter": "thinking", "disabled_value": {"type": "disabled"},
            "disable_when_tool_choice_forced": True})
        forced = {"messages": [{"role": "user", "content": "go"}],
                  "tools": [{"type": "function", "function": {"name": "f"}}],
                  "tool_choice": {"type": "function", "function": {"name": "f"}},
                  "max_tokens": 8}
        auto = dict(forced, tool_choice="auto")

        forced_payload = self.router.provider_payload(forced, provider, True)
        auto_payload = self.router.provider_payload(auto, provider, True)

        self.assertEqual(forced_payload["thinking"], {"type": "disabled"})
        self.assertNotIn("thinking", auto_payload,
                         "a normal turn must keep the provider's own reasoning default")

    def test_named_tool_choice_through_the_responses_endpoint(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1",'
            b'"type":"function","function":{"name":"exec_command","arguments":"{\\"cmd\\": "}}]}}]}\n\n',
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,'
            b'"function":{"arguments":"\\"ls\\"}"}}]},"finish_reason":"tool_calls"}]}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body(tools=[
                {"type": "function", "name": "exec_command",
                 "parameters": {"type": "object", "properties": {"cmd": {"type": "string"}}}}],
                tool_choice={"type": "function", "name": "exec_command"}))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertEqual(ScriptedProvider.requests[0]["tool_choice"],
                         {"type": "function", "function": {"name": "exec_command"}})

    # ── Streamed tool calls ──────────────────────────────────────────────

    def test_streamed_tool_call_fragments_become_one_genuine_function_call(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc",'
            b'"type":"function","function":{"name":"exec_command","arguments":"{\\"cmd\\":"}}]}}]}\n\n',
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,'
            b'"function":{"arguments":"\\"ls"}}]}}]}\n\n',
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,'
            b'"function":{"arguments":" -la\\"}"}}]},"finish_reason":"tool_calls"}]}\n\n',
            b'data: {"model":"up-model","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":4}}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body(tools=[
                {"type": "function", "name": "exec_command",
                 "parameters": {"type": "object", "properties": {"cmd": {"type": "string"}}}}]))
        finally:
            server.shutdown()
            server.server_close()

        events = self.events(raw)
        added = [e for e in events if e["type"] == "response.output_item.added"]
        done = [e for e in events if e["type"] == "response.output_item.done"]
        deltas = [e["delta"] for e in events if e["type"] == "response.function_call_arguments.delta"]
        final = events[-1]

        self.assertEqual(status, 200)
        self.assertEqual(added[0]["item"]["type"], "function_call")
        self.assertEqual(added[0]["item"]["call_id"], "call_abc")
        self.assertEqual(deltas, ['{"cmd":', '"ls', ' -la"}'],
                         "every argument fragment must reach the client unchanged")
        self.assertEqual(done[0]["item"]["type"], "function_call")
        self.assertEqual(done[0]["item"]["call_id"], "call_abc")
        self.assertEqual(done[0]["item"]["name"], "exec_command")
        self.assertEqual(json.loads(done[0]["item"]["arguments"]), {"cmd": "ls -la"})
        self.assertEqual(final["type"], "response.completed")
        self.assertEqual(final["response"]["model"], "up-model", "the answering model, not the alias")
        self.assertEqual(final["response"]["usage"], {"input_tokens": 9, "output_tokens": 4, "total_tokens": 13})

    def test_stream_ends_at_done_without_waiting_for_the_socket_to_close(self):
        """A keep-alive upstream used to hang the handler until it gave up."""
        def hold(handler, body):
            sse(handler, [b'data: {"model":"up-model","choices":[{"delta":{"content":"ok"}}]}\n\n',
                          b"data: [DONE]\n\n"])
            time.sleep(20)  # the socket stays open; only `[DONE]` ends the answer

        ScriptedProvider.script = hold
        server = self.serve()
        started = time.monotonic()
        try:
            status, raw = self.post(server, self.responses_body(), timeout=10)
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertLess(time.monotonic() - started, 10, "the stream must not wait for EOF")
        self.assertIn('"type": "response.completed"', raw)

    # ── Tool round trip ──────────────────────────────────────────────────

    def test_tool_result_round_trip_keeps_call_id_and_name(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"done"}}]}\n\n',
            b'data: {"model":"up-model","choices":[],"usage":{"prompt_tokens":20,"completion_tokens":2}}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            self.post(server, self.responses_body(input=[
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "read it"}]},
                {"type": "function_call", "call_id": "call_1", "name": "exec_command",
                 "arguments": '{"cmd":"cat f"}'},
                {"type": "function_call_output", "call_id": "call_1", "output": "MARKER"}]))
        finally:
            server.shutdown()
            server.server_close()

        messages = ScriptedProvider.requests[0]["messages"]
        self.assertEqual([m["role"] for m in messages], ["system", "user", "assistant", "tool"])
        self.assertEqual(messages[2]["tool_calls"][0]["id"], "call_1")
        self.assertEqual(messages[2]["tool_calls"][0]["function"]["name"], "exec_command")
        self.assertEqual(messages[3], {"role": "tool", "tool_call_id": "call_1", "content": "MARKER"})

    # ── Freeform (custom) tools ──────────────────────────────────────────

    def test_custom_tool_is_carried_as_one_string_argument(self):
        chat = router_module.responses_request_to_chat({
            "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
            "tools": [{"type": "custom", "name": "apply_patch",
                       "format": {"type": "grammar", "syntax": "lark", "definition": "start: x"}}]})

        tool = chat["tools"][0]["function"]
        self.assertEqual(tool["name"], "apply_patch")
        self.assertEqual(tool["parameters"]["required"], ["input"])
        self.assertEqual(tool["parameters"]["properties"]["input"]["type"], "string")

    def test_custom_tool_call_comes_back_as_a_custom_tool_call_item(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"tool_calls":[{"index":0,'
            b'"id":"call_p","type":"function","function":{"name":"apply_patch","arguments":'
            b'"{\\"input\\":\\"*** Begin Patch\\\\n*** End Patch\\"}"}}]},"finish_reason":"tool_calls"}]}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body(tools=[
                {"type": "custom", "name": "apply_patch",
                 "format": {"type": "grammar", "syntax": "lark", "definition": "start: x"}}]))
        finally:
            server.shutdown()
            server.server_close()

        item = [e for e in self.events(raw) if e["type"] == "response.output_item.done"][0]["item"]
        self.assertEqual(status, 200)
        self.assertEqual(item["type"], "custom_tool_call")
        self.assertEqual(item["call_id"], "call_p")
        self.assertEqual(item["name"], "apply_patch")
        self.assertEqual(item["input"], "*** Begin Patch\n*** End Patch")
        self.assertNotIn("arguments", item, "a custom call must not be dressed as a function call")

    def test_custom_tool_result_round_trip(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"applied"}}]}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, _ = self.post(server, self.responses_body(input=[
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "patch it"}]},
                {"type": "custom_tool_call", "call_id": "call_p", "name": "apply_patch",
                 "input": "*** Begin Patch"},
                {"type": "custom_tool_call_output", "call_id": "call_p", "output": "Success"}]))
        finally:
            server.shutdown()
            server.server_close()

        messages = ScriptedProvider.requests[0]["messages"]
        self.assertEqual(status, 200)
        self.assertEqual(messages[2]["tool_calls"][0]["id"], "call_p")
        self.assertEqual(json.loads(messages[2]["tool_calls"][0]["function"]["arguments"]),
                         {"input": "*** Begin Patch"})
        self.assertEqual(messages[3]["tool_call_id"], "call_p")

    def test_a_namespaced_tool_still_flattens(self):
        chat = router_module.responses_request_to_chat({
            "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
            "tools": [{"type": "namespace", "name": "collaboration", "tools": [
                {"type": "function", "name": "followup_task", "parameters": {"type": "object"}}]}]})

        self.assertEqual([t["function"]["name"] for t in chat["tools"]], ["followup_task"])

    # ── Unsupported tools ────────────────────────────────────────────────

    def test_unsupported_tool_is_named_rather_than_dropped(self):
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body(tools=[
                {"type": "computer_use", "display_width": 1024}]))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 400)
        self.assertEqual(json.loads(raw)["error"]["type"], "unsupported_feature")
        self.assertIn("computer_use", raw)
        self.assertEqual(ScriptedProvider.requests, [], "nothing may reach a provider")

    def test_enabled_web_search_is_refused_and_a_disabled_one_is_not(self):
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body(tools=[
                {"type": "web_search", "external_web_access": True}]))
            self.assertEqual(status, 400)
            self.assertIn("web_search", raw)
        finally:
            server.shutdown()
            server.server_close()

        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"ok"}}]}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, _ = self.post(server, self.responses_body(tools=[
                {"type": "web_search", "external_web_access": False}]))
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(status, 200, "a search the client itself disabled is not a failure")

    def test_a_function_tool_without_a_name_is_refused(self):
        with self.assertRaises(router_module.UnsupportedFeature):
            router_module.responses_request_to_chat({
                "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}],
                "tools": [{"type": "function", "parameters": {"type": "object"}}]})

    # ── Failure semantics ────────────────────────────────────────────────

    def test_an_upstream_that_dies_mid_stream_fails_the_response(self):
        """A truncated answer must never be reported as completed."""
        def truncate(handler, body):
            sse(handler, [b'data: {"model":"up-model","choices":[{"delta":{"content":"half"}}]}\n\n'])
            handler.close_connection = True

        ScriptedProvider.script = truncate
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body())
        finally:
            server.shutdown()
            server.server_close()

        events = self.events(raw)
        self.assertEqual(status, 200, "the headers were already sent, so the failure is in-band")
        self.assertEqual(events[-1]["type"], "response.failed")
        self.assertEqual(events[-1]["response"]["status"], "failed")
        self.assertNotIn("response.completed", raw)
        self.assertIn("half", raw, "what did arrive is still delivered")

    def test_a_provider_error_before_any_event_is_a_json_failure(self):
        def fail(handler, body):
            handler.send_response(500)
            handler.send_header("Content-Length", "2")
            handler.end_headers()
            handler.wfile.write(b"{}")

        ScriptedProvider.script = fail
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body())
        finally:
            server.shutdown()
            server.server_close()

        body = json.loads(raw)
        self.assertEqual(status, 502)
        self.assertEqual(body["error"]["type"], "no_provider_succeeded")
        self.assertEqual(body["error"]["providers"][0]["provider"], "up")
        self.assertEqual(body["error"]["providers"][0]["status"], 500)
        self.assertTrue(body["error"]["request_id"], "a failure names the request that produced it")
        self.assertGreater(body["error"]["providers"][0]["cooldown_seconds"], 0)

    def test_a_provider_that_never_finishes_is_not_called_complete(self):
        """No `[DONE]` and no finish reason means the answer was cut short."""
        def cut(handler, body):
            sse(handler, [b'data: {"model":"up-model","choices":[{"delta":{"content":"partial"}}]}\n\n'])
            handler.wfile.write(b"")  # then the body simply stops

        ScriptedProvider.script = cut
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body())
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertNotIn("response.completed", raw)
        self.assertEqual(self.events(raw)[-1]["type"], "response.failed")

    def test_truncated_by_max_tokens_is_incomplete_not_completed(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"half"},"finish_reason":"length"}]}\n\n',
            b"data: [DONE]\n\n"])
        server = self.serve()
        try:
            status, raw = self.post(server, self.responses_body())
        finally:
            server.shutdown()
            server.server_close()

        events = self.events(raw)
        self.assertEqual(status, 200)
        self.assertEqual(events[-1]["type"], "response.incomplete")
        self.assertEqual(events[-1]["response"]["incomplete_details"]["reason"], "max_output_tokens")

    # ── Capability-aware fallback ────────────────────────────────────────

    def test_a_tool_request_skips_a_provider_that_cannot_call_tools(self):
        self.config["providers"]["up"]["supports_tools"] = False
        self.config["providers"]["text"] = {
            "base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "text",
            "supports_tools": False, "input_usd_per_million": 1, "output_usd_per_million": 1}
        self.config["routes"]["routine"] = ["up", "text"]
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"ok"}}]}\n\n',
            b"data: [DONE]\n\n"])

        status, body = self.router.complete(
            {"messages": [{"role": "user", "content": "go"}], "max_tokens": 8,
             "tools": [{"type": "function", "function": {"name": "f", "parameters": {"type": "object"}}}]}, "alice")

        self.assertEqual(status, 502, "a text-only provider must not be handed an agent request")
        self.assertEqual(ScriptedProvider.requests, [])
        self.assertIn("not declared tool-capable", " ".join(body["error"]["attempts"]))

    def test_the_same_request_without_tools_still_uses_that_provider(self):
        self.config["providers"]["up"]["supports_tools"] = False
        self.script_json()
        status, _ = self.router.complete({"messages": [{"role": "user", "content": "go"}], "max_tokens": 8}, "alice")
        self.assertEqual(status, 200, "plain text work does not need tool support")

    # ── Disconnect handling ──────────────────────────────────────────────

    def test_a_client_disconnect_releases_the_reservation(self):
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"a"}}]}\n\n',
            b'data: {"model":"up-model","choices":[{"delta":{"content":"b"}}]}\n\n',
            b'data: {"model":"up-model","choices":[],"usage":{"prompt_tokens":5,"completion_tokens":2}}\n\n',
            b"data: [DONE]\n\n"])
        chunks = []

        def send(chunk):
            chunks.append(chunk)
            if len(chunks) > 3:
                raise router_module.ClientDisconnected()

        outcome = self.router.stream({"messages": [{"role": "user", "content": "hi"}], "max_tokens": 8},
                                     "alice", lambda: None, send)

        self.assertIsNone(outcome)
        self.assertEqual(self.router.snapshot()["reserved_usd"], 0, "the reservation must be released")
        self.assertEqual(self.router.snapshot()["inflight"], 0)
        self.assertGreater(self.router.snapshot()["spent_usd"], 0,
                           "what the provider already produced is still charged")

    def test_a_disconnect_before_the_first_event_releases_the_reservation(self):
        def send(chunk):
            raise router_module.ClientDisconnected()

        # A provider that is never reached: the very first send is the one that
        # fails, which is the path a client closing immediately takes.
        ScriptedProvider.script = lambda handler, body: sse(handler, [
            b'data: {"model":"up-model","choices":[{"delta":{"content":"a"}}]}\n\n', b"data: [DONE]\n\n"])
        outcome = self.router.stream({"messages": [{"role": "user", "content": "hi"}], "max_tokens": 8},
                                     "alice", lambda: None, send)

        self.assertIsNone(outcome)
        self.assertEqual(self.router.snapshot()["reserved_usd"], 0)

    # ── Accounting ───────────────────────────────────────────────────────

    def test_the_provider_is_asked_for_no_more_than_the_reservation_covers(self):
        self.script_json()
        self.router.complete({"messages": [{"role": "user", "content": "go"}]}, "alice")

        self.assertEqual(self.config.get("default_max_output_tokens", router_module.DEFAULT_OUTPUT_TOKENS),
                         ScriptedProvider.requests[0]["max_tokens"],
                         "an unset output bound must be pinned so the estimate is an upper bound")


class CapabilityProbeTests(unittest.TestCase):
    """A declaration is a claim; the probe is the check behind it."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        ScriptedProvider.requests = []
        ScriptedProvider.script = None
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), ScriptedProvider)
        self.upstream.daemon_threads = True
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 100000,
            "max_request_bytes": 2000000, "routes": {"routine": ["up"], "critical": ["up"]},
            "providers": {"up": {"base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1",
                                 "model": "up-model", "critical_allowed": True, "tool_probe": True,
                                 "input_usd_per_million": 1, "output_usd_per_million": 1,
                                 "supports_tools": True}},
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.tmp.cleanup()

    def script_message(self, message, finish="stop"):
        def respond(handler, body):
            payload = json.dumps({"model": "up-model",
                                  "choices": [{"message": message, "finish_reason": finish}],
                                  "usage": {"prompt_tokens": 3, "completion_tokens": 1}}).encode()
            handler.send_response(200)
            handler.send_header("Content-Type", "application/json")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)
        ScriptedProvider.script = respond

    def genuine_tool_call(self):
        self.script_message({"role": "assistant", "content": None, "tool_calls": [
            {"id": "call_probe", "type": "function",
             "function": {"name": router_module.PROBE_TOOL, "arguments": '{"value":"ok"}'}}]},
            finish="tool_calls")

    def prose_about_a_tool_call(self):
        self.script_message({"role": "assistant",
                             "content": '{"name": "' + router_module.PROBE_TOOL + '", "arguments": {"value": "ok"}}'})

    def agent_request(self):
        return {"messages": [{"role": "user", "content": "go"}], "max_tokens": 8,
                "tools": [{"type": "function", "function": {"name": "f", "parameters": {"type": "object"}}}]}

    def test_the_probe_asks_for_a_required_tool_call(self):
        self.genuine_tool_call()
        self.router.probe_tools("up", self.config["providers"]["up"])

        probe = ScriptedProvider.requests[0]
        self.assertEqual(probe["tool_choice"], "required")
        self.assertEqual(probe["tools"][0]["function"]["name"], router_module.PROBE_TOOL)
        self.assertEqual(probe["model"], "up-model")

    def test_the_probe_obeys_the_provider_reasoning_rules(self):
        """A forced tool choice would otherwise be rejected before it is answered."""
        self.genuine_tool_call()
        provider = dict(self.config["providers"]["up"], thinking={
            "parameter": "thinking", "disabled_value": {"type": "disabled"},
            "disable_when_tool_choice_forced": True})
        self.router.config["providers"]["up"] = provider

        self.router.probe_tools("up", provider)

        self.assertEqual(ScriptedProvider.requests[0]["thinking"], {"type": "disabled"})

    def test_the_probe_honours_a_provider_token_budget(self):
        """A thinking model can spend a 64-token budget before it emits the call."""
        self.genuine_tool_call()
        provider = dict(self.config["providers"]["up"], tool_probe_max_tokens=512)

        self.router.probe_tools("up", provider)

        self.assertEqual(ScriptedProvider.requests[0]["max_tokens"], 512)

    def test_a_genuine_tool_call_keeps_the_provider_for_agent_requests(self):
        self.genuine_tool_call()
        verdict = self.router.probe_tools("up", self.config["providers"]["up"])

        self.assertTrue(verdict["tools"])
        self.assertIn("genuine tool call", verdict["detail"])
        self.assertIsNone(self.router.provider_skip("up", self.config["providers"]["up"],
                                                    "routine", self.agent_request()))

    def test_prose_about_a_tool_call_revokes_the_capability(self):
        """The exact shape a local model was observed producing."""
        self.prose_about_a_tool_call()
        verdict = self.router.probe_tools("up", self.config["providers"]["up"])

        self.assertFalse(verdict["tools"], "JSON in the message body is not a tool call")
        skip = self.router.provider_skip("up", self.config["providers"]["up"], "routine", self.agent_request())
        self.assertIsNotNone(skip, "an agent request must not go to a provider that only narrates")
        self.assertIn("tool probe", skip["attempt"])

    def test_a_revoked_capability_still_serves_plain_text(self):
        self.prose_about_a_tool_call()
        self.router.probe_tools("up", self.config["providers"]["up"])
        self.assertIsNone(self.router.provider_skip("up", self.config["providers"]["up"], "routine",
                                                    {"messages": [{"role": "user", "content": "go"}]}))

    def test_a_probe_that_cannot_run_does_not_revoke_a_declaration(self):
        def explode(handler, body):
            handler.send_response(500)
            handler.send_header("Content-Length", "2")
            handler.end_headers()
            handler.wfile.write(b"{}")

        ScriptedProvider.script = explode
        verdict = self.router.probe_tools("up", self.config["providers"]["up"])

        self.assertIsNone(verdict["tools"], "an unrunnable probe is unknown, not a refusal")
        self.assertIn("probe failed", verdict["detail"])
        self.assertIsNone(self.router.provider_skip("up", self.config["providers"]["up"],
                                                    "routine", self.agent_request()))

    def test_the_verdict_is_cached_instead_of_probed_per_request(self):
        self.genuine_tool_call()
        provider = self.config["providers"]["up"]
        for _ in range(3):
            self.router.probe_tools("up", provider)
        self.assertEqual(len(ScriptedProvider.requests), 3,
                         "one sampling round per provider and model, not one per request")
        self.assertEqual(self.router.probe_tools("up", provider)["samples"], 3)
        self.assertEqual(len(ScriptedProvider.requests), 3, "the cached verdict is reused")

    def test_a_partially_reliable_provider_is_measured_not_assumed(self):
        """A free model called the tool 4 times in 6; one sample says nothing."""
        # Cycles, so a second sampling round sees the same distribution rather
        # than running out of scripted answers.
        outcomes = [True, True, False, True, True, False]
        draw = itertools.cycle(outcomes)

        def sometimes(handler, body):
            genuine = next(draw)
            message = ({"role": "assistant", "content": None, "tool_calls": [
                {"id": "c", "type": "function",
                 "function": {"name": router_module.PROBE_TOOL, "arguments": '{"value":"ok"}'}}]}
                if genuine else
                {"role": "assistant", "content": '{"name": "' + router_module.PROBE_TOOL + '", "arguments": {}}'})
            payload = json.dumps({"model": "up-model",
                                  "choices": [{"message": message, "finish_reason": "tool_calls"}]}).encode()
            handler.send_response(200)
            handler.send_header("Content-Type", "application/json")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)

        ScriptedProvider.script = sometimes
        provider = dict(self.config["providers"]["up"], probe_samples=6)
        verdict = self.router.probe_tools("up", provider)

        self.assertEqual((verdict["genuine"], verdict["samples"]), (4, 6))
        self.assertEqual(verdict["success_rate"], 0.6667)
        self.assertFalse(verdict["tools"], "a provider that narrates a call a third of the time is not reliable")

        # The same measurements, with the operator accepting that failure rate.
        self.router.capabilities.clear()
        lenient = dict(provider, probe_min_success_rate=0.6)
        self.assertTrue(self.router.probe_tools("up", lenient)["tools"])

    def test_an_expired_verdict_is_probed_again(self):
        self.genuine_tool_call()
        provider = self.config["providers"]["up"]
        stale = router_module.time.time() - self.config.get("capability_probe_ttl_seconds", 3600) - 1
        self.router.capabilities[("up", "up-model")] = {"tools": True, "detail": "returned a genuine tool call",
                                                        "checked_at": stale}
        self.router.probe_tools("up", provider)
        self.assertEqual(len(ScriptedProvider.requests), provider.get("probe_samples", 3),
                         "the expired verdict is re-sampled")

    def test_a_provider_that_did_not_opt_in_is_never_probed(self):
        provider = {key: value for key, value in self.config["providers"]["up"].items() if key != "tool_probe"}
        verdict = self.router.probe_tools("up", provider)
        self.assertEqual((verdict["tools"], verdict["detail"]), (None, "not probed"))
        self.assertEqual(ScriptedProvider.requests, [])

    def test_warming_capabilities_survives_an_unreachable_provider(self):
        self.genuine_tool_call()
        self.config["providers"]["dead"] = {"base_url": "http://127.0.0.1:1/v1", "model": "dead",
                                            "tool_probe": True, "probe_timeout_seconds": 1}
        self.router.warm_capabilities()  # must not raise
        self.assertIn(("dead", "dead"), self.router.capabilities)

    def test_the_capability_report_separates_a_claim_from_a_verdict(self):
        self.config["providers"]["unprobed"] = {"base_url": "http://127.0.0.1:1/v1",
                                                "model": "unprobed", "supports_tools": True}
        self.prose_about_a_tool_call()
        self.router.warm_capabilities()
        report = {row["provider"]: row for row in self.router.capability_report()}

        self.assertTrue(report["up"]["declared_tools"])
        self.assertFalse(report["up"]["probed_tools"], "the verdict disagrees with the declaration")
        self.assertTrue(report["up"]["critical_allowed"])
        self.assertIsNone(report["unprobed"]["probed_tools"])
        self.assertEqual(report["unprobed"]["probe_detail"], "not probed")

    def test_the_capabilities_endpoint_is_served_and_gated(self):
        self.prose_about_a_tool_call()
        self.router.warm_capabilities()
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        previous = os.environ.get("X3_ROUTER_TOKEN")
        os.environ["X3_ROUTER_TOKEN"] = "test-secret"
        try:
            url = f"http://127.0.0.1:{server.server_port}/v1/capabilities"
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(url)
            self.assertEqual(rejected.exception.code, 401)
            request = urllib.request.Request(url, headers={"Authorization": "Bearer test-secret"})
            with urllib.request.urlopen(request) as response:
                rows = {row["provider"]: row for row in json.load(response)["capabilities"]}
            self.assertFalse(rows["up"]["probed_tools"])
            self.assertEqual(rows["up"]["model"], "up-model")
        finally:
            if previous is None:
                os.environ.pop("X3_ROUTER_TOKEN", None)
            else:
                os.environ["X3_ROUTER_TOKEN"] = previous
            server.shutdown()
            server.server_close()

    # ── Reasoning effort fidelity ────────────────────────────────────────

    def test_reasoning_effort_is_carried_from_the_responses_request(self):
        chat = router_module.responses_request_to_chat({
            "instructions": "sys", "reasoning": {"effort": "low"},
            "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}]})
        self.assertEqual(chat["reasoning_effort"], "low")

    def test_reasoning_effort_reaches_only_a_provider_that_accepts_it(self):
        accepting = dict(self.config["providers"]["up"], reasoning_effort=True)
        refusing = dict(self.config["providers"]["up"])
        chat = {"messages": [{"role": "user", "content": "go"}], "max_tokens": 8, "reasoning_effort": "low"}

        self.assertEqual(self.router.provider_payload(chat, accepting, False)["reasoning_effort"], "low")
        self.assertNotIn("reasoning_effort", self.router.provider_payload(chat, refusing, False),
                         "an undeclared field must not be forwarded to every provider")

    def test_reasoning_effort_is_kept_when_a_forced_tool_choice_disables_thinking(self):
        provider = dict(self.config["providers"]["up"], reasoning_effort=True, thinking={
            "parameter": "thinking", "disabled_value": {"type": "disabled"},
            "disable_when_tool_choice_forced": True})
        payload = self.router.provider_payload(
            {"messages": [{"role": "user", "content": "go"}], "max_tokens": 8, "reasoning_effort": "low",
             "tools": [{"type": "function", "function": {"name": "f"}}],
             "tool_choice": {"type": "function", "function": {"name": "f"}}}, provider, False)

        # Verified live: DeepSeek accepts the pair, so the client's effort must
        # not be silently dropped on the turns that need thinking switched off.
        self.assertEqual(payload["thinking"], {"type": "disabled"})
        self.assertEqual(payload["reasoning_effort"], "low")

    def test_a_payload_without_a_reasoning_effort_is_left_alone(self):
        accepting = dict(self.config["providers"]["up"], reasoning_effort=True)
        payload = self.router.provider_payload({"messages": [{"role": "user", "content": "go"}], "max_tokens": 8},
                                               accepting, False)
        self.assertNotIn("reasoning_effort", payload)


class RoutingIntelligenceTests(unittest.TestCase):
    """§1–3 and §50: classification, policy routing, registry, retries."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        ScriptedProvider.requests = []
        ScriptedProvider.script = None
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), ScriptedProvider)
        self.upstream.daemon_threads = True
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        self.base = f"http://127.0.0.1:{self.upstream.server_port}/v1"
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 100000,
            "max_request_bytes": 2000000, "routes": {"routine": ["up"], "critical": ["up"]},
            "retry_attempts": 1, "retry_backoff_ms": 1, "retry_backoff_max_ms": 5,
            # The full policy set, with this fixture's providers substituted, so
            # class routing is exercised the way the shipped config uses it.
            "policies": {
                "x3-auto": {"tier": "auto", "order": ["up"]},
                "x3-fast": {"tier": "routine", "order": ["local", "up"]},
                "x3-code": {"tier": "routine", "order": ["up"]},
                "x3-deep": {"tier": "routine", "order": ["up"]},
                "x3-security": {"tier": "critical", "order": ["up"]},
                "x3-review": {"tier": "routine", "order": ["up"]},
                "x3-local": {"tier": "routine", "order": ["local"]},
            },
            "providers": {
                "up": {"base_url": self.base, "model": "up-model", "critical_allowed": True,
                       "input_usd_per_million": 1, "output_usd_per_million": 1, "supports_tools": True},
                "local": {"base_url": self.base, "model": "local-model", "supports_tools": True},
            },
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.tmp.cleanup()

    def reply_ok(self):
        ScriptedProvider.script = lambda handler, body: self.reply_json(handler, "ok")

    @staticmethod
    def reply_json(handler, content):
        payload = json.dumps({"model": "up-model",
                              "choices": [{"message": {"role": "assistant", "content": content}}],
                              "usage": {"prompt_tokens": 3, "completion_tokens": 1}}).encode()
        handler.send_response(200)
        handler.send_header("Content-Type", "application/json")
        handler.send_header("Content-Length", str(len(payload)))
        handler.end_headers()
        handler.wfile.write(payload)

    def chat(self, text, **extra):
        body = {"messages": [{"role": "system", "content": "You are a coding agent."},
                             {"role": "user", "content": text}]}
        body.update(extra)
        return body

    # ── §2 task classification ───────────────────────────────────────────

    def test_the_classifier_separates_the_classes_it_claims_to(self):
        cases = {
            "Rename the field in this struct": "SIMPLE_EDIT",
            "Add a unit test for the parser": "TEST_GENERATION",
            "Update the README with the new flag": "DOCUMENTATION",
            "Where is the settlement code?": "REPOSITORY_SEARCH",
            "Why does the validator panic on restart?": "DEBUGGING",
            "Profile the transaction pool and find the bottleneck": "PERFORMANCE",
            "Write a Solidity contract and test it with foundry": "EVM",
            "Derive the PDA and sign with invoke_signed": "SVM",
            "Review this patch and critique the design": "CODE_REVIEW",
            "Post-mortem the incident and find what went wrong": "FAILURE_ANALYSIS",
            "the coordinator refund is failing again after a claim": "DEBUGGING",
            "the build is broken since the last merge": "DEBUGGING",
            "Add a fuzz target for the codec": "FUZZING",
            "Plan the migration for the sqlite schema": "DATABASE",
        }
        for text, expected in cases.items():
            with self.subTest(text=text):
                result = router_module.classify(self.chat(text))
                self.assertEqual(result["task_class"], expected, result["scores"])

    def test_the_system_prompt_does_not_decide_the_class(self):
        # Codex's instructions mention security, performance and testing in the
        # abstract. If they were scored, every request would classify the same.
        noisy = {"messages": [
            {"role": "system", "content": "You are a security expert. Consider consensus, "
                                          "cryptography, performance and fuzzing at all times."},
            {"role": "user", "content": "Rename the field"}]}
        self.assertEqual(router_module.classify(noisy)["task_class"], "SIMPLE_EDIT")

    def test_classification_is_deterministic(self):
        body = self.chat("Debug the consensus finality bug in the cross-chain bridge")
        first = router_module.classify(body)
        second = router_module.classify(body)
        self.assertEqual(first, second)

    def test_a_critical_request_is_critical_whatever_it_classifies_as(self):
        # "Read the README about slashing" classifies as DOCUMENTATION, which
        # routes to the cheap policy — but it is still consensus material and
        # must not be downgraded.
        result = router_module.classify(self.chat("Read the README section about slashing"))
        self.assertEqual(result["risk"], "critical")
        self.assertIn("slashing", result["blast_radius"])

    def test_the_estimates_are_present_and_bounded(self):
        result = router_module.classify(self.chat("Implement the atomic settlement path"))
        self.assertIn(result["complexity"], ("low", "medium", "high"))
        self.assertIn(result["parallelizable"], ("low", "medium", "high"))
        self.assertGreater(result["context_tokens"], 0)
        self.assertTrue(result["verification"], "a task class must carry a verification requirement")

    def test_every_class_has_a_route_and_a_verification_requirement(self):
        for name in router_module.TASK_CLASSES:
            self.assertIn(name, router_module.CLASS_TERMS, name)
            self.assertIn(name, router_module.VERIFICATION_BY_CLASS, name)
            self.assertIn(name, router_module.PARALLEL_BY_CLASS, name)

    # ── §1 logical models as policies ────────────────────────────────────

    def test_the_class_chooses_the_policy_for_x3_auto(self):
        _, _, _, logical = self.router.choose(self.chat("Review this patch"))
        self.assertEqual(logical, "x3-review")
        _, _, _, logical = self.router.choose(self.chat("Update the README"))
        self.assertEqual(logical, "x3-fast")

    def test_an_explicit_logical_model_overrides_the_class(self):
        _, chain, _, logical = self.router.choose(self.chat("Update the README", model="x3-security"))
        self.assertEqual(logical, "x3-security")
        self.assertEqual(chain, ["up"])

    def test_x3_local_stays_on_the_local_provider(self):
        _, chain, _, _ = self.router.choose(self.chat("anything at all", model="x3-local"))
        self.assertEqual(chain, ["local"])

    def test_a_policy_cannot_downgrade_a_critical_classification(self):
        # x3-fast is a routine policy, but the request carries consensus terms.
        tier, _, classification, logical = self.router.choose(
            self.chat("Update the README about finality", model="x3-fast"))
        self.assertEqual(logical, "x3-fast")
        self.assertEqual(classification["risk"], "critical")
        self.assertEqual(tier, "critical", "privacy is a floor, not a preference")

    def test_an_unknown_logical_model_falls_back_rather_than_failing(self):
        tier, chain, _, logical = self.router.choose(self.chat("hello", model="gpt-9-mystery"))
        self.assertIn(logical, router_module.DEFAULT_POLICIES)
        self.assertNotEqual(logical, "gpt-9-mystery")
        self.assertTrue(chain)
        self.assertIn(tier, ("routine", "critical"))

    def test_a_config_without_policies_keeps_the_old_route_behaviour(self):
        legacy = dict(self.config)
        legacy.pop("policies")
        legacy["routes"] = {"routine": ["local"], "critical": ["up"]}
        router = router_module.Router(legacy, self.tmp.name + "/legacy.db")
        tier, chain, _, _ = router.choose(self.chat("format this"))
        self.assertEqual((tier, chain), ("routine", ["local"]))

    # ── §3 measured registry + latency accounting ────────────────────────

    def test_the_registry_records_latency_and_outcomes(self):
        self.reply_ok()
        self.router.complete(self.chat("hello", model="x3-code"), "alice")
        self.router.complete(self.chat("hello", model="x3-code"), "alice")

        entry = self.router.provider_registry()[0]
        self.assertEqual((entry["provider"], entry["model"]), ("up", "up-model"))
        self.assertEqual(entry["attempts"], 2)
        self.assertEqual(entry["successes"], 2)
        self.assertEqual(entry["failures"], 0)
        self.assertEqual(entry["retries"], 0)
        self.assertIsNotNone(entry["average_latency_ms"])
        self.assertGreater(entry["latency_samples"], 0)
        self.assertEqual(entry["input_tokens"], 6)

    def test_failed_attempts_are_counted_against_the_provider(self):
        def fail(handler, body):
            handler.send_response(400)
            handler.send_header("Content-Length", "2")
            handler.end_headers()
            handler.wfile.write(b"{}")

        ScriptedProvider.script = fail
        self.router.complete(self.chat("hello", model="x3-code"), "alice")

        entry = self.router.provider_registry()[0]
        self.assertEqual(entry["failures"], 1)
        self.assertEqual(entry["successes"], 0)
        self.assertEqual(entry["failure_rate"], 1.0)
        # Latency is recorded for failures too: a provider that is slow when it
        # breaks is a different routing proposition from one that is fast.
        self.assertIsNotNone(entry["average_latency_ms"])

    def test_the_registry_reports_a_verified_patch_rate_from_task_feedback(self):
        self.reply_ok()
        revision = "a" * 40
        self.router.begin_task("task-9", "alice", revision, "router")
        self.router.complete(self.chat("hello", model="x3-code"), "alice")
        self.router.end_task_request(5)
        self.router.task_outcome({
            "task_id": "task-9", "revision": revision, "scope": "router",
            "checks": [{"name": "router-tests", "exit_code": 0, "output_sha256": "b" * 64}]})

        entry = self.router.provider_registry()[0]
        self.assertEqual(entry["passed_tasks"], 1)
        self.assertEqual(entry["verified_patch_rate"], 1.0)

    # ── §50 bounded retries ──────────────────────────────────────────────

    def test_a_transient_failure_is_retried_once_and_can_succeed(self):
        calls = {"n": 0}

        def flaky(handler, body):
            calls["n"] += 1
            if calls["n"] == 1:
                handler.send_response(503)
                handler.send_header("Content-Length", "2")
                handler.end_headers()
                handler.wfile.write(b"{}")
                return
            self.reply_json(handler, "recovered")

        ScriptedProvider.script = flaky
        status, result = self.router.complete(self.chat("hello", model="x3-code"), "alice")

        self.assertEqual(status, 200)
        self.assertEqual(result["choices"][0]["message"]["content"], "recovered")
        self.assertEqual(calls["n"], 2, "one retry, not a retry storm")
        entry = self.router.provider_registry()[0]
        self.assertEqual(entry["attempts"], 2)
        self.assertEqual(entry["retries"], 1)

    def test_a_terminal_failure_is_not_retried(self):
        calls = {"n": 0}

        def unauthorized(handler, body):
            calls["n"] += 1
            handler.send_response(401)
            handler.send_header("Content-Length", "2")
            handler.end_headers()
            handler.wfile.write(b"{}")

        ScriptedProvider.script = unauthorized
        status, _ = self.router.complete(self.chat("hello", model="x3-code"), "alice")

        self.assertEqual(status, 502)
        self.assertEqual(calls["n"], 1, "retrying a 401 only spends money to get the same answer")

    def test_retries_are_bounded_by_configuration(self):
        calls = {"n": 0}

        def always_503(handler, body):
            calls["n"] += 1
            handler.send_response(503)
            handler.send_header("Content-Length", "2")
            handler.end_headers()
            handler.wfile.write(b"{}")

        ScriptedProvider.script = always_503
        self.router.config["providers"]["up"]["retry_attempts"] = 3
        self.router.complete(self.chat("hello", model="x3-code"), "alice")

        self.assertEqual(calls["n"], 4, "one attempt plus three bounded retries")
        self.assertEqual(self.router.provider_registry()[0]["retries"], 3)

    def test_a_malformed_body_is_not_retried(self):
        calls = {"n": 0}

        def nonsense(handler, body):
            calls["n"] += 1
            payload = b'{"not":"a completion"}'
            handler.send_response(200)
            handler.send_header("Content-Type", "application/json")
            handler.send_header("Content-Length", str(len(payload)))
            handler.end_headers()
            handler.wfile.write(payload)

        ScriptedProvider.script = nonsense
        self.router.complete(self.chat("hello", model="x3-code"), "alice")
        self.assertEqual(calls["n"], 1)

    def test_the_registry_and_explain_endpoints_answer(self):
        self.reply_ok()
        self.router.complete(self.chat("hello", model="x3-code"), "alice")
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            url = f"http://127.0.0.1:{server.server_port}"
            with urllib.request.urlopen(url + "/v1/registry") as response:
                registry = json.load(response)["registry"]
            self.assertEqual(registry[0]["provider"], "up")

            with urllib.request.urlopen(url + "/v1/models") as response:
                ids = [entry["id"] for entry in json.load(response)["data"]]
            self.assertIn("x3-security", ids)
            self.assertIn("x3-local", ids)

            body = json.dumps({"messages": [{"role": "user",
                                             "content": "Review the consensus change"}]}).encode()
            request = urllib.request.Request(url + "/v1/explain", body,
                                             {"Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                decision = json.load(response)
            self.assertEqual(decision["tier"], "critical")
            self.assertIn(decision["policy"], ("x3-security", "x3-review"))
            self.assertEqual(decision["provider_order"], ["up"])
        finally:
            server.shutdown()
            server.server_close()


class ContextIntegrationTests(unittest.TestCase):
    """§7: the router reaches the Forge context compiler, and says so when it cannot."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        # A stand-in for `tools/x3-forge/context.py`. This tests the router's
        # integration — argument passing, timeouts, exit codes, malformed
        # output — not the compiler, which has its own suite and is verified
        # live against the real index.
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 1000,
            "max_request_bytes": 100000, "routes": {"routine": ["up"], "critical": ["up"]},
            "providers": {"up": {"base_url": "http://127.0.0.1:1/v1", "model": "up",
                                 "supports_tools": True}},
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.tmp.cleanup()

    def compiler_returning(self, payload, exit_code=0, stderr=""):
        script = ("import json,sys;"
                  f"sys.stdout.write(json.dumps({payload!r}));"
                  f"sys.stderr.write({stderr!r});"
                  f"sys.exit({exit_code})")
        self.config["context_compiler"] = {"command": ["python3", "-c", script]}
        return self.config["context_compiler"]

    def test_a_compiled_package_comes_back_through_the_router(self):
        self.compiler_returning({"included": [{"path": "src/lib.rs"}], "terms": ["settlement"]})
        package, error = self.router.compile_context("where is settlement enforced")
        self.assertIsNone(error)
        self.assertEqual(package["included"][0]["path"], "src/lib.rs")

    def test_an_unconfigured_compiler_says_so_rather_than_pretending(self):
        package, error = self.router.compile_context("anything")
        self.assertIsNone(package)
        self.assertIn("no context compiler", error)

    def test_a_failing_compiler_reports_its_exit_code_and_last_error_line(self):
        self.compiler_returning({"never": "printed"}, exit_code=3, stderr="index missing: run index.py build")
        package, error = self.router.compile_context("anything")
        self.assertIsNone(package)
        self.assertIn("exited 3", error)
        self.assertIn("index.py build", error)

    def test_output_that_is_not_json_is_rejected(self):
        self.config["context_compiler"] = {"command": ["python3", "-c", "print('not json')"]}
        package, error = self.router.compile_context("anything")
        self.assertIsNone(package)
        self.assertIn("did not return JSON", error)

    def test_a_missing_command_is_an_error_not_a_hang(self):
        self.config["context_compiler"] = {"command": ["/nonexistent/context-compiler"]}
        package, error = self.router.compile_context("anything")
        self.assertIsNone(package)
        self.assertIn("could not run", error)

    def test_a_slow_compiler_times_out_rather_than_blocking_the_router(self):
        self.config["context_compiler"] = {"command": ["python3", "-c", "import time; time.sleep(30)"],
                                           "timeout_seconds": 0.5}
        package, error = self.router.compile_context("anything")
        self.assertIsNone(package)
        self.assertIn("timed out", error)

    def test_the_query_is_the_classified_task_text_not_the_system_prompt(self):
        captured = tempfile.NamedTemporaryFile(delete=False, suffix=".txt")
        self.addCleanup(os.unlink, captured.name)
        script = ("import sys;"
                  f"open({captured.name!r},'w').write(sys.argv[-1]);"
                  "print('{}')")
        self.config["context_compiler"] = {"command": ["python3", "-c", script]}
        chat = {"messages": [
            {"role": "system", "content": "You are a coding agent. Think about settlement."},
            {"role": "user", "content": "where is the refund enforced"}]}
        self.router.compile_context(router_module.task_text(chat))
        with open(captured.name, encoding="utf-8") as handle:
            sent = handle.read()
        self.assertIn("refund", sent)
        self.assertNotIn("coding agent", sent)

    def test_the_endpoint_serves_a_package_and_refuses_an_empty_query(self):
        self.compiler_returning({"included": []})
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            url = f"http://127.0.0.1:{server.server_port}/v1/context"
            request = urllib.request.Request(url + "?q=atomic%20settlement")
            with urllib.request.urlopen(request) as response:
                body = json.load(response)
            self.assertIn("package", body)
            self.assertIn("elapsed_ms", body)

            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(url)
            self.assertEqual(rejected.exception.code, 400)
        finally:
            server.shutdown()
            server.server_close()

    def test_the_endpoint_reports_an_unavailable_compiler_as_502(self):
        # No compiler configured at all.
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            request = urllib.request.Request(
                f"http://127.0.0.1:{server.server_port}/v1/context?q=x")
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(request)
            self.assertEqual(rejected.exception.code, 502)
            body = json.loads(rejected.exception.read())
            self.assertEqual(body["error"]["type"], "context_unavailable")
        finally:
            server.shutdown()
            server.server_close()


class FailureMemoryIntegrationTests(unittest.TestCase):
    """§56's last item: the router reads and writes the Forge memories."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.store = Path(self.tmp.name) / "memory.jsonl"
        self.recorder = Path(self.tmp.name) / "recorded.json"
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 1000,
            "max_request_bytes": 100000, "routes": {"routine": ["up"], "critical": ["up"]},
            "providers": {"up": {"base_url": "http://127.0.0.1:1/v1", "model": "up",
                                 "supports_tools": True}},
            "failure_memory": {
                "command": ["python3", str(Path(__file__).parent.parent.parent /
                                           "tools/x3-forge/failure_memory.py"),
                            "--store", str(self.store)],
            },
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.tmp.cleanup()

    def submit(self, outcome_ok, task_id="task-1", revision="a" * 40):
        self.router.begin_task(task_id, "alice", revision, "router")
        self.router.end_task_request(1)
        self.router.task_outcome({
            "task_id": task_id, "revision": revision, "scope": "router",
            "checks": [{"name": "router-tests", "exit_code": 0 if outcome_ok else 1,
                        "output_sha256": "b" * 64}]})

    # ── writing ──────────────────────────────────────────────────────────

    def test_a_failed_task_becomes_a_failure_entry(self):
        self.submit(False)
        rows = [json.loads(line) for line in self.store.read_text().splitlines() if line.strip()]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["kind"], "failure")
        self.assertEqual(rows[0]["component"], "router")
        self.assertEqual(rows[0]["commit"], "a" * 40)
        self.assertIn("router-tests", rows[0]["error"])

    def test_a_passed_task_becomes_a_success_entry(self):
        self.submit(True)
        rows = [json.loads(line) for line in self.store.read_text().splitlines() if line.strip()]
        self.assertEqual(rows[0]["kind"], "success")
        self.assertIn("router-tests", rows[0]["fix"])

    def test_the_outcome_names_the_provider_that_produced_it(self):
        self.router.config["providers"]["up"]["base_url"] = "http://127.0.0.1:1/v1"
        self.router.db.execute(
            "INSERT INTO usage (day,agent,provider,model,input_tokens,output_tokens,cost_usd,task_id) "
            "VALUES (?,?,?,?,?,?,?,?)",
            ("2026-09-30", "alice", "deepseek", "deepseek-flash", 10, 5, 0.0, "task-1"))
        self.router.db.commit()
        self.submit(False)
        rows = [json.loads(line) for line in self.store.read_text().splitlines() if line.strip()]
        self.assertEqual(rows[0]["model"], "deepseek/deepseek-flash")

    def test_the_outcome_reports_whether_the_memory_write_landed(self):
        self.router.begin_task("task-9", "alice", "c" * 40, "router")
        self.router.end_task_request(1)
        result = self.router.task_outcome({
            "task_id": "task-9", "revision": "c" * 40, "scope": "router",
            "checks": [{"name": "router-tests", "exit_code": 1, "output_sha256": "d" * 64}]})
        self.assertTrue(result["memory"]["recorded"])
        self.assertTrue(result["memory"]["fingerprint"])
        self.assertEqual(result["memory"]["kind"], "failure")

    def test_a_memory_outage_does_not_undo_the_verification_result(self):
        self.router.config["failure_memory"] = {"command": ["/nonexistent/memory"]}
        self.router.begin_task("task-2", "alice", "e" * 40, "router")
        self.router.end_task_request(1)
        result = self.router.task_outcome({
            "task_id": "task-2", "revision": "e" * 40, "scope": "router",
            "checks": [{"name": "router-tests", "exit_code": 0, "output_sha256": "f" * 64}]})
        self.assertEqual(result["outcome"], "checks_passed", "the checks result is durable")
        self.assertFalse(result["memory"]["recorded"])
        self.assertEqual(self.router.task_stats()[0]["outcome"], "checks_passed")

    def test_repeated_failures_aggregate_into_one_memory_row(self):
        for index in range(3):
            self.submit(False, task_id=f"task-{index}", revision=str(index) * 40)
        rows = [json.loads(line) for line in self.store.read_text().splitlines() if line.strip()]
        self.assertEqual(len(rows), 3, "every sighting is appended")
        self.assertEqual(len({row["fingerprint"] for row in rows}), 1,
                         "the same failure has one fingerprint however many times it is seen")

    # ── reading ──────────────────────────────────────────────────────────

    def test_search_finds_a_recorded_failure(self):
        self.submit(False)
        matches, reason = self.router.search_memory("router tests failing")
        self.assertIsNone(reason)
        self.assertEqual(len(matches), 1)
        self.assertEqual(matches[0]["component"], "router")

    def test_an_unconfigured_memory_says_so_rather_than_answering_empty(self):
        self.router.config.pop("failure_memory")
        matches, reason = self.router.search_memory("anything")
        self.assertIsNone(matches)
        self.assertIn("no failure memory", reason)

    def test_a_failing_memory_tool_is_reported_with_its_exit_code(self):
        self.router.config["failure_memory"] = {"command": ["python3", "-c",
                                                            "import sys; sys.stderr.write('boom'); sys.exit(4)"]}
        matches, reason = self.router.search_memory("anything")
        self.assertIsNone(matches)
        self.assertIn("exited 4", reason)
        self.assertIn("boom", reason)

    def test_the_memory_endpoint_serves_matches_and_rejects_an_empty_query(self):
        self.submit(False)
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            url = f"http://127.0.0.1:{server.server_port}/v1/memory"
            with urllib.request.urlopen(url + "?q=router+tests") as response:
                body = json.load(response)
            self.assertEqual(len(body["matches"]), 1)
            with self.assertRaises(urllib.error.HTTPError) as rejected:
                urllib.request.urlopen(url)
            self.assertEqual(rejected.exception.code, 400)
            with self.assertRaises(urllib.error.HTTPError) as bad_limit:
                urllib.request.urlopen(url + "?q=x&limit=lots")
            self.assertEqual(bad_limit.exception.code, 400)
        finally:
            server.shutdown()
            server.server_close()

    def test_explain_can_consult_the_memory_before_the_work_starts(self):
        self.submit(False)
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            body = json.dumps({"memory": True, "messages": [
                {"role": "user", "content": "the router tests are failing again"}]}).encode()
            request = urllib.request.Request(
                f"http://127.0.0.1:{server.server_port}/v1/explain", body,
                {"Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                decision = json.load(response)
            self.assertEqual(len(decision["memory"]["matches"]), 1)
            self.assertIsNone(decision["context"], "context stays opt-in")
        finally:
            server.shutdown()
            server.server_close()


class NativeResponsesProvider(BaseHTTPRequestHandler):
    """An upstream that speaks the Responses protocol natively.

    Records the exact body, path and headers it is handed so a test can assert
    what actually left the router, and answers with a real Responses object or
    a semantic event stream (never `data: [DONE]`).
    """

    requests = []
    paths = []
    auth = []
    headers = []
    replies = []
    script = None

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        NativeResponsesProvider.requests.append(body)
        NativeResponsesProvider.paths.append(self.path)
        NativeResponsesProvider.auth.append(self.headers.get("Authorization"))
        NativeResponsesProvider.headers.append(dict(self.headers))
        if NativeResponsesProvider.script is not None:
            NativeResponsesProvider.script(self, body)
            return
        if NativeResponsesProvider.replies:
            status, content_type, payload = NativeResponsesProvider.replies.pop(0)
            raw = payload.encode() if isinstance(payload, str) else payload
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)
            return
        if body.get("stream"):
            events = [
                ("response.created",
                 {"type": "response.created", "response": {"id": "resp_native", "status": "in_progress",
                                                           "model": "deepseek-flash", "output": []}}),
                ("response.output_text.delta",
                 {"type": "response.output_text.delta", "item_id": "msg_1", "output_index": 1,
                  "content_index": 0, "delta": "X3_ROUTER_OK"}),
                ("response.completed",
                 {"type": "response.completed", "response": {
                     "id": "resp_native", "object": "response", "status": "completed",
                     "model": "deepseek-flash",
                     "output": [{"type": "message", "id": "msg_1", "role": "assistant",
                                 "status": "completed",
                                 "content": [{"type": "output_text", "text": "X3_ROUTER_OK",
                                              "annotations": []}]}],
                     "usage": {"input_tokens": 39, "output_tokens": 26, "total_tokens": 65}}}),
            ]
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for name, event in events:
                self.wfile.write(("event: " + name + "\ndata: " + json.dumps(event) + "\n\n").encode())
                self.wfile.flush()
            return
        raw = json.dumps({
            "id": "resp_native", "object": "response", "created_at": 1790806694,
            "status": "completed", "model": "deepseek-flash",
            "output": [{"type": "message", "id": "msg_1", "role": "assistant",
                        "status": "completed",
                        "content": [{"type": "output_text", "text": "X3_ROUTER_OK",
                                     "annotations": []}]}],
            "usage": {"input_tokens": 39, "output_tokens": 26, "total_tokens": 65},
        }).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def log_message(self, *_):
        pass


SECRET = "sk-secret-should-never-leak-1234567890"


class NativeResponsesTests(unittest.TestCase):
    """The Responses surface served by a provider that speaks it natively.

    These are the tests for the production bug: Codex's `/v1/responses` must
    reach DeepSeek's own `/responses` endpoint with the Responses body, not a
    Chat Completions translation posted to `/chat/completions`.
    """

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        for attribute in ("requests", "paths", "auth", "headers", "replies"):
            setattr(NativeResponsesProvider, attribute, [])
        NativeResponsesProvider.script = None
        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), NativeResponsesProvider)
        self.upstream.daemon_threads = True
        threading.Thread(target=self.upstream.serve_forever, daemon=True).start()
        os.environ["X3_TEST_DEEPSEEK_KEY"] = SECRET
        base = f"http://127.0.0.1:{self.upstream.server_port}/v1"
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1,
            "max_input_tokens": 100000, "max_request_bytes": 2000000,
            "max_output_tokens": 32768, "default_max_output_tokens": 4096,
            "routes": {"routine": ["native"], "critical": ["native"]},
            "providers": {"native": {
                "base_url": base, "protocol": "responses", "model": "deepseek-flash",
                "api_key_env": "X3_TEST_DEEPSEEK_KEY",
                "input_usd_per_million": 0.3, "output_usd_per_million": 1.2,
                "pricing_checked_on": "2026-09-30", "critical_allowed": True,
                "supports_tools": True, "tool_probe": False,
            }},
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/usage.db")

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        os.environ.pop("X3_TEST_DEEPSEEK_KEY", None)
        self.tmp.cleanup()

    def serve(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(self.router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        return server

    def post(self, server, body):
        request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                         json.dumps(body).encode(),
                                         {"Content-Type": "application/json", "X-X3-Agent": "codex"})
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.status, response.read().decode()
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read().decode()

    def body(self, **overrides):
        value = {"model": "x3-auto", "stream": False, "instructions": "be brief",
                 "input": [{"type": "message", "role": "user",
                            "content": [{"type": "input_text", "text": "hi"}]}]}
        value.update(overrides)
        return value

    # ── Native forwarding ────────────────────────────────────────────────

    def test_non_stream_responses_is_forwarded_to_the_responses_endpoint(self):
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertEqual(NativeResponsesProvider.paths, ["/v1/responses"],
                         "a responses provider must not be sent to /chat/completions")
        sent = NativeResponsesProvider.requests[0]
        self.assertIn("input", sent, "the native Responses body must survive")
        self.assertNotIn("messages", sent, "the body must not be translated to chat")
        self.assertEqual(sent["instructions"], "be brief")
        answer = json.loads(raw)
        self.assertEqual(answer["object"], "response")
        self.assertEqual(answer["output"][0]["content"][0]["text"], "X3_ROUTER_OK")

    def test_streaming_relays_semantic_events_and_never_expects_done(self):
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(stream=True))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertIn("response.created", raw)
        self.assertIn("response.completed", raw)
        self.assertNotIn("[DONE]", raw, "the Responses protocol does not terminate with [DONE]")
        self.assertIn("X3_ROUTER_OK", raw)

    def test_the_client_model_alias_is_replaced_with_the_provider_model(self):
        server = self.serve()
        try:
            status, _ = self.post(server, self.body(model="x3-auto"))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertEqual(NativeResponsesProvider.requests[0]["model"], "deepseek-flash",
                         "x3-auto is a routing alias, not a model the provider knows")

    def test_a_plain_string_input_is_accepted(self):
        """The Responses API allows `input` to be a string, not only a list."""
        server = self.serve()
        try:
            status, raw = self.post(server, {"model": "x3-auto", "stream": False,
                                             "input": "Reply with exactly X3_ROUTER_OK"})
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        sent = NativeResponsesProvider.requests[0]
        self.assertEqual(sent["input"][0]["content"][0]["text"], "Reply with exactly X3_ROUTER_OK")
        self.assertIn("X3_ROUTER_OK", raw)

    def test_the_authorization_header_is_sent_without_being_logged(self):
        server = self.serve()
        try:
            self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(NativeResponsesProvider.auth[0], "Bearer " + SECRET)

    # ── Token accounting ─────────────────────────────────────────────────

    def test_max_output_tokens_bounds_the_reservation(self):
        self.assertEqual(router_module.output_bound({"max_output_tokens": 1234}, {}), 1234)
        self.assertEqual(router_module.output_bound({"max_completion_tokens": 77}, {}), 77)
        self.assertEqual(router_module.output_bound({"max_tokens": 5}, {}), 5)
        self.assertEqual(router_module.output_bound({}, {"default_max_output_tokens": 4096}), 4096)
        self.assertIsNotNone(router_module.request_error({"max_output_tokens": 10 ** 9}, self.config),
                             "an unbounded max_output_tokens must be rejected, not reserved at the default")

        server = self.serve()
        try:
            status, _ = self.post(server, self.body(max_output_tokens=1234))
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(status, 200)
        self.assertEqual(NativeResponsesProvider.requests[0]["max_output_tokens"], 1234)

    def test_responses_usage_is_recorded_as_real_tokens(self):
        server = self.serve()
        try:
            self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()
        row = self.router.stats()[0]
        self.assertEqual(row["provider"], "native")
        totals = self.router.snapshot()
        self.assertEqual(totals["input_tokens"], 39, "input_tokens must not be booked as zero")
        self.assertEqual(totals["output_tokens"], 26, "output_tokens must not be booked as zero")

    def test_streaming_responses_usage_is_recorded(self):
        server = self.serve()
        try:
            self.post(server, self.body(stream=True))
        finally:
            server.shutdown()
            server.server_close()
        totals = self.router.snapshot()
        self.assertEqual(totals["input_tokens"], 39)
        self.assertEqual(totals["output_tokens"], 26)

    # ── Classification of the Responses shape ────────────────────────────

    def test_a_critical_term_in_responses_input_is_classified_critical(self):
        decision = router_module.classify({
            "input": [{"type": "message", "role": "user",
                       "content": [{"type": "input_text", "text": "audit the slashing path"}]}]})
        self.assertEqual(decision["risk"], "critical")
        self.assertIn("slashing", decision["critical_terms"])

    def test_a_critical_term_in_instructions_is_classified_critical(self):
        decision = router_module.classify({
            "instructions": "You are reviewing X3 settlement and finality code.",
            "input": [{"type": "message", "role": "user",
                       "content": [{"type": "input_text", "text": "continue"}]}]})
        self.assertEqual(decision["risk"], "critical")
        self.assertIn("settlement", decision["critical_terms"])

    def test_critical_routing_is_chosen_from_instructions(self):
        for phrase, expected in (("settlement", "settlement"), ("finality", "finality"),
                                 ("consensus", "consensus"), ("cross-vm", "cross-vm"),
                                 ("runtime upgrade", "runtime upgrade")):
            with self.subTest(phrase=phrase):
                decision = router_module.classify({
                    "instructions": "Follow the project rules about " + phrase + ".",
                    "input": [{"type": "message", "role": "user",
                               "content": [{"type": "input_text", "text": "go"}]}]})
                self.assertEqual(decision["risk"], "critical")
                self.assertIn(expected, decision["critical_terms"])

        server = self.serve()
        try:
            body = json.dumps({"instructions": "settlement code", "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]}]}).encode()
            request = urllib.request.Request(
                f"http://127.0.0.1:{server.server_port}/v1/explain", body,
                {"Content-Type": "application/json"})
            with urllib.request.urlopen(request) as response:
                decision = json.load(response)
        finally:
            server.shutdown()
            server.server_close()
        self.assertEqual(decision["tier"], "critical")

    # ── Tool capability normalization ────────────────────────────────────

    def tools(self):
        return [
            {"type": "function", "name": "exec_command",
             "parameters": {"type": "object", "properties": {"cmd": {"type": "string"}}}},
            {"type": "custom", "name": "apply_patch",
             "format": {"type": "grammar", "syntax": "lark", "definition": "start: x"}},
            {"type": "web_search"},
            {"type": "computer_use", "display_width": 1024},
            {"type": "namespace", "name": "collaboration", "tools": [
                {"type": "function", "name": "followup_task",
                 "parameters": {"type": "object", "properties": {}}}]},
        ]

    def test_unsupported_hosted_tools_do_not_break_a_native_provider(self):
        server = self.serve()
        try:
            status, _ = self.post(server, self.body(tools=self.tools()))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200, "a hosted tool must not sink the whole request")
        sent = NativeResponsesProvider.requests[0]["tools"]
        kinds = sorted(tool["type"] for tool in sent)
        self.assertEqual(kinds, ["custom", "function", "function"],
                         "only function and custom tools survive; namespaces flatten")
        for tool in sent:
            self.assertNotIn(tool["type"], router_module.HOSTED_TOOL_TYPES)

    def test_function_and_apply_patch_tools_survive_unchanged(self):
        server = self.serve()
        try:
            self.post(server, self.body(tools=self.tools()))
        finally:
            server.shutdown()
            server.server_close()

        sent = NativeResponsesProvider.requests[0]["tools"]
        by_name = {tool.get("name"): tool for tool in sent}
        self.assertEqual(by_name["exec_command"]["parameters"]["properties"]["cmd"]["type"], "string")
        self.assertEqual(by_name["apply_patch"]["type"], "custom")
        self.assertEqual(by_name["apply_patch"]["format"]["syntax"], "lark")
        self.assertEqual(by_name["followup_task"]["type"], "function")

    def test_tool_choice_is_relaxed_only_when_no_tool_is_left(self):
        payload = {"tools": [{"type": "computer_use"}], "tool_choice": "required"}
        dropped = router_module.normalize_responses_tools(payload, {"protocol": "responses"})
        self.assertEqual(dropped, ["computer_use"])
        self.assertNotIn("tools", payload)
        self.assertEqual(payload["tool_choice"], "auto",
                         "a forced choice with no tools left is a guaranteed 400")

        payload = {"tools": [{"type": "function", "name": "f"}, {"type": "computer_use"}],
                   "tool_choice": "required"}
        router_module.normalize_responses_tools(payload, {"protocol": "responses"})
        self.assertEqual(payload["tool_choice"], "required",
                         "a real tool is still there, so the choice is not weakened")

    # ── Failure reporting, failover and secret handling ──────────────────

    def test_a_provider_error_is_reported_with_a_sanitized_message(self):
        NativeResponsesProvider.replies = [(400, "application/json", json.dumps(
            {"error": {"message": "unsupported field 'foo' for model deepseek-flash (key "
                                 + SECRET + ")"}}))]
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 502)
        self.assertIn("HTTP 400", raw)
        self.assertIn("unsupported field", raw, "the provider's own message must reach the operator")
        self.assertNotIn(SECRET, raw, "an error body must never carry a credential")
        self.assertNotIn("Bearer", raw)

        health = {row["provider"]: row for row in self.router.provider_health()}
        self.assertIn("HTTP 400", health["native"]["last_error"])
        self.assertIn("unsupported field", health["native"]["last_error"])

    def test_a_failing_provider_does_not_corrupt_the_next_one(self):
        failing = ThreadingHTTPServer(("127.0.0.1", 0), NativeResponsesProvider)
        failing.daemon_threads = True
        threading.Thread(target=failing.serve_forever, daemon=True).start()

        def script(handler, body):
            if body.get("model") == "broken-model":
                raw = json.dumps({"error": {"message": "upstream exploded"}}).encode()
                handler.send_response(400)
                handler.send_header("Content-Length", str(len(raw)))
                handler.end_headers()
                handler.wfile.write(raw)
                return
            raw = json.dumps({"id": "resp_ok", "object": "response", "status": "completed",
                              "model": body["model"],
                              "output": [{"type": "message", "role": "assistant", "status": "completed",
                                          "content": [{"type": "output_text", "text": "SECOND",
                                                       "annotations": []}]}],
                              "usage": {"input_tokens": 3, "output_tokens": 1, "total_tokens": 4}}).encode()
            handler.send_response(200)
            handler.send_header("Content-Type", "application/json")
            handler.send_header("Content-Length", str(len(raw)))
            handler.end_headers()
            handler.wfile.write(raw)

        NativeResponsesProvider.script = script
        self.config["routes"] = {"routine": ["broken", "good"], "critical": ["broken", "good"]}
        self.config["providers"]["broken"] = dict(self.config["providers"]["native"],
                                                  model="broken-model", tool_probe=False)
        self.config["providers"]["good"] = dict(self.config["providers"]["native"], model="good-model")
        router = router_module.Router(self.config, self.tmp.name + "/failover.db")
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            request = urllib.request.Request(f"http://127.0.0.1:{server.server_port}/v1/responses",
                                             json.dumps(self.body(stream=False)).encode(),
                                             {"Content-Type": "application/json"})
            with urllib.request.urlopen(request, timeout=30) as response:
                status = response.status
                answer = json.loads(response.read())
        finally:
            server.shutdown()
            server.server_close()
            failing.shutdown()
            failing.server_close()

        self.assertEqual(status, 200)
        self.assertEqual(answer["output"][0]["content"][0]["text"], "SECOND")
        models = [attempt["model"] for attempt in NativeResponsesProvider.requests]
        self.assertEqual(models, ["broken-model", "good-model"])
        survivor = NativeResponsesProvider.requests[1]
        self.assertIn("input", survivor, "the second provider gets the untouched body")
        self.assertNotIn("messages", survivor)

    # ── A native failure is a failure, not an answer ─────────────────────

    def script_native_failure(self, truncated=False):
        """Answer the way a failing Responses provider does.

        Non-streaming: HTTP 200 with `status: "failed"`. Streaming: a semantic
        `response.failed` terminal event, or — when `truncated` — a stream that
        closes after a delta and never terminates.
        """
        def script(handler, body):
            if not body.get("stream"):
                raw = json.dumps({
                    "id": "resp_native", "object": "response", "status": "failed",
                    "model": "deepseek-flash", "output": [],
                    "error": {"code": "upstream_error", "message": "upstream exploded"},
                    "usage": {"input_tokens": 11, "output_tokens": 0, "total_tokens": 11},
                }).encode()
                handler.send_response(200)
                handler.send_header("Content-Type", "application/json")
                handler.send_header("Content-Length", str(len(raw)))
                handler.end_headers()
                handler.wfile.write(raw)
                return
            events = [
                ("response.created", {"type": "response.created",
                                      "response": {"id": "resp_native", "status": "in_progress",
                                                   "model": "deepseek-flash", "output": []}}),
                ("response.output_text.delta", {"type": "response.output_text.delta", "item_id": "msg_1",
                                                "output_index": 1, "content_index": 0, "delta": "partial"}),
            ]
            if not truncated:
                events.append(("response.failed", {
                    "type": "response.failed", "sequence_number": 2,
                    "response": {"id": "resp_native", "object": "response", "status": "failed",
                                 "model": "deepseek-flash", "output": [],
                                 "error": {"code": "upstream_error", "message": "upstream exploded"},
                                 "usage": {"input_tokens": 11, "output_tokens": 3, "total_tokens": 14}}}))
            handler.send_response(200)
            handler.send_header("Content-Type", "text/event-stream")
            handler.end_headers()
            for name, event in events:
                handler.wfile.write(("event: " + name + "\ndata: " + json.dumps(event) + "\n\n").encode())
                handler.wfile.flush()
        NativeResponsesProvider.script = script

    def test_native_nonstream_failed_response_falls_back_and_marks_health(self):
        self.script_native_failure()
        chat = ThreadingHTTPServer(("127.0.0.1", 0), Provider)
        chat.daemon_threads = True
        threading.Thread(target=chat.serve_forever, daemon=True).start()
        self.config["routes"] = {"routine": ["native", "chat"], "critical": ["native", "chat"]}
        self.config["providers"]["chat"] = {
            "base_url": f"http://127.0.0.1:{chat.server_port}/v1", "model": "chat-model",
            "input_usd_per_million": 1, "output_usd_per_million": 1,
            "supports_tools": True, "tool_probe": False}
        router = router_module.Router(self.config, self.tmp.name + "/failed-native.db")
        server = ThreadingHTTPServer(("127.0.0.1", 0), router_module.handler_for(router))
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            status, raw = self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()
            chat.shutdown()
            chat.server_close()

        self.assertEqual(status, 200)
        answer = json.loads(raw)
        self.assertEqual(answer["output"][0]["content"][0]["text"], "ok",
                         "a 200 with a failed body must fail over, not reach the client")
        health = {row["provider"]: row for row in router.provider_health()}
        self.assertIn("response.failed", health["native"]["last_error"])

    def test_native_stream_failed_terminal_is_not_recorded_as_success(self):
        self.script_native_failure()
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(stream=True))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertIn("event: response.failed", raw)
        self.assertNotIn("response.completed", raw)
        health = {row["provider"]: row for row in self.router.provider_health()}
        self.assertIn("response.failed", health["native"]["last_error"])

    def test_truncated_native_stream_gets_explicit_failed_terminal_event(self):
        self.script_native_failure(truncated=True)
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(stream=True))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertIn("event: response.failed", raw,
                      "a stream that closes without a terminal event must not look finished")
        self.assertNotIn("response.completed", raw)
        health = {row["provider"]: row for row in self.router.provider_health()}
        self.assertIn("terminal", health["native"]["last_error"])

    def test_native_responses_preserves_custom_tool_history(self):
        custom_input = [
            {"type": "custom_tool_call", "call_id": "patch-1", "name": "apply_patch",
             "input": "*** Begin Patch\n*** End Patch"},
            {"type": "custom_tool_call_output", "call_id": "patch-1", "output": "Done!"},
        ]
        server = self.serve()
        try:
            status, raw = self.post(server, self.body(
                stream=False, input=custom_input,
                tools=[{"type": "custom", "name": "apply_patch"}]))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 200)
        self.assertEqual(json.loads(raw)["status"], "completed")
        sent = NativeResponsesProvider.requests[0]
        self.assertEqual(sent["input"], custom_input,
                         "custom tool history must reach the provider exactly as Codex sent it")
        self.assertEqual(sent["tools"], [{"type": "custom", "name": "apply_patch"}],
                         "a custom tool a Responses provider can run must not be stripped")

    def test_no_provider_failure_ever_prints_a_credential(self):
        # A 4xx is not retried, so this reaches the terminal failure path in one
        # attempt; a 5xx would be retried and the queued reply would be replaced
        # by the default success.
        NativeResponsesProvider.replies = [(400, "application/json", json.dumps(
            {"error": {"message": "boom " + SECRET}}))]
        server = self.serve()
        captured = io.StringIO()
        try:
            with contextlib.redirect_stderr(captured):
                status, raw = self.post(server, self.body(stream=False))
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(status, 502)
        logs = captured.getvalue()
        self.assertNotIn(SECRET, raw)
        self.assertNotIn(SECRET, logs)
        self.assertNotIn("Bearer", logs)
        self.assertIn("[redacted]", logs, "the credential must be visibly scrubbed, not quietly dropped")


class SlowProvider(BaseHTTPRequestHandler):
    """Answers after a delay and records which port served each request."""
    delay = 0.4

    def do_POST(self):
        self.rfile.read(int(self.headers["Content-Length"]))
        time.sleep(self.delay)
        body = json.dumps({"choices": [{"message": {"role": "assistant", "content": str(self.server.server_port)}}],
                           "usage": {"prompt_tokens": 1, "completion_tokens": 1}}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class DualGpuTests(unittest.TestCase):
    """Two local workers: saturation spills to the idle card, death fails over."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.servers = []
        for _ in range(2):
            server = ThreadingHTTPServer(("127.0.0.1", 0), SlowProvider)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            self.servers.append(server)
        rtx, gtx = (s.server_port for s in self.servers)
        local = {"protocol": "chat_completions", "supports_tools": False, "critical_allowed": True,
                 "max_in_flight": 1}
        self.config = {
            "daily_budget_usd": 1, "agent_daily_budget_usd": 1, "max_input_tokens": 1000,
            "max_output_tokens": 100, "default_max_output_tokens": 16, "retry_attempts": 0,
            "providers": {
                "rtx": dict(local, base_url=f"http://127.0.0.1:{rtx}/v1", model="m"),
                "gtx": dict(local, base_url=f"http://127.0.0.1:{gtx}/v1", model="m"),
            },
            "policies": {"x3-local": {"tier": "routine", "order": ["rtx", "gtx"]}},
            "routes": {"routine": ["rtx", "gtx"], "critical": ["rtx", "gtx"]},
            "budget_fallback": [],
        }
        self.router = router_module.Router(self.config, self.tmp.name + "/nested/dir/usage.db")

    def tearDown(self):
        for server in self.servers:
            server.shutdown()
            server.server_close()
        self.router.db.close()
        self.tmp.cleanup()

    def ask(self):
        status, body = self.router.complete({"model": "x3-local", "messages": [{"role": "user", "content": "hi"}]}, "a")
        self.assertEqual(status, 200, body)
        return int(body["choices"][0]["message"]["content"])

    def test_database_directory_is_created_and_uses_wal(self):
        self.assertTrue(os.path.isdir(self.tmp.name + "/nested/dir"))
        self.assertEqual(self.router.db.execute("PRAGMA journal_mode").fetchone()[0], "wal")

    def test_default_db_path_is_absolute_and_overridable(self):
        with unittest.mock.patch.dict(os.environ, {"X3_ROUTER_DB": "", "XDG_DATA_HOME": "/data"}):
            self.assertEqual(router_module.default_db_path(), "/data/x3-router/usage.sqlite3")
        with unittest.mock.patch.dict(os.environ, {"X3_ROUTER_DB": "/x/u.db"}):
            self.assertEqual(router_module.default_db_path(), "/x/u.db")

    def test_saturated_worker_is_moved_last_not_dropped(self):
        self.assertEqual(self.router.attempt_order(["rtx", "gtx"]), ["rtx", "gtx"])
        with self.router.track("rtx"):
            self.assertEqual(self.router.attempt_order(["rtx", "gtx"]), ["gtx", "rtx"])
        self.assertEqual(self.router.load(), {})

    def test_providers_sharing_a_worker_share_its_slots(self):
        self.config["providers"]["rtx_big"] = dict(self.config["providers"]["rtx"], worker="card0")
        self.config["providers"]["rtx"]["worker"] = "card0"
        with self.router.track("rtx"):
            self.assertEqual(self.router.attempt_order(["rtx", "rtx_big", "gtx"]), ["gtx", "rtx", "rtx_big"])
            self.assertEqual(self.router.load(), {"card0": 1})

    def test_concurrent_requests_use_both_workers(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            started = time.monotonic()
            ports = set(pool.map(lambda _: self.ask(), range(2)))
            elapsed = time.monotonic() - started
        self.assertEqual(ports, {s.server_port for s in self.servers})
        self.assertLess(elapsed, 2 * SlowProvider.delay, "the two requests must run in parallel")

    def test_dead_worker_fails_over_to_the_other(self):
        dead = self.servers.pop(0)
        dead.shutdown()
        dead.server_close()
        self.assertEqual(self.ask(), self.servers[0].server_port)
        self.assertGreater(self.router.provider_cooldown("rtx"), 0, "a dead worker must be put on cooldown")
        self.assertEqual(self.ask(), self.servers[0].server_port)

    def test_both_workers_dead_reports_every_failure(self):
        for server in self.servers:
            server.shutdown()
            server.server_close()
        self.servers = []
        status, body = self.router.complete({"model": "x3-local", "messages": [{"role": "user", "content": "hi"}]}, "a")
        self.assertGreaterEqual(status, 500)
        text = json.dumps(body)
        self.assertIn("rtx", text)
        self.assertIn("gtx", text)


class RegistrationTests(unittest.TestCase):
    """GPU nodes join through providers.d drop-ins, not config edits."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.config = {"providers": {"ollama": {"base_url": "http://127.0.0.1:11434/v1", "model": "m"}},
                       "policies": {"x3-local": {"tier": "routine", "order": ["ollama"]}},
                       "budget_fallback": ["ollama"]}

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name, data):
        Path(self.tmp.name, name).write_text(json.dumps(data))

    def test_registration_appends_providers_after_configured_ones(self):
        self.write("x3gpu2.json", {"node": "x3gpu2",
                                   "providers": {"g2": {"base_url": "http://x3gpu2:11434/v1", "model": "qwen3:8b",
                                                        "worker": "x3gpu2-gpu0", "max_in_flight": 1}},
                                   "policies": {"x3-local": ["g2"]}})
        accepted, rejected = router_module.apply_registrations(self.config, self.tmp.name)
        self.assertEqual(rejected, [])
        self.assertEqual(accepted[0]["providers"], ["g2"])
        self.assertEqual(self.config["policies"]["x3-local"]["order"], ["ollama", "g2"])
        self.assertEqual(self.config["budget_fallback"], ["ollama", "g2"])
        self.assertFalse(self.config["providers"]["g2"]["critical_allowed"], "registered workers are not critical by default")

    def test_registration_cannot_carry_credentials_prices_or_overwrite(self):
        self.write("a.json", {"providers": {"x": {"base_url": "http://h/v1", "model": "m", "api_key_env": "HOME"}}})
        self.write("b.json", {"providers": {"y": {"base_url": "http://h/v1", "model": "m",
                                                  "input_usd_per_million": 9}}})
        self.write("c.json", {"providers": {"ollama": {"base_url": "http://h/v1", "model": "evil"}}})
        self.write("d.json", {"providers": {"z": {"base_url": "file:///etc/passwd", "model": "m"}}})
        self.write("e.json", {"providers": {"w": {"base_url": "http://h/v1", "model": "m"}},
                              "policies": {"x3-local": ["ollama"]}})
        self.write("f.json", "not an object")
        accepted, rejected = router_module.apply_registrations(self.config, self.tmp.name)
        self.assertEqual(accepted, [])
        self.assertEqual(len(rejected), 6)
        self.assertEqual(self.config["providers"]["ollama"]["model"], "m")
        self.assertEqual(self.config["policies"]["x3-local"]["order"], ["ollama"])

    def test_missing_directory_is_not_an_error(self):
        self.assertEqual(router_module.apply_registrations(self.config, self.tmp.name + "/none"), ([], []))


if __name__ == "__main__":
    unittest.main()
