"""RUSTSEC-2026-0381/0383: the distrusted emSign ECC Root CA - G3 must not be trusted.

`deny.toml` and `.cargo/audit.toml` ignore both advisories, because their published fixed
range (`>= 1.0.10`) also matches two things that are not affected and cannot be upgraded:

* webpki-roots 0.25.4, pulled by libp2p-websocket inside polkadot-sdk's sc-network. It did
  bundle the root; this repository builds it from `patches/webpki-roots-0.25.4`, which removes
  exactly that anchor, the same change upstream made in 1.0.10.
* the 0.26.11 shims of webpki-roots and webpki-root-certs, which bundle no roots at all and
  re-export the 1.x crate.

An ignore is only as good as the facts behind it, so this test re-checks them on every run and
fails the moment one stops being true. See docs/security/RUSTSEC-2026-0381.md.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
PATCH = ROOT / "patches/webpki-roots-0.25.4"
PROVENANCE = ROOT / "patches/webpki-roots-provenance.json"
G3_LABEL = "emSign ECC Root CA - G3"
G3_SHA256 = "86:a1:ec:ba:08:9c:4a:8d:3b:be:27:34:c6:12:ba:34:1d:81:3e:04:3c:f9:e8:a8:62:cd:5c:57:a3:6b:be:6b"
FIXED = (1, 0, 10)
PACKAGES = ("webpki-roots", "webpki-root-certs")

# Workspaces that ship, and therefore must build 0.25.x from the patch, never crates.io.
SHIPPED_LOCKS = ("Cargo.lock", "crates/x3-sidecar/Cargo.lock")

# Lockfiles allowed to keep an unpatched pre-0.26 copy, each with the reason. A new entry here is
# a decision, not a convenience: it must say why that workspace is not a shipped artifact.
UNPATCHED_PRE_026 = {
    "X3-contracts/svm/programs/x3_htlc/tests-live/Cargo.lock":
        "Solana HTLC live-test harness (solana-client); not a shipped artifact",
    "programs/svm/x3_atomic_swap/Cargo.lock":
        "Solana program workspace; webpki-roots only reaches its off-chain test client",
    "patches/sc-rpc-server/Cargo.lock":
        "a vendored crate's own lockfile; cargo never reads the lock of a path dependency",
}
# Lockfiles allowed to keep a pre-fix 1.x copy, for the same reason as the last entry above.
STALE_VENDORED_1X = {
    "patches/quinn-proto/Cargo.lock":
        "a vendored crate's own lockfile; cargo never reads the lock of a path dependency",
}



def version_tuple(raw):
    return tuple(int(part) for part in re.match(r"(\d+)\.(\d+)\.(\d+)", raw).groups())


def packages(lock_text):
    """Every [[package]] in a Cargo.lock: {name, version, source, dependencies}."""
    found = []
    for block in lock_text.split("[[package]]")[1:]:
        name = re.search(r'^name = "([^"]+)"', block, re.M)
        version = re.search(r'^version = "([^"]+)"', block, re.M)
        source = re.search(r'^source = "([^"]+)"', block, re.M)
        deps = re.findall(r'^ "([^"]+)",$', block, re.M)
        if name and version:
            found.append({"name": name.group(1), "version": version.group(1),
                          "source": source.group(1) if source else None, "dependencies": deps})
    return found


def violations(lock_text, shipped, allow_unpatched_pre_026, allow_stale_1x):
    """What is wrong with one lockfile, as readable strings (empty when it is fine)."""
    problems = []
    entries = packages(lock_text)
    one_x = {e["name"]: e["version"] for e in entries
             if e["name"] in PACKAGES and e["version"].startswith("1.")}
    for e in entries:
        if e["name"] not in PACKAGES:
            continue
        v = version_tuple(e["version"])
        if v[0] == 1 and v < FIXED and not allow_stale_1x:
            problems.append(f"{e['name']} {e['version']} is below the fixed 1.0.10")
        elif v[:2] == (0, 26):
            parents = [d for d in e["dependencies"] if d.split(" ")[0] == e["name"]]
            if not parents:
                problems.append(f"{e['name']} {e['version']} no longer re-exports a 1.x crate")
            for parent in parents:
                parent_version = parent.split(" ")[1] if " " in parent else one_x.get(e["name"])
                if parent_version is None or version_tuple(parent_version) < FIXED:
                    problems.append(f"{e['name']} {e['version']} re-exports {parent_version}, "
                                    "below the fixed 1.0.10")
        elif v < (0, 26, 0) and e["name"] == "webpki-roots":
            if shipped and e["source"] is not None:
                problems.append(f"webpki-roots {e['version']} resolves from {e['source']}, not "
                                "patches/webpki-roots-0.25.4: the [patch.crates-io] entry is gone")
            elif not shipped and not allow_unpatched_pre_026:
                problems.append(f"unpatched webpki-roots {e['version']} in a lockfile that is "
                                "not listed in UNPATCHED_PRE_026")
    return problems


def lockfiles():
    """The tree's lockfiles, by the same walk `advisory scope` uses (it skips snapshots and
    vendored sources), so the two gates cannot disagree about what "the tree" is."""
    spec = importlib.util.spec_from_file_location(
        "check_advisory_scope", ROOT / "scripts/check-advisory-scope.py")
    scope = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(scope)
    for path in scope.lockfiles():
        yield path.relative_to(ROOT).as_posix()


class VendoredRootsTests(unittest.TestCase):
    def test_the_patch_removes_exactly_the_distrusted_root(self):
        source = (PATCH / "src/lib.rs").read_text()
        self.assertNotIn(G3_LABEL, source)
        self.assertNotIn(G3_SHA256, source)
        self.assertEqual(source.count("TrustAnchor {"), 146, "147 upstream anchors minus G3")
        for kept in ("emSign Root CA - C1", "emSign Root CA - G1", "emSign ECC Root CA - C3"):
            self.assertIn(kept, source, "1.0.10 keeps the other emSign roots; so does the patch")

    def test_the_patched_source_is_the_one_recorded_in_provenance(self):
        record = json.loads(PROVENANCE.read_text())[0]
        digest = hashlib.sha256((PATCH / "src/lib.rs").read_bytes()).hexdigest()
        self.assertEqual(digest, record["patched_lib_rs_sha256"])
        self.assertNotEqual(record["upstream_lib_rs_sha256"], record["patched_lib_rs_sha256"])


class TreeTests(unittest.TestCase):
    def test_every_lockfile_in_the_tree(self):
        seen = set()
        for rel in lockfiles():
            text = (ROOT / rel).read_text()
            if not any(f'name = "{p}"' in text for p in PACKAGES):
                continue
            seen.add(rel)
            with self.subTest(lockfile=rel):
                self.assertEqual(violations(text, rel in SHIPPED_LOCKS, rel in UNPATCHED_PRE_026,
                                            rel in STALE_VENDORED_1X), [])
        for rel in (*SHIPPED_LOCKS, *UNPATCHED_PRE_026, *STALE_VENDORED_1X):
            self.assertIn(rel, seen, f"{rel} is named here but no longer carries webpki roots; "
                                     "drop the entry")

    def test_shipped_workspaces_build_the_patch(self):
        for rel in SHIPPED_LOCKS:
            with self.subTest(lockfile=rel):
                pre_026 = [e for e in packages((ROOT / rel).read_text())
                           if e["name"] == "webpki-roots" and version_tuple(e["version"]) < (0, 26, 0)]
                self.assertTrue(pre_026, "expected the libp2p-websocket copy; re-check this test")
                self.assertTrue(all(e["source"] is None for e in pre_026), pre_026)


def lock(*blocks):
    return "".join("\n[[package]]\n" + b for b in blocks)


REGISTRY = 'source = "registry+https://github.com/rust-lang/crates.io-index"\n'


class DetectorTests(unittest.TestCase):
    """The checks above have to be able to fail."""

    def test_a_pre_fix_1x_is_reported(self):
        text = lock('name = "webpki-roots"\nversion = "1.0.9"\n' + REGISTRY)
        self.assertEqual(len(violations(text, True, False, False)), 1)

    def test_a_shim_over_a_pre_fix_parent_is_reported(self):
        text = lock('name = "webpki-roots"\nversion = "0.26.11"\n' + REGISTRY
                    + 'dependencies = [\n "webpki-roots 1.0.9",\n]\n',
                    'name = "webpki-roots"\nversion = "1.0.9"\n' + REGISTRY)
        self.assertTrue(any("re-exports 1.0.9" in p for p in violations(text, True, False, True)))

    def test_a_shim_over_the_fixed_parent_is_clean(self):
        text = lock('name = "webpki-roots"\nversion = "0.26.11"\n' + REGISTRY
                    + 'dependencies = [\n "webpki-roots 1.0.10",\n]\n',
                    'name = "webpki-roots"\nversion = "1.0.10"\n' + REGISTRY)
        self.assertEqual(violations(text, True, False, False), [])

    def test_a_shipped_lock_resolving_0_25_from_crates_io_is_reported(self):
        text = lock('name = "webpki-roots"\nversion = "0.25.4"\n' + REGISTRY)
        self.assertTrue(any("patch.crates-io" in p for p in violations(text, True, False, False)))

    def test_the_patched_0_25_is_clean(self):
        self.assertEqual(violations(lock('name = "webpki-roots"\nversion = "0.25.4"\n'),
                                    True, False, False), [])

    def test_an_unlisted_unpatched_copy_is_reported(self):
        text = lock('name = "webpki-roots"\nversion = "0.24.0"\n' + REGISTRY)
        self.assertTrue(violations(text, False, False, False))
        self.assertEqual(violations(text, False, True, False), [])


if __name__ == "__main__":
    unittest.main()
