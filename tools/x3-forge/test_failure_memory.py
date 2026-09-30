#!/usr/bin/env python3
"""Tests for the Forge memory store (§17-19).

The properties worth pinning are the ones that make the memory trustworthy:
that repeats of the same failure collapse into one row with a count, that two
different problems do not collapse into one, and that a damaged file degrades
to "some entries unreadable" rather than "memory gone".
"""
import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "failure_memory", Path(__file__).with_name("failure_memory.py"))
memory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(memory)


class MemoryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.store = Path(self.tmp.name) / "memory.jsonl"

    def tearDown(self):
        self.tmp.cleanup()

    def add(self, **entry):
        entry.setdefault("kind", "failure")
        return memory.record(self.store, entry, now="2026-09-30T00:00:00+00:00")

    # ── fingerprinting ───────────────────────────────────────────────────

    def test_a_line_number_does_not_make_it_a_different_failure(self):
        first = memory.fingerprint("crate", "refund failed at line 412")
        second = memory.fingerprint("crate", "refund failed at line 987")
        self.assertEqual(first, second)

    def test_an_address_does_not_make_it_a_different_failure(self):
        first = memory.fingerprint("crate", "null pointer at 0xdeadbeef")
        second = memory.fingerprint("crate", "null pointer at 0x12345678")
        self.assertEqual(first, second)

    def test_a_different_component_is_a_different_failure(self):
        self.assertNotEqual(
            memory.fingerprint("crates/svm", "replay accepted"),
            memory.fingerprint("crates/evm", "replay accepted"))

    def test_a_different_error_is_a_different_failure(self):
        self.assertNotEqual(
            memory.fingerprint("crate", "replay accepted"),
            memory.fingerprint("crate", "refund accepted"))

    def test_whitespace_and_case_do_not_split_a_fingerprint(self):
        self.assertEqual(
            memory.fingerprint("Crate", "Refund   Failed"),
            memory.fingerprint("crate", "refund failed"))

    # ── recording ────────────────────────────────────────────────────────

    def test_a_record_carries_the_fields_the_spec_names(self):
        saved = self.add(component="crates/cross-vm-coordinator",
                         error="abort after complete refunded both legs",
                         root_cause="abort bypassed the transition table",
                         fix="route abort through the table",
                         regression="crates/x3-sim/tests/refund_after_claim.rs",
                         invariant="CLAIM_REFUND_MIX",
                         historical_analogue="HTLC-2022-04",
                         model="deepseek-flash",
                         commit="a" * 40)
        for field in ("component", "error", "root_cause", "fix", "regression",
                      "invariant", "historical_analogue", "model", "commit"):
            self.assertIn(field, saved, field)
        self.assertTrue(saved["fingerprint"])

    def test_an_unknown_kind_is_refused(self):
        with self.assertRaises(ValueError):
            self.add(kind="rumour", component="x", error="y")

    def test_the_store_is_append_only(self):
        self.add(component="crate", error="refund failed at line 1")
        self.add(component="crate", error="refund failed at line 2")
        lines = self.store.read_text(encoding="utf-8").strip().splitlines()
        self.assertEqual(len(lines), 2, "a repeat is a new line, not a rewrite")
        rows = memory.aggregate(memory.load(self.store))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["occurrences"], 2)

    def test_a_later_record_supplies_the_better_understanding(self):
        self.add(component="crate", error="refund failed")
        self.add(component="crate", error="refund failed", fix="check the journal first")
        row = memory.aggregate(memory.load(self.store))[0]
        self.assertEqual(row["fix"], "check the journal first")
        self.assertEqual(row["occurrences"], 2)

    def test_empty_fields_are_not_stored(self):
        saved = memory.record(self.store, {"kind": "success", "component": "crate",
                                           "error": "", "fix": None})
        self.assertNotIn("fix", saved)
        self.assertNotIn("error", saved)

    def test_the_three_memories_are_distinct_rows(self):
        self.add(kind="failure", component="crate", error="transaction pool collapse")
        self.add(kind="dead_end", component="crate", error="transaction pool collapse")
        self.add(kind="success", component="crate", error="transaction pool collapse")
        summary = memory.summarise(memory.load(self.store))
        self.assertEqual(summary["by_kind"]["failure"], 1)
        self.assertEqual(summary["by_kind"]["dead_end"], 1)
        self.assertEqual(summary["by_kind"]["success"], 1)

    # ── search ───────────────────────────────────────────────────────────

    def test_search_ranks_a_component_match_first(self):
        self.add(component="crates/cross-vm-coordinator",
                 error="abort after complete refunded both legs",
                 fix="route abort through the table")
        self.add(component="docs", error="typo in the refund paragraph")
        matches = memory.search(memory.load(self.store), "refund claim coordinator")
        self.assertEqual(matches[0]["component"], "crates/cross-vm-coordinator")
        self.assertGreater(matches[0]["match_score"], 0)

    def test_search_returns_nothing_for_an_unrelated_query(self):
        self.add(component="crates/svm", error="signature verification failed")
        self.assertEqual(memory.search(memory.load(self.store), "css layout grid"), [])

    def test_search_can_be_filtered_by_kind_and_component(self):
        self.add(kind="failure", component="a", error="timeout in refund path")
        self.add(kind="dead_end", component="a", error="timeout in refund path")
        self.add(kind="failure", component="b", error="timeout in refund path")
        entries = memory.load(self.store)
        self.assertEqual(len(memory.search(entries, "refund timeout", kind="dead_end")), 1)
        self.assertEqual(len(memory.search(entries, "refund timeout", component="b")), 1)

    def test_a_repeated_failure_is_ranked_above_a_single_sighting(self):
        for _ in range(4):
            self.add(component="crate", error="pool collapse under load")
        self.add(component="crate", error="pool collapse under different load")
        matches = memory.search(memory.load(self.store), "pool collapse load")
        self.assertEqual(matches[0]["occurrences"], 4)

    def test_search_respects_the_limit(self):
        for index in range(20):
            self.add(component="crate", error=f"failure number {index} in the refund path")
        self.assertEqual(len(memory.search(memory.load(self.store), "refund path", limit=5)), 5)

    # ── durability ───────────────────────────────────────────────────────

    def test_a_damaged_line_does_not_destroy_the_memory(self):
        self.add(component="crate", error="first")
        with self.store.open("a", encoding="utf-8") as handle:
            handle.write("{ this is not json\n")
        self.add(component="crate", error="third")
        entries = memory.load(self.store)
        self.assertEqual(len(entries), 2, "the readable entries survive")

    def test_a_missing_store_reads_as_empty_rather_than_failing(self):
        self.assertEqual(memory.load(Path(self.tmp.name) / "does-not-exist.jsonl"), [])

    def test_stats_count_components(self):
        self.add(component="a", error="one")
        self.add(component="a", error="two")
        self.add(component="b", error="three")
        summary = memory.summarise(memory.load(self.store))
        self.assertEqual(summary["entries"], 3)
        self.assertEqual(summary["distinct"], 3)
        self.assertEqual(summary["by_component"], {"a": 2, "b": 1})

    # ── command line ─────────────────────────────────────────────────────

    def test_the_cli_records_and_finds(self):
        code = memory.main(["--store", str(self.store), "add", "--kind", "dead_end",
                            "--component", "crates/svm", "--error",
                            "anchor build fails against solana 2.x",
                            "--conditions", "solana 2.x"])
        self.assertEqual(code, 0)
        found = memory.search(memory.load(self.store), "anchor build solana")
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["kind"], "dead_end")
        self.assertEqual(found[0]["conditions"], "solana 2.x")


if __name__ == "__main__":
    unittest.main()
