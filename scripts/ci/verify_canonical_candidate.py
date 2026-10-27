#!/usr/bin/env python3
"""Read-only provenance and feature-scope gate for the X3 public-testnet candidate.

Passing this gate proves only source identity, configuration and ancestry. It
does NOT prove consensus determinism, live execution, validator economics,
independent audits or release approval.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).with_name("canonical-candidate-manifest.json")
SHA = re.compile(r"^[0-9a-f]{40}$")
PR_IDS = {681, 683, 691, 700, 701, 702}
REQUIRED_PROOFS = {
    "atomic_execution", "real_dex_liquidity", "stateful_evm", "svm_onchain",
    "external_bridges", "validator_economics", "security", "operations",
}


def require_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or not SHA.fullmatch(value):
        raise ValueError(f"{label}: invalid full Git SHA")
    return value


def safe_path(raw: object) -> str:
    if not isinstance(raw, str) or not raw or len(raw) > 250:
        raise ValueError("missing or overlong manifest file path")
    p = PurePosixPath(raw)
    if (p.is_absolute() or ".." in p.parts or "." in p.parts or "\\" in raw or
            str(p) != raw or raw.startswith("-")):
        raise ValueError(f"unsafe file path {raw!r}")
    return raw


def validate_manifest(manifest: dict) -> list[dict]:
    if not isinstance(manifest, dict) or manifest.get("schema_version") != 1:
        raise ValueError("unknown candidate manifest schema")
    require_sha(manifest.get("execution_parent_sha"), "execution parent")
    require_sha(manifest.get("required_upstream_security_commit"), "security floor")
    if manifest.get("release_profile") != "mainnet-rc1":
        raise ValueError("canonical candidate must use narrowed mainnet-rc1")
    if manifest.get("external_bridges_must_remain_disabled") is not True:
        raise ValueError("external bridging must remain disabled")
    if manifest.get("release_authorized") is not False:
        raise ValueError("manifest cannot authorize a launch")

    domains = manifest.get("required_proofs")
    if not isinstance(domains, list) or len(domains) != len(REQUIRED_PROOFS):
        raise ValueError("missing or duplicate release proof domains")
    names = [entry.get("gate") for entry in domains if isinstance(entry, dict)]
    if set(names) != REQUIRED_PROOFS or len(set(names)) != len(names):
        raise ValueError("missing or duplicate mandatory proof domain")
    if any(not isinstance(entry.get("evidence"), str) or len(entry["evidence"]) < 15
           for entry in domains):
        raise ValueError("empty release proof obligation")

    sources = manifest.get("source_overlays")
    if not isinstance(sources, list) or len(sources) != len(PR_IDS):
        raise ValueError("incomplete source PR lineage")
    seen_prs, seen_paths, entries = set(), set(), []
    for item in sources:
        if not isinstance(item, dict) or type(item.get("pr")) is not int:
            raise ValueError("invalid PR provenance")
        pr = item["pr"]
        if pr not in PR_IDS or pr in seen_prs:
            raise ValueError("duplicate or unsupported source PR")
        seen_prs.add(pr)
        require_sha(item.get("head_sha"), f"PR #{pr} source head")
        files = item.get("files")
        if not isinstance(files, list) or not files:
            raise ValueError("source PR missing file identities")
        for file in files:
            if not isinstance(file, dict):
                raise ValueError("malformed source file")
            path = safe_path(file.get("path"))
            if path in seen_paths:
                raise ValueError("multiple sources modify the same file")
            seen_paths.add(path)
            require_sha(file.get("blob_sha"), f"{path} content")
            entries.append(file)
    if seen_prs != PR_IDS or len(entries) != 36:
        raise ValueError("candidate must contain precisely 36 frozen overlay files")
    if [f["path"] for f in sources[[s["pr"] for s in sources].index(701)]["files"]] != [
        ".github/workflows/x3-automation-consensus.yml"
    ]:
        raise ValueError("older automation implementation must not overwrite integrated code")
    return entries


def git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=root, capture_output=True,
        text=True, check=False, timeout=30,
    )


def git_ancestor(root: Path, older: str, newer: str) -> bool:
    return git(root, "merge-base", "--is-ancestor", older, newer).returncode == 0


def verify_source_blobs(root: Path, files: list[dict]) -> list[str]:
    failures = []
    for source in files:
        relative = source["path"]
        path = root / relative
        if (path.is_symlink() or not path.is_file() or
                not path.resolve().is_relative_to(root.resolve())):
            failures.append(f"missing or unsafe source file: {relative}")
            continue
        result = git(root, "hash-object", "--", relative)
        if result.returncode or result.stdout.strip() != source["blob_sha"]:
            failures.append(f"source blob changed or unreadable: {relative}")
    return failures


def toml_document(path: Path) -> dict:
    try:
        import tomllib
    except ModuleNotFoundError:
        try:
            import tomli as tomllib
        except ModuleNotFoundError as exc:
            raise ValueError("tomllib/tomli missing; install requirements-dev.txt") from exc
    with path.open("rb") as file:
        return tomllib.load(file)


def check_rc1_runtime_pallet_scope(source: str) -> list[str]:
    """Verify the actual narrow construct_runtime! variant, not just TOML flags."""
    start = source.find("// ── mainnet-rc1:")
    end = source.find("// ── default (", start + 1)
    if start < 0 or end <= start:
        return ["runtime: explicit narrowed RC-1 construct_runtime variant missing"]
    body = source[start:end]
    failures = []
    for component in (
        "AtomicTradeEngine", "X3AtomicKernel", "X3CrossVmRouter",
        "X3SettlementEngine", "X3SupplyLedger", "SvmRuntime",
    ):
        if not re.search(r"^\\s*" + re.escape(component) + r"\\s*:", body, re.MULTILINE):
            failures.append(f"runtime: required RC-1 pallet {component} is not active")
    for experimental in ("X3Dex", "X3Automation", "Evm", "Ethereum"):
        if re.search(r"^\\s*" + re.escape(experimental) + r"\\s*:", body, re.MULTILINE):
            failures.append(f"runtime: experimental {experimental} unexpectedly active in RC-1")
    if 'not(feature = "frontier")' not in body:
        failures.append("runtime: RC-1 lacks explicit non-Frontier feature guard")
    return failures


def verify_bridge_feature_scope(root: Path) -> list[str]:
    files = {
        "node": root / "node/Cargo.toml",
        "runtime": root / "runtime/Cargo.toml",
        "router": root / "pallets/x3-cross-vm-router/Cargo.toml",
    }
    doc = {key: toml_document(path) for key, path in files.items()}
    failures = []
    for component in doc:
        features = doc[component].get("features", {})
        rc1 = features.get("mainnet-rc1")
        if not isinstance(rc1, list) or not rc1:
            failures.append(f"{component}: mainnet-rc1 profile missing")
            continue
        if any(not isinstance(x, str) for x in rc1):
            failures.append(f"{component}: malformed RC-1 features")
            continue
        forbidden = ("external-gateway", "mainnet-v1", "frontier")
        if any(any(term in x for term in forbidden) for x in rc1):
            failures.append(f"{component}: RC-1 enables external or experimental VM/gateway")
    router = doc["router"].get("features", {})
    runtime = doc["runtime"].get("features", {})
    if "external-gateway" not in router.get("mainnet-v1", []):
        failures.append("router: mainnet-v1 gateway feature contract changed")
    if "pallet-x3-cross-vm-router/mainnet-rc1" not in runtime.get("mainnet-rc1", []):
        failures.append("runtime: router scope-lock not activated by RC-1")
    if "x3-chain-runtime/mainnet-rc1" not in doc["node"].get("features", {}).get("mainnet-rc1", []):
        failures.append("node: narrowed runtime RC-1 profile not forwarded")
    failures.extend(check_rc1_runtime_pallet_scope(
        (root / "runtime/src/lib.rs").read_text(encoding="utf-8")
    ))
    return failures


def evaluate(manifest: dict, root: Path, expected_head: str) -> dict:
    require_sha(expected_head, "expected head")
    files = validate_manifest(manifest)
    actual = git(root, "rev-parse", "--verify", "HEAD")
    failures = []
    if actual.returncode or actual.stdout.strip() != expected_head:
        failures.append("checkout is not the exact expected candidate head")
    if not git_ancestor(root, manifest["execution_parent_sha"], expected_head):
        failures.append("canonical execution parent is not an ancestor")
    if not git_ancestor(root, manifest["required_upstream_security_commit"], expected_head):
        failures.append("new master security baseline not reconciled into candidate")
    failures.extend(verify_source_blobs(root, files))
    failures.extend(verify_bridge_feature_scope(root))
    return {
        "schema_version": 1,
        "candidate_sha": expected_head,
        "decision": "BLOCKED" if failures else "SOURCE_IDENTITY_ONLY",
        "blockers": sorted(set(failures)),
        "verified_blob_count": len(files) if not failures else None,
        "runtime_atomicity_verified": False,
        "stateful_evm_verified": False,
        "onchain_svm_verified": False,
        "independent_audits_verified": False,
        "live_validator_fleet_verified": False,
        "external_bridges_authorized": False,
        "release_authorized": False,
        "note": "Source identity, ancestry and feature profile only; separate exact-head tests and human security review required.",
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        policy = json.loads(args.manifest.read_text(encoding="utf-8"))
        report = evaluate(policy, args.root, args.expected_head)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0 if report["decision"] == "SOURCE_IDENTITY_ONLY" else 1
    except (OSError, ValueError, TimeoutError, KeyError, TypeError) as exc:
        print(f"X3 candidate provenance cannot be verified ({type(exc).__name__}): {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
