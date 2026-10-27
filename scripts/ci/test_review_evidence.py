"""Review output adapter cannot turn AI prose into proof of clean code."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("review_evidence", Path(__file__).with_name("review_evidence.py"))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
SHA = "a" * 40


class ReviewEvidenceTests(unittest.TestCase):
    def test_extracts_source_location_only_as_unverified_candidate(self):
        evidence = mod.bridge(683, SHA, "Possible regression at pallets/dex/src/lib.rs:123", 0)
        self.assertEqual(evidence["findings"][0]["path"], "pallets/dex/src/lib.rs")
        self.assertEqual(evidence["findings"][0]["verification"], "UNVERIFIED")
        self.assertFalse(evidence["review_coverage_complete"])
        self.assertNotIn("Possible regression", str(evidence))

    def test_clean_sounding_output_does_not_prove_coverage(self):
        evidence = mod.bridge(683, SHA, "No major issues detected", 0)
        self.assertTrue(evidence["review_completed"])
        self.assertEqual(evidence["findings"], [])
        self.assertFalse(evidence["review_coverage_complete"])
        self.assertFalse(evidence["release_authorized"])

    def test_failed_process_cannot_complete(self):
        evidence = mod.bridge(683, SHA, "pallets/dex/src/lib.rs:12", 1)
        self.assertFalse(evidence["review_completed"])
        self.assertEqual(evidence["findings"], [])

    def test_empty_tool_artifact_cannot_complete(self):
        evidence = mod.bridge(683, SHA, "(the tool ran; nothing to publish)", 0)
        self.assertFalse(evidence["review_completed"])

    def test_duplicate_locations_deduplicate(self):
        evidence = mod.bridge(683, SHA, "pallets/dex/src/lib.rs:12 pallets/dex/src/lib.rs:12", 0)
        self.assertEqual(len(evidence["findings"]), 1)

    def test_invalid_identity_rejected(self):
        with self.assertRaises(ValueError):
            mod.bridge(683, "short", "text", 0)


if __name__ == "__main__":
    unittest.main()
