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


if __name__ == "__main__":
    unittest.main()
