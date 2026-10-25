#!/usr/bin/env python3
"""Bind a launch approval to current, re-readable evidence.

Schema 1 tied an RC log to the genesis bytes it promoted. Schema 2 extends that
identity to every claim a public mainnet launch makes: the exact source commit,
generated artifacts, feature-matrix digest, P0 snapshot, test-gate logs, two
independent audit scopes, bounty and public-testnet windows, bridge exposure,
the seven-validator inventory, and commit-bound approvals.

    python3 scripts/mainnet/launch_record.py build \
        --spec chain-specs/x3-mainnet-plain.json \
        --raw chain-specs/x3-mainnet-raw.json \
        --rc-log reports/rc/mainnet_rc_gate.log \
        --evidence-pack reports/launch/evidence-pack.json \
        --approved-by "<release approver>" \
        --out reports/launch/promotion.json

`build` writes deterministic JSON with a `record_sha256` over every field other
than itself. `verify` recompiles the same record from the files currently on
disk and reports every evidence class that has changed. Local hashes detect
tampering and drift; they do not authenticate an approver's identity.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import feature_matrix  # noqa: E402

DEFAULT_RC_MARKER = "== X3 MAINNET RC GATE PASSED =="
SCHEMA = 2
P0_RELEASE_FLOOR = 95
P0_BASELINE_COUNT = 82
P0_CAPABILITY_COUNT = 79
P0_CLAIM_HYGIENE_COUNT = 3
CLAIM_HYGIENE_SUBSYSTEM = "claims_hygiene"
BOUNTY_MIN_POOL_USD = 100_000
BOUNTY_MIN_AGE = dt.timedelta(days=28)
PUBLIC_TESTNET_MIN_DURATION = dt.timedelta(weeks=8)
PUBLIC_TESTNET_MIN_VALIDATORS = 50
PUBLIC_TESTNET_MIN_CONTINENTS = 5
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
APPROVAL_ROLES = {"release", "security", "operations"}


class EvidenceError(ValueError):
    """A release record cannot be built or revalidated from current evidence."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def digest_value(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def fail(message: str) -> None:
    print(f"FAILED: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def git_state(root: Path) -> tuple[str | None, bool]:
    """(commit, dirty) for the tree, or (None, False) outside a git work tree."""
    try:
        commit = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if commit.returncode != 0:
            return None, False
        status = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        return commit.stdout.strip(), bool(status.stdout.strip())
    except (OSError, subprocess.SubprocessError):
        return None, False


def resolve_path(root: Path, raw: Any, label: str) -> Path:
    require(isinstance(raw, str) and bool(raw.strip()), f"{label}: path is missing")
    path = Path(raw)
    return path if path.is_absolute() else root / path


def require_file(root: Path, raw: Any, label: str) -> Path:
    path = resolve_path(root, raw, label)
    if not path.is_file():
        raise EvidenceError(f"{label} not found: {path}")
    return path


def load_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise EvidenceError(f"{label}: cannot read JSON at {path}: {exc}") from exc


def parse_time(value: Any, label: str) -> dt.datetime:
    require(isinstance(value, str) and bool(value.strip()), f"{label}: timestamp is missing")
    text = value.strip().replace("Z", "+00:00")
    try:
        parsed = dt.datetime.fromisoformat(text)
    except ValueError as exc:
        raise EvidenceError(f"{label}: invalid ISO-8601 timestamp {value!r}") from exc
    require(parsed.tzinfo is not None, f"{label}: timestamp must include a timezone")
    return parsed.astimezone(dt.timezone.utc)


def iso_utc(value: dt.datetime) -> str:
    return value.astimezone(dt.timezone.utc).replace(microsecond=0).isoformat().replace(
        "+00:00", "Z"
    )


def describe_spec(path: Path, role: str) -> dict[str, Any]:
    """The artifact's identity, hashed. A spec that does not parse cannot be promoted."""
    spec = load_json(path, role)
    require(isinstance(spec, dict), f"{path}: not a JSON object")
    row = {
        "role": role,
        "path": str(path),
        "sha256": sha256(path),
        "id": spec.get("id"),
        "name": spec.get("name"),
        "chain_type": spec.get("chainType"),
    }
    config = (spec.get("genesis") or {}).get("runtimeGenesis", {}).get("config")
    if isinstance(config, dict):
        row["aura_authorities"] = len((config.get("aura") or {}).get("authorities") or [])
        row["grandpa_authorities"] = len(
            (config.get("grandpa") or {}).get("authorities") or []
        )
        row["bootnodes"] = len(spec.get("bootNodes") or [])
    return row


def feature_snapshot(root: Path, raw_path: Any) -> tuple[dict[str, Any], dict[str, Any]]:
    """Hash the canonical matrix and capture every launch-blocking row."""
    matrix_path = require_file(root, raw_path, "feature_matrix path")
    try:
        matrix = feature_matrix.load_matrix(matrix_path)
    except (OSError, ValueError) as exc:
        raise EvidenceError(f"feature_matrix: cannot load {matrix_path}: {exc}") from exc

    fragments = [matrix_path.parent / rel for rel in matrix.get("includes", [])]
    sources: dict[str, str] = {}
    for path in [matrix_path, *sorted(fragments)]:
        try:
            relative = str(path.resolve().relative_to(root.resolve()))
        except ValueError as exc:
            raise EvidenceError(
                f"feature_matrix: source must be inside the repository root: {path}"
            ) from exc
        sources[relative] = sha256(path)
    source_digest = hashlib.sha256(
        "".join(f"{name}:{digest}" for name, digest in sorted(sources.items())).encode(
            "utf-8"
        )
    ).hexdigest()

    p0 = sorted(
        (
            feature
            for feature in matrix.get("feature", [])
            if isinstance(feature, dict) and feature.get("priority") == "P0"
        ),
        key=lambda feature: str(feature.get("id", "")),
    )
    require(
        len(p0) == P0_BASELINE_COUNT,
        f"feature_matrix: contains {len(p0)} P0 rows; launch requires "
        f"{P0_BASELINE_COUNT}",
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
    require(
        len(capability_rows) == P0_CAPABILITY_COUNT,
        f"feature_matrix: contains {len(capability_rows)} capability P0 rows; "
        f"launch requires {P0_CAPABILITY_COUNT}",
    )
    require(
        len(hygiene_rows) == P0_CLAIM_HYGIENE_COUNT,
        f"feature_matrix: contains {len(hygiene_rows)} claim-hygiene P0 rows; "
        f"launch requires {P0_CLAIM_HYGIENE_COUNT}",
    )
    for feature in hygiene_rows:
        label = str(feature.get("id") or feature.get("name") or "<unknown claim row>")
        require(
            feature.get("claim_risk") is True,
            f"feature_matrix: {label} claim-hygiene row must set claim_risk = true",
        )
        require(
            feature.get("launch_scope") == "research",
            f"feature_matrix: {label} claim-hygiene row must remain research scope",
        )
    rows: list[dict[str, Any]] = []
    for feature in capability_rows:
        label = str(feature.get("id") or feature.get("name") or "<unknown P0>")
        for field in ("implemented", "tested", "mainnet_ready"):
            score = feature.get(field)
            require(
                isinstance(score, int) and not isinstance(score, bool),
                f"feature_matrix: {label}.{field} is not an integer",
            )
            require(
                score >= P0_RELEASE_FLOOR,
                f"feature_matrix: {label}.{field} is {score}; launch requires "
                f"{P0_RELEASE_FLOOR}",
            )
        blockers = feature.get("blockers", [])
        require(
            isinstance(blockers, list) and not blockers,
            f"feature_matrix: {label} still has open blockers",
        )
        rows.append(
            {
                "id": label,
                "implemented": feature["implemented"],
                "tested": feature["tested"],
                "mainnet_ready": feature["mainnet_ready"],
            }
        )
    for feature in hygiene_rows:
        rows.append(
            {
                "id": str(feature.get("id") or feature.get("name")),
                "implemented": feature.get("implemented"),
                "tested": feature.get("tested"),
                "mainnet_ready": feature.get("mainnet_ready"),
            }
        )
    rows.sort(key=lambda row: row["id"])

    snapshot = {
        "count": len(rows),
        "capability_count": len(capability_rows),
        "claim_hygiene_count": len(hygiene_rows),
        "rows": rows,
        "sha256": digest_value(rows),
    }
    matrix_record = {
        "path": str(matrix_path),
        "source_digest": source_digest,
        "sources": sources,
        "p0_count": len(rows),
        "capability_count": len(capability_rows),
        "claim_hygiene_count": len(hygiene_rows),
        "p0_snapshot_sha256": snapshot["sha256"],
    }
    return matrix_record, snapshot


def compile_test_gates(root: Path, raw: Any) -> list[dict[str, Any]]:
    require(isinstance(raw, list) and raw, "test_gates: at least one gate log is required")
    rows: list[dict[str, Any]] = []
    names: set[str] = set()
    for index, gate in enumerate(raw):
        require(isinstance(gate, dict), f"test_gates[{index}]: not an object")
        name = str(gate.get("name", "")).strip()
        require(bool(name), f"test_gates[{index}]: name is missing")
        require(name not in names, f"test_gates: duplicate gate name {name}")
        names.add(name)
        require(gate.get("status") == "PASS", f"test_gates: test gate {name} did not pass")
        exit_code = gate.get("exit_code")
        require(
            isinstance(exit_code, int)
            and not isinstance(exit_code, bool)
            and exit_code == 0,
            f"test_gates: test gate {name} has a non-zero exit code",
        )
        command = gate.get("command")
        require(
            isinstance(command, str) and bool(command.strip()),
            f"test_gates: test gate {name} has no command",
        )
        log = require_file(root, gate.get("log"), f"test_gates: gate {name} log")
        text = log.read_text(encoding="utf-8", errors="replace")
        require("PASS" in text, f"test_gates: gate {name} log has no PASS marker")
        rows.append(
            {
                "name": name,
                "command": command.strip(),
                "status": "PASS",
                "exit_code": 0,
                "log": str(log),
                "sha256": sha256(log),
            }
        )
    return rows


def compile_audits(root: Path, raw: Any) -> list[dict[str, Any]]:
    require(isinstance(raw, list), "audits: must be an array")
    require(len(raw) >= 2, "audits: two independent audit reports are required")
    rows: list[dict[str, Any]] = []
    providers: set[str] = set()
    scope_groups: list[set[str]] = []
    for index, audit in enumerate(raw):
        require(isinstance(audit, dict), f"audits[{index}]: not an object")
        provider = str(audit.get("provider", "")).strip()
        require(bool(provider), f"audits[{index}]: provider is missing")
        require(provider not in providers, f"audits: duplicate provider {provider}")
        providers.add(provider)
        scopes = audit.get("scope")
        if isinstance(scopes, str):
            scopes = [scopes]
        require(
            isinstance(scopes, list)
            and bool(scopes)
            and all(isinstance(scope, str) and scope.strip() for scope in scopes),
            f"audits: provider {provider} has no scope",
        )
        normalized_scopes = sorted({scope.strip().lower() for scope in scopes})
        known_scopes = {
            "runtime",
            "consensus",
            "atomic",
            "bridges",
            "contracts",
            "operations",
        }
        require(
            set(normalized_scopes) <= known_scopes,
            f"audits: provider {provider} has an unknown scope",
        )
        scope_groups.append(set(normalized_scopes))
        for field in ("critical_open", "high_open"):
            value = audit.get(field)
            require(
                value == 0
                and isinstance(value, int)
                and not isinstance(value, bool),
                f"audits: provider {provider} has open {field.replace('_open', '')} findings",
            )
        report = require_file(root, audit.get("report"), f"audits: {provider} report")
        rows.append(
            {
                "provider": provider,
                "scope": normalized_scopes,
                "critical_open": 0,
                "high_open": 0,
                "report": str(report),
                "sha256": sha256(report),
            }
        )
    runtime_scopes = {"runtime", "consensus", "atomic"}
    bridge_scopes = {"bridges", "contracts", "operations"}
    require(
        any(runtime_scopes <= scopes for scopes in scope_groups),
        "audits: one report must cover runtime, consensus and atomic execution",
    )
    require(
        any(bridge_scopes <= scopes for scopes in scope_groups),
        "audits: one report must cover bridges, contracts and operations",
    )
    require(
        not any(
            runtime_scopes <= scopes and bridge_scopes <= scopes
            for scopes in scope_groups
        ),
        "audits: independent scope groups cannot be pooled into one report",
    )
    return rows


def compile_bounty(
    root: Path, raw: Any, recorded_at: dt.datetime
) -> dict[str, Any]:
    require(isinstance(raw, dict), "bounty window: missing")
    starts_at = parse_time(raw.get("starts_at"), "bounty window start")
    ends_at = parse_time(raw.get("ends_at"), "bounty window end")
    pool = raw.get("pool_usd")
    require(
        isinstance(pool, int)
        and not isinstance(pool, bool)
        and pool >= BOUNTY_MIN_POOL_USD,
        f"bounty window: pool must be at least ${BOUNTY_MIN_POOL_USD:,}",
    )
    require(
        recorded_at - starts_at >= BOUNTY_MIN_AGE,
        "bounty window: must be live for at least four weeks before launch",
    )
    require(ends_at > recorded_at, "bounty window: has already ended")
    url = raw.get("url")
    require(
        isinstance(url, str) and url.startswith(("https://", "http://")),
        "bounty window: public URL is missing",
    )
    evidence = require_file(root, raw.get("evidence"), "bounty window evidence")
    return {
        "url": url,
        "pool_usd": pool,
        "starts_at": iso_utc(starts_at),
        "ends_at": iso_utc(ends_at),
        "evidence": str(evidence),
        "sha256": sha256(evidence),
    }


def compile_public_testnet(
    root: Path, raw: Any, recorded_at: dt.datetime
) -> dict[str, Any]:
    require(isinstance(raw, dict), "public-testnet window: missing")
    started_at = parse_time(raw.get("started_at"), "public-testnet window start")
    ended_at = parse_time(raw.get("ended_at"), "public-testnet window end")
    require(
        ended_at - started_at >= PUBLIC_TESTNET_MIN_DURATION,
        "public-testnet window: must run for at least eight weeks",
    )
    require(
        ended_at <= recorded_at,
        "public-testnet window: end must not be after the release record",
    )
    validators = raw.get("validator_count")
    continents = raw.get("continents")
    require(
        isinstance(validators, int)
        and not isinstance(validators, bool)
        and validators >= PUBLIC_TESTNET_MIN_VALIDATORS,
        "public-testnet window: at least 50 external validators are required",
    )
    require(
        isinstance(continents, int)
        and not isinstance(continents, bool)
        and continents >= PUBLIC_TESTNET_MIN_CONTINENTS,
        "public-testnet window: at least five continents are required",
    )
    evidence = require_file(root, raw.get("evidence"), "public-testnet evidence")
    return {
        "started_at": iso_utc(started_at),
        "ended_at": iso_utc(ended_at),
        "validator_count": validators,
        "continents": continents,
        "evidence": str(evidence),
        "sha256": sha256(evidence),
    }


def compile_bridges(root: Path, raw: Any) -> dict[str, Any]:
    require(isinstance(raw, dict), "bridge_configuration: missing")
    path = require_file(root, raw.get("path"), "bridge_configuration")
    config = load_json(path, "bridge_configuration")
    require(isinstance(config, dict), "bridge_configuration: not a JSON object")
    routes = config.get("active_routes")
    require(
        isinstance(routes, list)
        and all(isinstance(route, str) for route in routes)
        and len(routes) == len(set(routes))
        and set(routes) == BRIDGE_ROUTES,
        "bridge_configuration: active routes must be exactly "
        "ethereum-mainnet, solana-mainnet-beta and bitcoin-mainnet",
    )
    for field, expected in BRIDGE_LIMITS.items():
        require(
            config.get(field) == expected,
            f"bridge_configuration: {field} must be {expected}",
        )
    require(
        config.get("kill_switch") is True,
        "bridge_configuration: kill switch must be enabled",
    )
    return {
        "path": str(path),
        "sha256": sha256(path),
        "snapshot": {
            "active_routes": sorted(routes),
            **BRIDGE_LIMITS,
            "kill_switch": True,
        },
    }


def compile_validators(root: Path, raw: Any) -> dict[str, Any]:
    require(isinstance(raw, dict), "validators: missing")
    path = require_file(root, raw.get("path"), "validators evidence")
    evidence = load_json(path, "validators")
    require(isinstance(evidence, dict), "validators: evidence is not a JSON object")
    entries = evidence.get("validators")
    require(isinstance(entries, list), "validators: validators must be an array")
    require(
        len(entries) == 7,
        f"validators: launch requires 7 independent validators, found {len(entries)}",
    )
    normalized: list[dict[str, str]] = []
    for field in ("host", "operator", "identity"):
        values = [
            str(entry.get(field, "")).strip()
            for entry in entries
            if isinstance(entry, dict)
        ]
        require(
            len(values) == len(entries),
            f"validators: every validator must be an object",
        )
        require(all(values), f"validators: every validator needs {field}")
        require(len(set(values)) == len(values), f"validators: duplicate {field}")
    for index, entry in enumerate(entries):
        require(entry.get("loopback") is False, f"validators[{index}]: loopback is forbidden")
        host = str(entry["host"]).strip().lower().strip("[]")
        require(
            host not in {"localhost", "localhost.", "::1", "0.0.0.0", "::"}
            and not host.startswith("127."),
            f"validators[{index}]: loopback validator host is forbidden",
        )
        normalized.append(
            {
                "host": str(entry["host"]).strip(),
                "operator": str(entry["operator"]).strip(),
                "identity": str(entry["identity"]).strip(),
            }
        )
    return {
        "path": str(path),
        "sha256": sha256(path),
        "entries": normalized,
    }


def compile_approvals(
    root: Path, raw: Any, commit: str, approved_by: str
) -> list[dict[str, Any]]:
    require(isinstance(raw, list), "approvals: must be an array")
    require(
        len(raw) >= 3,
        "approvals: release, security and operations approval are required",
    )
    rows: list[dict[str, Any]] = []
    approvers: set[str] = set()
    roles: set[str] = set()
    for index, approval in enumerate(raw):
        require(isinstance(approval, dict), f"approvals[{index}]: not an object")
        approver = str(approval.get("approver", "")).strip()
        role = str(approval.get("role", "")).strip().lower()
        require(bool(approver), f"approvals[{index}]: approver is missing")
        require(approver not in approvers, f"approvals: duplicate approver {approver}")
        require(role in APPROVAL_ROLES, f"approvals: {approver} has unknown role {role!r}")
        require(
            approval.get("approved_commit") == commit,
            f"approvals: approval {approver} names another commit",
        )
        evidence = require_file(
            root, approval.get("evidence"), f"approvals: {approver} evidence"
        )
        approvers.add(approver)
        roles.add(role)
        rows.append(
            {
                "approver": approver,
                "role": role,
                "approved_commit": commit,
                "evidence": str(evidence),
                "sha256": sha256(evidence),
            }
        )
    missing_roles = sorted(APPROVAL_ROLES - roles)
    require(not missing_roles, f"approvals: missing roles {', '.join(missing_roles)}")
    require(
        approved_by in approvers,
        f"approvals: --approved-by {approved_by!r} is not in the approval set",
    )
    return rows


def compile_record(
    *,
    root: Path,
    spec: Path,
    raw: Path,
    rc_log: Path,
    evidence_pack: Path,
    expect_authorities: int,
    expect_passed: str,
    approved_by: str,
    commit: str,
    recorded_at: dt.datetime,
) -> dict[str, Any]:
    """Compile schema 2 from current bytes without writing anything."""
    pack = load_json(evidence_pack, "evidence_pack")
    require(isinstance(pack, dict), "evidence_pack: not a JSON object")

    spec_row = describe_spec(spec, "mainnet-plain")
    raw_row = describe_spec(raw, "mainnet-raw")
    require(
        spec_row.get("chain_type") == "Live",
        f"{spec}: chainType is {spec_row.get('chain_type')!r}, not 'Live' "
        "— that is not a mainnet spec",
    )
    require(
        raw_row.get("chain_type") == "Live",
        f"{raw}: chainType is {raw_row.get('chain_type')!r}, not 'Live' "
        "— that is not a mainnet spec",
    )
    authorities = spec_row.get("aura_authorities")
    require(
        authorities is not None,
        f"{spec}: no genesis.runtimeGenesis.config to read an authority set from",
    )
    require(
        authorities == expect_authorities,
        f"{spec}: {authorities} Aura authorities, but this launch expects "
        f"{expect_authorities} — a spec cannot be promoted for a validator set it "
        "does not have",
    )
    require(
        spec_row.get("grandpa_authorities") == authorities,
        f"{spec}: Aura and GRANDPA authority counts disagree",
    )
    require(
        bool(spec_row.get("bootnodes")),
        f"{spec}: no bootNodes — a promoted chain nobody can join is not a launch",
    )

    log_text = rc_log.read_text(encoding="utf-8", errors="replace")
    require(
        expect_passed in log_text,
        f"{rc_log}: no {expect_passed!r} in the log — a passing gate has to say so itself",
    )
    matrix_record, p0_snapshot = feature_snapshot(
        root, (pack.get("feature_matrix") or {}).get("path")
    )
    test_gates = compile_test_gates(root, pack.get("test_gates"))
    audits = compile_audits(root, pack.get("audits"))
    bounty = compile_bounty(root, pack.get("bounty_window"), recorded_at)
    public_testnet = compile_public_testnet(
        root, pack.get("public_testnet_window"), recorded_at
    )
    bridges = compile_bridges(root, pack.get("bridge_configuration"))
    validators = compile_validators(root, pack.get("validators"))
    approvals = compile_approvals(root, pack.get("approvals"), commit, approved_by)

    return {
        "schema": SCHEMA,
        "recorded_at": iso_utc(recorded_at),
        "approved_by": approved_by,
        "commit": commit,
        "expect_authorities": expect_authorities,
        "rc_gate": {
            "log": str(rc_log),
            "sha256": sha256(rc_log),
            "passed_marker": expect_passed,
        },
        "artifacts": [spec_row, raw_row],
        "evidence_pack": {
            "path": str(evidence_pack),
            "sha256": sha256(evidence_pack),
        },
        "feature_matrix": matrix_record,
        "p0_snapshot": p0_snapshot,
        "test_gates": test_gates,
        "audits": audits,
        "bounty_window": bounty,
        "public_testnet_window": public_testnet,
        "bridge_configuration": bridges,
        "validators": validators,
        "approvals": approvals,
    }


def record_hash(record: dict[str, Any]) -> str:
    unsigned = {key: value for key, value in record.items() if key != "record_sha256"}
    return digest_value(unsigned)


def build(args: argparse.Namespace) -> int:
    root = Path(args.root)
    spec = Path(args.spec)
    raw = Path(args.raw)
    rc_log = Path(args.rc_log)
    evidence_pack = Path(args.evidence_pack)

    for label, path in (
        ("spec", spec),
        ("raw spec", raw),
        ("rc log", rc_log),
        ("evidence pack", evidence_pack),
    ):
        if not path.is_file():
            fail(f"{label} not found: {path}")

    approved_by = args.approved_by.strip()
    if not approved_by:
        fail("no operator: pass --approved-by '<who approved this launch>'")

    commit, dirty = git_state(root)
    if args.commit:
        commit = args.commit
    if commit is None:
        fail("cannot read the commit to promote (not a git work tree?); pass --commit <sha>")
    if dirty and not args.allow_dirty:
        fail(
            "the working tree is dirty: a promotion record names bytes that may not "
            "be what is committed"
        )

    try:
        record = compile_record(
            root=root,
            spec=spec,
            raw=raw,
            rc_log=rc_log,
            evidence_pack=evidence_pack,
            expect_authorities=args.expect_authorities,
            expect_passed=args.expect_passed,
            approved_by=approved_by,
            commit=commit,
            recorded_at=dt.datetime.now(dt.timezone.utc).replace(microsecond=0),
        )
    except EvidenceError as exc:
        fail(str(exc))

    record["record_sha256"] = record_hash(record)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {out}")
    print(f"  schema            {SCHEMA}")
    print(f"  commit            {record['commit']}")
    print(f"  approved by       {record['approved_by']}")
    print(f"  P0 snapshot       {record['p0_snapshot']['count']} rows")
    print(f"  audits            {len(record['audits'])}")
    print(f"  validators        {len(record['validators']['entries'])}")
    print(f"  record sha256     {record['record_sha256']}")
    return 0


def verify(args: argparse.Namespace) -> int:
    root = Path(args.root)
    record_path = Path(args.record)
    if not record_path.is_file():
        fail(f"no record at {record_path}")
    try:
        record = json.loads(record_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"{record_path}: cannot read record: {exc}")
    if not isinstance(record, dict):
        fail(f"{record_path}: record is not a JSON object")
    if record.get("schema") != SCHEMA:
        fail(f"{record_path}: schema {record.get('schema')!r}, this tool verifies {SCHEMA}")

    problems: list[str] = []
    actual_record_hash = record_hash(record)
    if record.get("record_sha256") != actual_record_hash:
        problems.append(
            f"record_sha256: {record.get('record_sha256')} != {actual_record_hash}"
        )
    if not record.get("approved_by"):
        problems.append("approved_by: no operator recorded")
    if not record.get("commit"):
        problems.append("commit: no source commit recorded")

    for artifact in record.get("artifacts", []):
        path = Path(artifact.get("path", ""))
        if not path.is_file():
            problems.append(f"artifacts: {path}: the record names an artifact that is gone")
            continue
        actual = sha256(path)
        if actual != artifact.get("sha256"):
            problems.append(
                f"artifacts: {path}: sha256 {actual} != recorded {artifact.get('sha256')}"
            )

    gate = record.get("rc_gate", {})
    log = Path(gate.get("log", ""))
    if not log.is_file():
        problems.append(f"rc_gate: {log}: the RC log this record cites is gone")
    elif sha256(log) != gate.get("sha256"):
        problems.append(f"rc_gate: {log}: the RC log has changed since the record was written")
    else:
        text = log.read_text(encoding="utf-8", errors="replace")
        if gate.get("passed_marker") not in text:
            problems.append(
                f"rc_gate: {log}: no {gate.get('passed_marker')!r} in the log any more"
            )

    current_commit, _ = git_state(root)
    if current_commit and current_commit != record.get("commit"):
        problems.append(
            f"commit: the tree is at {current_commit}, not the recorded {record.get('commit')}"
        )

    artifacts = {
        artifact.get("role"): artifact.get("path")
        for artifact in record.get("artifacts", [])
    }
    try:
        expected = compile_record(
            root=root,
            spec=Path(artifacts["mainnet-plain"]),
            raw=Path(artifacts["mainnet-raw"]),
            rc_log=log,
            evidence_pack=Path(record.get("evidence_pack", {}).get("path", "")),
            expect_authorities=record["expect_authorities"],
            expect_passed=gate["passed_marker"],
            approved_by=record["approved_by"],
            commit=record["commit"],
            recorded_at=parse_time(record["recorded_at"], "recorded_at"),
        )
    except (EvidenceError, KeyError, TypeError) as exc:
        problems.append(f"evidence: current evidence cannot be recompiled: {exc}")
    else:
        for key, value in expected.items():
            if record.get(key) != value:
                problems.append(f"{key}: current evidence does not match recorded value")

    if problems:
        for problem in dict.fromkeys(problems):
            print(f"FAILED: {problem}", file=sys.stderr)
        return 1
    print(
        f"{record_path}: verified — {record['approved_by']} promoted "
        f"{record['commit']} with {record['p0_snapshot']['count']} current P0 rows"
    )
    print(f"  record sha256     {record['record_sha256']}")
    print(f"  audits            {len(record['audits'])}")
    print(f"  validators        {len(record['validators']['entries'])}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Record and verify a launch promotion")
    parser.add_argument("--root", default=".", help="repository root (default: .)")
    sub = parser.add_subparsers(dest="command", required=True)

    build_parser = sub.add_parser("build", help="write a schema-2 promotion record")
    build_parser.add_argument("--spec", required=True, help="plain mainnet chain spec")
    build_parser.add_argument("--raw", required=True, help="raw mainnet chain spec")
    build_parser.add_argument("--rc-log", required=True, help="the RC gate's log")
    build_parser.add_argument(
        "--evidence-pack",
        required=True,
        help="JSON index of matrix, gates, audits, windows, bridges, validators and approvals",
    )
    build_parser.add_argument("--expect-authorities", type=int, default=7)
    build_parser.add_argument("--expect-passed", default=DEFAULT_RC_MARKER)
    build_parser.add_argument("--approved-by", required=True)
    build_parser.add_argument("--commit", default="", help="override the commit (default: HEAD)")
    build_parser.add_argument("--allow-dirty", action="store_true")
    build_parser.add_argument("--out", required=True)
    build_parser.set_defaults(func=build)

    verify_parser = sub.add_parser("verify", help="re-check a record against current evidence")
    verify_parser.add_argument("--record", required=True)
    verify_parser.set_defaults(func=verify)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
