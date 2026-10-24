#!/usr/bin/env python3
"""
Mainnet Release Gate — Launch-critical validation.

Replaces the prior documentation-only check with:
  1. Build validation (x3-chain-node, x3-chain-runtime WASM)
  2. Chain-spec / genesis artifact verification
  3. Critical runtime and pallet test suites
  4. Reproducible-build prerequisite check (srtool)
  5. Required documentation check (preserved from original)
  6. Northern Swarm + Reactor on-chain compute hard gate
  7. Forbidden-secret scanning (preserved from original)

Exit 0 → gate PASSES.
Exit 1 → gate FAILS — do NOT cut a release.
"""

import json
import hashlib
import os
import pathlib
import re
import shutil
import subprocess
import sys
import fcntl
import datetime as dt

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import feature_matrix  # noqa: E402

FAILURES: list[str] = []


# ── helpers ──────────────────────────────────────────────────────────────────

def run(
    cmd: list[str],
    cwd: pathlib.Path | None = None,
    env: dict | None = None,
) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd or ROOT, env=env)


def node_binary_env() -> dict:
    """The environment that pins a child gate to the binary stage 2 built.

    The chain-booting gates look for a node binary in `CARGO_TARGET_DIR` (or
    `target/`), which is the same directory stage 2 builds into — but only when
    the ambient environment happens to agree. Passing the path explicitly makes
    these stages test the artifact the release gate just built and verified,
    whichever target directory this run uses.
    """
    return {**os.environ, "X3_NODE_BIN": str(TARGET_DIR / "release" / "x3-chain-node")}


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


# ── 0. P0 release scope and candidate identity ───────────────────────────────

P0_RELEASE_FLOOR = 95
P0_BASELINE_COUNT = 82
P0_CAPABILITY_COUNT = 79
P0_CLAIM_HYGIENE_COUNT = 3
CLAIM_HYGIENE_SUBSYSTEM = "claims_hygiene"
LAUNCH_RECORD_PATH = ROOT / "reports" / "launch" / "promotion.json"
LAUNCH_RECORD_SCHEMA = 2
AUDIT_SCOPES_A = {"runtime", "consensus", "atomic"}
AUDIT_SCOPES_B = {"bridges", "contracts", "operations"}
APPROVAL_ROLES = {"release", "security", "operations"}
BRIDGE_ROUTES = {
    "ethereum-mainnet",
    "solana-mainnet-beta",
    "bitcoin-mainnet",
}
BRIDGE_LIMITS = {
    "per_transfer_limit_usd": 1_000,
    "daily_per_route_limit_usd": 10_000,
    "total_exposure_limit_usd": 50_000,
}
BOUNTY_MIN_POOL_USD = 100_000
BOUNTY_MIN_AGE = dt.timedelta(days=28)
PUBLIC_TESTNET_MIN_DURATION = dt.timedelta(weeks=8)
PUBLIC_TESTNET_MAX_AGE = dt.timedelta(days=90)
PUBLIC_TESTNET_MIN_VALIDATORS = 50
PUBLIC_TESTNET_MIN_CONTINENTS = 5
NON_REPOSITORY_EVIDENCE_PREFIXES = (
    "claim:",
    "commit:",
    "note:",
    "pr #",
    "pr:",
    "registry:",
    "workflow:",
)


def load_matrix(path: pathlib.Path | str) -> dict:
    return feature_matrix.load_matrix(pathlib.Path(path))


def _local_evidence_path(value: object) -> str | None:
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.lower().startswith(("http://", "https://")):
        return None
    if text.lower().startswith(NON_REPOSITORY_EVIDENCE_PREFIXES):
        return None
    return text.split("#", 1)[0].strip()


def _current_evidence_errors(
    feature: dict, root: pathlib.Path
) -> list[str]:
    label = str(feature.get("id") or feature.get("name") or "<unknown P0 feature>")
    evidence = feature.get("evidence", [])
    test_evidence = feature.get("test_evidence", [])
    if not isinstance(evidence, list) or not isinstance(test_evidence, list):
        return [f"{label}: evidence and test_evidence must be arrays"]

    local_evidence: list[str] = []
    errors: list[str] = []
    for item in evidence + test_evidence:
        local = _local_evidence_path(item)
        if local is None:
            continue
        path = root / local
        if not path.is_file():
            errors.append(f"{label}: evidence artifact is missing: {local}")
            continue
        try:
            if path.stat().st_size == 0:
                errors.append(f"{label}: evidence artifact is empty: {local}")
                continue
        except OSError as exc:
            errors.append(f"{label}: evidence artifact cannot be read: {local}: {exc}")
            continue
        local_evidence.append(local)

    if not local_evidence:
        errors.append(
            f"{label}: no current repository evidence artifact is recorded; "
            "URLs, PR references, commits and notes are claims, not launch evidence"
        )
    return errors


def evaluate_p0_scope(matrix: dict, repo_root: pathlib.Path | str) -> list[str]:
    """Return every reason the matrix fails the public-launch contract."""
    root = pathlib.Path(repo_root)
    features = matrix.get("feature", [])
    p0 = [
        feature
        for feature in features
        if isinstance(feature, dict) and feature.get("priority") == "P0"
    ]
    if not p0:
        return ["P0 release scope is empty"]

    errors: list[str] = []
    if len(p0) != P0_BASELINE_COUNT:
        errors.append(
            f"P0 release scope has {len(p0)} rows; the approved baseline is "
            f"{P0_BASELINE_COUNT}"
        )

    capability_rows = [
        feature
        for feature in p0
        if feature.get("subsystem") != CLAIM_HYGIENE_SUBSYSTEM
    ]
    hygiene_rows = [
        feature
        for feature in p0
        if feature.get("subsystem") == CLAIM_HYGIENE_SUBSYSTEM
    ]
    if len(capability_rows) != P0_CAPABILITY_COUNT:
        errors.append(
            f"P0 release scope has {len(capability_rows)} capability rows; the "
            f"approved baseline is {P0_CAPABILITY_COUNT}"
        )
    if len(hygiene_rows) != P0_CLAIM_HYGIENE_COUNT:
        errors.append(
            f"P0 release scope has {len(hygiene_rows)} claim-hygiene rows; the "
            f"approved baseline is {P0_CLAIM_HYGIENE_COUNT}"
        )

    for feature in hygiene_rows:
        label = str(feature.get("id") or feature.get("name") or "<unknown claim row>")
        if feature.get("claim_risk") is not True:
            errors.append(f"{label}: claim-hygiene row must set claim_risk = true")
        if feature.get("launch_scope") != "research":
            errors.append(f"{label}: claim-hygiene row must remain research scope")

    for feature in capability_rows:
        label = str(feature.get("id") or feature.get("name") or "<unknown P0 feature>")
        for field in ("implemented", "tested", "mainnet_ready"):
            score = feature.get(field)
            if not isinstance(score, int) or isinstance(score, bool):
                errors.append(f"{label}: {field} is not an integer")
            elif score < P0_RELEASE_FLOOR:
                errors.append(
                    f"{label}: {field} is {score}; P0 release requires at least "
                    f"{P0_RELEASE_FLOOR}"
                )

        blockers = feature.get("blockers", [])
        if not isinstance(blockers, list):
            errors.append(f"{label}: blockers is not an array")
        elif blockers:
            errors.append(
                f"{label}: P0 release has {len(blockers)} open blocker"
                f"{'s' if len(blockers) != 1 else ''}: {'; '.join(map(str, blockers))}"
            )

        required_tests = feature.get("required_tests", [])
        if not isinstance(required_tests, list):
            errors.append(f"{label}: required_tests is not an array")
        else:
            if not required_tests and not feature.get("test_evidence"):
                errors.append(f"{label}: no required test evidence is recorded")
            for test_name in required_tests:
                if not isinstance(test_name, str) or not feature_matrix._test_exists(
                    root,
                    feature.get("paths", []),
                    feature.get("test_paths", []),
                    test_name,
                ):
                    errors.append(f"{label}: required test does not resolve: {test_name}")
        errors.extend(_current_evidence_errors(feature, root))
    return errors


def claims_hygiene_errors(repo_root: pathlib.Path | str) -> list[str]:
    """Claim-risk rows are scanner gates, never launch capabilities."""
    root = pathlib.Path(repo_root)
    script = root / "scripts" / "ci" / "check-claims-hygiene.py"
    if not script.is_file():
        return ["claims hygiene scanner is missing: scripts/ci/check-claims-hygiene.py"]
    result = run([sys.executable, str(script)], cwd=root)
    if result.returncode == 0:
        return []
    output = (result.stderr or result.stdout).strip()
    detail = output.splitlines()[-1] if output else f"exit {result.returncode}"
    return [f"claims hygiene scanner failed: {detail}"]


def audit_artifact_errors(repo_root: pathlib.Path | str) -> list[str]:
    """Reject generated matrix artifacts that no longer describe their sources."""
    root = pathlib.Path(repo_root)
    script = root / "scripts" / "x3_audit_matrix.py"
    if not script.is_file():
        return [
            "generated matrix artifacts cannot be verified: "
            "scripts/x3_audit_matrix.py is missing"
        ]
    result = run([sys.executable, str(script), "--check"], cwd=root)
    if result.returncode == 0:
        return []
    stale = [
        line.split("missing/stale:", 1)[1].strip()
        for line in result.stderr.splitlines()
        if "missing/stale:" in line
    ]
    if stale:
        return [
            "generated matrix artifacts are missing or stale: "
            + ", ".join(sorted(stale))
        ]
    detail = next(
        (line.strip() for line in result.stderr.splitlines() if line.strip()),
        f"x3_audit_matrix.py --check exited {result.returncode}",
    )
    return [f"generated matrix artifacts cannot be verified: {detail}"]


def check_p0_scope(
    matrix_path: pathlib.Path | str | None = None,
    repo_root: pathlib.Path | str | None = None,
) -> list[str]:
    """Load and validate the complete P0 release scope."""
    root = pathlib.Path(repo_root) if repo_root is not None else ROOT
    matrix_file = (
        pathlib.Path(matrix_path)
        if matrix_path is not None
        else root / "FEATURE_MATRIX.toml"
    )
    try:
        matrix = load_matrix(matrix_file)
    except (OSError, ValueError) as exc:
        return [f"P0 release scope cannot be loaded from {matrix_file}: {exc}"]
    errors = evaluate_p0_scope(matrix, root)
    p0_hygiene = [
        feature
        for feature in matrix.get("feature", [])
        if isinstance(feature, dict)
        and feature.get("priority") == "P0"
        and feature.get("subsystem") == CLAIM_HYGIENE_SUBSYSTEM
    ]
    if p0_hygiene:
        errors += claims_hygiene_errors(root)
    return errors + audit_artifact_errors(root)


def working_tree_changes(root: pathlib.Path | str = ROOT) -> list[str]:
    """Return porcelain status lines; a candidate release must name committed bytes."""
    result = run(
        ["git", "status", "--porcelain", "--untracked-files=normal"],
        cwd=pathlib.Path(root),
    )
    if result.returncode != 0:
        return ["cannot determine working-tree state for a release candidate"]
    return [line for line in result.stdout.splitlines() if line.strip()]


def check_candidate_identity() -> None:
    print("\n── 0b. Candidate identity ──")
    changes = working_tree_changes()
    if changes:
        fail(
            "release candidate working tree is dirty; commit or remove every staged, "
            f"modified and untracked change first ({len(changes)} path(s))"
        )
        for line in changes[:20]:
            print(f"    {line}")
        if len(changes) > 20:
            print(f"    … {len(changes) - 20} more")
    else:
        ok("release candidate working tree is clean")


def _sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def _record_digest(record: dict) -> str:
    body = {key: value for key, value in record.items() if key != "record_sha256"}
    return hashlib.sha256(_canonical_json(body)).hexdigest()


def _parse_timestamp(value: object, label: str) -> dt.datetime | None:
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.strip().replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(dt.timezone.utc)


def _record_artifact_error(
    artifact: object, root: pathlib.Path, label: str
) -> str | None:
    if not isinstance(artifact, dict):
        return f"{label}: artifact record is missing"
    raw_path = (
        artifact.get("path")
        or artifact.get("report")
        or artifact.get("evidence")
    )
    expected = artifact.get("sha256")
    if not isinstance(raw_path, str) or not raw_path.strip():
        return f"{label}: artifact path is missing"
    path = root / raw_path
    if not path.is_file():
        return f"{label}: artifact is missing: {raw_path}"
    actual = _sha256(path)
    if not isinstance(expected, str) or actual != expected:
        return f"{label}: artifact digest does not match: {raw_path}"
    return None


def _record_matrix_errors(
    record: dict, root: pathlib.Path, matrix: dict
) -> list[str]:
    feature_record = record.get("feature_matrix")
    if not isinstance(feature_record, dict):
        return ["launch record: feature_matrix evidence is missing"]
    sources = feature_record.get("sources")
    if not isinstance(sources, dict) or not sources:
        return ["launch record: feature_matrix sources are missing"]

    errors: list[str] = []
    for raw_path, expected in sorted(sources.items()):
        if not isinstance(raw_path, str) or not isinstance(expected, str):
            errors.append("launch record: malformed feature_matrix source entry")
            continue
        path = root / raw_path
        if not path.is_file() or _sha256(path) != expected:
            errors.append(
                f"launch record: feature_matrix source does not match: {raw_path}"
            )

    p0_rows = [
        feature
        for feature in matrix.get("feature", [])
        if isinstance(feature, dict) and feature.get("priority") == "P0"
    ]
    capability_rows = [
        feature
        for feature in p0_rows
        if feature.get("subsystem") != CLAIM_HYGIENE_SUBSYSTEM
    ]
    hygiene_rows = [
        feature
        for feature in p0_rows
        if feature.get("subsystem") == CLAIM_HYGIENE_SUBSYSTEM
    ]
    if len(p0_rows) != P0_BASELINE_COUNT:
        errors.append(
            f"launch record: matrix has {len(p0_rows)} P0 rows; expected "
            f"{P0_BASELINE_COUNT}"
        )
    if len(capability_rows) != P0_CAPABILITY_COUNT:
        errors.append(
            f"launch record: matrix has {len(capability_rows)} capability P0 rows; "
            f"expected {P0_CAPABILITY_COUNT}"
        )
    if len(hygiene_rows) != P0_CLAIM_HYGIENE_COUNT:
        errors.append(
            f"launch record: matrix has {len(hygiene_rows)} claim-hygiene P0 rows; "
            f"expected {P0_CLAIM_HYGIENE_COUNT}"
        )
    for feature in hygiene_rows:
        label = str(feature.get("id") or feature.get("name") or "<unknown claim row>")
        if feature.get("claim_risk") is not True:
            errors.append(
                f"launch record: {label} claim-hygiene row must set claim_risk = true"
            )
        if feature.get("launch_scope") != "research":
            errors.append(
                f"launch record: {label} claim-hygiene row must remain research scope"
            )

    current_rows = sorted(
        (
            {
                "id": str(feature.get("id") or feature.get("name")),
                "implemented": feature.get("implemented"),
                "tested": feature.get("tested"),
                "mainnet_ready": feature.get("mainnet_ready"),
            }
            for feature in p0_rows
        ),
        key=lambda row: row["id"],
    )
    snapshot = record.get("p0_snapshot")
    expected_rows = snapshot.get("rows") if isinstance(snapshot, dict) else None
    if expected_rows != current_rows:
        errors.append("launch record: P0 snapshot does not match FEATURE_MATRIX.toml")
    return errors


def _record_test_gate_errors(record: dict, root: pathlib.Path) -> list[str]:
    gates = record.get("test_gates")
    if not isinstance(gates, list) or not gates:
        return ["launch record: at least one passing test gate is required"]
    errors: list[str] = []
    names: set[str] = set()
    for index, test_gate in enumerate(gates):
        if not isinstance(test_gate, dict):
            errors.append(f"launch record: test_gates[{index}] is malformed")
            continue
        name = test_gate.get("name")
        if not isinstance(name, str) or not name.strip():
            errors.append(f"launch record: test_gates[{index}] has no name")
            continue
        name = name.strip()
        if name in names:
            errors.append(f"launch record: duplicate test gate {name}")
        names.add(name)
        if (
            test_gate.get("status") != "PASS"
            or test_gate.get("exit_code") != 0
            or isinstance(test_gate.get("exit_code"), bool)
        ):
            errors.append(f"launch record: test gate {name} did not pass")
        if not isinstance(test_gate.get("command"), str) or not test_gate[
            "command"
        ].strip():
            errors.append(f"launch record: test gate {name} has no command")
        log = test_gate.get("log")
        problem = _record_artifact_error(
            {"path": log, "sha256": test_gate.get("sha256")},
            root,
            f"launch record: test gate {name}",
        )
        if problem:
            errors.append(problem)
            continue
        text = (root / log).read_text(encoding="utf-8", errors="replace")
        if "PASS" not in text:
            errors.append(f"launch record: test gate {name} log has no PASS marker")
    return errors


def evaluate_launch_record(
    record: object,
    repo_root: pathlib.Path | str,
    head_commit: str,
    matrix: dict,
    now: dt.datetime | None = None,
) -> list[str]:
    """Re-read every launch claim that is not a code or build result."""
    if not isinstance(record, dict):
        return ["launch record: root must be a JSON object"]
    root = pathlib.Path(repo_root)
    errors: list[str] = []
    if record.get("schema") != LAUNCH_RECORD_SCHEMA:
        errors.append(
            f"launch record: schema {record.get('schema')!r}; expected "
            f"{LAUNCH_RECORD_SCHEMA}"
        )
    if not isinstance(record.get("approved_by"), str) or not record["approved_by"].strip():
        errors.append("launch record: approved_by is missing")
    if record.get("commit") != head_commit:
        errors.append(
            f"launch record: commit {record.get('commit')!r} does not match "
            f"candidate {head_commit}"
        )
    if record.get("record_sha256") != _record_digest(record):
        errors.append("launch record: record_sha256 does not match its contents")

    recorded_at = _parse_timestamp(record.get("recorded_at"), "recorded_at")
    if recorded_at is None:
        errors.append("launch record: recorded_at is not a timezone-aware timestamp")
        recorded_at = now or dt.datetime.now(dt.timezone.utc)

    artifacts = record.get("artifacts")
    if not isinstance(artifacts, list):
        errors.append("launch record: genesis artifacts are missing")
    else:
        roles = {
            artifact.get("role")
            for artifact in artifacts
            if isinstance(artifact, dict)
        }
        for role in ("mainnet-plain", "mainnet-raw"):
            matching = [
                artifact
                for artifact in artifacts
                if isinstance(artifact, dict) and artifact.get("role") == role
            ]
            if len(matching) != 1:
                errors.append(f"launch record: exactly one {role} genesis is required")
                continue
            artifact = matching[0]
            problem = _record_artifact_error(artifact, root, f"launch record: {role}")
            if problem:
                errors.append(problem)
            if artifact.get("aura_authorities") != 7:
                errors.append(f"launch record: {role} does not have 7 Aura authorities")
            if artifact.get("grandpa_authorities") != 7:
                errors.append(
                    f"launch record: {role} does not have 7 GRANDPA authorities"
                )
        if roles != {"mainnet-plain", "mainnet-raw"}:
            errors.append("launch record: unexpected genesis artifact roles")

    errors.extend(_record_matrix_errors(record, root, matrix))
    evidence_pack_error = _record_artifact_error(
        record.get("evidence_pack"), root, "launch record: evidence pack"
    )
    if evidence_pack_error:
        errors.append(evidence_pack_error)
    errors.extend(_record_test_gate_errors(record, root))

    audits = record.get("audits")
    if not isinstance(audits, list) or len(audits) < 2:
        errors.append("launch record: two independent audit reports are required")
    else:
        providers: set[str] = set()
        scopes: list[set[str]] = []
        for index, audit in enumerate(audits):
            if not isinstance(audit, dict):
                errors.append(f"launch record: audits[{index}] is malformed")
                continue
            provider = audit.get("provider")
            if not isinstance(provider, str) or not provider.strip():
                errors.append(f"launch record: audits[{index}] has no provider")
            elif provider in providers:
                errors.append(f"launch record: duplicate audit provider {provider}")
            else:
                providers.add(provider)
            scope = audit.get("scope")
            if isinstance(scope, str):
                scope = [scope]
            normalized = (
                {item.strip().lower() for item in scope if isinstance(item, str)}
                if isinstance(scope, list)
                else set()
            )
            scopes.append(normalized)
            if not normalized:
                errors.append(
                    f"launch record: audit {provider or index} has no scope"
                )
            elif not normalized.issubset(AUDIT_SCOPES_A | AUDIT_SCOPES_B):
                errors.append(
                    f"launch record: audit {provider or index} has an unknown scope"
                )
            if audit.get("critical_open") != 0 or audit.get("high_open") != 0:
                errors.append(
                    f"launch record: audit {provider or index} has open critical/high findings"
                )
            problem = _record_artifact_error(
                audit, root, f"launch record: audit {provider or index}"
            )
            if problem:
                errors.append(problem)
        if len(providers) < 2:
            errors.append("launch record: audit providers are not independent")
        if not any(AUDIT_SCOPES_A <= scope for scope in scopes):
            errors.append("launch record: runtime/consensus audit scope is missing")
        if not any(AUDIT_SCOPES_B <= scope for scope in scopes):
            errors.append("launch record: bridges/contracts/operations audit scope is missing")
        if any(
            AUDIT_SCOPES_A <= scope and AUDIT_SCOPES_B <= scope
            for scope in scopes
        ):
            errors.append(
                "launch record: independent audit scope groups cannot be pooled "
                "into one report"
            )

    bounty = record.get("bounty_window")
    if not isinstance(bounty, dict):
        errors.append("launch record: bounty window is missing")
    else:
        pool = bounty.get("pool_usd")
        if (
            not isinstance(pool, int)
            or isinstance(pool, bool)
            or pool < BOUNTY_MIN_POOL_USD
        ):
            errors.append(
                f"launch record: bounty pool is below ${BOUNTY_MIN_POOL_USD:,}"
            )
        starts = _parse_timestamp(bounty.get("starts_at"), "bounty starts_at")
        ends = _parse_timestamp(bounty.get("ends_at"), "bounty ends_at")
        if starts is None or ends is None or ends - starts < BOUNTY_MIN_AGE:
            errors.append("launch record: bounty window is shorter than 28 days")
        elif not (starts <= recorded_at <= ends):
            errors.append("launch record: bounty was not active when launch was recorded")
        problem = _record_artifact_error(bounty, root, "launch record: bounty")
        if problem:
            errors.append(problem)

    testnet = record.get("public_testnet_window")
    if not isinstance(testnet, dict):
        errors.append("launch record: public testnet window is missing")
    else:
        starts = _parse_timestamp(testnet.get("started_at"), "testnet started_at")
        ends = _parse_timestamp(testnet.get("ended_at"), "testnet ended_at")
        if starts is None or ends is None:
            errors.append("launch record: public testnet timestamps are invalid")
        else:
            duration = ends - starts
            if duration < PUBLIC_TESTNET_MIN_DURATION:
                errors.append("launch record: public testnet is shorter than 8 weeks")
            if ends > recorded_at:
                errors.append("launch record: public testnet had not completed at approval")
            if recorded_at - ends > PUBLIC_TESTNET_MAX_AGE:
                errors.append("launch record: public testnet evidence has expired")
        if testnet.get("validator_count", 0) < PUBLIC_TESTNET_MIN_VALIDATORS:
            errors.append("launch record: public testnet has fewer than 50 validators")
        if testnet.get("continents", 0) < PUBLIC_TESTNET_MIN_CONTINENTS:
            errors.append("launch record: public testnet spans fewer than 5 continents")
        problem = _record_artifact_error(testnet, root, "launch record: public testnet")
        if problem:
            errors.append(problem)

    bridge = record.get("bridge_configuration")
    if not isinstance(bridge, dict):
        errors.append("launch record: bridge configuration is missing")
    else:
        raw_path = bridge.get("path")
        path = root / raw_path if isinstance(raw_path, str) else None
        if path is None or not path.is_file():
            errors.append("launch record: bridge configuration file is missing")
        else:
            try:
                config = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                errors.append("launch record: bridge configuration is not valid JSON")
            else:
                if set(config.get("active_routes", [])) != BRIDGE_ROUTES:
                    errors.append(
                        "launch record: external routes must be exactly Ethereum, "
                        "Solana and Bitcoin mainnet"
                    )
                for field, cap in BRIDGE_LIMITS.items():
                    value = config.get(field)
                    if not isinstance(value, int) or isinstance(value, bool) or value > cap:
                        errors.append(
                            f"launch record: bridge {field} exceeds ${cap:,}"
                        )
                if config.get("kill_switch") is not True:
                    errors.append("launch record: bridge kill switch is not enabled")
        problem = _record_artifact_error(
            bridge, root, "launch record: bridge configuration"
        )
        if problem:
            errors.append(problem)

    validators = record.get("validators")
    entries = validators.get("entries") if isinstance(validators, dict) else None
    if not isinstance(entries, list) or len(entries) != 7:
        errors.append("launch record: exactly seven genesis validators are required")
    else:
        for field, minimum in (("host", 7), ("operator", 7), ("identity", 7), ("provider", 5), ("jurisdiction", 3)):
            values = [entry.get(field) for entry in entries if isinstance(entry, dict)]
            if (
                len(values) != 7
                or any(not isinstance(value, str) or not value.strip() for value in values)
                or len(set(values)) < minimum
            ):
                errors.append(
                    f"launch record: fewer than {minimum} distinct validator {field} values"
                )
        if any(entry.get("loopback") is not False for entry in entries):
            errors.append("launch record: validator hosts must be non-loopback")
    if isinstance(validators, dict):
        problem = _record_artifact_error(validators, root, "launch record: validators")
        if problem:
            errors.append(problem)

    approvals = record.get("approvals")
    roles: dict[str, list[dict]] = {}
    if not isinstance(approvals, list):
        errors.append("launch record: release approvals are missing")
    else:
        for approval in approvals:
            if not isinstance(approval, dict):
                continue
            role = approval.get("role")
            if role in APPROVAL_ROLES:
                roles.setdefault(role, []).append(approval)
            if approval.get("approved_commit") != head_commit:
                errors.append(
                    f"launch record: approval {approval.get('approver', role)!r} "
                    "is not bound to the candidate commit"
                )
            evidence = approval.get("evidence")
            problem = _record_artifact_error(
                {**approval, "path": evidence}
                if isinstance(evidence, str)
                else approval,
                root,
                f"launch record: approval {role!r}",
            )
            if problem:
                errors.append(problem)
        if set(roles) != APPROVAL_ROLES or any(len(rows) != 1 for rows in roles.values()):
            errors.append(
                "launch record: one release, security and operations approval each is required"
            )
    return errors


def check_launch_record(
    record_path: pathlib.Path | str | None = None,
    repo_root: pathlib.Path | str | None = None,
) -> list[str]:
    root = pathlib.Path(repo_root) if repo_root is not None else ROOT
    path = (
        pathlib.Path(record_path)
        if record_path is not None
        else root / LAUNCH_RECORD_PATH.relative_to(ROOT)
    )
    if not path.is_file():
        return [f"launch record is missing: {path}"]
    try:
        record = json.loads(path.read_text(encoding="utf-8"))
        matrix = load_matrix(root / "FEATURE_MATRIX.toml")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return [f"launch record cannot be loaded: {exc}"]
    commit = run(["git", "rev-parse", "HEAD"], cwd=root)
    if commit.returncode != 0:
        return ["launch record: cannot resolve candidate commit"]
    return evaluate_launch_record(record, root, commit.stdout.strip(), matrix)


# ── 1. Required documentation ────────────────────────────────────────────────

REQUIRED_DOCS = [
    "MAINNET_READINESS.md",
    "INVARIANTS.md",
    "RELEASE_GATES.md",
    "SECURITY.md",
    "TESTING.md",
    "AUDIT_SPEC.md",
]


BUILD_LOCK = ROOT / ".x3-release-build.lock"


def acquire_build_lock():
    """Take the exclusive lock every runtime-build-bearing run shares.

    Returns the open handle (kept alive for the process lifetime) or `None` when another run holds
    it, in which case the caller must not proceed: the two would write the same
    `runtime/target/srtool` and the hashes one of them reports would describe the other's build.
    The lock is advisory and process-scoped, so it disappears with the holder.
    """
    # `a+`, not `w`: opening for write truncates the file, which would erase the
    # holding run's pid before we can name it in the refusal.
    handle = open(BUILD_LOCK, "a+")
    try:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        holder = ""
        try:
            holder = BUILD_LOCK.read_text().strip()
        except OSError:
            holder = ""
        print("::error:: another release-gate or runtime-attestation run holds "
              f"{BUILD_LOCK.name}" + (f" (pid {holder})" if holder else ""))
        print("  Both build runtime/target/srtool and boot nodes on fixed ports, so running two at")
        print("  once produces evidence that describes neither. Wait for it to finish, or run")
        print("  `scripts/local-ci.sh --only <gate>` for a gate that does not rebuild the runtime.")
        handle.close()
        return None
    handle.seek(0)
    handle.truncate()
    handle.write(f"{os.getpid()}\n")
    handle.flush()
    return handle


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
        artifact = TARGET_DIR / artifact_rel
        # Always build. This used to `continue` as soon as the artifact existed,
        # so every stage after it — the dev chain boot, the validator set, the
        # chain-spec checks and the production-genesis gate — ran against
        # whatever binary happened to be sitting in the target directory from an
        # earlier run, however old. Cargo makes this a no-op when the tree has
        # not changed, and the printed digest makes the artifact auditable.
        print(f"  building {pkg}...")
        result = run(["cargo", "build", "--release", "-p", pkg])
        if result.returncode != 0:
            fail(f"{pkg} build failed:\n{result.stderr}")
            continue
        if not artifact.exists():
            fail(f"{pkg} artifact not found after build at {artifact}")
            continue
        digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
        ok(
            f"{pkg} built: {artifact.stat().st_size} bytes, sha256:{digest[:16]}… "
            f"({artifact})"
        )


# ── 3. Chain-spec / genesis artifact verification ────────────────────────────

def check_chain_runs() -> None:
    """Boot the node on a dev chain and prove it authors and finalizes blocks.

    Every other stage checks code — that it builds, that its tests pass, that
    the runtime upgrades, that the WASM is reproducible. None of them starts a
    chain. This does, and asks the operator CLI for the authority set and an
    asset through the node's own RPC on the way.
    """
    print("\n── 2b. The chain runs ──")
    script = ROOT / "scripts" / "local-node-smoke.sh"
    if not script.exists():
        fail("scripts/local-node-smoke.sh is missing — nothing boots a chain")
        return

    result = run(["bash", str(script)], env=node_binary_env())
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("the chain did not run (scripts/local-node-smoke.sh failed)")
        for line in output.splitlines()[-15:]:
            print(f"    {line}")
        return

    summary = next(
        (line.split("PASS — ", 1)[1] for line in output.splitlines() if "PASS — " in line),
        "the node booted, authored, finalized, and answered its CLI",
    )
    ok(summary)


def check_validators_agree() -> None:
    """Three validators on the built-in `local3` chain have to agree.

    One node authoring blocks proves the node runs. This proves the part a
    blockchain actually needs: independent validators find each other, finalize,
    and return the same canonical hash for the same finalized height.
    """
    print("\n── 2c. Validators agree ──")
    script = ROOT / "scripts" / "local-network-smoke.sh"
    if not script.exists():
        fail("scripts/local-network-smoke.sh is missing — nothing runs a validator set")
        return

    result = run(["bash", str(script)])
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("the three-validator local chain did not converge")
        for line in output.splitlines()[-15:]:
            print(f"    {line}")
        return

    summary = next(
        (line.split("PASS — ", 1)[1] for line in output.splitlines() if "PASS — " in line),
        "three validators finalized and agreed on the canonical hash",
    )
    ok(summary)


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
                    # Parsing is not loading. `genesis.runtime` is the pre-`runtimeGenesis`
                    # format and the node rejects it outright:
                    #   unknown variant `runtime`, expected one of `raw`, `stateRootHash`, `runtimeGenesis`
                    # That is how a shipped plain spec sat in this repository unable
                    # to start a chain.
                    genesis = data["genesis"]
                    expected = "raw" if rel.endswith("-raw.json") else "runtimeGenesis"
                    if expected not in genesis:
                        fail(
                            f"{rel} is in a format this node does not load: genesis has "
                            f"{sorted(genesis)} but the node expects '{expected}'. "
                            "Regenerate with `x3-chain-node build-spec --chain local3`."
                        )
                    else:
                        ok(f"{rel} valid genesis spec (genesis.{expected})")
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


def check_shipped_genesis_boots() -> None:
    """The genesis this repository ships has to start a network.

    `check_chain_spec_artifacts` parses the files; a spec can parse and still be
    rejected by the node (that is exactly what the old `genesis.runtime` format
    did). And one node cannot decide it either: with three GRANDPA authorities a
    solo node holds 1/3 of the set, below the 2/3 needed to finalize, so it looks
    like a broken spec when it is not. This stage boots **three** validators on
    the shipped file and requires a finalized, agreed height.
    """
    print("\n── 3c. The shipped genesis boots a network ──")
    script = ROOT / "scripts" / "local-network-smoke.sh"
    spec = ROOT / "chain-specs" / "x3-local3-current-plain.json"
    if not script.exists():
        fail("scripts/local-network-smoke.sh is missing — nothing boots the shipped genesis")
        return
    if not spec.exists():
        fail(f"{spec.relative_to(ROOT)} is missing — nothing to boot")
        return

    result = run(
        ["bash", str(script)],
        env={**node_binary_env(), "X3_NETWORK_SMOKE_CHAIN_SPEC": str(spec)},
    )
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("the genesis this repository ships did not start a three-validator network")
        for line in output.splitlines()[-15:]:
            print(f"    {line}")
        return
    summary = next(
        (line.split("PASS — ", 1)[1] for line in output.splitlines() if "PASS — " in line),
        "three validators finalized and agreed on the shipped genesis",
    )
    ok(f"shipped genesis: {summary}")


def check_release_artifacts() -> None:
    """The files a release is made of have to be installable.

    The release pipeline has never executed a step: `v0.4.0-rc.1` is 1,027 commits
    behind master and the workflow file *at that tag* asked for `ubuntu-latest`,
    which is billing-locked here, so both runs finished in ~4 seconds with zero
    steps and the draft release has no assets. Nothing checked that the artifact
    set is coherent — that the checksum file names the binary, that the tarball
    extracts to something runnable, or that the installer accepts what the release
    ships. This stage builds the bundle from the binary stage 2 produced and does
    exactly that, including refusing a tampered copy.
    """
    print("\n── 2e. Release artifacts are installable ──")
    script = ROOT / "scripts" / "mainnet" / "release-artifacts-gate.sh"
    if not script.exists():
        fail(
            "scripts/mainnet/release-artifacts-gate.sh is missing — nothing checks "
            "the release bundle"
        )
        return
    result = run(["bash", str(script)], env=node_binary_env())
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("the release artifact bundle is not installable")
        for line in output.splitlines()[-12:]:
            print(f"    {line}")
        return
    summary = next(
        (
            line.split("] ", 1)[1]
            for line in output.splitlines()
            if "[artifact-gate] " in line and "PASS" in line
        ),
        "release bundle verifies, extracts, runs and installs",
    )
    ok(summary)


def check_validator_install() -> None:
    """The operator's install path has to work, or nobody can run a validator.

    `scripts/install-validator.sh` downloaded `releases/download/latest/…` — a
    draft release with no assets — printed "WARNING: No checksum file found.
    Skipping verification." and installed a systemd unit pointing at a chain spec
    it had failed to fetch. Nothing in the repository ran the script, so none of
    that was visible. This stage runs it in `--check` mode (no root, no writes)
    against a real generated Live genesis and a real built binary, and against
    each way the inputs can be wrong.
    """
    print("\n── 2d. Validator install path ──")
    script = ROOT / "scripts" / "mainnet" / "validator_install_gate.sh"
    if not script.exists():
        fail(
            "scripts/mainnet/validator_install_gate.sh is missing — nothing "
            "exercises the validator install path"
        )
        return

    result = run(["bash", str(script)], env=node_binary_env())
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("the validator install path did not behave as required")
        for line in output.splitlines()[-15:]:
            print(f"    {line}")
        return

    accepted = sum("-> exit 0 (ok)" in line for line in output.splitlines())
    refused = sum("-> exit 1 (fail)" in line for line in output.splitlines())
    if accepted < 3 or refused < 5:
        fail(
            f"the install gate only exercised {accepted} accepted and {refused} "
            "refused cases; expected at least 3 and 5"
        )
        return
    ok(
        f"install path: {accepted} cases accepted, {refused} refused, "
        "nothing written in check mode"
    )


def run_genesis_gate(chain: str, label: str) -> None:
    """Build a built-in Live genesis from fixture keys and boot a network on it.

    `check_chain_spec_artifacts` parses the local3 specs and greps
    `chain_spec.rs` for the literal strings "production_config"/"testnet_config".
    It never calls either function, so a builder that cannot produce a genesis —
    or a spec that parses but cannot start a chain — passed the release gate.
    This runs the whole path with fixture keys: derive authorities, build the
    Live spec, assert the artifact, boot three validators on it, and require
    finalized agreement.
    """
    print(f"\n── {label} ──")
    script = ROOT / "scripts" / "mainnet" / "production_genesis_gate.sh"
    if not script.exists():
        fail(
            "scripts/mainnet/production_genesis_gate.sh is missing — nothing "
            f"builds or boots the {chain} genesis"
        )
        return

    result = run(
        ["bash", str(script)],
        env={**node_binary_env(), "X3_GENESIS_CHAIN": chain},
    )
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail(f"the {chain} genesis did not build, boot and finalize")
        for line in output.splitlines()[-15:]:
            print(f"    {line}")
        return

    summary = next(
        (line.split("PASS — ", 1)[1] for line in output.splitlines() if "PASS — " in line),
        f"{chain} genesis builds, boots and finalizes",
    )
    ok(summary)


def check_production_genesis() -> None:
    run_genesis_gate("production", "3b. Production (mainnet) genesis builds and boots")


def check_testnet_genesis() -> None:
    """Same path for the chain a public testnet runs: `testnet_config()`.

    It is a different builder with a different env prefix (X3_TESTNET_*) and a
    different gateway default, and it is what a testnet launch actually uses, so
    it needs its own proof rather than riding on production's.
    """
    run_genesis_gate("testnet", "3d. Testnet genesis builds and boots")


# ── 4. Critical runtime & pallet test suites ─────────────────────────────────


TEST_PACKAGES = [
    # Each entry: (cargo-flag, pkg-name, features-list)
    ("-p", "x3-chain-runtime", ["--features", "try-runtime"]),
    ("-p", "pallet-x3-supply-ledger", []),
    ("-p", "x3-packet-standard", []),
    ("-p", "x3-bridge", []),
    ("-p", "x3-fees", []),
    ("-p", "pallet-x3-slash", []),
    # The settlement engine was missing from this list while it gained the
    # cross-domain proof gate, and its test target stopped compiling: `cargo test`
    # for it is not part of any other stage here, so master went red without this
    # gate noticing (the workspace clippy gate caught it, which is not part of the
    # release bar). Every pallet that holds a terminal state belongs here.
    ("-p", "pallet-x3-settlement-engine", []),
    # Swarm/compute release surface: both halves must compile and test under
    # the same mainnet bar as settlement and runtime code.
    ("-p", "pallet-northern-swarm", []),
    ("-p", "northern-swarm", []),
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

def check_panic_ratchet() -> None:
    """No new panics in consensus-critical code.

    A panic in `on_initialize`/`on_finalize`/`offchain_worker` stops block
    production. The old audit classified by file path, so in-file `#[cfg(test)]`
    modules were counted as production (~5,170 findings, nearly all test
    assertions) and the script always exited 0 while its own report said
    "gate: FAIL" — the `|| exit 1` guard in `mainnet_rc_gate.sh` could never fire.
    The audit now classifies by enclosing function, excludes test items, and
    compares against a committed baseline.
    """
    print("\n── 4b. Panic ratchet (block hooks and production paths) ──")
    script = ROOT / "scripts" / "mainnet" / "panic_unwrap_audit.sh"
    if not script.exists():
        fail("scripts/mainnet/panic_unwrap_audit.sh is missing — nothing checks for new panics")
        return

    result = run(["bash", str(script)])
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("new panic/unwrap in consensus-critical code")
        for line in output.splitlines()[-12:]:
            print(f"    {line}")
        return

    counts_line = next(
        (line.strip() for line in output.splitlines() if "runtime-hook=" in line), ""
    )
    ok(counts_line or "panic ratchet holds")


def check_swarm_reactor_gate() -> None:
    """Northern Swarm/Reactor must be real on-chain compute before mainnet.

    This delegates detailed structural and executable checks to a focused gate
    so swarm-specific policy can evolve without bloating this release driver.
    The focused gate intentionally fails while the current single-executor,
    unsigned-submission, no-GPU path remains incomplete.
    """
    print("\n── 4c. Northern Swarm + Reactor on-chain compute ──")
    script = ROOT / "scripts" / "mainnet" / "swarm_reactor_gate.py"
    if not script.exists():
        fail("scripts/mainnet/swarm_reactor_gate.py is missing")
        return

    result = run([sys.executable, str(script)])
    output = result.stdout + result.stderr
    if result.returncode != 0:
        fail("Northern Swarm + Reactor on-chain compute gate failed")
        for line in output.splitlines()[-30:]:
            print(f"    {line}")
        return

    summary = next(
        (line.strip() for line in output.splitlines() if "swarm_reactor_gate: PASS" in line),
        "swarm/reactor compute gate passed",
    )
    ok(summary)


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
        # The runtime's own version and metadata version are in the same block the
        # hashes come from. The record's top-level `runtime_version` used to be
        # carried over untouched by every re-attestation, so after a `spec_version`
        # bump the record described the previous artifact (recorded "x3-chain-11"
        # for a wasm that reports "x3-chain-12"). Parsing them here lets stage 6b
        # compare them and `update-runtime-hashes.sh` write them.
        "version": "Version",
        "metadata": "Metadata",
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

    check_candidate_identity()
    if FAILURES:
        print(f"\n  ❌ GATE FAILED — {len(FAILURES)} candidate-identity failure(s)")
        print(f"{'═' * 60}")
        return 1

    print("\n── 0. P0 release scope ──")
    p0_errors = check_p0_scope()
    if p0_errors:
        for message in p0_errors:
            fail(message)
        print(f"\n  ❌ GATE FAILED — {len(p0_errors)} P0 release-contract failure(s)")
        print(f"{'═' * 60}")
        return 1
    ok("all P0 rows meet the 95-point launch contract with current evidence")

    print("\n── 0c. Launch evidence ──")
    launch_errors = check_launch_record()
    if launch_errors:
        for message in launch_errors:
            fail(message)
        print(
            f"\n  ❌ GATE FAILED — {len(launch_errors)} launch-evidence failure(s)"
        )
        print(f"{'═' * 60}")
        return 1
    ok("launch record re-reads current evidence and matches the candidate")

    # This gate builds the runtime in `runtime/target/srtool` and boots nodes on fixed ports. Two
    # concurrent runs therefore share both, and the second one corrupts the first one's evidence
    # rather than failing loudly: measured 2026-09-27, four `make mainnet-check` runs overlapped and
    # the srtool stage spent ~50 minutes in a three-way build race. `scripts/update-runtime-hashes.sh`
    # takes the same lock, because it builds the same target directory.
    lock_handle = acquire_build_lock()
    if lock_handle is None:
        return 2

    check_required_docs()
    check_build()
    check_chain_runs()
    check_validators_agree()
    check_validator_install()
    check_release_artifacts()
    check_chain_spec_artifacts()
    check_shipped_genesis_boots()
    check_production_genesis()
    check_testnet_genesis()
    check_test_suites()
    check_panic_ratchet()
    check_swarm_reactor_gate()
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
