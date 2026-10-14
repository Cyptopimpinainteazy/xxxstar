#!/usr/bin/env python3
"""Pin executable skip detection and comment false positives in both scan paths."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("source_scan", ROOT / "scripts/x3_fake_code_scan.py")
scanner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scanner)


class CheatScannerTests(unittest.TestCase):
    def findings(self, fallback=False):
        ignore = "#[" + "ignore]"
        skipped = "test.sk" + "ip('disabled')"
        xit = "x" + "it('disabled')"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = {
                "tests/live.rs": ignore + "\n#[test]\nfn live() { assert_eq!(actual, expected); }\n",
                "tests/feature.ts": skipped + "\n",
                "tests/feature.py": "# " + skipped + "\n" + xit + "\n",
                "scripts/check.sh": "# " + ignore + " is a Rust test attribute\n# " + skipped + "\n",
            }
            for path, content in files.items():
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(content)
            with patch.object(scanner, "REPO_ROOT", root):
                if fallback:
                    with patch.object(scanner.shutil, "which", return_value=None):
                        return scanner.scan_cheats()
                return scanner.scan_cheats()

    def test_hash_comments_are_not_skipped_tests(self):
        findings = self.findings()
        self.assertEqual([(f["kind"], f["path"]) for f in findings], [
            ("skip", "tests/feature.py"), ("skip", "tests/feature.ts"), ("skip", "tests/live.rs")])

    def test_python_fallback_matches_rg_detection(self):
        self.assertIsNotNone(scanner.shutil.which('rg'), 'Install rg to verify both scan paths')
        self.assertEqual(self.findings(), self.findings(fallback=True))

    def test_installed_dependencies_and_build_outputs_are_excluded(self):
        self.assertIsNotNone(scanner.shutil.which('rg'), 'Install rg to verify glob exclusions')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in scanner.PRUNE_DIR_NAMES - {'.git'}:
                path = root / 'apps' / 'wallet' / name / 'dependency.js'
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('// TO' + 'DO dependency\n' + 'test.sk' + "ip('dependency')\n")
            source = root / 'src' / 'feature.js'
            source.parent.mkdir()
            source.write_text('// TO' + 'DO project\n')
            with patch.object(scanner, 'REPO_ROOT', root):
                for scan in (scanner.scan_stubs, scanner.scan_cheats):
                    actual = scan()
                    with patch.object(scanner.shutil, 'which', return_value=None):
                        fallback = scan()
                    self.assertEqual(actual, fallback)
                    self.assertTrue(all(f['path'] == 'src/feature.js' for f in actual))


class ProdMockTests(unittest.TestCase):
    """A mock used inside `#[cfg(test)]` is test code; one outside it is not."""

    SOURCE = """pub struct Engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        use crate::mock::Test as MockRuntime;
        let braces = "{ not code }";
        assert_eq!(engine::<MockRuntime>(), 0);
    }
}

pub struct Mock;
"""

    def kinds(self, fallback=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "pallets" / "engine" / "src" / "lib.rs"
            path.parent.mkdir(parents=True)
            path.write_text(self.SOURCE)
            with patch.object(scanner, "REPO_ROOT", root):
                if fallback:
                    with patch.object(scanner.shutil, "which", return_value=None):
                        found = scanner.scan_cheats()
                else:
                    found = scanner.scan_cheats()
        return [(f["kind"], f["line"]) for f in found if f["kind"] == "prod-mock"]

    def test_only_the_mock_outside_the_test_module_is_flagged(self):
        line = self.SOURCE.splitlines().index("pub struct Mock;") + 1
        self.assertEqual(self.kinds(), [("prod-mock", line)])
        self.assertEqual(self.kinds(fallback=True), [("prod-mock", line)])


class ScopeTests(unittest.TestCase):
    """Recovered from x3gpu1: links are not sources, and the vocabulary exemption is exact."""

    def test_links_and_pruned_paths_are_not_sources_on_either_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("src/real.rs", "src/.hidden.rs", "apps/demo/node_modules/dependency.rs", "dist/generated.rs"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("todo!();\n")
            # A file link would scan real.rs twice; a directory link back to the root loops.
            (root / "src/link.rs").symlink_to(root / "src/real.rs")
            (root / "loop").symlink_to(root, target_is_directory=True)
            walked = [rel for rel, _ in scanner.iter_source_files(root)]
            self.assertEqual(walked, ["src/real.rs"])
            listed = scanner.rg_file_list(root)
            if listed is not None:
                self.assertEqual(listed, ["src/real.rs"])

    def test_vocabulary_exemption_preserves_implementation_findings(self):
        path = "services/x3-ai-router/router.py"
        vocabulary = '    "BOILERPLATE": ("scaffold", "stub out"),'
        lines = [
            (path, 1, vocabulary),
            (path, 2, vocabulary + " # TODO: implement this"),
            (path, 3, '    "OTHER": ("stub out",),'),
            ("services/other.py", 4, vocabulary),
            (path, 5, 'todo!();'),
            (path, 6, '    "BOILERPLATE": ("scaffold",), # ' + vocabulary),
        ]
        with patch.object(scanner, "rg_lines", return_value=lines):
            findings = scanner.scan_stubs()
        self.assertEqual(sorted(f["line"] for f in findings), [2, 3, 4, 5, 6])
        self.assertEqual(next(f for f in findings if f["line"] == 5)["kind"], "explicit-stub")


if __name__ == "__main__":
    unittest.main()
