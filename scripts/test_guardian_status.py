#!/usr/bin/env python3
"""Tests for the Guardian status tracker.

The tracker's value is that the completion count cannot drift. These tests pin
the one rule that guards against the most tempting drift: marking an item DONE
without evidence. Everything runs against in-memory or scratch-copy data, so the
tracked `docs/guardian/checklist.json` is never written.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import guardian_status as gs  # noqa: E402


def _items():
    return [
        {"id": "X1", "sec": "1", "item": "one", "status": "TODO", "evidence": ""},
        {"id": "X2", "sec": "1", "item": "two", "status": "TODO", "evidence": "prior"},
    ]


def test_done_without_evidence_is_refused():
    items = _items()
    with pytest.raises(SystemExit):
        gs.apply_mark(items, "X1", "DONE", None)
    # The refusal must not mutate anything.
    assert items[0]["status"] == "TODO"


def test_done_with_blank_evidence_is_refused():
    items = _items()
    with pytest.raises(SystemExit):
        gs.apply_mark(items, "X1", "DONE", "   ")
    assert items[0]["status"] == "TODO"


def test_done_with_existing_evidence_is_allowed():
    items = _items()
    assert gs.apply_mark(items, "X2", "DONE", None) == "DONE"
    assert items[1]["status"] == "DONE"


def test_done_with_new_evidence_is_recorded():
    items = _items()
    gs.apply_mark(items, "X1", "done", "cargo test -p pallet-x3-app-registry -> 22 passed")
    assert items[0]["status"] == "DONE"
    assert "22 passed" in items[0]["evidence"]


def test_non_done_needs_no_evidence():
    items = _items()
    assert gs.apply_mark(items, "X1", "DOING", None) == "DOING"
    assert items[0]["status"] == "DOING"


def test_unknown_id_and_bad_status_are_refused():
    items = _items()
    with pytest.raises(SystemExit):
        gs.apply_mark(items, "NOPE", "DONE", "evidence")
    with pytest.raises(SystemExit):
        gs.apply_mark(items, "X1", "MAYBE", "evidence")


def test_cli_refuses_done_without_evidence_and_leaves_file_untouched(tmp_path):
    scratch = tmp_path / "checklist.json"
    original = {
        "project": "X3 Guardian",
        "spec": "s",
        "derived_from": "d",
        "status_values": gs.ORDER,
        "items": _items(),
    }
    scratch.write_text(json.dumps(original, indent=2) + "\n", encoding="utf-8")

    env = dict(os.environ, X3_GUARDIAN_CHECKLIST=str(scratch))
    proc = subprocess.run(
        [sys.executable, str(HERE / "guardian_status.py"), "--mark", "X1", "DONE", "--evidence", ""],
        capture_output=True,
        text=True,
        env=env,
    )
    assert proc.returncode != 0
    assert "no evidence" in (proc.stdout + proc.stderr)
    # A refused mark never writes: the file still says TODO.
    assert json.loads(scratch.read_text())["items"][0]["status"] == "TODO"

    ok = subprocess.run(
        [sys.executable, str(HERE / "guardian_status.py"), "--mark", "X1", "DONE",
         "--evidence", "proof"],
        capture_output=True,
        text=True,
        env=env,
    )
    assert ok.returncode == 0, ok.stdout + ok.stderr
    assert json.loads(scratch.read_text())["items"][0]["status"] == "DONE"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-q"]))
