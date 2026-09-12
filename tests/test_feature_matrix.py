from __future__ import annotations

import json
from pathlib import Path
import sys

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import feature_matrix as fm


def base_feature(**overrides):
    feature = {
        "id": "X3-TEST-001",
        "name": "Valid feature",
        "subsystem": "runtime_core",
        "paths": ["crate/src/lib.rs"],
        "source": "master",
        "implemented": 70,
        "tested": 70,
        "mainnet_ready": 60,
        "priority": "P1",
        "confidence": "high",
        "blockers": ["Needs production network proof"],
        "evidence": ["evidence/report.md"],
        "required_tests": ["works"],
        "launch_scope": "core",
        "claim_risk": False,
    }
    feature.update(overrides)
    return feature


def repo_fixture(tmp_path: Path) -> Path:
    (tmp_path / "crate/src").mkdir(parents=True)
    (tmp_path / "crate/src/lib.rs").write_text("#[test]\nfn works() {}\n")
    (tmp_path / "evidence").mkdir()
    (tmp_path / "evidence/report.md").write_text("proof\n")
    return tmp_path


def registry():
    return {"atomic_router": {"readiness_score": 88}}


def test_valid_feature_passes(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, warnings = fm.validate_matrix({"feature": [base_feature()]}, repo, registry())
    assert errors == []
    assert warnings == []


def test_duplicate_ids_fail(tmp_path):
    repo = repo_fixture(tmp_path)
    f1 = base_feature()
    f2 = base_feature(name="Other")
    errors, _ = fm.validate_matrix({"feature": [f1, f2]}, repo, registry())
    assert any("duplicate feature id" in e for e in errors)


def test_invalid_score_range_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix({"feature": [base_feature(implemented=101)]}, repo, registry())
    assert any("implemented must be between 0 and 100" in e for e in errors)


def test_missing_path_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix({"feature": [base_feature(paths=["missing.rs"])]}, repo, registry())
    assert any("path does not exist" in e for e in errors)


def test_missing_required_test_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix({"feature": [base_feature(required_tests=["not_real"])]}, repo, registry())
    assert any("required test not found" in e for e in errors)


def test_missing_registry_key_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(registry_feature="missing")]}, repo, registry()
    )
    assert any("registry feature 'missing' does not exist" in e for e in errors)


def test_p0_at_80_mainnet_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(priority="P0", mainnet_ready=80)]}, repo, registry()
    )
    assert any("P0 feature cannot claim mainnet_ready >= 80" in e for e in errors)


def test_open_pr_at_80_mainnet_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(source="open_pr", open_prs=[166], mainnet_ready=80)]},
        repo,
        registry(),
    )
    assert any("open PR feature cannot claim mainnet_ready >= 80" in e for e in errors)


def test_claim_risk_inflation_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(claim_risk=True, paths=[], mainnet_ready=40)]}, repo, registry()
    )
    assert any("claim-risk feature must remain below 40" in e for e in errors)


def test_research_inflation_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(launch_scope="research", paths=[], mainnet_ready=40)]}, repo, registry()
    )
    assert any("research feature must remain below 40" in e for e in errors)


def test_high_test_score_without_test_evidence_fails(tmp_path):
    repo = repo_fixture(tmp_path)
    errors, _ = fm.validate_matrix(
        {"feature": [base_feature(tested=80, required_tests=[], test_evidence=[])]}, repo, registry()
    )
    assert any("tested >= 80 requires required_tests or test_evidence" in e for e in errors)


def test_score_gap_warnings(tmp_path):
    repo = repo_fixture(tmp_path)
    feature = base_feature(implemented=90, tested=50, mainnet_ready=15, launch_scope="guarded")
    errors, warnings = fm.validate_matrix({"feature": [feature]}, repo, registry())
    assert errors == []
    assert any("implementation exceeds testing" in w for w in warnings)
    assert any("testing exceeds mainnet readiness" in w for w in warnings)


def test_composite_and_class():
    feature = base_feature(implemented=80, tested=60, mainnet_ready=50)
    assert fm.composite(feature) == 63
    assert fm.readiness_class(80) == "mainnet_candidate"
    assert fm.readiness_class(60) == "guarded_near_ready"
    assert fm.readiness_class(40) == "partial"
    assert fm.readiness_class(20) == "experimental"
    assert fm.readiness_class(19) == "claim_or_research"


def test_generation_is_deterministic(tmp_path):
    repo = repo_fixture(tmp_path)
    matrix = {"meta": {"version": 1}, "feature": [base_feature()]}
    out1 = tmp_path / "out1"
    out2 = tmp_path / "out2"
    fm.generate_outputs(matrix, repo, out1, registry())
    fm.generate_outputs(matrix, repo, out2, registry())
    for name in [
        "X3_FEATURE_MATRIX.json",
        "X3_FEATURE_MATRIX.csv",
        "X3_FEATURE_MATRIX.md",
        "X3_FEATURE_SUMMARY.md",
    ]:
        assert (out1 / name).read_bytes() == (out2 / name).read_bytes()


def test_real_repository_matrix_validates():
    matrix_path = ROOT / "FEATURE_MATRIX.toml"
    if not matrix_path.exists():
        pytest.skip("matrix seed is added in task 2")
    matrix = fm.load_matrix(matrix_path)
    reg = fm.load_registry(ROOT / "FEATURE_REGISTRY.toml")
    errors, _ = fm.validate_matrix(matrix, ROOT, reg)
    assert errors == []
