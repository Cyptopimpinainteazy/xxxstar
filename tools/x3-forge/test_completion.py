#!/usr/bin/env python3
"""Tests for the completion engine.

Most of these pin *severity* decisions, because every one of them was a false
positive found by running the engine against the real repo and disbelieving
the result: a pallet whose tests live in sibling files reported as "tested with
no tests"; a binary crate reported as "nothing depends on it"; and a gate
script reported as "declared path is missing" when the file was right there and
only the index could not see it.

A completion report that cries wolf is worse than none: it sends agents at
problems that do not exist.
"""
import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "completion", Path(__file__).with_name("completion.py"))
completion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(completion)


class FakeMatrix:
    """The two functions the engine uses from scripts/feature_matrix.py."""

    @staticmethod
    def composite(feature):
        return round(feature["implemented"] * .35 + feature["tested"] * .25
                     + feature["mainnet_ready"] * .40)

    @staticmethod
    def readiness_class(score):
        return "partial" if score >= 40 else "experimental"


def index_with(files, manifests=()):
    return {"commit": "0" * 40, "root": "/tmp", "crates": len(manifests),
            "manifests": list(manifests), "files": files, "stats": {}}


def source(path, lang="rust", crate="pallet-x", items=None):
    return {"sha256": "0" * 64, "bytes": 10, "lang": lang, "crate": crate,
            "items": items if items is not None else [{"kind": "fn", "name": "f", "line": 1}]}


def feature(**over):
    row = {"id": "X3-TEST-001", "name": "Test row", "subsystem": "test",
           "paths": ["pallets/x/src/lib.rs"], "implemented": 80, "tested": 70,
           "mainnet_ready": 60, "priority": "P0", "launch_scope": "core",
           "required_tests": [], "evidence": [], "blockers": []}
    row.update(over)
    return row


class CompletionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def analyze(self, files, features, manifests=()):
        index = index_with(files, manifests)
        return completion.analyze(index, FakeMatrix, features, root=self.root)

    def kinds(self, analysis):
        return sorted(gap["kind"] for gap in analysis["gaps"])

    def test_tests_in_a_sibling_file_count_for_a_declared_file(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {
            "pallets/x/src/lib.rs": source("pallets/x/src/lib.rs"),
            "pallets/x/src/tests.rs": source("pallets/x/src/tests.rs", items=[
                {"kind": "fn", "name": "t", "line": 1, "test": True}]),
        }
        analysis = self.analyze(files, [feature()])[0]
        self.assertEqual(analysis["evidence"]["test_items"], 1)
        self.assertEqual(self.kinds(analysis), [])
        self.assertEqual(analysis["state"], "TESTED")

    def test_a_declared_directory_is_searched_recursively(self):
        files = {
            "pallets/x/src/lib.rs": source("pallets/x/src/lib.rs"),
            "pallets/x/src/deep/tests.rs": source("pallets/x/src/deep/tests.rs", items=[
                {"kind": "fn", "name": "t", "line": 1, "test": True}]),
        }
        analysis = self.analyze(files, [feature(paths=["pallets/x"])])[0]
        self.assertEqual(analysis["evidence"]["test_items"], 1)

    def test_a_library_claiming_tests_without_any_is_flagged(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs")}
        analysis = self.analyze(files, [feature()])[0]
        self.assertIn("tested_without_tests", self.kinds(analysis))
        self.assertEqual(completion.severity_of(analysis), "high")

    def test_a_gate_script_is_not_flagged_for_having_no_test_symbol(self):
        (self.root / "scripts").mkdir(parents=True)
        files = {"scripts/gate.py": source("scripts/gate.py", lang="python", crate=None)}
        analysis = self.analyze(files, [feature(paths=["scripts/gate.py"])])[0]
        self.assertNotIn("tested_without_tests", self.kinds(analysis))

    def test_a_binary_crate_is_not_called_unwired(self):
        (self.root / "node/src").mkdir(parents=True)
        (self.root / "node/src/main.rs").write_text("fn main() {}")
        (self.root / "node/Cargo.toml").write_text('[package]\nname = "x3-node"\n')
        manifest = {"sha256": "0" * 64, "bytes": 1, "lang": "toml", "crate": None, "items": []}
        files = {
            "node/Cargo.toml": manifest,
            "node/src/main.rs": source("node/src/main.rs", crate="x3-node"),
            "node/src/service.rs": source("node/src/service.rs", crate="x3-node"),
        }
        analysis = self.analyze(files, [feature(paths=["node/src/service.rs"])],
                                manifests=["node/Cargo.toml"])[0]
        self.assertNotIn("unwired_crate", self.kinds(analysis))

    def test_a_library_nothing_depends_on_is_called_unwired(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        (self.root / "pallets/x/Cargo.toml").write_text('[package]\nname = "pallet-x"\n')
        manifest = {"sha256": "0" * 64, "bytes": 1, "lang": "toml", "crate": None, "items": []}
        files = {
            "pallets/x/Cargo.toml": manifest,
            "pallets/x/src/lib.rs": source("pallets/x/src/lib.rs", crate="pallet-x"),
        }
        analysis = self.analyze(files, [feature()], manifests=["pallets/x/Cargo.toml"])[0]
        self.assertIn("unwired_crate", self.kinds(analysis))

    def test_a_path_absent_from_disk_is_critical(self):
        analysis = self.analyze({}, [feature()])[0]
        self.assertIn("missing_path", self.kinds(analysis))
        self.assertEqual(completion.severity_of(analysis), "critical")

    def test_a_path_the_index_cannot_see_is_low_not_critical(self):
        (self.root / "formal-proofs/tla").mkdir(parents=True)
        (self.root / "Makefile").write_text("all:\n")
        analysis = self.analyze({}, [feature(paths=["formal-proofs/tla", "Makefile"])])[0]
        self.assertIn("unindexed_path", self.kinds(analysis))
        self.assertNotEqual(completion.severity_of(analysis), "critical")

    def test_a_required_test_that_is_not_indexed_is_flagged(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs")}
        row = feature(required_tests=["test_lock", "test_claim", "test_refund"])
        analysis = self.analyze(files, [row])[0]
        gap = next(g for g in analysis["gaps"] if g["kind"] == "unsupported_test_claim")
        self.assertIn("3 of 3", gap["detail"])
        self.assertEqual(analysis["evidence"]["required_tests_missing"], 3)

    def test_a_required_test_that_is_indexed_is_not_flagged(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs", items=[
            {"kind": "fn", "name": "test_lock", "line": 9, "test": True}])}
        analysis = self.analyze(files, [feature(required_tests=["test_lock"])])[0]
        self.assertNotIn("unsupported_test_claim", self.kinds(analysis))

    def test_an_ignored_required_test_whose_target_is_gated_is_not_flagged(self):
        (self.root / "node/tests").mkdir(parents=True)
        (self.root / "scripts").mkdir(parents=True)
        (self.root / "scripts/local-ci.sh").write_text(
            "cargo test -p x3-chain-node --test supply_invariant_distributed "
            "-- --ignored --nocapture\n")
        files = {"node/tests/supply_invariant_distributed.rs": source(
            "node/tests/supply_invariant_distributed.rs",
            items=[{"kind": "fn", "name": "supply_is_conserved",
                    "line": 9, "test": True, "ignored": True}])}
        row = feature(paths=["node/tests/supply_invariant_distributed.rs"],
                      required_tests=["supply_is_conserved"])
        analysis = self.analyze(files, [row])[0]
        self.assertNotIn("unsupported_test_claim", self.kinds(analysis))
        self.assertNotIn("ignored_required_test", self.kinds(analysis))

    def test_an_ignored_required_test_no_gate_runs_is_flagged(self):
        (self.root / "node/tests").mkdir(parents=True)
        (self.root / "scripts").mkdir(parents=True)
        (self.root / "scripts/local-ci.sh").write_text("cargo test -p x3-chain-node\n")
        files = {"node/tests/other.rs": source(
            "node/tests/other.rs",
            items=[{"kind": "fn", "name": "long_soak",
                    "line": 3, "test": True, "ignored": True}])}
        row = feature(paths=["node/tests/other.rs"], required_tests=["long_soak"])
        analysis = self.analyze(files, [row])[0]
        self.assertIn("ignored_required_test", self.kinds(analysis))
        self.assertEqual(analysis["evidence"]["required_tests_ignored_ungated"], 1)
        gap = next(g for g in analysis["gaps"] if g["kind"] == "ignored_required_test")
        self.assertIn("skipped test is not a passing test", gap["detail"])

    def test_a_name_ignored_in_one_file_but_live_in_another_is_not_flagged(self):
        # The pack-05 false positive: a frozen extract kept the ignore attribute on
        # tests the live suite runs. The marker belongs to the declaration,
        # not the name, so one un-ignored declaration means the test runs.
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {
            "pallets/x/src/tests.rs": source("pallets/x/src/tests.rs", items=[
                {"kind": "fn", "name": "test_roundtrip", "line": 9, "test": True}]),
            "archive/extract/tests.rs": source("archive/extract/tests.rs", items=[
                {"kind": "fn", "name": "test_roundtrip",
                 "line": 3, "test": True, "ignored": True}]),
        }
        row = feature(paths=["pallets/x/src/tests.rs"], required_tests=["test_roundtrip"])
        analysis = self.analyze(files, [row])[0]
        self.assertNotIn("ignored_required_test", self.kinds(analysis))
        self.assertEqual(analysis["evidence"]["required_tests_ignored_ungated"], 0)

    def test_ignored_in_every_declaration_but_gated_in_one_is_not_flagged(self):
        (self.root / "node/tests").mkdir(parents=True)
        (self.root / "scripts").mkdir(parents=True)
        (self.root / "scripts/local-ci.sh").write_text(
            "cargo test -p x3-chain-node --test supply_invariant_distributed "
            "-- --ignored --nocapture\n")
        files = {
            "node/tests/supply_invariant_distributed.rs": source(
                "node/tests/supply_invariant_distributed.rs",
                items=[{"kind": "fn", "name": "supply_is_conserved",
                        "line": 9, "test": True, "ignored": True}]),
            "archive/old/supply.rs": source("archive/old/supply.rs",
                items=[{"kind": "fn", "name": "supply_is_conserved",
                        "line": 4, "test": True, "ignored": True}]),
        }
        row = feature(paths=["node/tests/supply_invariant_distributed.rs"],
                      required_tests=["supply_is_conserved"])
        analysis = self.analyze(files, [row])[0]
        self.assertNotIn("ignored_required_test", self.kinds(analysis))

    def test_markers_are_reported_but_never_decide_the_state(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs", items=[
            {"kind": "todo", "name": "TODO: finish this", "line": 3},
            {"kind": "fn", "name": "t", "line": 9, "test": True}])}
        analysis = self.analyze(files, [feature()])[0]
        self.assertIn("marker", self.kinds(analysis))
        self.assertEqual(analysis["state"], "TESTED", "a marker does not change the state")

    def test_the_states_are_never_collapsed_into_done(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs")}
        analysis = self.analyze(files, [feature()])[0]
        for state in ("ADVERSARIALLY_TESTED", "VERIFIED", "RELEASE_GATED"):
            self.assertIn(state, analysis["states_not_determinable"])
        self.assertIn(analysis["state"], completion.STATES)
        self.assertNotIn("DONE", completion.STATES)

    def test_the_score_is_the_projects_own_formula(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs")}
        analysis = self.analyze(files, [feature(implemented=75, tested=55, mainnet_ready=60)])[0]
        # 75*.35 + 55*.25 + 60*.40 = 63.75 -> 64, the matrix's own composite.
        self.assertEqual(analysis["score"], 64)

    def test_rank_weights_priority_and_scope_and_admits_no_cost_model(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs")}
        core = self.analyze(files, [feature(priority="P0", launch_scope="core")])[0]
        minor = self.analyze(files, [feature(priority="P3", launch_scope="research")])[0]
        core_score, why = completion.rank_key(core)
        minor_score, _ = completion.rank_key(minor)
        self.assertGreater(core_score, minor_score)
        self.assertIn("not modelled", why["note"])

    def test_a_clean_feature_ranks_zero(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs", items=[
            {"kind": "fn", "name": "t", "line": 1, "test": True}])}
        analysis = self.analyze(files, [feature(mainnet_ready=0)])[0]
        score, _ = completion.rank_key(analysis)
        self.assertEqual(score, 0)

    def test_one_missing_file_explains_many_rows(self):
        rows = [feature(id=f"X3-TEST-00{n}", paths=["pallets/x/src/lib.rs"]) for n in range(1, 4)]
        analyses = self.analyze({}, rows)
        blockers = completion.root_blockers(analyses)
        self.assertEqual(blockers[0]["blocks"], 3)
        self.assertEqual(blockers[0]["kind"], "missing_path")

    def test_the_summary_counts_states_and_severities(self):
        (self.root / "pallets/x/src").mkdir(parents=True)
        files = {"pallets/x/src/lib.rs": source("pallets/x/src/lib.rs", items=[
            {"kind": "fn", "name": "t", "line": 1, "test": True}])}
        summary = completion.scan_summary(self.analyze(files, [feature()]))
        self.assertEqual(summary["features"], 1)
        self.assertEqual(summary["by_state"]["TESTED"], 1)
        self.assertEqual(summary["clean"], 1)


class RealRepositoryTests(unittest.TestCase):
    """The wiring cannot rot: the shipped index and matrix must still work."""

    def analyses(self):
        if not completion.DEFAULT_INDEX.exists():
            self.skipTest("no repository index built")
        matrix = completion.feature_matrix_module()
        return completion.analyze(
            completion.load_index(completion.DEFAULT_INDEX), matrix,
            matrix.load_matrix(completion.DEFAULT_MATRIX)["feature"])

    def test_the_shipped_sources_load_and_analyse(self):
        analyses = self.analyses()
        self.assertGreater(len(analyses), 100, "the matrix should have its full row count")
        for analysis in analyses:
            self.assertIn(analysis["state"], completion.STATES)
            self.assertIsInstance(analysis["score"], int)
            for gap in analysis["gaps"]:
                self.assertTrue(gap.get("kind"))
                self.assertTrue(gap.get("detail"))

    def test_critical_is_reserved_for_a_path_that_is_not_on_disk(self):
        for analysis in self.analyses():
            for gap in analysis["gaps"]:
                if gap["severity"] == "critical":
                    self.assertEqual(gap["kind"], "missing_path")


if __name__ == "__main__":
    unittest.main()
