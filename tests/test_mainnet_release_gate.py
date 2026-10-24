from __future__ import annotations

import importlib.util
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GATE_PATH = ROOT / "scripts" / "mainnet_release_gate.py"


def load_gate():
    spec = importlib.util.spec_from_file_location("x3_mainnet_release_gate", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


gate = load_gate()


def p0_feature(index: int, **overrides):
    feature = {
        "id": f"X3-P0-{index:03d}",
        "name": f"P0 feature {index}",
        "subsystem": "runtime_core",
        "paths": ["src/p0.rs"],
        "test_paths": ["src/p0.rs"],
        "source": "master",
        "implemented": 95,
        "tested": 95,
        "mainnet_ready": 95,
        "priority": "P0",
        "confidence": "high",
        "blockers": [],
        "evidence": ["evidence.md"],
        "required_tests": [f"p0_test_{index}"],
        "launch_scope": "core",
    }
    feature.update(overrides)
    return feature


def claim_feature(index: int, **overrides):
    feature = p0_feature(
        index,
        id=f"X3-CLAIM-{index:03d}",
        name=f"Retracted claim {index}",
        subsystem="claims_hygiene",
        source="claim_risk",
        launch_scope="research",
        claim_risk=True,
        implemented=55,
        tested=25,
        mainnet_ready=35,
        blockers=["historical claim remains capped below mainnet readiness"],
    )
    feature.update(overrides)
    return feature


def matrix_fixture(tmp_path: Path, count: int = 79) -> tuple[Path, int]:
    (tmp_path / "src").mkdir(parents=True)
    (tmp_path / "src/p0.rs").write_text(
        "\n".join(f"#[test]\nfn p0_test_{i}() {{}}" for i in range(count))
    )
    (tmp_path / "evidence.md").write_text("measured evidence\n")
    matrix = tmp_path / "FEATURE_MATRIX.toml"
    capability_rows = [
        (
            "[[feature]]\n"
            f'id = "X3-P0-{i:03d}"\n'
            f'name = "P0 feature {i}"\n'
            'subsystem = "runtime_core"\n'
            'paths = ["src/p0.rs"]\n'
            'test_paths = ["src/p0.rs"]\n'
            'source = "master"\n'
            "implemented = 95\n"
            "tested = 95\n"
            "mainnet_ready = 95\n"
            'priority = "P0"\n'
            'confidence = "high"\n'
            "blockers = []\n"
            'evidence = ["evidence.md"]\n'
            f'required_tests = ["p0_test_{i}"]\n'
            'launch_scope = "core"\n'
        )
        for i in range(count)
    ]
    claim_rows = [
        (
            "[[feature]]\n"
            f'id = "X3-CLAIM-{i:03d}"\n'
            f'name = "Retracted claim {i}"\n'
            'subsystem = "claims_hygiene"\n'
            'source = "claim_risk"\n'
            "implemented = 55\n"
            "tested = 25\n"
            "mainnet_ready = 35\n"
            'priority = "P0"\n'
            'confidence = "medium"\n'
            "claim_risk = true\n"
            'blockers = ["historical claim remains capped below mainnet readiness"]\n'
            'launch_scope = "research"\n'
        )
        for i in range(3)
    ]
    matrix.write_text(
        "[meta]\nversion = 1\n\n" + "\n".join(capability_rows + claim_rows)
    )
    return matrix, count + 3


def scope_errors(matrix_path: Path, repo_root: Path):
    matrix = gate.load_matrix(matrix_path)
    return gate.evaluate_p0_scope(matrix, repo_root)


def test_all_82_p0_rows_pass_with_current_local_evidence(tmp_path, monkeypatch):
    matrix, count = matrix_fixture(tmp_path)
    monkeypatch.setattr(gate, "claims_hygiene_errors", lambda root: [])

    assert count == 82
    assert scope_errors(matrix, tmp_path) == []


def test_p0_scope_count_drift_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path, count=80)
    assert scope_errors(matrix, tmp_path) == [
        "P0 release scope has 83 rows; the approved baseline is 82"
    ]


def test_one_p0_row_at_94_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["implemented"] = 94
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: implemented is 94; P0 release requires at least 95"
    ]


def test_a_nonempty_p0_blocker_list_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["blockers"] = ["External mainnet proof is still pending"]
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: P0 release has 1 open blocker: "
        "External mainnet proof is still pending"
    ]


def test_a_named_required_test_that_does_not_resolve_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["required_tests"] = ["p0_test_missing"]
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: required test does not resolve: p0_test_missing"
    ]


def test_note_only_evidence_fails_as_stale_claim_context(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["evidence"] = ["note: measured somewhere else"]
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: no current repository evidence artifact is recorded; "
        "URLs, PR references, commits and notes are claims, not launch evidence"
    ]


def test_missing_or_empty_local_evidence_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["evidence"] = ["missing-proof.json"]
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: evidence artifact is missing: missing-proof.json",
        "X3-P0-000: no current repository evidence artifact is recorded; "
        "URLs, PR references, commits and notes are claims, not launch evidence",
    ]

    (tmp_path / "empty.txt").write_text("")
    features[0]["evidence"] = ["empty.txt"]
    assert gate.evaluate_p0_scope({"feature": features}, tmp_path) == [
        "X3-P0-000: evidence artifact is empty: empty.txt",
        "X3-P0-000: no current repository evidence artifact is recorded; "
        "URLs, PR references, commits and notes are claims, not launch evidence",
    ]


def test_claim_hygiene_scanner_failure_blocks_release(tmp_path, monkeypatch):
    matrix, _ = matrix_fixture(tmp_path)
    (tmp_path / "scripts").mkdir()
    (tmp_path / "scripts/x3_audit_matrix.py").write_text("# fixture checker\n")
    monkeypatch.setattr(gate, "audit_artifact_errors", lambda root: [])
    monkeypatch.setattr(
        gate,
        "claims_hygiene_errors",
        lambda root: ["claims hygiene scanner failed: forbidden claim reintroduced"],
    )
    assert gate.check_p0_scope(matrix_path=matrix, repo_root=tmp_path) == [
        "claims hygiene scanner failed: forbidden claim reintroduced"
    ]


def test_stale_generated_matrix_artifacts_fail(tmp_path, monkeypatch):
    matrix, _ = matrix_fixture(tmp_path)
    (tmp_path / "scripts").mkdir()
    (tmp_path / "scripts/x3_audit_matrix.py").write_text("# fixture checker\n")

    def stale_run(cmd, cwd=None, env=None):
        assert cmd[-1] == "--check"
        return subprocess.CompletedProcess(
            cmd,
            1,
            stdout="",
            stderr="ERROR: audit artifact is missing/stale: audit-artifacts/current/feature-status.json\n",
        )

    monkeypatch.setattr(gate, "run", stale_run)
    monkeypatch.setattr(gate, "claims_hygiene_errors", lambda root: [])
    assert gate.check_p0_scope(matrix_path=matrix, repo_root=tmp_path) == [
        "generated matrix artifacts are missing or stale: "
        "audit-artifacts/current/feature-status.json"
    ]


def test_dirty_candidate_is_rejected_even_when_p0_rows_are_green(
    tmp_path, monkeypatch
):
    failures: list[str] = []
    monkeypatch.setattr(
        gate,
        "working_tree_changes",
        lambda root=gate.ROOT: [
            " M scripts/mainnet_release_gate.py",
            "?? evidence.json",
        ],
    )
    monkeypatch.setattr(gate, "FAILURES", failures)

    gate.check_candidate_identity()

    assert failures == [
        "release candidate working tree is dirty; commit or remove every staged, "
        "modified and untracked change first (2 path(s))"
    ]


def test_p0_scope_and_candidate_identity_run_before_expensive_work():
    source = GATE_PATH.read_text(encoding="utf-8")
    main = source[source.index("def main()") :]
    assert main.index("check_p0_scope()") < main.index("check_required_docs()")
    assert main.index("check_candidate_identity()") < main.index("check_build()")


def launch_record_fixture(tmp_path: Path, matrix_path: Path, commit: str):
    def write(relative: str, text: str) -> dict:
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return {"path": relative, "sha256": gate._sha256(path)}

    plain = write(
        "chain-specs/mainnet-plain.json",
        json.dumps(
            {
                "id": "x3-mainnet",
                "name": "X3 Mainnet",
                "chainType": "Live",
                "bootNodes": ["/dns4/boot.example/tcp/30333"],
                "genesis": {
                    "runtimeGenesis": {
                        "config": {
                            "aura": {"authorities": [str(i) for i in range(7)]},
                            "grandpa": {"authorities": [str(i) for i in range(7)]},
                        }
                    }
                },
            }
        ),
    )
    plain.update(
        {"role": "mainnet-plain", "aura_authorities": 7, "grandpa_authorities": 7}
    )
    raw = write(
        "chain-specs/mainnet-raw.json",
        json.dumps(
            {
                "id": "x3-mainnet",
                "name": "X3 Mainnet",
                "chainType": "Live",
                "genesis": {"raw": {"top": {}}},
            }
        ),
    )
    raw.update(
        {"role": "mainnet-raw", "aura_authorities": 7, "grandpa_authorities": 7}
    )

    audit_a = write("evidence/audit-a.txt", "runtime consensus atomic\n")
    audit_a.update(
        {
            "provider": "Auditor A",
            "scope": ["runtime", "consensus", "atomic"],
            "critical_open": 0,
            "high_open": 0,
        }
    )
    audit_a["report"] = audit_a.pop("path")
    audit_b = write("evidence/audit-b.txt", "bridges contracts operations\n")
    audit_b.update(
        {
            "provider": "Auditor B",
            "scope": ["bridges", "contracts", "operations"],
            "critical_open": 0,
            "high_open": 0,
        }
    )
    audit_b["report"] = audit_b.pop("path")

    bounty = write("evidence/bounty.txt", "active bounty\n")
    bounty.update(
        {
            "pool_usd": 100_000,
            "starts_at": "2026-08-01T00:00:00Z",
            "ends_at": "2099-01-01T00:00:00Z",
        }
    )
    testnet = write("evidence/testnet.txt", "50 validators across 5 continents\n")
    testnet.update(
        {
            "started_at": "2026-06-01T00:00:00Z",
            "ended_at": "2026-08-01T00:00:00Z",
            "validator_count": 50,
            "continents": 5,
        }
    )
    bridge_config = write(
        "evidence/bridges.json",
        json.dumps(
            {
                "active_routes": sorted(gate.BRIDGE_ROUTES),
                "per_transfer_limit_usd": 1000,
                "daily_per_route_limit_usd": 10000,
                "total_exposure_limit_usd": 50000,
                "kill_switch": True,
            }
        ),
    )
    validators_rows = [
        {
            "host": f"validator-{i}.example",
            "operator": f"operator-{i}",
            "identity": f"identity-{i}",
            "provider": f"provider-{i % 5}",
            "jurisdiction": f"jurisdiction-{i % 3}",
            "loopback": False,
        }
        for i in range(7)
    ]
    validators = write("evidence/validators.json", json.dumps({"validators": validators_rows}))
    validators["entries"] = validators_rows

    approvals = []
    for role in sorted(gate.APPROVAL_ROLES):
        evidence = write(f"evidence/approval-{role}.txt", f"{role} approved {commit}\n")
        approvals.append(
            {
                "approver": f"{role}-approver",
                "role": role,
                "approved_commit": commit,
                "evidence": evidence["path"],
                "sha256": evidence["sha256"],
            }
        )

    evidence_pack = write("evidence-pack.json", '{"fixture":true}\n')
    gate_log = write("evidence/gates.log", "release gate PASS\n")
    matrix = gate.load_matrix(matrix_path)
    rows = sorted(
        (
            {
                "id": feature["id"],
                "implemented": feature["implemented"],
                "tested": feature["tested"],
                "mainnet_ready": feature["mainnet_ready"],
            }
            for feature in matrix["feature"]
            if feature.get("priority") == "P0"
        ),
        key=lambda row: row["id"],
    )
    record = {
        "schema": 2,
        "recorded_at": "2026-10-01T00:00:00Z",
        "approved_by": "release-approver",
        "commit": commit,
        "expect_authorities": 7,
        "artifacts": [plain, raw],
        "feature_matrix": {
            "sources": {matrix_path.name: gate._sha256(matrix_path)},
        },
        "p0_snapshot": {"rows": rows},
        "evidence_pack": evidence_pack,
        "test_gates": [
            {
                "name": "release",
                "command": "make mainnet-check",
                "status": "PASS",
                "exit_code": 0,
                "log": gate_log["path"],
                "sha256": gate_log["sha256"],
            }
        ],
        "audits": [audit_a, audit_b],
        "bounty_window": {**bounty, "evidence": bounty.pop("path")},
        "public_testnet_window": {**testnet, "evidence": testnet.pop("path")},
        "bridge_configuration": bridge_config,
        "validators": validators,
        "approvals": approvals,
    }
    record["record_sha256"] = gate._record_digest(record)
    return record


def test_complete_launch_evidence_record_passes(tmp_path, monkeypatch):
    matrix, _ = matrix_fixture(tmp_path)
    monkeypatch.setattr(gate, "claims_hygiene_errors", lambda root: [])
    record = launch_record_fixture(tmp_path, matrix, "abc123")

    assert (
        gate.evaluate_launch_record(
            record,
            tmp_path,
            "abc123",
            gate.load_matrix(matrix),
        )
        == []
    )


def test_missing_second_audit_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    record["audits"] = record["audits"][:1]
    record["record_sha256"] = gate._record_digest(record)

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert "launch record: two independent audit reports are required" in errors


def test_audit_scope_groups_cannot_be_pooled_into_one_report(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    record["audits"][0]["scope"] = [
        "runtime",
        "consensus",
        "atomic",
        "bridges",
        "contracts",
        "operations",
    ]
    record["audits"][1]["scope"] = []
    record["record_sha256"] = gate._record_digest(record)

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert "launch record: audit Auditor B has no scope" in errors


def test_both_audits_in_one_scope_group_do_not_satisfy_the_other(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    record["audits"][1]["scope"] = ["runtime"]
    record["record_sha256"] = gate._record_digest(record)

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert (
        "launch record: bridges/contracts/operations audit scope is missing"
    ) in errors


def test_expired_public_testnet_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    record["recorded_at"] = "2027-01-01T00:00:00Z"
    record["record_sha256"] = gate._record_digest(record)

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert "launch record: public testnet evidence has expired" in errors


def test_mismatched_genesis_artifact_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    (tmp_path / "chain-specs/mainnet-plain.json").write_text(
        '{"chainType":"Live","different":true}'
    )

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert (
        "launch record: mainnet-plain: artifact digest does not match: "
        "chain-specs/mainnet-plain.json"
    ) in errors


def test_record_from_a_different_candidate_commit_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")

    errors = gate.evaluate_launch_record(
        record, tmp_path, "def456", gate.load_matrix(matrix)
    )

    assert (
        "launch record: commit 'abc123' does not match candidate def456"
    ) in errors


def test_missing_test_gate_evidence_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    record["test_gates"] = []
    record["record_sha256"] = gate._record_digest(record)

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert "launch record: at least one passing test gate is required" in errors


def test_changed_test_gate_log_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    (tmp_path / "evidence/gates.log").write_text("release gate FAIL\n")

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert (
        "launch record: test gate release: artifact digest does not match: "
        "evidence/gates.log"
    ) in errors


def test_changed_evidence_pack_is_rejected(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    record = launch_record_fixture(tmp_path, matrix, "abc123")
    (tmp_path / "evidence-pack.json").write_text('{"fixture":false}\n')

    errors = gate.evaluate_launch_record(
        record, tmp_path, "abc123", gate.load_matrix(matrix)
    )

    assert (
        "launch record: evidence pack: artifact digest does not match: "
        "evidence-pack.json"
    ) in errors
