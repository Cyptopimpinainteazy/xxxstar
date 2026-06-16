#!/usr/bin/env python3
"""
Mainnet Release Gate — Launch-critical validation.

Replaces the prior documentation-only check with:
  1. Build validation (x3-chain-node, x3-chain-runtime WASM)
  2. Chain-spec / genesis artifact verification
  3. Critical runtime and pallet test suites
  4. Reproducible-build prerequisite check (srtool)
  5. Required documentation check (preserved from original)
  6. Forbidden-secret scanning (preserved from original)

Exit 0 → gate PASSES.
Exit 1 → gate FAILS — do NOT cut a release.
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
FAILURES: list[str] = []


# ── helpers ──────────────────────────────────────────────────────────────────

def run(cmd: list[str], cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd or ROOT)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"  ✗ {msg}")


def ok(msg: str) -> None:
    print(f"  ✓ {msg}")


def path_exists(rel: str) -> pathlib.Path:
    p = ROOT / rel
    if not p.exists():
        fail(f"required path missing: {rel}")
    else:
        ok(f"found {rel}")
    return p


# ── 1. Required documentation ────────────────────────────────────────────────

REQUIRED_DOCS = [
    "MAINNET_READINESS.md",
    "INVARIANTS.md",
    "RELEASE_GATES.md",
    "SECURITY.md",
    "TESTING.md",
    "AUDIT_SPEC.md",
]


def check_required_docs() -> None:
    print("\n── 1. Required documentation ──")
    missing = [d for d in REQUIRED_DOCS if not (ROOT / d).exists()]
    if missing:
        for m in missing:
            fail(f"missing required doc: {m}")
    else:
        ok("all required docs present")


# ── 2. Build validation ─────────────────────────────────────────────────────

BUILD_TARGETS = [
    ("x3-chain-node", "target/release/x3-chain-node"),
    ("x3-chain-runtime", "target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"),
]


def check_build() -> None:
    print("\n── 2. Build validation ──")
    for pkg, artifact_rel in BUILD_TARGETS:
        # Try to find already-built artifact
        artifact = ROOT / artifact_rel
        if artifact.exists():
            ok(f"{pkg} binary found at {artifact_rel}")
            continue
        # Build it
        print(f"  building {pkg}...")
        result = run(["cargo", "build", "--release", "-p", pkg])
        if result.returncode != 0:
            fail(f"{pkg} build failed:\n{result.stderr}")
        elif artifact.exists():
            ok(f"{pkg} built at {artifact_rel}")
        else:
            fail(f"{pkg} artifact not found after build at {artifact_rel}")


# ── 3. Chain-spec / genesis artifact verification ────────────────────────────

ARTIFACT_CHECKS = [
    "chain-specs/x3-local3-current-plain.json",
    "chain-specs/x3-local3-current-raw.json",
]


def check_chain_spec_artifacts() -> None:
    print("\n── 3. Chain-spec / genesis artifacts ──")
    for rel in ARTIFACT_CHECKS:
        p = path_exists(rel)
        if p.exists():
            # Quick validity: must be parseable JSON with expected keys
            try:
                data = json.loads(p.read_text())
                if not isinstance(data, dict) or "genesis" not in data:
                    fail(f"{rel} is valid JSON but missing 'genesis' key")
                else:
                    ok(f"{rel} valid genesis spec")
            except (json.JSONDecodeError, ValueError):
                fail(f"{rel} is not valid JSON")

    # Verify a mainnet-ready spec path exists in chain_spec.rs
    spec_src = ROOT / "node/src/chain_spec.rs"
    if spec_src.exists():
        content = spec_src.read_text()
        if "production_config" in content:
            ok("production_config() found in node/src/chain_spec.rs")
        else:
            fail("production_config() not found in node/src/chain_spec.rs")
    else:
        fail("node/src/chain_spec.rs not found")


# ── 4. Critical runtime & pallet test suites ─────────────────────────────────

TEST_PACKAGES = [
    # Each entry: (cargo-flag, pkg-name, features-list)
    ("-p", "x3-chain-runtime", ["--features", "try-runtime"]),
    ("-p", "pallet-x3-supply-ledger", []),
    ("-p", "x3-packet-standard", []),
    ("-p", "x3-bridge", []),
    ("-p", "x3-fees", []),
    ("-p", "pallet-x3-slash", []),
]


def check_test_suites() -> None:
    print("\n── 4. Critical runtime & pallet test suites ──")
    for _flag, pkg_name, features in TEST_PACKAGES:
        print(f"  running tests for {pkg_name}...")
        cmd = ["cargo", "test", "-p", pkg_name, "--lib", "--no-fail-fast", "-q"]
        if features:
            cmd.extend(features)
        cmd.extend(["--", "--nocapture"])
        result = run(cmd)
        if result.returncode != 0:
            # Show last 20 lines of test output on failure
            lines = result.stdout.splitlines()[-20:]
            stderr_lines = result.stderr.splitlines()[-20:]
            fail(f"{pkg_name} tests failed")
            for l in lines:
                print(f"    {l}")
            for l in stderr_lines:
                print(f"    {l}")
        else:
            ok(f"{pkg_name} tests passed")


# ── 5. Reproducible-build prerequisites ──────────────────────────────────────

def check_reproducible_build_prereqs() -> None:
    print("\n── 5. Reproducible-build prerequisites ──")
    # Check srtool availability
    result = run(["which", "srtool"])
    if result.returncode == 0:
        ok("srtool installed (reproducible WASM builds possible)")
    else:
        fail("srtool NOT found — install from https://github.com/paritytech/srtool")
        print("    Without srtool, WASM builds are non-deterministic.")
        print("    Mainnet genesis artifacts MUST be reproducible.")

    # Check docker (required by srtool)
    result = run(["docker", "--version"])
    if result.returncode == 0:
        ok("docker available (required by srtool)")
    else:
        fail("docker NOT found — srtool will not function")

    # Verify SKIP_WASM_BUILD is not forced in a way that would skip embedded WASM
    if "SKIP_WASM_BUILD" in (ROOT / ".cargo/config.toml").read_text() if (ROOT / ".cargo/config.toml").exists() else "":
        fail("SKIP_WASM_BUILD set in .cargo/config.toml — embedded WASM will be missing")
    else:
        ok("no SKIP_WASM_BUILD override detected")


# ── 6. Forbidden secrets ─────────────────────────────────────────────────────

def has_forbidden_secrets() -> bool:
    print("\n── 6. Forbidden secrets scan ──")
    assignment_re = re.compile(r"(?m)^\s*(?:export\s+)?(?:PRIVATE_KEY|MNEMONIC)\s*=\s*([^\s#]+)")
    aws_key_re = re.compile(r"AKIA[0-9A-Z]{16}")
    example_value_prefixes = ("replace_", "your_", "<", "$")
    ignored_dirs = {".git", "target", "node_modules", ".venv", ".cocoindex_code"}
    found = False
    for p in ROOT.rglob("*"):
        if not p.is_file() or any(x in p.parts for x in ignored_dirs):
            continue
        try:
            txt = p.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        has_secret_assignment = any(
            not match.group(1).strip("\"'").lower().startswith(example_value_prefixes)
            for match in assignment_re.finditer(txt)
        )
        if has_secret_assignment or aws_key_re.search(txt):
            fail(f"secret-like token found: {p.relative_to(ROOT)}")
            found = True
    if not found:
        ok("no hardcoded secrets detected")
    return found


# ── main ──────────────────────────────────────────────────────────────────────

def main() -> int:
    print("═" * 60)
    print("  Mainnet Release Gate")
    print("═" * 60)

    check_required_docs()
    check_build()
    check_chain_spec_artifacts()
    check_test_suites()
    check_reproducible_build_prereqs()
    has_forbidden_secrets()

    print(f"\n{'═' * 60}")
    if FAILURES:
        print(f"  ❌ GATE FAILED — {len(FAILURES)} failure(s):")
        for f in FAILURES:
            print(f"    • {f}")
        print(f"{'═' * 60}")
        return 1
    else:
        print("  ✅ mainnet_release_gate: PASS")
        print(f"{'═' * 60}")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())