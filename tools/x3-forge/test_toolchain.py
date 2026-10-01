#!/usr/bin/env python3
"""Tests for the external toolchain inventory (PHASE 0).

Two of these pin bugs that made the inventory lie about the whole catalogue:

* the catalogue's own file counted as evidence for every tool it lists, so all
  66 were reported GATED and every other state was empty — the inventory was
  measuring itself;
* every tool was checked with `which`, so crate libraries that are legitimately
  depended on (`proptest`, `revm`) were reported not installed.

The rest pin the rule the master prompt states outright: an installed
executable does not count as implemented, so nothing here may upgrade a tool
past WIRED on the strength of a mention.
"""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "toolchain", Path(__file__).with_name("toolchain.py"))
toolchain = importlib.util.module_from_spec(spec)
spec.loader.exec_module(toolchain)


class CorpusTests(unittest.TestCase):
    def test_hidden_directories_are_skipped_but_github_is_kept(self):
        self.assertTrue(toolchain._skip(".wt-agent"))
        self.assertTrue(toolchain._skip(".git"))
        self.assertTrue(toolchain._skip("target"))
        self.assertFalse(toolchain._skip(".github"))
        self.assertFalse(toolchain._skip(".cargo"))
        self.assertFalse(toolchain._skip("scripts"))

    def test_worktree_copies_are_not_walked(self):
        """They are full copies of the repo; walking them tripled the runtime."""
        for part in (".wt-agent", ".wt-matrix", ".wt-baseline", ".roo"):
            self.assertTrue(toolchain._skip(part), part)

    def test_evidence_is_graded_by_where_it_was_found(self):
        self.assertEqual(toolchain.evidence_strength("Cargo.toml"), "dependency")
        self.assertEqual(toolchain.evidence_strength(".github/workflows/x.yml"), "workflow")
        self.assertEqual(toolchain.evidence_strength("scripts/gate.sh"), "script")
        self.assertEqual(toolchain.evidence_strength("crates/x/src/lib.rs"), "source")
        self.assertEqual(toolchain.evidence_strength("docs/README.md"), "prose")


class ClassificationTests(unittest.TestCase):
    def classify(self, name, terms, executable=None, corpus=None, hints=(), recorded=None):
        # `recorded` defaults to nothing here even though production scans the
        # live evidence directories: a unit test must not let this repository's
        # artifacts answer for the fixture.
        return toolchain.classify(name, terms, executable, corpus or {}, hints,
                                  recorded=[] if recorded is None else recorded)

    def test_a_library_dependency_counts_as_installed(self):
        row = self.classify("proptest", ("proptest",), None, {"Cargo.lock": "proptest 1.0"})
        self.assertEqual(row["kind"], "crate")
        self.assertTrue(row["installed"])
        self.assertEqual(row["installed_path"], "Cargo.lock")

    def test_a_crate_that_is_mentioned_but_not_depended_on_is_not_installed(self):
        row = self.classify("revm", ("revm",), None, {"docs/x.md": "we could use revm"})
        self.assertFalse(row["installed"])
        self.assertTrue(row["referenced_only"])

    def test_a_reference_project_has_no_installation_state(self):
        row = self.classify("malachite", ("malachite",), None, {"docs/x.md": "malachite"})
        self.assertEqual(row["kind"], "reference")
        self.assertIsNone(row["installed"])

    def test_a_workflow_mention_gates_the_tool(self):
        row = self.classify("kani", ("kani",), None,
                            {".github/workflows/formal-verification.yml": "cargo kani"})
        self.assertTrue(row["gated"])
        self.assertEqual(toolchain.status_of(row), "GATED")

    def test_a_bare_prose_mention_never_reaches_wired_or_gated(self):
        row = self.classify("k6", ("k6",), None, {"docs/plan.md": "we might use k6"})
        self.assertFalse(row["wired"])
        self.assertFalse(row["gated"])
        self.assertNotIn(toolchain.status_of(row), ("WIRED", "GATED"))

    def test_an_installed_executable_with_no_wiring_is_only_installed(self):
        row = self.classify("k6", ("k6",), "python3", {})
        self.assertTrue(row["installed"])
        self.assertEqual(toolchain.status_of(row), "INSTALLED_ONLY")

    def test_nothing_here_claims_a_tool_was_exercised(self):
        row = self.classify("kani", ("kani",), None,
                            {".github/workflows/formal-verification.yml": "cargo kani"})
        for state in ("exercised", "repeatable"):
            self.assertEqual(row[state], "not_determinable", state)
        # `evidenced` is the artifact-based rung: a bool, False with no
        # recorded run, and still never an EXERCISED/VERIFIED claim.
        self.assertIs(row["evidenced"], False)
        self.assertTrue(row["why_not_determinable"])

    def test_a_recorded_artifact_is_evidenced_never_verified(self):
        recorded = [{"file": "reports/x.json", "term": "k6", "mtime": 1}]
        row = self.classify("k6", ("k6",), "python3", {}, recorded=recorded)
        self.assertEqual(toolchain.status_of(row), "EVIDENCED")
        self.assertEqual(row["exercised"], "not_determinable")
        self.assertEqual(row["repeatable"], "not_determinable")
        # A gate is the stronger claim and stays on top of the ladder.
        gated = self.classify("kani", ("kani",), None,
                              {".github/workflows/formal-verification.yml": "cargo kani"},
                              recorded=recorded)
        self.assertEqual(toolchain.status_of(gated), "GATED")

    def test_the_status_ladder_is_ordered(self):
        self.assertEqual(toolchain.status_of({"gated": True, "wired": True, "configured": True,
                                              "installed": True, "available": True, "evidenced": False,
                                              "referenced_only": False}), "GATED")
        self.assertEqual(toolchain.status_of({"gated": False, "wired": True, "configured": True,
                                              "installed": True, "available": True, "evidenced": False,
                                              "referenced_only": False}), "WIRED")
        self.assertEqual(toolchain.status_of({"gated": False, "wired": False, "configured": False,
                                              "installed": True, "available": True, "evidenced": True,
                                              "referenced_only": False}), "EVIDENCED")
        self.assertEqual(toolchain.status_of({"gated": False, "wired": False, "configured": True,
                                              "installed": True, "available": True, "evidenced": False,
                                              "referenced_only": False}), "CONFIGURED")
        self.assertEqual(toolchain.status_of({"gated": False, "wired": False, "configured": False,
                                              "installed": False, "available": False, "evidenced": False,
                                              "referenced_only": False}), "MISSING")


class RealRepositoryTests(unittest.TestCase):
    """The scan must not count itself, and must keep its provenance."""

    @classmethod
    def setUpClass(cls):
        cls.rows, cls.corpus = toolchain.build()

    def test_the_catalogue_file_is_not_in_its_own_corpus(self):
        for name in toolchain.SELF_FILES:
            self.assertNotIn(name, self.corpus,
                             "the catalogue cannot be evidence for the tools it lists")

    def test_every_catalogue_entry_is_classified(self):
        self.assertEqual(len(self.rows), len(toolchain.CATALOG))
        statuses = {row["status"] for row in self.rows}
        self.assertTrue(statuses <= {"GATED", "EVIDENCED", "WIRED", "CONFIGURED", "INSTALLED_ONLY",
                                     "REFERENCE_ONLY", "AVAILABLE", "MISSING"}, statuses)

    def test_every_wiring_claim_carries_a_file_and_a_line(self):
        for row in self.rows:
            if not row["wired"]:
                continue
            self.assertTrue(row["evidence"], row["tool"])
            for hit in row["evidence"]:
                self.assertTrue(hit["file"])
                self.assertGreater(hit["line"], 0)
                self.assertIn(hit["strength"],
                              {"dependency", "workflow", "script", "infrastructure",
                               "source", "prose"})

    def test_the_missing_set_is_not_empty_and_not_everything(self):
        """If everything is MISSING the matcher is broken; if nothing is, it is too loose."""
        statuses = [row["status"] for row in self.rows]
        self.assertIn("MISSING", statuses)
        self.assertLess(statuses.count("MISSING"), len(statuses))


if __name__ == "__main__":
    unittest.main()
