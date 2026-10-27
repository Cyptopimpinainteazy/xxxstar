"""Behavioral integration tests for the Serena-independent Forge MCP adapter."""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "tools" / "x3-mcp" / "src"))
from x3_mcp import forge_bridge as brain  # noqa: E402


def git(root, *args):
    return subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()


class ForgeBrainTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        modules = self.root / "tools" / "x3-forge"
        modules.mkdir(parents=True)
        for name in ("index", "context", "failure_memory"):
            shutil.copyfile(REPO / "tools" / "x3-forge" / (name + ".py"),
                            modules / (name + ".py"))
        self.source = self.root / "crates" / "demo" / "src" / "lib.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("pub fn settlement_guard() {}\n", encoding="utf-8")
        git(self.root, "init", "-q")
        git(self.root, "add", "-A")
        git(self.root, "-c", "user.name=X3 CI", "-c",
            "user.email=x3-ci@example.invalid", "commit", "-qm", "baseline")
        index = brain._module(self.root, "index").build(self.root)
        index_path = self.root / ".x3-forge" / "index.json"
        index_path.parent.mkdir()
        index_path.write_text(json.dumps(index), encoding="utf-8")

    def test_context_finds_real_symbol_with_hash_and_head(self):
        package = brain.context(self.root, "settlement_guard")
        paths = {file["path"]: file for file in package["included"]}
        self.assertIn("crates/demo/src/lib.rs", paths)
        self.assertEqual(paths["crates/demo/src/lib.rs"]["sha256"],
                         brain.hashlib.sha256(self.source.read_bytes()).hexdigest())
        self.assertEqual(package["index_commit"], git(self.root, "rev-parse", "HEAD"))
        self.assertIn("lexical", package["retrieval"])

    def test_mutated_file_fails_closed_even_when_head_is_same(self):
        self.source.write_text("pub fn settlement_guard() { panic!() }\n")
        with self.assertRaisesRegex(RuntimeError, "indexed file changed"):
            brain.context(self.root, "settlement_guard")

    def test_changed_head_requires_rebuild(self):
        extra = self.root / "new.py"
        extra.write_text("def new_feature(): pass\n")
        git(self.root, "add", "-A")
        git(self.root, "-c", "user.name=X3 CI", "-c",
            "user.email=x3-ci@example.invalid", "commit", "-qm", "next")
        with self.assertRaisesRegex(RuntimeError, "different HEAD"):
            brain.context(self.root, "settlement_guard")

    def test_missing_index_reports_exact_setup_command(self):
        (self.root / ".x3-forge" / "index.json").unlink()
        with self.assertRaisesRegex(RuntimeError, "index.py build"):
            brain.context(self.root, "settlement_guard")

    def test_note_writes_disabled_by_default(self):
        with patch.dict(os.environ, {"X3_FORGE_ALLOW_WRITES": "0"}):
            with self.assertRaises(PermissionError):
                brain.remember(self.root, "failure", "settlement", "refund timeout")
        self.assertFalse((self.root / ".x3-forge" / "memory.jsonl").exists())

    def test_shared_memory_survives_new_call_and_aggregates(self):
        with patch.dict(os.environ, {"X3_FORGE_ALLOW_WRITES": "1"}):
            first = brain.remember(self.root, "failure", "settlement",
                                   "refund timeout", "repair timeout guard")
            brain.remember(self.root, "failure", "settlement",
                           "refund timeout", "repair timeout guard")
        self.assertEqual(first["saved"]["commit"], git(self.root, "rev-parse", "HEAD"))
        result = brain.recall(self.root, "refund timeout", "failure")
        self.assertEqual(result["total_entries"], 2)
        self.assertEqual(len(result["matches"]), 1)
        self.assertEqual(result["matches"][0]["occurrences"], 2)
        self.assertIn("unverified", result["trust"])

    def test_bad_memory_type_and_invalid_query_rejected(self):
        with patch.dict(os.environ, {"X3_FORGE_ALLOW_WRITES": "1"}):
            with self.assertRaises(ValueError):
                brain.remember(self.root, "rumor", "x3", "unconfirmed")
        with self.assertRaises(ValueError):
            brain.recall(self.root, "")
        with self.assertRaises(ValueError):
            brain.context(self.root, "settlement", budget_tokens=0)

    def test_mcp_tools_really_register_and_call_adapter(self):
        class FakeMCP:
            def __init__(self):
                self.tools = {}
            def tool(self):
                def attach(fn):
                    self.tools[fn.__name__] = fn
                    return fn
                return attach
        server = FakeMCP()
        brain.register(server, self.root)
        self.assertEqual(set(server.tools), {
            "x3_brain_context", "x3_brain_recall", "x3_brain_remember",
        })
        result = server.tools["x3_brain_context"]("settlement_guard")
        self.assertGreater(result["files_included"], 0)
        with self.assertRaises(PermissionError):
            with patch.dict(os.environ, {"X3_FORGE_ALLOW_WRITES": "0"}):
                server.tools["x3_brain_remember"]("failure", "x3", "blocked")


if __name__ == "__main__":
    unittest.main()
