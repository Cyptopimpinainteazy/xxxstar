#!/usr/bin/env python3
"""Tests for `scripts/snapshot-rpc-export.py`.

The exporter is the piece that lets a snapshot be taken without stopping the
node, so the interesting cases are the ones where it must *not* hand back a
state: a key the walk enumerated but could not read, a page walk that does not
terminate, an anchor the chain does not agree is canonical, an anchor with no
finality proof, and an anchor whose state the node has already pruned.

Every test drives the real script against a real HTTP JSON-RPC server on
loopback, so the paging, the anchor pinning and the refusal messages are the
ones the operator gets. Nothing here mocks the exporter itself.

    python3 tests/test_snapshot_rpc_export.py
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "snapshot-rpc-export.py"


def load_exporter():
    spec = importlib.util.spec_from_file_location("snapshot_rpc_export", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


exporter = load_exporter()


def h32(byte: int) -> str:
    return "0x" + bytes([byte] * 32).hex()


class Chain:
    """A tiny chain the fake RPC serves.

    Heights 1..`height` exist, each with its own block hash. `state` is the
    storage the anchor exposes; `justified` names the heights that carry a
    justification. The `behaviour` flags let a test make one read misbehave
    without touching the others.
    """

    def __init__(
        self,
        height: int = 4,
        state: dict[str, str] | None = None,
        justified: set[int] | None = None,
        *,
        canonical_override: dict[int, str] | None = None,
        fork_headers: dict[str, int] | None = None,
        unreadable_keys: set[str] | None = None,
        repeat_page: bool = False,
        state_discarded: bool = False,
        missing_method: str | None = None,
        justification_bytes: bytes = b"\x01\x02\x03\x04",
    ) -> None:
        self.height = height
        self.state = dict(state if state is not None else {"0x01": "0xaa", "0x02": "0xbb"})
        # `justified=None` means "the anchor has one"; an empty set means "nothing
        # on this chain does", which is a different scenario and one the tests
        # need — `justified or {height}` would silently turn the second into the
        # first.
        self.justified = {height} if justified is None else set(justified)
        self.canonical_override = canonical_override or {}
        # Blocks the chain will answer a header for but which are *not* canonical
        # at their height: a fork branch, which a snapshot must refuse.
        self.fork_headers = dict(fork_headers or {})
        self.unreadable_keys = unreadable_keys or set()
        self.repeat_page = repeat_page
        self.state_discarded = state_discarded
        self.missing_method = missing_method
        self.justification_bytes = justification_bytes
        self.calls: list[tuple[str, list]] = []

    # ── the chain's own answers ──────────────────────────────────────────────
    def hash_at(self, number: int) -> str | None:
        if number < 0 or number > self.height:
            return None
        return self.canonical_override.get(number, h32(number + 1))

    def header(self, block_hash: str) -> dict | None:
        for number in range(0, self.height + 1):
            if self.hash_at(number) == block_hash:
                return {
                    "parentHash": h32(number),
                    "number": hex(number),
                    "stateRoot": h32(200 + number),
                    "extrinsicsRoot": h32(100),
                }
        if block_hash in self.fork_headers:
            number = self.fork_headers[block_hash]
            return {
                "parentHash": h32(number),
                "number": hex(number),
                "stateRoot": h32(190 + number),
                "extrinsicsRoot": h32(100),
            }
        return None

    def block(self, block_hash: str) -> dict | None:
        header = self.header(block_hash)
        if header is None:
            return None
        number = int(header["number"], 16)
        justifications = None
        if number in self.justified:
            justifications = [
                [list(b"FRNK"), list(self.justification_bytes)],
                [list(b"OTHER"), list(b"\x00")],
            ]
        return {"block": {"header": header, "extrinsics": []}, "justifications": justifications}

    def dispatch(self, method: str, params: list):
        self.calls.append((method, params))
        if method == self.missing_method:
            raise KeyError(method)
        if method == "chain_getFinalizedHead":
            return self.hash_at(self.height)
        if method == "chain_getBlockHash":
            return self.hash_at(params[0])
        if method == "chain_getHeader":
            return self.header(params[0])
        if method == "chain_getBlock":
            return self.block(params[0])
        if method == "state_getRuntimeVersion":
            return {"specVersion": 4242}
        if method == "state_getKeysPaged":
            if self.state_discarded:
                raise ValueError("Client error: UnknownBlock: State already discarded")
            start, page_size = params[2], params[1]
            keys = sorted(self.state)
            if start is not None:
                keys = [key for key in keys if key > start]
            page = keys[:page_size]
            if self.repeat_page:
                # A page that never advances: the walk cannot terminate.
                page = sorted(self.state)[:page_size]
            return page
        if method == "state_getStorage":
            if self.state_discarded:
                raise ValueError("Client error: UnknownBlock: State already discarded")
            key = params[0]
            if key in self.unreadable_keys:
                return None
            return self.state.get(key)
        raise KeyError(method)


class Handler(BaseHTTPRequestHandler):
    chain: Chain

    def log_message(self, *_args):  # keep the test output readable
        pass

    def do_POST(self):  # noqa: N802 — the stdlib's spelling
        length = int(self.headers.get("Content-Length", "0"))
        request = json.loads(self.rfile.read(length) or b"{}")
        try:
            result = self.chain.dispatch(request.get("method"), request.get("params") or [])
            body = {"jsonrpc": "2.0", "id": request.get("id", 1), "result": result}
        except Exception as exc:  # noqa: BLE001 — any failure becomes a JSON-RPC error
            body = {
                "jsonrpc": "2.0",
                "id": request.get("id", 1),
                "error": {"code": -32601, "message": str(exc) or exc.__class__.__name__},
            }
        payload = json.dumps(body).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


class RpcProtocolCase(unittest.TestCase):
    def setUp(self) -> None:
        self.requests = []
        self.reply = lambda request: {"jsonrpc": "2.0", "id": request["id"], "result": None}
        case = self

        class ProtocolHandler(BaseHTTPRequestHandler):
            def do_POST(self):
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                case.requests.append(request)
                payload = json.dumps(case.reply(request)).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            def log_message(self, *_):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), ProtocolHandler)
        self.server.daemon_threads = True
        self.worker = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.worker.start()
        self.rpc = exporter.Rpc(f"http://127.0.0.1:{self.server.server_port}", timeout=2)
        self.addCleanup(self.cleanup_server)

    def cleanup_server(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join(timeout=2)

    def test_rejects_malformed_or_uncorrelated_envelopes(self):
        mutations = [
            lambda doc: doc.pop("jsonrpc"),
            lambda doc: doc.update(jsonrpc="1.0"),
            lambda doc: doc.pop("id"),
            lambda doc: doc.update(id=99999),
            lambda doc: doc.update(id=str(doc["id"])),
            lambda doc: doc.update(id=True),
            lambda doc: doc.update(error={"code": -32601, "message": "Unknown method"}),
            lambda doc: doc.update(error=None),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(case=index):
                def reply(request):
                    document = {"jsonrpc": "2.0", "id": request["id"], "result": None}
                    mutate(document)
                    return document

                self.reply = reply
                with self.assertRaises(exporter.RpcError):
                    self.rpc.call("chain_getHeader", [])

    def test_null_result_is_a_valid_rpc_answer(self):
        self.assertIsNone(self.rpc.call("chain_getHeader", []))

    def test_sequential_calls_use_distinct_ids(self):
        self.rpc.call("chain_getHeader", [])
        self.rpc.call("chain_getFinalizedHead", [])
        self.assertNotEqual(self.requests[0]["id"], self.requests[1]["id"])

    def test_rejects_malformed_error_objects(self):
        errors = [None, [], "failure", {}, {"code": True, "message": "bad code"},
                  {"code": -32601}, {"code": -32601, "message": 42}]
        for error in errors:
            with self.subTest(error=error):
                self.reply = lambda request: {"jsonrpc": "2.0", "id": request["id"], "error": error}
                with self.assertRaisesRegex(exporter.RpcError, "invalid error"):
                    self.rpc.call("chain_getHeader", [])

    def test_valid_rpc_error_keeps_method_and_provider_message(self):
        self.reply = lambda request: {"jsonrpc": "2.0", "id": request["id"],
                                      "error": {"code": -32601, "message": "Unknown method"}}
        with self.assertRaisesRegex(exporter.RpcError, "chain_getHeader.*Unknown method"):
            self.rpc.call("chain_getHeader", [])

    def test_boolean_id_cannot_match_numeric_request_id(self):
        self.reply = lambda request: {"jsonrpc": "2.0", "id": True, "result": None}
        with self.assertRaises(exporter.RpcError):
            self.rpc.call("chain_getHeader", [])


class ExportCase(unittest.TestCase):
    def setUp(self) -> None:
        self.work = tempfile.TemporaryDirectory()
        self.addCleanup(self.work.cleanup)
        self.dir = pathlib.Path(self.work.name)

    def serve(self, chain: Chain) -> str:
        handler = type("BoundHandler", (Handler,), {"chain": chain})
        server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.shutdown)
        self.addCleanup(server.server_close)
        return f"http://127.0.0.1:{server.server_address[1]}"

    def run_export(self, chain: Chain, *extra: str) -> tuple[int, pathlib.Path, pathlib.Path]:
        url = self.serve(chain)
        spec = self.dir / "out.json"
        report = self.dir / "anchor.json"
        code = exporter.main(
            [
                "--rpc",
                url,
                "--out",
                str(spec),
                "--report",
                str(report),
                "--force",
                *extra,
            ]
        )
        return code, spec, report

    # ── the happy path, measured rather than assumed ─────────────────────────
    def test_missing_finalized_header_refuses_without_writing_snapshot(self):
        for missing in (None, {}, {"number": None}):
            with self.subTest(header=missing):
                class MissingHeader(Chain):
                    def header(self, block_hash):
                        if block_hash == self.hash_at(self.height):
                            return missing
                        return super().header(block_hash)

                chain = MissingHeader()
                code, spec, report = self.run_export(chain, "--at", chain.hash_at(3))
                self.assertEqual(code, 1)
                self.assertFalse(spec.exists())
                self.assertFalse(report.exists())

    def test_late_rpc_refusal_preserves_existing_snapshot_and_report(self):
        class LateMissingHeader(Chain):
            finishing = False
            finishing_reads = 0

            def dispatch(self, method, params):
                if method == "state_getRuntimeVersion":
                    self.finishing = True
                return super().dispatch(method, params)

            def header(self, block_hash):
                if self.finishing:
                    self.finishing_reads += 1
                    if self.finishing_reads == 1:
                        return None
                return super().header(block_hash)

        spec = self.dir / "out.json"
        report = self.dir / "anchor.json"
        spec.write_text("old snapshot")
        report.write_text("old report")
        code, _, _ = self.run_export(LateMissingHeader())
        self.assertEqual(code, 1)
        self.assertEqual(spec.read_text(), "old snapshot")
        self.assertEqual(report.read_text(), "old report")

    def test_report_does_not_reread_pruned_start_header(self):
        class PruningChain(Chain):
            finished = False
            start = None

            def dispatch(self, method, params):
                if method == 'chain_getFinalizedHead' and self.start is None:
                    self.start = self.hash_at(self.height)
                if method == 'state_getRuntimeVersion':
                    self.finished = True
                    self.height += 1
                return super().dispatch(method, params)

            def header(self, block_hash):
                if self.finished and block_hash == self.start:
                    return None
                return super().header(block_hash)

        code, spec, report = self.run_export(PruningChain())
        self.assertEqual(code, 0)
        self.assertTrue(spec.exists())
        self.assertEqual(json.loads(report.read_text())['finalized_head_advanced_by'], 1)

    def test_export_writes_the_served_state_and_the_chains_anchor(self) -> None:
        chain = Chain(height=4, state={"0x01": "0xaa", "0x02": "0xbb", "0x03": "0x"})
        code, spec, report = self.run_export(chain)
        self.assertEqual(code, 0)
        document = json.loads(spec.read_text())
        self.assertEqual(document["genesis"]["raw"]["top"], chain.state)
        self.assertEqual(document["genesis"]["raw"]["childrenDefault"], {})
        anchor = json.loads(report.read_text())
        self.assertEqual(anchor["block_number"], 4)
        self.assertEqual(anchor["block_hash"], chain.hash_at(4))
        self.assertEqual(anchor["state_root"], chain.header(chain.hash_at(4))["stateRoot"])
        self.assertEqual(anchor["key_count"], 3)
        self.assertEqual(anchor["empty_values"], 1)
        self.assertEqual(anchor["runtime_spec_version"], 4242)

    def test_the_finality_proof_is_the_chains_justification(self) -> None:
        chain = Chain(height=3, justified={3}, justification_bytes=b"\xde\xad\xbe\xef")
        code, _spec, report = self.run_export(chain)
        self.assertEqual(code, 0)
        anchor = json.loads(report.read_text())
        self.assertEqual(anchor["finality_proof"], "0xdeadbeef")

    def test_paging_walks_past_the_first_page(self) -> None:
        state = {f"0x{i:02x}": f"0x{i:02x}" for i in range(1, 8)}
        chain = Chain(height=2, state=state)
        code, spec, _report = self.run_export(chain, "--page-size", "2")
        self.assertEqual(code, 0)
        document = json.loads(spec.read_text())
        self.assertEqual(document["genesis"]["raw"]["top"], state)

    # ── the refusals ────────────────────────────────────────────────────────
    def test_a_key_enumerated_without_a_value_is_refused(self) -> None:
        chain = Chain(height=2, state={"0x01": "0xaa", "0x02": "0xbb"}, unreadable_keys={"0x02"})
        code, spec, _report = self.run_export(chain)
        self.assertEqual(code, 1)
        self.assertFalse(spec.exists(), "a refused export must not leave a spec behind")

    def test_a_page_walk_that_never_advances_is_refused(self) -> None:
        chain = Chain(height=2, state={f"0x{i:02x}": "0x01" for i in range(1, 6)}, repeat_page=True)
        code, spec, _report = self.run_export(chain, "--page-size", "3")
        self.assertEqual(code, 1)
        self.assertFalse(spec.exists())

    def test_an_anchor_without_a_justification_is_refused(self) -> None:
        chain = Chain(height=5, justified=set())
        code, spec, _report = self.run_export(chain)
        self.assertEqual(code, 1)
        self.assertFalse(spec.exists())

    def test_an_optional_finality_proof_records_the_absence_and_invents_nothing(self) -> None:
        chain = Chain(height=5, justified=set())
        code, spec, report = self.run_export(chain, "--finality-proof", "optional")
        self.assertEqual(code, 0)
        self.assertTrue(spec.exists())
        anchor = json.loads(report.read_text())
        self.assertIsNone(anchor["finality_proof"])
        self.assertEqual(anchor["block_number"], 5)

    def test_the_walk_back_anchors_at_the_newest_justified_block(self) -> None:
        chain = Chain(height=7, justified={3})
        code, spec, report = self.run_export(chain, "--justified-ancestor")
        self.assertEqual(code, 0)
        anchor = json.loads(report.read_text())
        self.assertEqual(anchor["block_number"], 3)
        self.assertEqual(anchor["requested_block_number"], 7)
        self.assertTrue(anchor["walked_back_to_justified_ancestor"])
        self.assertNotEqual(anchor["block_hash"], chain.hash_at(7))

    def test_an_anchor_that_is_not_canonical_is_refused(self) -> None:
        fork_hash = h32(0xAB)
        chain = Chain(height=4, fork_headers={fork_hash: 4})
        code, spec, _report = self.run_export(chain, "--at", fork_hash)
        self.assertEqual(code, 1)
        self.assertFalse(spec.exists())

    def test_a_pruned_anchor_is_refused_and_names_pruning(self) -> None:
        chain = Chain(height=4, state_discarded=True)
        url = self.serve(chain)
        spec = self.dir / "out.json"
        import contextlib
        import io

        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            code = exporter.main(
                ["--rpc", url, "--out", str(spec), "--force"]
            )
        self.assertEqual(code, 1, "a pruned anchor is a refusal, not a harness failure")
        self.assertIn("pruned", stderr.getvalue())
        self.assertFalse(spec.exists())

    def test_a_missing_rpc_method_is_a_usage_failure(self) -> None:
        chain = Chain(height=4, missing_method="chain_getFinalizedHead")
        code, spec, _report = self.run_export(chain)
        self.assertEqual(code, 2)
        self.assertFalse(spec.exists())

    def test_a_child_trie_template_is_refused(self) -> None:
        template = self.dir / "template.json"
        template.write_text(
            json.dumps(
                {
                    "name": "T",
                    "id": "t",
                    "genesis": {"raw": {"top": {}, "childrenDefault": {"0x01": "0x02"}}},
                }
            )
        )
        chain = Chain(height=2)
        code, spec, _report = self.run_export(chain, "--from-spec", str(template))
        self.assertEqual(code, 1)
        self.assertFalse(spec.exists())

    def test_an_existing_output_is_not_replaced_without_force(self) -> None:
        chain = Chain(height=2)
        url = self.serve(chain)
        spec = self.dir / "out.json"
        spec.write_text("do not lose me")
        code = exporter.main(["--rpc", url, "--out", str(spec)])
        self.assertEqual(code, 1)
        self.assertEqual(spec.read_text(), "do not lose me")

    def test_a_template_contributes_metadata_and_not_state(self) -> None:
        template = self.dir / "template.json"
        template.write_text(
            json.dumps(
                {
                    "name": "X3 local",
                    "id": "x3-local",
                    "chainType": "Development",
                    "properties": {"tokenSymbol": "X3"},
                    "genesis": {"raw": {"top": {"0xff": "0x99"}}},
                }
            )
        )
        chain = Chain(height=2, state={"0x01": "0xaa"})
        code, spec, _report = self.run_export(chain, "--from-spec", str(template))
        self.assertEqual(code, 0)
        document = json.loads(spec.read_text())
        self.assertEqual(document["name"], "X3 local")
        self.assertEqual(document["chainType"], "Development")
        # The template's own state is replaced wholesale, never merged: merging
        # would produce a state neither the chain nor the export can name.
        self.assertEqual(document["genesis"]["raw"]["top"], chain.state)


if __name__ == "__main__":
    unittest.main(verbosity=2 if "-v" in sys.argv else 1)
