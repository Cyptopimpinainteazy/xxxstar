import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[2] / "scripts/funding/legacy_catalog.py"
spec = importlib.util.spec_from_file_location("legacy_catalog", MODULE)
legacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(legacy)


class CatalogTests(unittest.TestCase):
    def test_excludes_category_rows_and_headers(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "grants"
            root.mkdir()
            (root / "C27-chain-grants.md").write_text(
                "| C27.1 | Polkadot / Substrate |\n"
                "| ID | Program | Covers | Award | Eligibility | URL | Notes | Status | Confidence |\n"
                "| C27.1.1 | Example program | Tooling | Variable | Unknown | https://example.org/grants | Verified 2026-09-05 | RESEARCH | 2 |\n",
                encoding="utf-8",
            )
            result = legacy.export(root)
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0]["legacy_id"], "C27.1.1")
            self.assertEqual(result[0]["historical_verification_date"], "2026-09-05")
            self.assertEqual(result[0]["current_availability"], "UNKNOWN")
            self.assertEqual(result[0]["current_eligibility"], "UNKNOWN")

    def test_defunct_is_not_rewritten_as_open(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "grants"
            root.mkdir()
            (root / "GRANTS_DATABASE.md").write_text(
                "| C4.1 | Defunct Fund | Grants | Previously up to $20k | Unknown | https://example.org | General grants discontinued | NOT-A-GRANT | 1 |\n"
                "| C1.3 | Cloud Credit | Credits | $100 | Unknown | https://example.org/startups | Research | RESEARCH | 3 |\n",
                encoding="utf-8",
            )
            records = legacy.export(root)
            self.assertEqual([r["historical_status"] for r in records], ["NOT-A-GRANT", "RESEARCH"])
            self.assertTrue(all(r["application_authorized"] is False for r in records))

    def test_c25_alphanumeric_and_false_award_guard(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "grants"
            root.mkdir()
            (root / "C25-vc-accelerators.md").write_text(
                "| C25.A1 | Example VC | Equity | Negotiated | Startup | https://example.org/contact | Source | QUALIFIED | 5 |\n",
                encoding="utf-8",
            )
            r = legacy.export(root)[0]
            self.assertEqual(r["legacy_id"], "C25.A1")
            self.assertEqual(r["current_availability"], "UNKNOWN")
            self.assertEqual(r["current_eligibility"], "UNKNOWN")
            self.assertEqual(r["historical_status"], "QUALIFIED")

    def test_same_file_duplicate_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "grants"
            root.mkdir()
            row = "| C1.1 | Example | C | A | E | https://example.org | note | RESEARCH | 4 |\n"
            (root / "GRANTS_DATABASE.md").write_text(row * 2, encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "Duplicate legacy ID"):
                legacy.export(root)


if __name__ == "__main__":
    unittest.main()
