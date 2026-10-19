import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[2] / "scripts/funding/stage_legacy_catalog.py"
spec = importlib.util.spec_from_file_location("stage_legacy_catalog", MODULE)
stage_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage_module)


def row(identifier="C27.1.1"):
    return {
        "legacy_id": identifier,
        "name": "Archive research",
        "category": "C27-chain-grants",
        "source_file": "grants/C27-chain-grants.md",
        "source_line": 14,
        "historical_status": "RESEARCH",
        "historical_verification_date": None,
        "raw_fields": ["Research", "https://example.org"],
        "candidate_urls": ["https://example.org"],
        "origin": "legacy_markdown_research",
        "current_availability": "UNKNOWN",
        "current_eligibility": "UNKNOWN",
        "application_authorized": False,
    }


class StageCatalogTests(unittest.TestCase):
    def test_requires_unknown_and_unapproved(self):
        self.assertEqual(stage_module.validate_record(row())["legacy_id"], "C27.1.1")
        forged = row()
        forged["application_authorized"] = True
        with self.assertRaisesRegex(ValueError, "assert live"):
            stage_module.validate_record(forged)
        forged = row()
        forged["current_availability"] = "OPEN"
        with self.assertRaisesRegex(ValueError, "assert live"):
            stage_module.validate_record(forged)

    def test_endpoint_requires_loopback_or_https(self):
        good = "http://127.0.0.1:3000/api/v1/admin/funding-swarm/legacy-leads"
        self.assertEqual(stage_module.validate_endpoint(good), good)
        self.assertEqual(
            stage_module.validate_endpoint("https://funding.example.org/api/v1/admin/funding-swarm/legacy-leads"),
            "https://funding.example.org/api/v1/admin/funding-swarm/legacy-leads",
        )
        for bad in ("http://192.168.0.30:3000/api/v1/admin/funding-swarm/legacy-leads",
                    "file:///api/v1/admin/funding-swarm/legacy-leads",
                    "https://user:pass@example.org/api/v1/admin/funding-swarm/legacy-leads",
                    "http://localhost:3000/api/v1/public/funding-swarm/grants"):
            with self.subTest(url=bad), self.assertRaises(ValueError):
                stage_module.validate_endpoint(bad)

    def test_reads_jsonl_and_batches_without_network(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "archive.jsonl"
            src.write_text("".join(json.dumps(row(f"C27.1.{i}")) + "\n" for i in range(5)),
                           encoding="utf-8")
            records = list(stage_module.read_catalog(src))
            groups = list(stage_module.batches(iter(records), 2))
            self.assertEqual([len(g) for g in groups], [2, 2, 1])

    def test_rejects_bad_record_even_in_preview(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "archive.jsonl"
            bad = row()
            bad["current_eligibility"] = "ELIGIBLE"
            src.write_text(json.dumps(bad) + "\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                list(stage_module.read_catalog(src))

    def test_request_has_admin_token_and_zero_live_claims(self):
        endpoint = "http://localhost:3000/api/v1/admin/funding-swarm/legacy-leads"
        response = io.BytesIO(json.dumps({
            "inserted": 1, "unchanged": 0, "total": 1,
            "current_opportunities_verified": 0,
        }).encode("utf-8"))
        with patch.object(stage_module, "urlopen", return_value=response) as send:
            result = stage_module.stage([row()], endpoint, "dummy-token")
        self.assertEqual(result["inserted"], 1)
        request = send.call_args.args[0]
        self.assertEqual(json.loads(request.data)["leads"][0]["legacy_id"], "C27.1.1")
        self.assertEqual(request.get_header("X-admin-token"), "dummy-token")

    def test_rejects_gateway_award_claim(self):
        response = io.BytesIO(json.dumps({
            "inserted": 1, "unchanged": 0, "total": 1,
            "current_opportunities_verified": 1,
        }).encode("utf-8"))
        with patch.object(stage_module, "urlopen", return_value=response):
            with self.assertRaisesRegex(ValueError, "inconsistent"):
                stage_module.stage([row()],
                    "http://localhost:3000/api/v1/admin/funding-swarm/legacy-leads", "dummy")


if __name__ == "__main__":
    unittest.main()
