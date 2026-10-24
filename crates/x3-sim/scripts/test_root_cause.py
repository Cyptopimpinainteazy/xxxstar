#!/usr/bin/env python3
"""Tests for the root-cause dispatcher.

The dispatcher is the boundary between machine-produced evidence and a model
answer, so the tests pin the two things that matter: the packet is fully
represented in the prompt, and a missing or non-conforming router answer never
produces a stored "cause".
"""

import json
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import root_cause  # noqa: E402


def sample_packet(failure_id="abc123", schema="x3-failure-packet-v1"):
    return {
        "schema": schema,
        "failure_id": failure_id,
        "producer": "x3-sim",
        "commit": "deadbeef",
        "seed": 928441783,
        "scenario": "claim-refund-race",
        "config": {"sessions": 12, "steps": 200, "nodes": 5},
        "invariant": "REFUND_AFTER_CLAIM",
        "session": "sim-3",
        "detail": "phase=Complete then Refunded",
        "first_bad_step_label": "step 42",
        "first_bad_op": "refund",
        "active_faults": ["0042 partition [0]|[1, 2, 3]"],
        "state_before": {"phase": "Complete"},
        "state_after": {"phase": "Refunded"},
        "suspected_code": [
            {
                "file": "crates/cross-vm-coordinator/src/state_machine.rs",
                "symbol": "SwapCoordinator::abort",
                "reason": "must honour the terminal-phase table",
            }
        ],
        "replay_command": "cargo run -p x3-sim -- --seed 928441783 --scenario claim-refund-race",
        "required_regression_test": "completed_swap_cannot_be_refunded",
        "minimized": {
            "verified": True,
            "replay_command": "cargo run -p x3-sim -- --seed 928441783 --scenario claim-refund-race --sessions 1 --steps 11 --nodes 3",
        },
    }


class ResponseServer(ThreadingHTTPServer):
    content = ""
    status = 200
    finish_reason = "stop"
    # When set, the handler sends this verbatim: the way to simulate a body
    # that is not the completion envelope at all.
    raw = None


class ResponseHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        ResponseServer.last_body = json.loads(self.rfile.read(length))
        if ResponseServer.raw is not None:
            payload = ResponseServer.raw.encode()
        else:
            payload = json.dumps(
                {
                    "id": "resp_test",
                    "model": "x3-auto",
                    "usage": {"prompt_tokens": 10, "completion_tokens": 10},
                    "choices": [
                        {
                            "message": {"content": ResponseServer.content},
                            "finish_reason": ResponseServer.finish_reason,
                        }
                    ],
                }
            ).encode()
        self.send_response(ResponseServer.status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_):
        pass


class RootCauseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.packet_path = Path(self.tmp.name) / "packet.json"
        self.packet_path.write_text(json.dumps(sample_packet()))
        # Server class state is shared between tests: reset it so one test's
        # answer cannot leak into the next.
        ResponseServer.content = ""
        ResponseServer.status = 200
        ResponseServer.finish_reason = "stop"
        ResponseServer.raw = None

    def tearDown(self):
        self.tmp.cleanup()

    def test_prompt_carries_every_field_an_investigator_needs(self):
        prompt = root_cause.build_prompt(sample_packet())
        for expected in (
            "REFUND_AFTER_CLAIM",
            "seed=928441783",
            "step 42",
            "partition",
            "SwapCoordinator::abort",
            "state before the bad step",
            "state after the bad step",
            "minimized (verified)",
            "--sessions 1 --steps 11 --nodes 3",
            "required regression test: completed_swap_cannot_be_refunded",
        ):
            self.assertIn(expected, prompt)

    def test_dry_run_prints_the_request_without_network(self):
        import contextlib
        import io

        captured = io.StringIO()
        with contextlib.redirect_stdout(captured):
            code = root_cause.main([str(self.packet_path), "--dry-run"])
        self.assertEqual(code, 0)
        printed = captured.getvalue()
        self.assertIn('"model"', printed, "the printed payload names its model")
        self.assertIn("REFUND_AFTER_CLAIM", printed, "the printed payload carries the packet")
        self.assertIn('"messages"', printed)

    def test_an_unsupported_schema_is_refused(self):
        bad = Path(self.tmp.name) / "bad.json"
        bad.write_text(json.dumps(sample_packet(schema="something-else")))
        self.assertEqual(root_cause.main([str(bad), "--dry-run"]), 2)

    def test_a_v2_simulator_packet_is_accepted(self):
        # v2 adds branch/worktree_dirty; a null worktree_dirty means git could
        # not verify the checkout, and the dispatcher must still accept it.
        packet = sample_packet(schema="x3-failure-packet-v2")
        packet["branch"] = "main"
        packet["worktree_dirty"] = None
        path = Path(self.tmp.name) / "v2.json"
        path.write_text(json.dumps(packet))
        self.assertEqual(root_cause.main([str(path), "--dry-run"]), 0)

    def test_an_incomplete_packet_is_refused(self):
        for bad in (
            {"schema": "x3-failure-packet-v1"},
            {"schema": "x3-failure-packet-v1", "failure_id": "x", "producer": "p",
             "scenario": "s", "invariant": "i", "session": "s", "detail": "d"},  # no replay_command
            dict(sample_packet(), seed="not-a-number"),
            dict(sample_packet(), config={"sessions": 1, "steps": 2}),  # no nodes
            {"schema": "x3-gate-failure-packet-v1", "failure_id": "g", "gate": "g",
             "command": "c", "first_error": "e", "replay_command": "c"},  # no exit_code
        ):
            with self.subTest(bad=bad):
                path = Path(self.tmp.name) / "incomplete.json"
                path.write_text(json.dumps(bad))
                self.assertEqual(
                    root_cause.main([str(path), "--dry-run"]),
                    2,
                    "an incomplete packet is not dispatchable evidence",
                )

    def test_the_prompt_renders_every_violation(self):
        packet = sample_packet()
        packet["all_violations"] = [
            {
                "code": "CLAIM_REFUND_MIX",
                "session": "sim-9",
                "detail": "fast=Claimed slow=Refunded",
            },
            {
                "code": "REFUND_AFTER_CLAIM",
                "session": "sim-9",
                "detail": "journal records a claim and then a refund",
            },
        ]
        prompt = root_cause.build_prompt(packet)
        self.assertIn("all violations in this run", prompt)
        self.assertIn("CLAIM_REFUND_MIX", prompt)
        self.assertIn("fast=Claimed slow=Refunded", prompt)

    def test_an_unreachable_router_stores_nothing(self):
        import socket

        # Bind an ephemeral port, learn it, then close it: connecting to that
        # exact port is a refusal, where a fixed port like 9 could be
        # answered or silently dropped by the environment.
        probe = socket.socket()
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
        probe.close()

        out = Path(self.tmp.name) / "out"
        code = root_cause.main(
            [
                str(self.packet_path),
                "--router",
                f"http://127.0.0.1:{port}",
                "--out",
                str(out),
                "--timeout",
                "5",
            ]
        )
        self.assertEqual(code, 2)
        self.assertFalse(out.exists(), "no directory, no cause, when the router is down")

    def test_a_contract_answer_is_stored_with_provenance(self):
        answer = {
            "causes": [
                {
                    "symbol": "SwapCoordinator::abort",
                    "file": "crates/cross-vm-coordinator/src/state_machine.rs",
                    "confidence": 0.9,
                    "reasoning": "terminal phases must be immutable",
                }
            ],
            "first_experiment": "run the minimized seed",
            "required_regression_test": "completed_swap_cannot_be_refunded",
            "notes": "hypothesis",
        }
        ResponseServer.content = json.dumps(answer)
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 0)
        stored = json.loads((out / "root-cause-abc123-refund_after_claim.json").read_text())
        self.assertEqual(stored["packet_failure_id"], "abc123")
        self.assertEqual(stored["answer"]["causes"][0]["symbol"], "SwapCoordinator::abort")
        self.assertTrue((out / "root-cause-abc123-refund_after_claim.md").exists())
        # The packet really went over the wire, not a summary of it.
        self.assertIn("REFUND_AFTER_CLAIM", ResponseServer.last_body["messages"][1]["content"])

    def test_a_nonconforming_answer_is_exit_3_and_never_stored_as_a_cause(self):
        ResponseServer.content = "I could not determine the cause."
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3)
        self.assertFalse((out / "root-cause-abc123-refund_after_claim.json").exists())

    def test_an_empty_truncated_answer_is_exit_3_with_the_reason_recorded(self):
        ResponseServer.content = ""
        ResponseServer.finish_reason = "length"
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3)
        raw = (out / "root-cause-abc123.invalid.txt").read_text()
        self.assertIn("finish_reason=length", raw)

    def test_a_cause_without_a_file_is_not_stored(self):
        ResponseServer.content = json.dumps(
            {"causes": [{"symbol": "SwapCoordinator::abort", "confidence": 0.5}]}
        )
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3, "a symbol with no file is not an actionable cause")
        self.assertFalse((out / "root-cause-abc123-refund_after_claim.json").exists())

    def test_contract_rejects_boolean_confidence_missing_reasoning_and_no_experiment(self):
        for bad in (
            {
                "causes": [
                    {
                        "symbol": "s",
                        "file": "f.rs",
                        "confidence": True,
                        "reasoning": "r",
                    }
                ],
                "first_experiment": "x",
            },
            {
                "causes": [{"symbol": "s", "file": "f.rs", "confidence": 0.5}],
                "first_experiment": "x",
            },
            {
                "causes": [
                    {"symbol": "s", "file": "f.rs", "confidence": 0.5, "reasoning": "r"}
                ]
            },
            {
                "causes": [
                    {"symbol": "s", "file": "f.rs", "confidence": 1.5, "reasoning": "r"}
                ],
                "first_experiment": "x",
            },
        ):
            with self.assertRaises(ValueError):
                root_cause.extract_contract(json.dumps(bad))

    def test_an_empty_causes_list_with_notes_is_a_valid_negative_answer(self):
        answer = root_cause.extract_contract(
            json.dumps({"causes": [], "notes": "the packet does not identify a cause"})
        )
        self.assertEqual(answer["causes"], [])

    def test_contract_requires_notes_and_the_regression_test(self):
        cause = {
            "symbol": "s",
            "file": "f.rs",
            "confidence": 0.5,
            "reasoning": "r",
        }
        for bad in (
            # Ranked answer without the regression test the contract promises.
            {"causes": [cause], "first_experiment": "x", "notes": "n"},
            # Ranked answer without notes: no stated assumptions or gaps.
            {
                "causes": [cause],
                "first_experiment": "x",
                "required_regression_test": "t",
            },
            # Negative answer that does not say why it found nothing.
            {"causes": []},
            {"causes": [], "notes": "   "},
        ):
            with self.subTest(bad=bad):
                with self.assertRaises(ValueError):
                    root_cause.extract_contract(json.dumps(bad))

    def test_a_namespaced_gate_label_stays_a_single_file(self):
        gate = {
            "schema": "x3-gate-failure-packet-v1",
            "failure_id": "gate0003",
            "gate": "crates/foo",
            "commit": "deadbeef",
            "command": "cargo test -p foo",
            "exit_code": 101,
            "first_error": "boom",
            "replay_command": "cargo test -p foo",
        }
        path = Path(self.tmp.name) / "namespaced-gate.json"
        path.write_text(json.dumps(gate))
        ResponseServer.content = json.dumps(
            {
                "causes": [
                    {"symbol": "foo", "file": "crates/foo/src/lib.rs", "confidence": 0.4,
                     "reasoning": "boom at the assertion"},
                ],
                "first_experiment": "cargo test -p foo",
                "required_regression_test": "foo_stays_green",
                "notes": "gate label names a crate",
            }
        )
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 0)
        stored = out / "root-cause-gate0003-crates-foo.md"
        self.assertTrue(stored.exists(), "a namespaced gate must not escape the output dir")
        self.assertFalse((out / "crates").exists(), "the separator must not create directories")

    def test_scalar_optional_fields_do_not_crash_the_prompt(self):
        packet = sample_packet()
        # A malformed optional field is not evidence to prompt on, but it must
        # not take the dispatcher down with it.
        packet["active_faults"] = "not-a-list"
        packet["suspected_code"] = {"symbol": "x"}
        packet["all_violations"] = 7
        packet["failing_tests"] = "one-test"
        packet["locations"] = "file.rs:1"
        prompt = root_cause.build_prompt(packet)
        self.assertIn("REFUND_AFTER_CLAIM", prompt)

    def test_an_invalid_answer_is_kept_under_a_sanitized_name(self):
        packet = sample_packet()
        packet["failure_id"] = "weird/../id"
        path = Path(self.tmp.name) / "weird-packet.json"
        path.write_text(json.dumps(packet))
        ResponseServer.content = "not json at all"
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3)
        kept = sorted(out.glob("root-cause-*.invalid.txt"))
        self.assertEqual(len(kept), 1, kept)
        self.assertEqual(kept[0].parent, out, "the id must not escape --out")

    def test_a_gate_packet_is_dispatched_with_its_error_as_the_invariant(self):
        gate = {
            "schema": "x3-gate-failure-packet-v1",
            "failure_id": "gate0001",
            "gate": "pallet-x3-supply-ledger",
            "commit": "deadbeef",
            "command": "cargo test -p pallet-x3-supply-ledger",
            "exit_code": 101,
            "first_error": "assertion `left == right` failed",
            "suspected_file": "pallets/x3-supply-ledger/src/lib.rs",
            "suspected_lines": [412],
            "replay_command": "cargo test -p pallet-x3-supply-ledger",
        }
        path = Path(self.tmp.name) / "gate.json"
        path.write_text(json.dumps(gate))
        prompt = root_cause.build_prompt(gate)
        self.assertIn("pallet-x3-supply-ledger", prompt)
        self.assertIn("assertion `left == right` failed", prompt)
        self.assertIn("412", prompt)

        ResponseServer.content = json.dumps(
            {
                "causes": [
                    {
                        "symbol": "check_supply",
                        "file": "pallets/x3-supply-ledger/src/lib.rs",
                        "confidence": 0.6,
                        "reasoning": "assertion at line 412",
                    }
                ],
                "first_experiment": "cargo test -p pallet-x3-supply-ledger check_supply",
                "required_regression_test": "check_supply_rejects_burned_issuance",
                "notes": "line numbers come from the packet",
            }
        )
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 0)
        stored = out / "root-cause-gate0001-pallet-x3-supply-ledger.md"
        self.assertTrue(stored.exists(), "the gate packet names its own kind, not 'None'")
        invariant_line = next(
            line
            for line in stored.read_text().splitlines()
            if line.startswith("- invariant:")
        )
        self.assertIn(gate["first_error"], invariant_line)
        self.assertNotIn("None", invariant_line)


    def test_an_http_error_status_is_a_bad_answer_not_an_unreachable_router(self):
        ResponseServer.status = 500
        ResponseServer.content = "database is on fire"
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3, "the router answered; the answer is the problem")
        raw = (out / "root-cause-abc123.invalid.txt").read_text()
        self.assertIn("database is on fire", raw)
        self.assertFalse((out / "root-cause-abc123-refund_after_claim.json").exists())

    def test_a_non_json_body_is_exit_3_not_an_unreachable_router(self):
        ResponseServer.raw = "<html>gateway timeout</html>"
        server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            out = Path(self.tmp.name) / "out"
            code = root_cause.main(
                [
                    str(self.packet_path),
                    "--router",
                    f"http://127.0.0.1:{server.server_port}",
                    "--out",
                    str(out),
                ]
            )
        finally:
            server.shutdown()
            server.server_close()

        self.assertEqual(code, 3)
        self.assertIn("gateway timeout", (out / "root-cause-abc123.invalid.txt").read_text())

    def test_a_malformed_envelope_takes_the_contract_path_instead_of_crashing(self):
        for raw in (
            json.dumps({"choices": []}),
            json.dumps({"choices": ["not an object"]}),
            json.dumps({"choices": [{"message": "not an object"}]}),
            json.dumps({"choices": [{"message": {"content": 42}}]}),
            json.dumps(["not", "an", "object"]),
        ):
            with self.subTest(raw=raw):
                ResponseServer.raw = raw
                server = ThreadingHTTPServer(("127.0.0.1", 0), ResponseHandler)
                threading.Thread(target=server.serve_forever, daemon=True).start()
                try:
                    out = Path(self.tmp.name) / "out"
                    code = root_cause.main(
                        [
                            str(self.packet_path),
                            "--router",
                            f"http://127.0.0.1:{server.server_port}",
                            "--out",
                            str(out),
                        ]
                    )
                finally:
                    server.shutdown()
                    server.server_close()
                self.assertEqual(code, 3, raw)

    def test_a_gate_prompt_carries_the_failing_test_and_log_excerpt(self):
        gate = {
            "schema": "x3-gate-failure-packet-v1",
            "failure_id": "gate0002",
            "gate": "runtime",
            "command": "cargo test -p x3-chain-runtime",
            "exit_code": 101,
            "first_error": "panicked at runtime/src/lib.rs:88:5",
            "failing_tests": ["runtime::tests::supply_is_conserved"],
            "locations": [{"file": "runtime/src/lib.rs", "line": 88}],
            "log_excerpt": "thread 'main' panicked at runtime/src/lib.rs:88:5",
        }
        prompt = root_cause.build_prompt(gate)
        self.assertIn("runtime::tests::supply_is_conserved", prompt)
        self.assertIn("location: runtime/src/lib.rs:88", prompt)
        self.assertIn("log excerpt", prompt)
        self.assertIn("runtime/src/lib.rs:88:5", prompt)
        self.assertIn("gate exit code: 101", prompt)


if __name__ == "__main__":
    unittest.main()
