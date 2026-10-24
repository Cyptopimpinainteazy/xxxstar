"""Regression tests for scanner scope and classification vocabulary."""

import importlib.util
import shutil
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "scanner", Path(__file__).with_name("x3_fake_code_scan.py")
)
scanner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scanner)


# Assemble detector tokens at runtime so fixture text is not counted as repository findings.
class ScopeTests(unittest.TestCase):
    def test_rg_and_python_prune_same_sources(self):
        if shutil.which("rg") is None:
            self.skipTest("ripgrep unavailable; scanner Python fallback remains supported")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("src/real.rs", "src/.hidden.rs", "apps/demo/node_modules/dependency.rs", "dist/generated.rs"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("to" + "do!();\n")
            (root / "src/link.rs").symlink_to(root / "src/real.rs")
            (root / "loop").symlink_to(root, target_is_directory=True)
            with patch.object(scanner, "REPO_ROOT", root):
                accelerated = scanner.scan_stubs()
                accelerated_cheats = scanner.scan_cheats()
                accelerated_files = scanner.rg_files(scanner.STUB_CANDIDATE_PATTERNS)
                with patch.object(scanner.shutil, "which", return_value=None):
                    fallback = scanner.scan_stubs()
                    fallback_cheats = scanner.scan_cheats()
            self.assertEqual(accelerated, fallback)
            self.assertEqual(accelerated_cheats, fallback_cheats)
            self.assertEqual([p.relative_to(root).as_posix() for p in accelerated_files], ["src/real.rs"])
            self.assertEqual([f["path"] for f in accelerated], ["src/real.rs"])

    def test_vocabulary_exemption_preserves_implementation_findings(self):
        path = "services/x3-ai-router/router.py"
        vocabulary = '    "BOILERPLATE": ("scaffold", "st' + 'ub out"),'
        lines = [
            (path, 1, vocabulary),
            (path, 2, vocabulary + " # TO" + "DO: implement this"),
            (path, 3, '    "OTHER": ("st' + 'ub out",),'),
            ("services/other.py", 4, vocabulary),
            (path, 5, 'to' + 'do!();'),
            (path, 6, '    "BOILERPLATE": ("scaffold",), # ' + vocabulary),
        ]
        with patch.object(scanner, "rg_lines", return_value=lines):
            findings = scanner.scan_stubs()
        self.assertEqual(sorted(f["line"] for f in findings), [2, 3, 4, 5, 6])
        self.assertEqual(next(f for f in findings if f["line"] == 5)["kind"], "explicit-" + "st" + "ub")


if __name__ == "__main__":
    unittest.main()
