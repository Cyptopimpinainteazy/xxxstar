"""Tests for the context compiler.

The failure mode that matters here is a package that looks reasonable and
quietly omits the file the agent needed, or that claims a budget it did not
respect. These tests pin the selection, the arithmetic, and the honesty of the
`excluded` list.
"""
import importlib.util
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


forge_index = load("index")
context = load("context")


def fixture_index(files):
    """A tiny index with the same shape the real one has."""
    entries = {}
    for path, (source, lang) in files.items():
        items = forge_index.parse_rust(source) if lang == "rust" else forge_index.parse_python(source)
        entries[path] = {
            "sha256": "0" * 64,
            "bytes": len(source),
            "lang": lang,
            "crate": "demo" if path.endswith(".rs") else None,
            "items": items,
        }
    return {"index_version": 1, "commit": "f" * 40, "manifests": ["Cargo.toml"], "files": entries}


class Selection(unittest.TestCase):
    def setUp(self):
        self.index = fixture_index({
            "src/lib.rs": ("pub struct SettlementLedger;\npub fn settle() {}\n", "rust"),
            "src/other.rs": ("pub fn unrelated() {}\n", "rust"),
            "src/tests.rs": ("#[test]\nfn settle_is_unique() {}\n", "rust"),
        })

    def test_a_named_symbol_selects_its_file(self):
        package = context.build_package(self.index, "SettlementLedger")
        paths = [i["path"] for i in package["included"]]
        self.assertEqual(paths[0], "src/lib.rs")

    def test_an_unmatched_query_returns_an_empty_package(self):
        package = context.build_package(self.index, "zzz_nothing_matches_zzz")
        self.assertEqual(package["included"], [])
        self.assertEqual(package["excluded"], [])
        self.assertFalse(package["truncated"], "nothing was truncated, nothing matched")
        self.assertEqual(package["files_matched"], 0)
        self.assertIn("nothing matched", context.render(package))

    def test_todo_items_are_never_reported_as_symbols(self):
        index = fixture_index({"src/lib.rs": ("// TODO: settle() is unchecked\nfn settle() {}\n", "rust")})
        package = context.build_package(index, "settle")
        for item in package["included"]:
            for symbol in item["symbols"]:
                self.assertNotEqual(symbol["kind"], "todo")
                self.assertNotIn("TODO", symbol["name"])

    def test_path_matches_are_found_without_symbol_matches(self):
        index = fixture_index({"crates/x3-guardian-runner/src/main.rs": ("fn main() {}\n", "rust")})
        package = context.build_package(index, "x3-guardian-runner")
        self.assertEqual([i["path"] for i in package["included"]], ["crates/x3-guardian-runner/src/main.rs"])

    def test_stopwords_do_not_drive_selection(self):
        self.assertEqual(context.terms_of("where is the settlement uniqueness enforced?"),
                         ["settlement", "uniqueness"])


class Budget(unittest.TestCase):
    def setUp(self):
        # 400-byte files => 100 estimated tokens each at 4 bytes/token.
        body = "\n".join(f"pub fn symbol_{n}() {{}}" for n in range(20))
        self.index = fixture_index({f"src/f{n}.rs": (body, "rust") for n in range(8)})

    def total(self, package):
        return sum(i["estimated_tokens"] for i in package["included"])

    def test_the_package_never_exceeds_its_budget(self):
        for budget in (1, 50, 101, 250, 400, 10_000):
            package = context.build_package(self.index, "symbol", budget_tokens=budget)
            self.assertLessEqual(self.total(package), budget, f"budget {budget} exceeded")

    def test_max_files_is_respected(self):
        package = context.build_package(self.index, "symbol", budget_tokens=10_000, max_files=3)
        self.assertLessEqual(len(package["included"]), 3)
        self.assertTrue(package["truncated"])

    def test_truncation_is_disclosed_with_a_reason(self):
        package = context.build_package(self.index, "symbol", budget_tokens=250, max_files=8)
        self.assertTrue(package["excluded"], "a truncated package must say what it dropped")
        for item in package["excluded"]:
            self.assertIn(item["reason"], {"budget", "max_files"})
            self.assertTrue(item["path"])

    def test_every_matched_file_is_either_included_or_explained(self):
        package = context.build_package(self.index, "symbol", budget_tokens=250, max_files=8)
        self.assertEqual(
            package["files_included"] + package["files_excluded"],
            package["files_matched"],
            "no matched file may vanish without being accounted for",
        )

    def test_a_budget_too_small_for_anything_returns_nothing_rather_than_overspending(self):
        package = context.build_package(self.index, "symbol", budget_tokens=10)
        self.assertEqual(package["included"], [])
        self.assertEqual(package["estimated_tokens"], 0)
        self.assertTrue(package["truncated"])


class DeterminismAndProvenance(unittest.TestCase):
    def setUp(self):
        self.index = fixture_index({
            "src/a.rs": ("pub fn alpha() {}\n", "rust"),
            "src/b.rs": ("pub fn alpha_beta() {}\n", "rust"),
        })

    def test_the_same_query_produces_an_identical_package(self):
        first = json.dumps(context.build_package(self.index, "alpha"), sort_keys=True)
        second = json.dumps(context.build_package(self.index, "alpha"), sort_keys=True)
        self.assertEqual(first, second)

    def test_every_included_file_carries_provenance(self):
        package = context.build_package(self.index, "alpha")
        self.assertTrue(package["included"])
        for item in package["included"]:
            self.assertRegex(item["sha256"], r"^[0-9a-f]{64}$")
            self.assertEqual(item["estimated_tokens"], max(1, item["bytes"] // 4) or 1)

    def test_every_reported_symbol_has_a_usable_line_number(self):
        package = context.build_package(self.index, "alpha")
        for item in package["included"]:
            for symbol in item["symbols"]:
                self.assertGreaterEqual(symbol["line"], 1)
                self.assertTrue(symbol["name"])

    def test_the_index_commit_is_carried_into_the_package(self):
        package = context.build_package(self.index, "alpha")
        self.assertEqual(package["index_commit"], self.index["commit"])


class RealRepository(unittest.TestCase):
    """Ground truth against the index of this repository."""

    @classmethod
    def setUpClass(cls):
        built = REPO / ".x3-forge/index.json"
        if built.exists():
            cls.index = json.loads(built.read_text(encoding="utf-8"))
        else:
            cls.index = forge_index.build(REPO)

    def test_the_registry_pallet_is_selected_for_its_own_symbol(self):
        package = context.build_package(self.index, "CertificationTier grants privileges")
        paths = [i["path"] for i in package["included"]]
        self.assertIn("pallets/x3-app-registry/src/lib.rs", paths)

    def test_a_concept_query_reaches_the_right_subsystem(self):
        package = context.build_package(self.index, "critical provider budget fallback")
        paths = [i["path"] for i in package["included"]]
        self.assertTrue(
            any(p.startswith("services/x3-ai-router/") for p in paths),
            f"expected the router in {paths[:6]}",
        )

    def test_morphology_reaches_the_certification_code(self):
        """Regression: substring matching alone could not connect the query word
        `certification` to `CertificationTier` / `certify_application`, so a
        query about certification did not return the certification code."""
        package = context.build_package(self.index, "guardian certification revocation")
        paths = [i["path"] for i in package["included"]]
        self.assertTrue(
            any(p.startswith("pallets/x3-app-registry/") for p in paths),
            f"expected the registry in {paths[:6]}",
        )

    def test_vendored_source_never_outranks_first_party_source(self):
        """Regression: a vendored copy used to outrank the module it vendors,
        because vendored crates simply contain more matching symbols."""
        package = context.build_package(self.index, "settlement provider fallback", budget_tokens=200_000, max_files=40)
        vendored_at = [n for n, i in enumerate(package["included"]) if i["vendored"]]
        first_party_at = [n for n, i in enumerate(package["included"]) if not i["vendored"]]
        if vendored_at and first_party_at:
            self.assertLess(max(first_party_at), min(vendored_at),
                            "first-party source must be offered before vendored copies")

    def test_the_compiler_stays_within_budget_on_the_real_index(self):
        package = context.build_package(self.index, "settlement atomic swap refund")
        self.assertLessEqual(package["estimated_tokens"], package["budget_tokens"])

    def test_cli_prints_something_useful(self):
        result = subprocess.run(
            [sys.executable, str(HERE / "context.py"), "--index", str(REPO / ".x3-forge/index.json"),
             "guardian certification"],
            capture_output=True, text=True, timeout=600, cwd=REPO,
        )
        if not (REPO / ".x3-forge/index.json").exists():
            self.skipTest("no index built")
        self.assertEqual(result.returncode, 0, result.stderr[-1500:])
        self.assertIn("package:", result.stdout)
        self.assertIn("guardian", result.stdout.lower())


if __name__ == "__main__":
    unittest.main()
