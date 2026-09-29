import importlib.util
import json
import os
import tempfile
import threading
import unittest
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
            "max_input_tokens": 1000, "routes": {"routine": ["down", "up"], "critical": ["up"]},
            "providers": {
                "down": {"base_url": "http://127.0.0.1:1/v1", "model": "down", "input_usd_per_million": 1, "output_usd_per_million": 1},
                "up": {"base_url": f"http://127.0.0.1:{self.upstream.server_port}/v1", "model": "up", "critical_allowed": True,
                       "input_usd_per_million": 1, "output_usd_per_million": 1}
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


if __name__ == "__main__":
    unittest.main()
