#!/usr/bin/env python3
"""Tests for Gate 7's forced-node-restart evidence rule.

`scripts/mainnet/public_testnet_gate.sh` Gate 7 decides whether a launch may be
certified by the report `scripts/drills/node_restart_drill.sh` writes. Until
2026-09-28 the rule was inline in the gate and read:

    if grep -q "restart_drill: PASS" "$RESTART_REPORT"; then pass; fi

so any file at that path containing the marker satisfied a launch criterion — a
report from a different chain, or from this chain's *previous* boot, which has the
same genesis hash and so is indistinguishable by chain identity alone. The rule
now also requires the report to name the chain it restarted and to be dated, and
it refuses a proof older than a day.

None of that could be tested while it was inline: the gate needs a
seven-validator network to reach Gate 7. It is now a function that takes the
report path and the current time as arguments
(`scripts/mainnet/restart_proof_verdict.sh`), so this file drives the real rule
with synthetic reports and a fixed clock.

What is proved here:

  * the rule's own verdicts — fresh passes, stale/missing/undated/foreign/wrong
    proofs refuse, and a refusal is never `pass`;
  * that the gate calls that rule instead of carrying a second copy of it;
  * that the drill still writes both fields the rule reads.

    python3 tests/test_restart_proof_verdict.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
HELPER = ROOT / "scripts" / "mainnet" / "restart_proof_verdict.sh"
GATE = ROOT / "scripts" / "mainnet" / "public_testnet_gate.sh"
DRILL = ROOT / "scripts" / "drills" / "node_restart_drill.sh"

GENESIS = "0x" + "ab" * 32
OTHER_GENESIS = "0x" + "cd" * 32
NOW = 1_800_000_000
MAX_AGE = 86400


def make_report(
    *,
    chain: str | None = GENESIS,
    epoch: int | str | None = NOW - 60,
    result: str = "PASS",
    omit_chain: bool = False,
    omit_epoch: bool = False,
) -> str:
    """A report shaped like the one scripts/drills/node_restart_drill.sh writes."""
    lines = ["# Node Restart Drill", ""]
    if not omit_chain:
        lines.append(f"- restart_drill_chain: {chain}")
    if not omit_epoch:
        lines.append(f"- restart_drill_epoch: {epoch}")
    lines.append("- restart_drill_utc: 2027-01-15T00:00:00Z")
    lines.append(f"- restart_drill: {result}")
    return "\n".join(lines) + "\n"


def verdict(
    report: str | None,
    *,
    genesis: str = GENESIS,
    now: int = NOW,
    max_age: str | int = MAX_AGE,
    rpc_url: str = "http://127.0.0.1:9933",
) -> tuple[int, str]:
    """Run the real helper over `report`; return (exit status, stdout)."""
    with tempfile.TemporaryDirectory() as tmp:
        report_path = pathlib.Path(tmp) / "drill_node_restart.md"
        if report is not None:
            report_path.write_text(report, encoding="utf-8")
        proc = subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; restart_proof_verdict "$2" "$3" "$4" "$5" "$6"',
                "bash",
                str(HELPER),
                str(report_path),
                genesis,
                str(now),
                str(max_age),
                rpc_url,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    return proc.returncode, proc.stdout.strip()


class RestartProofVerdictTests(unittest.TestCase):
    def assert_pass(self, report: str | None, **kwargs) -> None:
        status, out = verdict(report, **kwargs)
        self.assertEqual(status, 0, f"the rule must not fail the caller: {out}")
        self.assertEqual(out, "pass", f"expected a pass, got: {out}")

    def assert_fail(self, report: str | None, needle: str, **kwargs) -> None:
        status, out = verdict(report, **kwargs)
        self.assertEqual(status, 0, f"a refusal must not exit non-zero: {out}")
        self.assertTrue(
            out.startswith("fail:"),
            f"a refusal must be reported as fail:<reason>, got: {out!r}",
        )
        self.assertIn(needle, out, f"refusal did not name the reason: {out!r}")

    # ── the happy path ────────────────────────────────────────────────────
    def test_fresh_proof_on_this_chain_passes(self):
        self.assert_pass(make_report())

    def test_proof_exactly_at_the_limit_passes(self):
        # The bound is `older than`, not `at least as old as`: a proof written
        # exactly max_age ago is still inside the window the policy describes.
        self.assert_pass(make_report(epoch=NOW - MAX_AGE))

    def test_small_clock_skew_ahead_of_the_gate_passes(self):
        # The drill host's clock being a minute ahead of the gate's is ordinary.
        self.assert_pass(make_report(epoch=NOW + 60))

    # ── the refusals the rule exists for ──────────────────────────────────
    def test_stale_proof_is_refused(self):
        # One second past the window, and the message has to say why: chain
        # identity cannot tell this boot from the previous one.
        self.assert_fail(
            make_report(epoch=NOW - MAX_AGE - 1),
            "old",
        )

    def test_a_proof_from_the_previous_boot_is_refused(self):
        # Same genesis hash (a re-boot of the same spec), written two days ago.
        self.assert_fail(
            make_report(chain=GENESIS, epoch=NOW - 2 * MAX_AGE),
            "previous boot",
        )

    def test_missing_report_is_refused(self):
        self.assert_fail(None, "no drill report")

    def test_report_without_the_pass_marker_is_refused(self):
        self.assert_fail(make_report(result="FAIL"), "not PASS")

    def test_report_that_does_not_name_its_chain_is_refused(self):
        self.assert_fail(make_report(omit_chain=True), "does not name the chain")

    def test_report_naming_an_unknown_chain_is_refused(self):
        self.assert_fail(make_report(chain="unknown"), "does not name the chain")

    def test_report_from_another_chain_is_refused(self):
        self.assert_fail(make_report(chain=OTHER_GENESIS), "different network")

    def test_unreadable_gate_genesis_is_refused(self):
        self.assert_fail(make_report(), "genesis hash", genesis="")

    def test_report_without_a_timestamp_is_refused(self):
        self.assert_fail(make_report(omit_epoch=True), "no restart_drill_epoch")

    def test_report_with_a_non_numeric_timestamp_is_refused(self):
        self.assert_fail(make_report(epoch="yesterday"), "not a unix timestamp")

    def test_proof_dated_far_in_the_future_is_refused(self):
        # Beyond the skew allowance the timestamp is not a time the gate can
        # place, so it is refused instead of being rounded into "fresh".
        self.assert_fail(make_report(epoch=NOW + 3600), "in the future")

    def test_unjudgeable_clock_is_refused_not_passed(self):
        # A caller bug must not read as a pass.
        self.assert_fail(make_report(), "cannot judge the age", now="now")  # type: ignore[arg-type]

    def test_every_refusal_is_a_fail_line(self):
        for report in (None, make_report(result="FAIL"), make_report(epoch=NOW - 2 * MAX_AGE)):
            _, out = verdict(report)
            self.assertEqual(out.split(":", 1)[0], "fail", out)


class WiringTests(unittest.TestCase):
    """The rule is only worth testing if the gate actually uses it."""

    def test_gate_sources_the_helper_and_calls_it(self):
        gate = GATE.read_text(encoding="utf-8")
        self.assertIn("restart_proof_verdict.sh", gate, "the gate does not source the rule")
        self.assertIn(
            'restart_proof_verdict "$RESTART_REPORT"',
            gate,
            "the gate sources the rule but does not call it with the report",
        )

    def test_gate_no_longer_decides_the_evidence_itself(self):
        gate = GATE.read_text(encoding="utf-8")
        self.assertNotIn(
            'grep -q "restart_drill: PASS"',
            gate,
            "the gate still has its own copy of the PASS-marker decision",
        )
        self.assertNotIn(
            "AGE=$(( $(date +%s) - REPORT_EPOCH ))",
            gate,
            "the gate still computes the proof age inline, untested",
        )

    def test_drill_writes_every_field_the_rule_reads(self):
        drill = DRILL.read_text(encoding="utf-8")
        for field in ("restart_drill: ", "restart_drill_chain: ", "restart_drill_epoch: "):
            self.assertIn(
                field,
                drill,
                f"scripts/drills/node_restart_drill.sh no longer writes `{field}`",
            )

    def test_the_gate_is_still_syntax_valid_shell(self):
        # Sourcing moves the Gate 7 block out of the gate; a stray `fi` is the
        # classic way that goes wrong.
        for script in (GATE, HELPER):
            proc = subprocess.run(
                ["bash", "-n", str(script)], capture_output=True, text=True, check=False
            )
            self.assertEqual(proc.returncode, 0, f"{script}: {proc.stderr}")


if __name__ == "__main__":
    # Fail loudly if the helper was deleted rather than silently reporting "OK".
    if not HELPER.is_file():
        raise SystemExit(f"missing {HELPER.relative_to(ROOT)}")
    if not re.search(r"^restart_proof_verdict\(\)", HELPER.read_text(encoding="utf-8"), re.M):
        raise SystemExit(f"{HELPER.relative_to(ROOT)} does not define restart_proof_verdict")
    unittest.main(verbosity=2)
