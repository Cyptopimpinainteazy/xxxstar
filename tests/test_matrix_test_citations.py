from __future__ import annotations

import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts" / "ci" / "check-matrix-tests-exist.py"


def load_checker():
    spec = importlib.util.spec_from_file_location("matrix_test_citations", CHECKER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_test_paths_are_part_of_the_citation_corpus(tmp_path, monkeypatch):
    (tmp_path / "src").mkdir()
    (tmp_path / "src/lib.rs").write_text("pub fn production() {}\n", encoding="utf-8")
    (tmp_path / "node/tests").mkdir(parents=True)
    (tmp_path / "node/tests/receipt.rs").write_text(
        "fn a_receipt_is_read_by_another_validator() {}\n",
        encoding="utf-8",
    )
    checker = load_checker()
    monkeypatch.setattr(checker, "ROOT", tmp_path)
    feature = {
        "id": "X3-LANG-TEST",
        "paths": ["src"],
        "test_paths": ["node/tests/receipt.rs"],
        "required_tests": ["a_receipt_is_read_by_another_validator"],
    }
    assert checker.unresolved_for(feature) == []


def test_a_citation_missing_from_both_path_lists_is_refused(tmp_path, monkeypatch):
    (tmp_path / "src").mkdir()
    (tmp_path / "src/lib.rs").write_text("pub fn production() {}\n", encoding="utf-8")
    checker = load_checker()
    monkeypatch.setattr(checker, "ROOT", tmp_path)
    feature = {
        "id": "X3-LANG-TEST",
        "paths": ["src"],
        "test_paths": [],
        "required_tests": ["not_a_real_test"],
    }
    missing = checker.unresolved_for(feature)
    assert len(missing) == 1
    assert "not_a_real_test" in missing[0]
