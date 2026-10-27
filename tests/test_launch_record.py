#!/usr/bin/env python3
"""Tests for `scripts/mainnet/launch_record.py`.

The launch record is what makes a launch decision re-readable: it names the operator, the commit,
the RC log that passed, and the hash of each genesis artifact. The cases that matter are the ones
where it must *refuse* — a spec that is not a mainnet spec, a spec whose authority set is not the
validator set being launched, an RC log that does not carry the gate's own finishing marker, a
missing operator, a dirty tree — and the one that comes after: a record whose genesis has changed
underneath it must fail `verify`.

    python3 tests/test_launch_record.py
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "mainnet" / "launch_record.py"

MARKER = "== X3 MAINNET RC GATE PASSED =="


def spec(authorities: int = 7, chain_type: str = "Live", bootnodes: int = 1) -> dict:
    return {
        "name": "X3 Chain Production",
        "id": "x3_chain_production",
        "chainType": chain_type,
        "bootNodes": [
            f"/dns4/node{i}.example/tcp/30333/p2p/12D3KooWMTqqXaRcNsqDE7kxXKy83YU66gHkQFTdkk4GMYQVm4Gn"
            for i in range(bootnodes)
        ],
        "genesis": {
            "runtimeGenesis": {
                "config": {
                    "aura": {"authorities": ["5" + "D" * 46] * authorities},
                    "grandpa": {"authorities": [["5" + "H" * 46, 1]] * authorities},
                }
            }
        },
    }


class LaunchRecord(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = pathlib.Path(self._tmp.name)
        self.spec = self.root / "chain-specs/x3-mainnet-plain.json"
        self.raw = self.root / "chain-specs/x3-mainnet-raw.json"
        self.log = self.root / "reports/mainnet_rc_gate.log"
        self.out = self.root / "reports/launch/promotion.json"
        self.pack = self.root / "evidence-pack.json"
        self.write_spec()
        raw = spec()
        raw["genesis"] = {"raw": {"top": {}}}
        self.raw.parent.mkdir(parents=True, exist_ok=True)
        self.raw.write_text(json.dumps(raw), encoding="utf-8")
        self.log.parent.mkdir(parents=True, exist_ok=True)
        self.log.write_text(f"checks…\n{MARKER}\n", encoding="utf-8")
        self.write_evidence_fixture()

    def write_spec(self, **kwargs: object) -> None:
        self.spec.parent.mkdir(parents=True, exist_ok=True)
        self.spec.write_text(json.dumps(spec(**kwargs)), encoding="utf-8")

    def write_evidence_fixture(self) -> None:
        capability_rows = []
        for index in range(79):
            capability_rows.append(
                "\n".join(
                    (
                        "[[feature]]",
                        f'id = "X3-P0-{index:03d}"',
                        f'name = "P0 feature {index}"',
                        'subsystem = "runtime_core"',
                        'source = "master"',
                        "implemented = 95",
                        "tested = 95",
                        "mainnet_ready = 95",
                        'priority = "P0"',
                        'confidence = "high"',
                        'blockers = []',
                        'evidence = ["evidence.md"]',
                        'required_tests = ["launch_record_scope"]',
                        'launch_scope = "core"',
                    )
                )
            )
        claim_rows = []
        for index in range(3):
            claim_rows.append(
                "\n".join(
                    (
                        "[[feature]]",
                        f'id = "X3-CLAIM-{index:03d}"',
                        f'name = "Retracted claim {index}"',
                        'subsystem = "claims_hygiene"',
                        'source = "claim_risk"',
                        "implemented = 55",
                        "tested = 25",
                        "mainnet_ready = 35",
                        'priority = "P0"',
                        'confidence = "medium"',
                        "claim_risk = true",
                        'blockers = ["historical claim remains capped"]',
                        'evidence = ["evidence.md"]',
                        'launch_scope = "research"',
                    )
                )
            )
        (self.root / "FEATURE_MATRIX.toml").write_text(
            "[meta]\nversion = 1\nfeature_count_expected = 82\n\n"
            + "\n\n".join(capability_rows + claim_rows)
            + "\n",
            encoding="utf-8",
        )
        (self.root / "evidence.md").write_text("P0 evidence\n", encoding="utf-8")

        evidence = self.root / "evidence"
        evidence.mkdir()
        gate_log = evidence / "gates.log"
        gate_log.write_text(
            "pytest: PASS\nlocal-ci --pre-push: PASS\nmake mainnet-check: PASS\n",
            encoding="utf-8",
        )
        audit_runtime = evidence / "audit-runtime.txt"
        audit_runtime.write_text("independent runtime and consensus audit\n", encoding="utf-8")
        audit_bridges = evidence / "audit-bridges.txt"
        audit_bridges.write_text("independent bridges and operations audit\n", encoding="utf-8")
        bounty = evidence / "bounty.txt"
        bounty.write_text("100000 USD pool active\n", encoding="utf-8")
        testnet = evidence / "public-testnet.txt"
        testnet.write_text("50 validators, 5 continents, 8 weeks\n", encoding="utf-8")

        bridge = evidence / "bridges.json"
        bridge.write_text(
            json.dumps(
                {
                    "active_routes": [
                        "ethereum-mainnet",
                        "solana-mainnet-beta",
                        "bitcoin-mainnet",
                    ],
                    "per_transfer_limit_usd": 1000,
                    "daily_per_route_limit_usd": 10000,
                    "total_exposure_limit_usd": 50000,
                    "kill_switch": True,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        validators = evidence / "validators.json"
        validators.write_text(
            json.dumps(
                {
                    "validators": [
                        {
                            "host": f"validator-{index}.example",
                            "operator": f"operator-{index}",
                            "identity": f"identity-{index}",
                            "loopback": False,
                        }
                        for index in range(7)
                    ]
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        approvals = []
        for role in ("release", "security", "operations"):
            path = evidence / f"approval-{role}.txt"
            path.write_text(f"{role} approved deadbeef\n", encoding="utf-8")
            approvals.append(
                {
                    "approver": f"{role}-approver",
                    "role": role,
                    "approved_commit": "deadbeef",
                    "evidence": str(path.relative_to(self.root)),
                }
            )

        self.pack.write_text(
            json.dumps(
                {
                    "feature_matrix": {"path": "FEATURE_MATRIX.toml"},
                    "test_gates": [
                        {
                            "name": "release",
                            "command": "make mainnet-check",
                            "status": "PASS",
                            "exit_code": 0,
                            "log": str(gate_log.relative_to(self.root)),
                        }
                    ],
                    "audits": [
                        {
                            "provider": "Auditor A",
                            "scope": ["runtime", "consensus", "atomic"],
                            "report": str(audit_runtime.relative_to(self.root)),
                            "critical_open": 0,
                            "high_open": 0,
                        },
                        {
                            "provider": "Auditor B",
                            "scope": ["bridges", "contracts", "operations"],
                            "report": str(audit_bridges.relative_to(self.root)),
                            "critical_open": 0,
                            "high_open": 0,
                        },
                    ],
                    "bounty_window": {
                        "url": "https://bounty.example/x3",
                        "pool_usd": 100000,
                        "starts_at": "2026-08-01T00:00:00Z",
                        "ends_at": "2099-01-01T00:00:00Z",
                        "evidence": str(bounty.relative_to(self.root)),
                    },
                    "public_testnet_window": {
                        "started_at": "2026-06-01T00:00:00Z",
                        "ended_at": "2026-08-01T00:00:00Z",
                        "validator_count": 50,
                        "continents": 5,
                        "evidence": str(testnet.relative_to(self.root)),
                    },
                    "bridge_configuration": {
                        "path": str(bridge.relative_to(self.root)),
                    },
                    "validators": {
                        "path": str(validators.relative_to(self.root)),
                    },
                    "approvals": approvals,
                },
                indent=2,
                sort_keys=True,
            ),
            encoding="utf-8",
        )

    def edit_pack(self, mutator) -> None:
        data = json.loads(self.pack.read_text())
        mutator(data)
        self.pack.write_text(json.dumps(data, indent=2, sort_keys=True), encoding="utf-8")

    def run_record(self, *extra: str, command: str = "build") -> subprocess.CompletedProcess:
        argv = ["python3", str(SCRIPT), "--root", str(self.root), command]
        if command == "build":
            argv += [
                "--spec",
                str(self.spec),
                "--raw",
                str(self.raw),
                "--rc-log",
                str(self.log),
                "--commit",
                "deadbeef",
                "--approved-by",
                "release-approver",
                "--evidence-pack",
                str(self.pack),
                "--out",
                str(self.out),
            ]
        else:
            argv += ["--record", str(self.out)]
        return subprocess.run(
            argv + list(extra), capture_output=True, text=True, timeout=120
        )

    def test_a_complete_promotion_is_recorded(self) -> None:
        result = self.run_record()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        record = json.loads(self.out.read_text())
        self.assertEqual(record["approved_by"], "release-approver")
        self.assertEqual(record["commit"], "deadbeef")
        self.assertEqual(record["expect_authorities"], 7)
        self.assertEqual(record["schema"], 2)
        self.assertEqual(
            set(record),
            {
                "schema",
                "recorded_at",
                "record_sha256",
                "approved_by",
                "commit",
                "expect_authorities",
                "rc_gate",
                "artifacts",
                "evidence_pack",
                "feature_matrix",
                "p0_snapshot",
                "test_gates",
                "audits",
                "bounty_window",
                "public_testnet_window",
                "bridge_configuration",
                "validators",
                "approvals",
            },
        )
        self.assertEqual(sorted(a["role"] for a in record["artifacts"]), ["mainnet-plain", "mainnet-raw"])
        self.assertTrue(record["artifacts"][0]["sha256"])
        self.assertEqual(record["rc_gate"]["passed_marker"], MARKER)
        self.assertEqual(record["feature_matrix"]["p0_count"], 82)
        self.assertEqual(record["feature_matrix"]["capability_count"], 79)
        self.assertEqual(record["feature_matrix"]["claim_hygiene_count"], 3)
        self.assertEqual(len(record["p0_snapshot"]["rows"]), 82)
        self.assertEqual(record["p0_snapshot"]["capability_count"], 79)
        self.assertEqual(record["p0_snapshot"]["claim_hygiene_count"], 3)
        self.assertEqual(len(record["audits"]), 2)
        self.assertEqual(len(record["validators"]["entries"]), 7)
        self.assertEqual(len(record["approvals"]), 3)
        self.assertEqual(len(record["record_sha256"]), 64)
    def test_a_capability_at_94_is_refused(self) -> None:
        matrix = self.root / "FEATURE_MATRIX.toml"
        matrix.write_text(
            matrix.read_text(encoding="utf-8").replace(
                'id = "X3-P0-001"\n'
                'name = "P0 feature 1"\n'
                'subsystem = "runtime_core"\n'
                'source = "master"\n'
                "implemented = 95",
                'id = "X3-P0-001"\n'
                'name = "P0 feature 1"\n'
                'subsystem = "runtime_core"\n'
                'source = "master"\n'
                "implemented = 94",
                1,
            ),
            encoding="utf-8",
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("X3-P0-001.implemented is 94", result.stderr)

    def test_capability_and_claim_hygiene_counts_cannot_drift(self) -> None:
        matrix = self.root / "FEATURE_MATRIX.toml"
        matrix.write_text(
            matrix.read_text(encoding="utf-8").replace(
                'subsystem = "runtime_core"',
                'subsystem = "claims_hygiene"',
                1,
            ),
            encoding="utf-8",
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("78 capability P0 rows", result.stderr)

    def test_one_audit_report_cannot_pool_both_scope_groups(self) -> None:
        self.edit_pack(
            lambda data: (
                data["audits"][0].update(
                    {
                        "scope": [
                            "runtime",
                            "consensus",
                            "atomic",
                            "bridges",
                            "contracts",
                            "operations",
                        ]
                    }
                ),
                data["audits"][1].update(
                    {"scope": ["bridges", "contracts", "operations"]}
                ),
            )
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "independent scope groups cannot be pooled into one report",
            result.stderr,
        )

    def test_a_spec_for_a_different_validator_set_is_refused(self) -> None:
        self.write_spec(authorities=3)
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Aura authorities", result.stderr)
        self.assertFalse(self.out.exists(), "a refused promotion must not leave a record")

    def test_a_non_live_spec_is_refused(self) -> None:
        self.write_spec(chain_type="Local")
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not 'Live'", result.stderr)

    def test_an_rc_log_without_the_marker_is_refused(self) -> None:
        self.log.write_text("checks…\nFAILED: something\n", encoding="utf-8")
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("PASSED", result.stderr)

    def test_a_missing_operator_is_refused(self) -> None:
        result = self.run_record("--approved-by", "   ")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("operator", result.stderr)

    def test_a_missing_artifact_is_refused(self) -> None:
        self.raw.unlink()
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("raw spec not found", result.stderr)

    def test_a_dirty_tree_is_refused(self) -> None:
        repo = self.root / "git-tree"
        repo.mkdir()
        (repo / "tracked.txt").write_text("original\n", encoding="utf-8")
        env = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.com",
               "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.com"}
        for argv in (
            ["git", "init", "-q"],
            ["git", "add", "tracked.txt"],
            ["git", "commit", "-q", "-m", "init"],
        ):
            subprocess.run(argv, cwd=repo, env=env, check=True, capture_output=True)
        (repo / "tracked.txt").write_text("changed\n", encoding="utf-8")
        result = self.run_record()
        self.assertEqual(result.returncode, 0, "the fixture tree outside git is fine")

        dirty = subprocess.run(
            ["python3", str(SCRIPT), "--root", str(repo), "build",
             "--spec", str(self.spec), "--raw", str(self.raw), "--rc-log", str(self.log),
             "--evidence-pack", str(self.pack), "--approved-by", "release-approver",
             "--out", str(repo / "out.json")],
            capture_output=True, text=True, timeout=120,
        )
        self.assertNotEqual(dirty.returncode, 0)
        self.assertIn("working tree is dirty", dirty.stderr)

    def test_verify_detects_a_changed_genesis(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        self.assertEqual(self.run_record(command="verify").returncode, 0)
        self.write_spec(authorities=7, bootnodes=2)  # same shape, different bytes
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("sha256", result.stderr)

    def test_verify_detects_a_changed_rc_log(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        self.log.write_text(f"{MARKER}\nplus a line nobody recorded\n", encoding="utf-8")
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("RC log has changed", result.stderr)

    def test_verify_detects_a_changed_feature_matrix(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        matrix = self.root / "FEATURE_MATRIX.toml"
        matrix.write_text(matrix.read_text() + "\n# changed after approval\n", encoding="utf-8")
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("feature_matrix", result.stderr)

    def test_a_missing_p0_snapshot_is_refused(self) -> None:
        matrix = self.root / "FEATURE_MATRIX.toml"
        matrix.write_text(
            matrix.read_text().replace('priority = "P0"', 'priority = "P1"', 1),
            encoding="utf-8",
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("launch requires 82", result.stderr)

    def test_a_missing_feature_matrix_reference_is_refused(self) -> None:
        self.edit_pack(lambda data: data.pop("feature_matrix"))
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("feature_matrix", result.stderr)

    def test_verify_detects_a_changed_test_gate_log(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        (self.root / "evidence/gates.log").write_text("PASS\nFAIL\n", encoding="utf-8")
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("test_gates", result.stderr)

    def test_a_failed_test_gate_is_refused(self) -> None:
        self.edit_pack(
            lambda data: data["test_gates"][0].update(
                {"status": "FAIL", "exit_code": 1}
            )
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("test gate release", result.stderr)

    def test_verify_detects_a_changed_audit_report(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        (self.root / "evidence/audit-runtime.txt").write_text(
            "edited audit report\n", encoding="utf-8"
        )
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("audits", result.stderr)

    def test_two_audit_providers_and_scopes_are_required(self) -> None:
        self.edit_pack(lambda data: data.pop("audits"))
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("audits", result.stderr)

    def test_a_too_short_bounty_window_is_refused(self) -> None:
        self.edit_pack(
            lambda data: data["bounty_window"].update(
                {"starts_at": "2026-10-20T00:00:00Z"}
            )
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("bounty window", result.stderr)

    def test_a_too_short_public_testnet_window_is_refused(self) -> None:
        self.edit_pack(
            lambda data: data["public_testnet_window"].update(
                {"ended_at": "2026-07-01T00:00:00Z"}
            )
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("public-testnet window", result.stderr)

    def test_verify_detects_changed_bridge_configuration(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        (self.root / "evidence/bridges.json").write_text(
            json.dumps(
                {
                    "active_routes": ["ethereum-mainnet"],
                    "per_transfer_limit_usd": 1000,
                    "daily_per_route_limit_usd": 10000,
                    "total_exposure_limit_usd": 50000,
                    "kill_switch": True,
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("bridge_configuration", result.stderr)

    def test_duplicate_bridge_routes_are_refused(self) -> None:
        bridge = self.root / "evidence/bridges.json"
        config = json.loads(bridge.read_text())
        config["active_routes"].append("ethereum-mainnet")
        bridge.write_text(json.dumps(config, sort_keys=True), encoding="utf-8")

        result = self.run_record()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("bridge_configuration", result.stderr)

    def test_verify_detects_changed_validator_evidence(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        evidence = json.loads((self.root / "evidence/validators.json").read_text())
        evidence["validators"][0]["host"] = "loopback"
        (self.root / "evidence/validators.json").write_text(
            json.dumps(evidence, sort_keys=True), encoding="utf-8"
        )
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("validators", result.stderr)

    def test_a_loopback_validator_host_is_refused(self) -> None:
        path = self.root / "evidence/validators.json"
        evidence = json.loads(path.read_text())
        evidence["validators"][0].update(host="127.0.0.1", loopback=False)
        path.write_text(json.dumps(evidence, sort_keys=True), encoding="utf-8")

        result = self.run_record()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("loopback validator", result.stderr)

    def test_an_approval_for_another_commit_is_refused(self) -> None:
        self.edit_pack(
            lambda data: data["approvals"][0].update(
                {"approved_commit": "feedface"}
            )
        )
        result = self.run_record()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("approval release-approver", result.stderr)

    def test_a_tampered_record_digest_is_refused(self) -> None:
        self.assertEqual(self.run_record().returncode, 0)
        record = json.loads(self.out.read_text())
        record["record_sha256"] = "0" * 64
        self.out.write_text(json.dumps(record, sort_keys=True), encoding="utf-8")
        result = self.run_record(command="verify")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("record_sha256", result.stderr)

    def test_the_record_is_cited_by_the_launch_gate_row(self) -> None:
        registry = (ROOT / "FEATURE_REGISTRY.toml").read_text(encoding="utf-8")
        row = registry.split("[launch_gate]", 1)[1].split("\n[", 1)[0]
        self.assertIn("launch_record.py", row)


def main() -> int:
    loader = unittest.TestLoader()
    suite = loader.loadTestsFromTestCase(LaunchRecord)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
