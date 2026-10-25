"""Regression tests for safe, fail-closed pre-push test selection."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from risk_plan import choose_profile


class RiskPlanTests(unittest.TestCase):
    def test_docs_only_uses_focus_without_cargo(self):
        mode, gates = choose_profile(["docs/guide.md", "README.md"])
        self.assertEqual(mode, "focused")
        self.assertIn("script-syntax", gates)
        self.assertNotIn("workspace-check", gates)

    def test_ui_only_tests_the_desktop(self):
        mode, gates = choose_profile(["apps/x3-desktop/src/App.tsx", "docs/notes.md"])
        self.assertEqual(mode, "focused")
        self.assertIn("test-js-x3-desktop", gates)

    def test_runtime_and_consensus_are_always_full(self):
        for path in ["runtime/src/lib.rs", "pallets/x3-settlement-engine/src/lib.rs", "node/src/main.rs", "x3-lang/compiler/src/lib.rs"]:
            with self.subTest(path=path):
                self.assertEqual(choose_profile([path])[0], "full")

    def test_scripts_and_ci_wiring_are_full(self):
        for path in ["scripts/local-ci.sh", ".github/workflows/local-ci.yml", ".githooks/pre-push", "scripts/ci/risk_plan.py"]:
            with self.subTest(path=path):
                self.assertEqual(choose_profile([path])[0], "full")

    def test_non_ui_desktop_files_are_full(self):
        for path in ["apps/x3-desktop/src-tauri/src/main.rs", "apps/x3-desktop/package.json", "apps/x3-desktop/src/api/ledger.py"]:
            with self.subTest(path=path):
                self.assertEqual(choose_profile([path])[0], "full")

    def test_unknown_and_mixed_critical_changes_fail_closed(self):
        for paths in [[], ["random/new.rs"], ["docs/guide.md", "runtime/src/lib.rs"], ["docs/reports/SECURITY_BLOCKERS.md"], ["docs/local-ci.md"], ["../README.md"]]:
            with self.subTest(paths=paths):
                self.assertEqual(choose_profile(paths)[0], "full")

    def test_mixed_safe_docs_and_ui_remains_scoped(self):
        mode, gates = choose_profile(["README.md", "apps/x3-desktop/src/styles.css"])
        self.assertEqual(mode, "focused")
        self.assertIn("test-js-x3-desktop", gates)


if __name__ == "__main__":
    unittest.main()
