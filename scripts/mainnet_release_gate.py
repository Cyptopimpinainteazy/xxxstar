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
import os
import pathlib
import re
import shutil
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

def target_dir() -> pathlib.Path:
    """Where cargo writes build artifacts.

    `CARGO_TARGET_DIR` wins when it is set — `scripts/local-ci.sh` sets it for
    every gate it runs — then `build.target-dir` from `.cargo/config.toml`, then
    `target/`. Hardcoding `ROOT/target` made this gate report

        ✗ x3-chain-node artifact not found after build at target/release/x3-chain-node

    for a build that had just succeeded into the overridden directory: a
    release-blocking failure raised by the gate itself, on any machine that
    redirects the target directory.
    """
    env = os.environ.get("CARGO_TARGET_DIR")
    if env:
        p = pathlib.Path(env)
        return p if p.is_absolute() else (ROOT / p)
    cfg = ROOT / ".cargo" / "config.toml"
    if cfg.exists():
        m = re.search(r'(?m)^\s*target-dir\s*=\s*"([^"]+)"', cfg.read_text())
        if m:
            p = pathlib.Path(m.group(1))
            return p if p.is_absolute() else (ROOT / p)
    return ROOT / "target"


TARGET_DIR = target_dir()

BUILD_TARGETS = [
    ("x3-chain-node", "release/x3-chain-node"),
    (
        "x3-chain-runtime",
        "release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm",
    ),
]


def check_build() -> None:
    print("\n── 2. Build validation ──")
    print(f"  cargo target dir: {TARGET_DIR}")
    for pkg, artifact_rel in BUILD_TARGETS:
        # Try to find already-built artifact
        artifact = TARGET_DIR / artifact_rel
        if artifact.exists():
            ok(f"{pkg} binary found at {artifact}")
            continue
        # Build it
        print(f"  building {pkg}...")
        result = run(["cargo", "build", "--release", "-p", pkg])
        if result.returncode != 0:
            fail(f"{pkg} build failed:\n{result.stderr}")
        elif artifact.exists():
            ok(f"{pkg} built at {artifact}")
        else:
            fail(f"{pkg} artifact not found after build at {artifact}")


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


# ── 5. Runtime upgrade rehearsal ────────────────────────────────────────────

def check_runtime_upgrade_rehearsal() -> None:
    print("\n── 5. Runtime upgrade rehearsal (migration dry-run per variant) ──")
    script = ROOT / "scripts" / "check-runtime-variants.sh"
    if not script.exists():
        fail(f"{script.relative_to(ROOT)} missing — no per-variant migration dry-run")
        return
    # Executes the same OnRuntimeUpgrade hooks an upgrade would, for every
    # construct_runtime! variant. An upgrade that cannot fit in a block, or a
    # variant whose hooks panic, must block a release.
    result = run(["bash", str(script)])
    if result.returncode == 0:
        ok("migration dry-run passes for every construct_runtime! variant")
    else:
        fail("runtime upgrade rehearsal failed for at least one variant")
        for line in result.stdout.splitlines()[-15:]:
            print(f"    {line}")


# ── 5. Reproducible-build prerequisites ──────────────────────────────────────

def check_reproducible_build_prereqs() -> None:
    print("\n── 6. Reproducible-build prerequisites ──")
    # Check srtool availability
    result = run(["which", "srtool"])
    if result.returncode == 0:
        ok("srtool installed (reproducible WASM builds possible)")
    else:
        # `cargo install` puts it in ~/.cargo/bin, which is not always on PATH for
        # non-interactive runs (cron, detached CI steps). Look there before
        # declaring a genuinely installed tool missing.
        fallback = pathlib.Path.home() / ".cargo" / "bin" / "srtool"
        if fallback.exists():
            ok(f"srtool installed at {fallback} (reproducible WASM builds possible)")
        else:
            fail("srtool NOT found — run `make srtool-install` (pinned to the same")
            print("    revision the release job uses; it needs docker and network).")
            print("    Without srtool, WASM builds are non-deterministic,")
            print("    and mainnet genesis artifacts MUST be reproducible.")

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


# ── 6b. The runtime hash, rebuilt and compared ───────────────────────────────

# The recorded output of a reproducible build. It is not documentation: the gate
# rebuilds the runtime inside the same image and fails when any value differs.
REPRODUCIBLE_RECORD = "docs/reports/runtime-wasm-hashes.json"


def _srtool_values(output: str) -> dict[str, dict[str, str]]:
    """Pull the per-runtime hash block out of srtool's output.

    srtool prints one block per artifact, introduced by `== Compact` /
    `== Compressed`:

        == Compressed
         Version          : x3-chain-11 (…)
         Metadata         : V14
         Size             : 1.37 MB (1441379 bytes)
         setCode          : 0xf4b7…
         authorizeUpgrade : 0xc0eb…
         IPFS             : QmRn…
         BLAKE2_256       : 0xa252…
    """
    wanted = {
        "size": "Size",
        "set_code": "setCode",
        "authorize_upgrade": "authorizeUpgrade",
        "ipfs": "IPFS",
        "blake2_256": "BLAKE2_256",
    }
    found: dict[str, dict[str, str]] = {}
    current: str | None = None
    for raw in output.splitlines():
        line = raw.strip()
        if line.startswith("== "):
            label = line[3:].strip().lower()
            current = label if label in ("compact", "compressed") else None
            if current:
                found.setdefault(current, {})
            continue
        if current is None or ":" not in line:
            continue
        key, _, value = line.partition(":")
        key = key.strip()
        value = value.strip()
        for field, label in wanted.items():
            if key != label:
                continue
            if field == "size":
                # `1.37 MB (1441379 bytes)` → `1441379`
                inner = re.search(r"\((\d+) bytes\)", value)
                found[current][field] = inner.group(1) if inner else value
            else:
                found[current][field] = value
    return found


def check_reproducible_build() -> None:
    """Rebuild the runtime in the pinned image and compare against the record.

    Stage 6 only proves srtool is *installed*, which verifies nothing about this
    source tree: a build script that injects a timestamp, a path or a host value
    would still pass it. This stage is the actual claim — same source, same
    image, same bytes — and it is why `make mainnet-check` takes about ten
    minutes longer than it used to.
    """
    print("\n── 6b. Runtime hash, rebuilt and compared ──")

    record_path = ROOT / REPRODUCIBLE_RECORD
    if not record_path.exists():
        fail(f"{REPRODUCIBLE_RECORD} is missing — there is nothing to compare against")
        return
    try:
        record = json.loads(record_path.read_text())
    except (json.JSONDecodeError, ValueError) as exc:
        fail(f"{REPRODUCIBLE_RECORD} is not valid JSON: {exc}")
        return

    expected_runtimes = record.get("runtimes", {})
    if not expected_runtimes:
        fail(f"{REPRODUCIBLE_RECORD} records no runtime hashes")
        return

    # A missing toolchain is already reported by stage 6; do not add a second
    # failure for the same cause.
    if not (shutil.which("srtool") or (pathlib.Path.home() / ".cargo" / "bin" / "srtool").exists()):
        print("  (skipped: srtool is not installed — stage 6 already failed)")
        return
    if shutil.which("docker") is None:
        print("  (skipped: docker is not available — stage 6 already failed)")
        return

    image = record.get("image", "the pinned image")
    print(f"  rebuilding the runtime in {image} (this takes ~10 minutes)…")
    result = run(["bash", str(ROOT / "scripts" / "run-srtool.sh"), "build"])
    if result.returncode != 0:
        fail("the reproducible build itself failed (see scripts/run-srtool.sh output)")
        for line in (result.stdout + result.stderr).splitlines()[-15:]:
            print(f"    {line}")
        return

    actual = _srtool_values(result.stdout + result.stderr)
    if not actual:
        fail("could not read any hash block out of the srtool output")
        return

    for runtime, expected in sorted(expected_runtimes.items()):
        got = actual.get(runtime)
        if got is None:
            fail(f"{runtime}: the rebuild produced no {runtime} artifact")
            continue
        mismatched = [
            f"{field}: expected {value}, rebuilt {got.get(field)}"
            for field, value in sorted(expected.items())
            if str(got.get(field)) != str(value)
        ]
        if mismatched:
            print(
                "    a hash changed. Two possibilities: the runtime source changed since"
            )
            print(
                "    docs/reports/runtime-wasm-hashes.json was recorded — run"
            )
            print(
                "    ./scripts/update-runtime-hashes.sh, which rebuilds twice, refuses to"
            )
            print(
                "    write unless they agree, and re-records it — or the build is not"
            )
            print(
                "    reproducible, in which case that script fails and the release stops."
            )
            for line in mismatched:
                fail(f"{runtime}: {line}")
        else:
            ok(f"{runtime}: rebuilt {got.get('blake2_256')} — matches the record")


# ── 7. Forbidden secrets ─────────────────────────────────────────────────────

def has_forbidden_secrets() -> bool:
    print("\n── 7. Forbidden secrets scan ──")
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
    check_runtime_upgrade_rehearsal()
    check_reproducible_build_prereqs()
    check_reproducible_build()
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
