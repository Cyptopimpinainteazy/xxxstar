from __future__ import annotations

import importlib.util
import subprocess
from pathlib import Path
import sys


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
    feature = p0_feature(index)
    feature.update(
        {
            "id": f"X3-CLAIM-{index:03d}",
            "name": f"Retracted claim {index}",
            "subsystem": "claims_hygiene",
            "source": "claim_risk",
            "launch_scope": "research",
            "claim_risk": True,
            "implemented": 55,
            "tested": 25,
            "mainnet_ready": 35,
            "blockers": ["historical claim remains capped below mainnet readiness"],
        }
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
            'blockers = []\n'
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
            'claim_risk = true\n'
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


def test_all_82_p0_rows_pass_capability_and_claim_hygiene_contracts(
    tmp_path, monkeypatch
):
    matrix, count = matrix_fixture(tmp_path)
    monkeypatch.setattr(gate, "claims_hygiene_errors", lambda root: [])

    assert count == 82
    assert scope_errors(matrix, tmp_path) == []


def test_p0_scope_count_drift_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path, count=80)

    errors = scope_errors(matrix, tmp_path)

    assert errors == ["P0 release scope has 83 rows; the approved baseline is 82"]


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

    errors = gate.check_p0_scope(matrix_path=matrix, repo_root=tmp_path)

    assert errors == ["claims hygiene scanner failed: forbidden claim reintroduced"]


def test_one_p0_row_at_94_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["implemented"] = 94

    errors = gate.evaluate_p0_scope({"feature": features}, tmp_path)

    assert errors == [
        "X3-P0-000: implemented is 94; P0 release requires at least 95"
    ]


def test_a_nonempty_p0_blocker_list_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["blockers"] = ["External mainnet proof is still pending"]

    errors = gate.evaluate_p0_scope({"feature": features}, tmp_path)

    assert errors == [
        "X3-P0-000: P0 release has 1 open blocker: "
        "External mainnet proof is still pending"
    ]


def test_a_named_required_test_that_does_not_resolve_fails(tmp_path):
    matrix, _ = matrix_fixture(tmp_path)
    features = gate.load_matrix(matrix)["feature"]
    features[0]["required_tests"] = ["p0_test_missing"]

    errors = gate.evaluate_p0_scope({"feature": features}, tmp_path)

    assert errors == [
        "X3-P0-000: required test does not resolve: p0_test_missing"
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

    errors = gate.check_p0_scope(matrix_path=matrix, repo_root=tmp_path)

    assert errors == [
        "generated matrix artifacts are missing or stale: "
        "audit-artifacts/current/feature-status.json"
    ]


def test_p0_scope_is_wired_into_the_release_gate_before_expensive_work():
    source = GATE_PATH.read_text(encoding="utf-8")
    main = source[source.index("def main()") :]
    assert main.index("check_p0_scope()") < main.index("check_required_docs()")
