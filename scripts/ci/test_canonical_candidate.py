"""Fail-closed tests for public-testnet candidate provenance and scope."""
import copy
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import verify_canonical_candidate as candidate  # noqa: E402

HEAD = "a" * 40


def policy():
    return json.loads(candidate.MANIFEST.read_text(encoding="utf-8"))


class CanonicalCandidateTests(unittest.TestCase):
    def test_manifest_freezes_36_imports_and_eight_real_proof_obligations(self):
        spec = policy()
        files = candidate.validate_manifest(spec)
        self.assertEqual(len(files), 36)
        self.assertEqual({x["gate"] for x in spec["required_proofs"]}, candidate.REQUIRED_PROOFS)
        self.assertFalse(spec["release_authorized"])

    def test_older_automation_code_cannot_be_overlaid(self):
        spec = policy()
        for item in spec["source_overlays"]:
            if item["pr"] == 701:
                item["files"].append({
                    "path": "pallets/x3-automation/src/lib.rs",
                    "blob_sha": "b" * 40,
                })
        with self.assertRaisesRegex(ValueError, "precisely 36|older automation"):
            candidate.validate_manifest(spec)

    def test_duplicate_source_identity_is_rejected(self):
        spec = policy()
        spec["source_overlays"][1]["pr"] = spec["source_overlays"][0]["pr"]
        with self.assertRaisesRegex(ValueError, "duplicate"):
            candidate.validate_manifest(spec)

    def test_overlapping_files_are_rejected(self):
        spec = policy()
        spec["source_overlays"][1]["files"][0]["path"] = spec["source_overlays"][0]["files"][0]["path"]
        with self.assertRaisesRegex(ValueError, "multiple sources"):
            candidate.validate_manifest(spec)

    def test_manifest_rejects_wrong_release_profile_or_enabled_bridges(self):
        for change in ({"release_profile": "mainnet-v1"},
                       {"external_bridges_must_remain_disabled": False},
                       {"release_authorized": True}):
            with self.subTest(change=change):
                spec = policy()
                spec.update(change)
                with self.assertRaises(ValueError):
                    candidate.validate_manifest(spec)

    def test_unsafe_paths_and_hashes_do_not_pass(self):
        for raw in ("../Cargo.lock", "/etc/passwd", "a/./b", "a//b", "a\\b"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                candidate.safe_path(raw)
        with self.assertRaises(ValueError):
            candidate.require_sha("not-a-sha", "test")

    def test_missing_economic_or_real_execution_proof_cannot_be_removed(self):
        spec = policy()
        spec["required_proofs"] = [
            x for x in spec["required_proofs"] if x["gate"] != "validator_economics"
        ]
        with self.assertRaisesRegex(ValueError, "missing or duplicate"):
            candidate.validate_manifest(spec)

    def test_security_ancestor_is_mandatory_even_if_every_file_matches(self):
        spec = policy()
        with (
            patch.object(candidate, "git", return_value=type("Result", (), {
                "returncode": 0, "stdout": HEAD + "\n",
            })()),
            patch.object(candidate, "git_ancestor", side_effect=[True, False]),
            patch.object(candidate, "verify_source_blobs", return_value=[]),
            patch.object(candidate, "verify_bridge_feature_scope", return_value=[]),
        ):
            report = candidate.evaluate(spec, Path("/tmp/not-used"), HEAD)
        self.assertEqual(report["decision"], "BLOCKED")
        self.assertTrue(any("security baseline" in x for x in report["blockers"]))
        self.assertFalse(report["release_authorized"])

    def test_all_static_sources_match_only_source_identity_not_release(self):
        spec = policy()
        with (
            patch.object(candidate, "git", return_value=type("Result", (), {
                "returncode": 0, "stdout": HEAD + "\n",
            })()),
            patch.object(candidate, "git_ancestor", return_value=True),
            patch.object(candidate, "verify_source_blobs", return_value=[]),
            patch.object(candidate, "verify_bridge_feature_scope", return_value=[]),
        ):
            report = candidate.evaluate(spec, Path("/tmp/not-used"), HEAD)
        self.assertEqual(report["decision"], "SOURCE_IDENTITY_ONLY")
        for key in ("release_authorized", "runtime_atomicity_verified",
                    "stateful_evm_verified", "onchain_svm_verified",
                    "live_validator_fleet_verified", "independent_audits_verified"):
            self.assertFalse(report[key])

    def test_mainnet_rc1_forbids_external_gateway(self):
        valid = [
            {"features": {"mainnet-rc1": ["x3-chain-runtime/mainnet-rc1"]}},
            {"features": {"mainnet-rc1": ["pallet-x3-cross-vm-router/mainnet-rc1"]}},
            {"features": {"mainnet-rc1": ["internal-ixl"],
                          "mainnet-v1": ["mainnet-rc1", "external-gateway"]}},
        ]
        with patch.object(candidate, "toml_document", side_effect=valid):
            self.assertEqual(candidate.verify_bridge_feature_scope(Path("/tmp/unneeded")), [])
        valid[1]["features"]["mainnet-rc1"] = ["external-gateway"]
        with patch.object(candidate, "toml_document", side_effect=valid):
            self.assertTrue(candidate.verify_bridge_feature_scope(Path("/tmp/unneeded")))

    def test_corrupted_source_file_never_passes(self):
        spec = policy()
        with (
            patch.object(candidate, "git", return_value=type("Result", (), {
                "returncode": 0, "stdout": HEAD + "\n",
            })()),
            patch.object(candidate, "git_ancestor", return_value=True),
            patch.object(candidate, "verify_source_blobs", return_value=["source blob changed"]),
            patch.object(candidate, "verify_bridge_feature_scope", return_value=[]),
        ):
            report = candidate.evaluate(spec, Path("/tmp/not-used"), HEAD)
        self.assertEqual(report["decision"], "BLOCKED")


if __name__ == "__main__":
    unittest.main()
