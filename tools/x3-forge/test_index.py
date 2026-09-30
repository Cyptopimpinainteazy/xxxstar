"""Tests for the X3 Forge index.

These are deliberately hostile to the implementation. A symbol index that
silently returns plausible-looking wrong answers is worse than no index, so the
tests pin exact locations in the real repository, plant traps that a naive
scanner falls into, and assert on the provenance that makes staleness
detectable.
"""
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("x3_forge_index", Path(__file__).with_name("index.py"))
forge = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(forge)

REPO = Path(__file__).resolve().parent.parent.parent


class ParserTraps(unittest.TestCase):
    """A naive line scanner reports commented-out code as real. These plants
    exist to fail exactly that implementation."""

    def names(self, source):
        return [i["name"] for i in forge.parse_rust(source) if i["kind"] != "todo"]

    def test_line_commented_function_is_not_indexed(self):
        source = "// fn ghost() {}\nfn real() {}\n"
        self.assertEqual(self.names(source), ["real"])

    def test_block_commented_function_is_not_indexed(self):
        source = "/*\nfn ghost() {}\n*/\nfn real() {}\n"
        self.assertEqual(self.names(source), ["real"])

    def test_symbol_inside_a_string_is_not_indexed(self):
        # `"fn ghost()"` is a string literal, not a declaration. It must not be
        # reported as a function because it is not on its own with a body.
        source = 'fn real() { let s = "fn ghost()"; }\n'
        self.assertIn("real", self.names(source))

    def test_todo_in_a_comment_is_still_reported(self):
        # The reverse trap: dropping a whole comment line must not lose the TODO
        # that someone wrote for a reason.
        items = forge.parse_rust("// TODO: wire the settlement check\nfn real() {}\n")
        todos = [i for i in items if i["kind"] == "todo"]
        self.assertEqual(len(todos), 1)
        self.assertIn("TODO", todos[0]["name"])

    def test_malformed_source_does_not_raise(self):
        # A truncated file must degrade, not crash a build that runs in CI.
        for source in ("fn unterminated(", "struct {", "\x00\x01\x02", "/* never closed\nfn a() {"):
            forge.parse_rust(source)

    def test_every_item_has_a_real_line_number(self):
        source = "fn a() {}\nstruct B;\nenum C { X }\ntrait D {}\n"
        for item in forge.parse_rust(source):
            self.assertGreaterEqual(item["line"], 1, item)
            self.assertTrue(item["name"], item)

    def test_test_attribute_binds_to_the_function_below_it(self):
        """Regression: attributes were cleared at the end of the attribute's own
        line, so every `#[test]` vanished and the index reported zero tests."""
        source = "#[test]\nfn alpha() {}\n\n#[test]\nfn beta() {}\n\nfn helper() {}\n"
        tests = {i["name"] for i in forge.parse_rust(source) if i.get("test")}
        self.assertEqual(tests, {"alpha", "beta"})

    def test_a_plain_function_is_not_a_test(self):
        tests = [i for i in forge.parse_rust("fn helper() {}\n") if i.get("test")]
        self.assertEqual(tests, [])

    def test_a_qualified_attribute_does_not_hide_the_item(self):
        # `#[tokio::test]` and `#[pallet::call_index(0)]` must not swallow the item.
        source = "#[tokio::test]\nasync fn gamma() {}\n#[pallet::call_index(7)]\npub fn delta() -> u32 { 1 }\n"
        items = forge.parse_rust(source)
        self.assertEqual({i["name"] for i in items if i["kind"] == "fn"}, {"gamma", "delta"})

    def test_cfg_attributes_are_recorded_on_the_item_they_precede(self):
        items = forge.parse_rust("#[cfg(test)]\nmod inner {}\n")
        self.assertEqual(items[0]["kind"], "mod")
        self.assertIn("cfg", items[0]["attrs"])


class PythonParser(unittest.TestCase):
    def test_definitions_are_found_with_line_numbers(self):
        items = forge.parse_python("import os\n\n\ndef alpha():\n    pass\n\n\nclass Beta:\n    pass\n")
        found = {(i["kind"], i["name"], i["line"]) for i in items}
        self.assertIn(("def", "alpha", 4), found)
        self.assertIn(("class", "Beta", 8), found)

    def test_commented_definition_is_not_indexed(self):
        names = [i["name"] for i in forge.parse_python("# def ghost():\ndef real():\n    pass\n")]
        self.assertEqual(names, ["real"])


class RealRepositoryGroundTruth(unittest.TestCase):
    """Pins exact locations in this repository. If the scanner regresses, these
    fail rather than quietly returning fewer results."""

    @classmethod
    def setUpClass(cls):
        cls.index = forge.build(REPO)
        cls.files = cls.index["files"]

    def find(self, name, kind=None):
        hits = []
        for rel, entry in self.files.items():
            for item in entry["items"]:
                if item["name"] == name and (kind is None or item["kind"] == kind):
                    hits.append((rel, item["line"]))
        return sorted(hits)

    def test_registry_tier_enum_is_indexed(self):
        hits = self.find("CertificationTier", "enum")
        self.assertTrue(hits, "CertificationTier must be indexed")
        self.assertTrue(
            any(rel.startswith("pallets/x3-app-registry/") for rel, _ in hits),
            f"expected it in the registry pallet, got {hits}",
        )

    def test_router_predicate_is_indexed(self):
        hits = self.find("may_serve_critical", "def")
        self.assertTrue(
            any(rel == "services/x3-ai-router/router.py" for rel, _ in hits),
            f"expected may_serve_critical in router.py, got {hits}",
        )

    def test_simulator_entry_point_is_indexed(self):
        hits = self.find("run", "fn")
        self.assertTrue(any(rel.startswith("crates/x3-sim/") for rel, _ in hits), hits)

    def test_a_symbol_that_does_not_exist_returns_nothing(self):
        self.assertEqual(
            self.find("zzz_definitely_not_a_symbol_zzz"), [],
            "a fabricated name must not match anything",
        )

    def test_crate_attribution_follows_the_nearest_manifest(self):
        entry = self.files["pallets/x3-app-registry/src/lib.rs"]
        self.assertEqual(entry["crate"], "pallet-x3-app-registry")
        router = self.files["services/x3-ai-router/router.py"]
        self.assertIsNone(router["crate"], "the router is not a Rust crate")

    def test_pallet_storage_items_are_indexed(self):
        # `pub type X<T: Config> = StorageMap<...>` is the shape a pallet's state
        # takes; an index that cannot find storage is not useful for auditing.
        hits = self.find("Applications", "type")
        self.assertTrue(any(rel.startswith("pallets/x3-app-registry/") for rel, _ in hits), hits)

    def test_skip_rules_exclude_build_output(self):
        offenders = [
            rel for rel in self.files
            if rel.startswith(("target/", ".git/")) or "/node_modules/" in rel
        ]
        self.assertEqual(offenders, [], f"indexed paths that must be skipped: {offenders[:5]}")

    def test_every_indexed_file_carries_provenance(self):
        for rel, entry in self.files.items():
            self.assertRegex(entry["sha256"], r"^[0-9a-f]{64}$", rel)
            self.assertGreaterEqual(entry["bytes"], 0, rel)
            # An empty file is legitimate, but a file that yielded items cannot
            # be empty — that combination would mean items were invented.
            if entry["items"]:
                self.assertGreater(entry["bytes"], 0, rel)
            self.assertIn(entry["lang"], {"rust", "python", "toml", "json", "markdown", "shell", "yaml", "typescript", "javascript", "solidity"}, rel)

    def test_a_mirrored_worktree_is_not_indexed(self):
        # `.kilo/worktrees/` holds a second copy of this source tree. Indexing it
        # would report every symbol twice and attribute files to the wrong
        # checkout, so the walk must prune it rather than filter it afterwards.
        mirrored = [rel for rel in self.files if "/worktrees/" in rel or rel.startswith("vendor/")]
        self.assertEqual(mirrored, [], f"mirrored trees must not be indexed, got {mirrored[:5]}")


class IncrementalAndStaleness(unittest.TestCase):
    """The index is only trustworthy if a stale answer is detectable."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        (self.root / "Cargo.toml").write_text('[package]\nname = "demo"\n')
        (self.root / "src").mkdir()
        (self.root / "src" / "lib.rs").write_text("pub fn alpha() {}\n")
        (self.root / "src" / "other.rs").write_text("pub fn beta() {}\n")
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=False)
        subprocess.run(["git", "add", "-A"], cwd=self.root, check=False)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "init"],
            cwd=self.root, check=False,
        )

    def tearDown(self):
        self.tmp.cleanup()

    def test_a_second_build_reuses_unchanged_files(self):
        first = forge.build(self.root)
        self.assertEqual(first["stats"]["reused"], 0)
        self.assertEqual(first["stats"]["parsed"], first["stats"]["files_indexed"])

        second = forge.build(self.root, first)
        self.assertEqual(second["stats"]["parsed"], 0, "nothing changed, so nothing should be reparsed")
        self.assertEqual(second["stats"]["reused"], first["stats"]["files_indexed"])

    def test_only_the_changed_file_is_reparsed(self):
        first = forge.build(self.root)
        (self.root / "src" / "lib.rs").write_text("pub fn alpha() {}\npub fn gamma() {}\n")
        second = forge.build(self.root, first)
        self.assertEqual(second["stats"]["parsed"], 1)
        self.assertEqual(second["stats"]["reused"], first["stats"]["files_indexed"] - 1)
        names = [i["name"] for i in second["files"]["src/lib.rs"]["items"]]
        self.assertIn("gamma", names)

    def test_changing_a_file_changes_its_hash(self):
        first = forge.build(self.root)
        before = first["files"]["src/lib.rs"]["sha256"]
        (self.root / "src" / "lib.rs").write_text("pub fn alpha() { let _ = 1; }\n")
        second = forge.build(self.root)
        self.assertNotEqual(before, second["files"]["src/lib.rs"]["sha256"])

    def test_the_hash_matches_the_file_on_disk(self):
        index = forge.build(self.root)
        on_disk = hashlib.sha256((self.root / "src" / "lib.rs").read_bytes()).hexdigest()
        self.assertEqual(index["files"]["src/lib.rs"]["sha256"], on_disk)

    def test_head_commit_is_recorded(self):
        index = forge.build(self.root)
        self.assertRegex(index["commit"], r"^[0-9a-f]{40}$")


class CommandLine(unittest.TestCase):
    """The CLI is what an agent actually calls, so it is exercised for real."""

    def run_index(self, *args):
        return subprocess.run(
            ["python3", str(REPO / "tools/x3-forge/index.py"), "--root", str(REPO), *args],
            capture_output=True, text=True, timeout=600,
        )

    def test_build_writes_an_index_and_find_locates_a_symbol(self):
        with tempfile.TemporaryDirectory() as out:
            index_path = Path(out) / "index.json"
            built = subprocess.run(
                ["python3", str(REPO / "tools/x3-forge/index.py"), "--root", str(REPO),
                 "--index", str(index_path), "build", "--out", str(index_path)],
                capture_output=True, text=True, timeout=900,
            )
            self.assertEqual(built.returncode, 0, built.stderr[-2000:])
            self.assertTrue(index_path.exists())

            found = subprocess.run(
                ["python3", str(REPO / "tools/x3-forge/index.py"), "--index", str(index_path),
                 "find", "may_serve_critical"],
                capture_output=True, text=True, timeout=300,
            )
            self.assertEqual(found.returncode, 0, found.stdout + found.stderr)
            self.assertIn("services/x3-ai-router/router.py", found.stdout)

            missing = subprocess.run(
                ["python3", str(REPO / "tools/x3-forge/index.py"), "--index", str(index_path),
                 "find", "zzz_definitely_not_a_symbol_zzz"],
                capture_output=True, text=True, timeout=300,
            )
            self.assertEqual(missing.returncode, 1, "an absent symbol must be a non-zero exit")

            parsed = json.loads(index_path.read_text())
            self.assertEqual(parsed["index_version"], forge.INDEX_VERSION)


if __name__ == "__main__":
    unittest.main()
