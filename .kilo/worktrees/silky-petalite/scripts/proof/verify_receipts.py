#!/usr/bin/env python3
"""Deterministic ProofForge receipt verifier.

This verifier intentionally uses explicit field checks instead of implicit
best-effort parsing so the same receipt corpus always yields the same
pass/fail output across environments.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


HEX_64_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{7,64}$")
CLAIM_ID_RE = re.compile(r"^x3\.[a-zA-Z0-9_.-]+$")
# RFC3339 with optional fractional second up to 9 digits.
TS_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$"
)

TOP_REQUIRED = (
    "repo_commit_hash",
    "command_run",
    "artifact_hash",
    "policy_hash",
    "relevant_files",
    "timestamp",
    "result",
    "limitations",
    "binding_hash",
)

RESULT_REQUIRED = (
    "claim_id",
    "claim",
    "status",
    "files_inspected",
    "commands_run",
    "passed_checks",
    "failed_checks",
    "missing_proofs",
    "blockers",
    "score",
    "evidence",
    "timestamp",
    "duration_ms",
)

ALLOWED_STATUS = {
    "verified",
    "pass",
    "passed",
    "partial",
    "failed",
    "unverified",
    "blocked",
    "proof_complete",
    "green",
    "red",
    "unknown",
}


@dataclass
class ValidationResult:
    state: str
    errors: list[str]
    claim_id: str | None


def parse_rfc3339(value: str) -> datetime | None:
    # Normalize nanosecond values for Python datetime parsing.
    match = re.match(r"^(.*\.)(\d+)(Z|[+-]\d{2}:\d{2})$", value)
    if match and len(match.group(2)) > 6:
        value = f"{match.group(1)}{match.group(2)[:6]}{match.group(3)}"
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(value)
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=timezone.utc)
        return parsed
    except ValueError:
        return None


def commit_exists(repo_root: Path, commit_hash: str) -> bool:
    result = subprocess.run(
        [
            "git",
            "cat-file",
            "-e",
            f"{commit_hash}^{{commit}}",
        ],
        cwd=repo_root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def is_string_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


def detect_legacy(payload: dict[str, Any]) -> bool:
    return (
        "claim_id" in payload
        and "status" in payload
        and "date" in payload
        and "verifier" in payload
        and "hash" in payload
        and "repo_commit_hash" not in payload
    )


def extract_filename_claim_id(file_path: Path) -> str | None:
    name = file_path.name
    suffix = ".receipt.json"
    if not name.endswith(suffix):
        return None
    return name[: -len(suffix)]


def validate_payload(
    payload: dict[str, Any],
    file_path: Path,
    repo_root: Path,
    max_age_hours: int,
    enforce_provenance: bool,
    enforce_freshness: bool,
) -> ValidationResult:
    errors: list[str] = []

    for key in TOP_REQUIRED:
        if key not in payload:
            errors.append(f"missing top-level field: {key}")

    if errors:
        return ValidationResult(state="INVALID", errors=errors, claim_id=None)

    if not isinstance(payload["repo_commit_hash"], str) or not COMMIT_RE.fullmatch(
        payload["repo_commit_hash"]
    ):
        errors.append("repo_commit_hash must match ^[0-9a-f]{7,64}$")

    for key in ("artifact_hash", "policy_hash", "binding_hash"):
        value = payload[key]
        if not isinstance(value, str) or not HEX_64_RE.fullmatch(value):
            errors.append(f"{key} must be 64-char lowercase hex")

    command_run = payload["command_run"]
    if not isinstance(command_run, str) or not command_run.strip():
        errors.append("command_run must be a non-empty string")
    elif not command_run.startswith("x3-proof verify "):
        errors.append("command_run must begin with 'x3-proof verify '")

    if not is_string_list(payload["relevant_files"]):
        errors.append("relevant_files must be an array of strings")

    top_level_ts = payload["timestamp"]
    if not isinstance(top_level_ts, str) or not TS_RE.fullmatch(top_level_ts):
        errors.append("timestamp must be RFC3339 string")
    else:
        parsed = parse_rfc3339(top_level_ts)
        if parsed is None:
            errors.append("timestamp could not be parsed")
        elif enforce_freshness:
            now = datetime.now(timezone.utc)
            if parsed < (now - timedelta(hours=max_age_hours)):
                errors.append(
                    f"timestamp is older than freshness window ({max_age_hours}h)"
                )

    if enforce_provenance and isinstance(payload["repo_commit_hash"], str):
        if not commit_exists(repo_root, payload["repo_commit_hash"]):
            errors.append("repo_commit_hash is not a valid commit in this repository")

    if not is_string_list(payload["limitations"]):
        errors.append("limitations must be an array of strings")

    result = payload["result"]
    if not isinstance(result, dict):
        errors.append("result must be an object")
        return ValidationResult(state="INVALID", errors=errors, claim_id=None)

    for key in RESULT_REQUIRED:
        if key not in result:
            errors.append(f"missing result field: {key}")

    if errors:
        return ValidationResult(state="INVALID", errors=errors, claim_id=None)

    claim_id = result.get("claim_id")
    if not isinstance(claim_id, str) or not CLAIM_ID_RE.fullmatch(claim_id):
        errors.append("result.claim_id must match ^x3\\.[a-zA-Z0-9_.-]+$")

    if "claim_id" in payload and payload["claim_id"] != claim_id:
        errors.append("top-level claim_id must match result.claim_id")

    filename_claim = extract_filename_claim_id(file_path)
    if filename_claim is not None and claim_id is not None and filename_claim != claim_id:
        errors.append("filename claim_id must match result.claim_id")

    claim_text = result.get("claim")
    if not isinstance(claim_text, str) or not claim_text.strip():
        errors.append("result.claim must be a non-empty string")

    status = result.get("status")
    if not isinstance(status, str):
        errors.append("result.status must be a string")
    elif status.lower() not in ALLOWED_STATUS:
        errors.append(f"result.status '{status}' is not in canonical status set")

    if not is_string_list(result.get("files_inspected")):
        errors.append("result.files_inspected must be an array of strings")
    if not is_string_list(result.get("commands_run")):
        errors.append("result.commands_run must be an array of strings")
    if not is_string_list(result.get("passed_checks")):
        errors.append("result.passed_checks must be an array of strings")
    if not is_string_list(result.get("failed_checks")):
        errors.append("result.failed_checks must be an array of strings")
    if not is_string_list(result.get("missing_proofs")):
        errors.append("result.missing_proofs must be an array of strings")
    if not is_string_list(result.get("blockers")):
        errors.append("result.blockers must be an array of strings")

    score = result.get("score")
    if not isinstance(score, (int, float)):
        errors.append("result.score must be numeric")
    elif score < 0 or score > 1:
        errors.append("result.score must be in [0,1]")

    if not isinstance(result.get("evidence"), dict):
        errors.append("result.evidence must be an object")

    result_timestamp = result.get("timestamp")
    if not isinstance(result_timestamp, str) or not TS_RE.fullmatch(result_timestamp):
        errors.append("result.timestamp must be RFC3339 string")
    else:
        parsed_result_ts = parse_rfc3339(result_timestamp)
        parsed_top_ts = parse_rfc3339(payload["timestamp"])
        if parsed_result_ts is None:
            errors.append("result.timestamp could not be parsed")
        elif parsed_top_ts is not None and parsed_result_ts > parsed_top_ts:
            errors.append("result.timestamp must not be later than top-level timestamp")

    duration = result.get("duration_ms")
    if not isinstance(duration, int) or duration < 0:
        errors.append("result.duration_ms must be a non-negative integer")

    return ValidationResult(
        state="OK" if not errors else "INVALID",
        errors=errors,
        claim_id=claim_id if isinstance(claim_id, str) else None,
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Verify ProofForge claim receipts")
    parser.add_argument("--root", required=True, help="Repository root")
    parser.add_argument(
        "--receipts-dir",
        default="proof/receipts/claims",
        help="Path to receipt directory, relative to --root",
    )
    parser.add_argument(
        "--schema-file",
        default="proof/schema/receipt-v2.schema.json",
        help="Path to canonical schema file, relative to --root",
    )
    parser.add_argument(
        "--max-age-hours",
        type=int,
        default=24,
        help="Maximum receipt age in hours when freshness is enforced",
    )
    parser.add_argument(
        "--skip-provenance-check",
        action="store_true",
        help="Disable git commit provenance checks",
    )
    parser.add_argument(
        "--skip-freshness-check",
        action="store_true",
        help="Disable timestamp freshness checks",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    receipts_dir = (root / args.receipts_dir).resolve()
    schema_file = (root / args.schema_file).resolve()

    if not receipts_dir.is_dir():
        print(f"No receipts directory found at {receipts_dir}")
        return 0

    if not schema_file.is_file():
        print(f"ERROR: Missing schema file: {schema_file}", file=sys.stderr)
        return 2

    print(f"Validating receipts in {receipts_dir}")

    receipt_paths = sorted(receipts_dir.rglob("*.json"))
    seen_claim_ids: dict[str, Path] = {}

    ok_count = 0
    legacy_count = 0
    invalid_count = 0

    for path in receipt_paths:
        rel = path.relative_to(root)
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"INVALID {rel}")
            print(f"  - unable to parse JSON: {exc}")
            invalid_count += 1
            continue

        if not isinstance(payload, dict):
            print(f"INVALID {rel}")
            print("  - top-level payload must be a JSON object")
            invalid_count += 1
            continue

        if detect_legacy(payload):
            print(f"LEGACY  {rel}")
            legacy_count += 1
            continue

        result = validate_payload(
            payload=payload,
            file_path=path,
            repo_root=root,
            max_age_hours=args.max_age_hours,
            enforce_provenance=not args.skip_provenance_check,
            enforce_freshness=not args.skip_freshness_check,
        )
        if result.state != "OK":
            print(f"INVALID {rel}")
            for err in result.errors:
                print(f"  - {err}")
            invalid_count += 1
            continue

        if result.claim_id in seen_claim_ids:
            print(f"INVALID {rel}")
            print(
                f"  - duplicate claim_id '{result.claim_id}' already seen in "
                f"{seen_claim_ids[result.claim_id].relative_to(root)}"
            )
            invalid_count += 1
            continue

        if result.claim_id is not None:
            seen_claim_ids[result.claim_id] = path

        print(f"OK      {rel}")
        ok_count += 1

    print()
    print("Summary:")
    print(f"  ok:      {ok_count}")
    print(f"  legacy:  {legacy_count}")
    print(f"  invalid: {invalid_count}")

    if legacy_count > 0 or invalid_count > 0:
        print(
            "Receipt validation FAILED: migrate legacy receipts and fix invalid shapes.",
            file=sys.stderr,
        )
        return 1

    print("Receipt validation PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())