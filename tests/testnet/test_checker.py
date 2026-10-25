"""Unit tests for the X3 read-only validator checker."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("checker", Path(__file__).with_name("check_seven_validators.py"))
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)

class RpcTests(unittest.TestCase):
    def test_rpc_rejects_error(self):
        class Response:
            def __enter__(self): return self
            def __exit__(self, *args): return False
            def read(self): return b'{"jsonrpc":"2.0","id":1,"error":{"code":-32601}}'
        with patch.object(checker.urllib.request, "urlopen", return_value=Response()):
            with self.assertRaises(RuntimeError):
                checker.rpc("http://127.0.0.1:9950/", "system_health", 1)

    def test_rpc_passes_parameters(self):
        class Response:
            def __enter__(self): return self
            def __exit__(self, *args): return False
            def read(self): return b'{"jsonrpc":"2.0","id":1,"result":"0xabc"}'
        def fake_open(request, timeout):
            self.assertEqual(json.loads(request.data)["params"], [0])
            return Response()
        with patch.object(checker.urllib.request, "urlopen", side_effect=fake_open):
            self.assertEqual(checker.rpc("http://127.0.0.1:9950/", "chain_getBlockHash", 1, [0]), "0xabc")

    def test_snapshot_genesis_and_finality(self):
        responses = {
            "system_health": {"peers": 6, "isSyncing": False},
            "chain_getHeader": {"number": "0x10"},
            "chain_getFinalizedHead": "0xfinal",
            "chain_getBlockHash": "0xgenesis",
        }
        with patch.object(checker, "rpc", side_effect=lambda url, method, timeout, params=None: responses[method]):
            with patch.object(checker.urllib.request, "urlopen") as opener:
                opener.return_value.__enter__.return_value.read.return_value = b'{"result":{"number":"0xe"}}'
                node = checker.snapshot("http://127.0.0.1:9950/", 1)
        self.assertEqual(node["genesis_hash"], "0xgenesis")
        self.assertEqual(node["finalized_number"], 14)

if __name__ == "__main__":
    unittest.main()
