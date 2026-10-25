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


class ConsensusInvariantTests(unittest.TestCase):
    def records(self):
        before = [
            {"url": "node0", "genesis_hash": "0xgenesis", "peers": 1, "isSyncing": False,
             "best_number": 14, "finalized_number": 10, "finalized_hash": "0xold"},
            {"url": "node1", "genesis_hash": "0xgenesis", "peers": 1, "isSyncing": False,
             "best_number": 14, "finalized_number": 10, "finalized_hash": "0xold"},
        ]
        after = [{**n, "best_number": 16, "finalized_number": 11,
                  "finalized_hash": "0xnew"} for n in before]
        return before, after

    def query(self, url, method, timeout, params):
        # Historical finalized hashes remain immutable, despite newer finality.
        return "0xold" if params == [10] else "0xnew"

    def test_progressing_consensus_accepts_stable_finality(self):
        before, after = self.records()
        self.assertEqual(checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, self.query), [])

    def test_prior_finality_divergence_is_rejected(self):
        before, after = self.records()
        def split_history(url, method, timeout, params):
            if url == "node1" and params == [10]:
                return "0xsplit"
            return self.query(url, method, timeout, params)
        self.assertTrue(any("prior finalized divergence" in e for e in checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, split_history)))

    def test_one_stalled_validator_fails_even_when_other_validators_advance(self):
        before, after = self.records()
        # node0 moves forward while node1 remains stuck on the old finalized head.
        after[1]["finalized_number"] = before[1]["finalized_number"]
        after[1]["finalized_hash"] = before[1]["finalized_hash"]
        errors = checker.validate_consensus(before, after, ["node0", "node1"], 1, 1, self.query)
        self.assertTrue(
            any("node1: no finality progress" in message for message in errors),
            f"a stalled validator must not pass because another node advanced: {errors}",
        )

    def test_individual_finality_regression_is_rejected(self):
        before, after = self.records()
        after[0]["finalized_number"] = 9
        self.assertTrue(any("regressed" in e for e in checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, self.query)))

    def test_rewritten_finalized_history_is_rejected(self):
        before, after = self.records()
        def rewritten(url, method, timeout, params):
            if url == "node0" and params == [10]:
                return "0xrewritten"
            return self.query(url, method, timeout, params)
        self.assertTrue(any("rewritten" in e for e in checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, rewritten)))

    def test_finality_ahead_of_best_head_is_rejected(self):
        before, after = self.records()
        after[0]["best_number"] = 10
        self.assertTrue(any("exceeds best" in e for e in checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, self.query)))

    def test_endpoint_genesis_change_is_rejected(self):
        before, after = self.records()
        after[0]["genesis_hash"] = "0xother"
        self.assertTrue(any("genesis" in e for e in checker.validate_consensus(
            before, after, ["node0", "node1"], 1, 1, self.query)))


if __name__ == "__main__":
    unittest.main()
