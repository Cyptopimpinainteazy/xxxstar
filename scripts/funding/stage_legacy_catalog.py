#!/usr/bin/env python3
"""Stage historical X3 funding catalog records privately in the Money Swarm gateway.

Default is offline preview only. --execute --confirm-stage explicitly enables
private HTTP ingestion. Does not contact funders, determine eligibility, or
create live opportunities/awards.
"""

from __future__ import annotations

import argparse
from collections import Counter
import ipaddress
import json
import os
from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

MAX_RECORD_BYTES = 64 * 1024
DEFAULT_BATCH = 32
MAX_BATCH = 32


def validate_endpoint(endpoint: str) -> str:
    url = urlsplit(endpoint)
    if (url.username or url.password or url.query or url.fragment
            or not url.hostname or not url.port and url.scheme not in ("http", "https")):
        raise ValueError("invalid funding gateway endpoint")
    if url.path.rstrip("/") != "/api/v1/admin/funding-swarm/legacy-leads":
        raise ValueError("endpoint must be the private historical-lead intake")
    if url.scheme not in ("http", "https"):
        raise ValueError("gateway endpoint must be HTTP(S)")
    if url.scheme == "http":
        try:
            local = ipaddress.ip_address(url.hostname).is_loopback
        except ValueError:
            local = url.hostname.lower() == "localhost"
        if not local:
            raise ValueError("HTTP is allowed only for loopback gateway addresses")
    return endpoint


def validate_record(record: object) -> dict:
    if not isinstance(record, dict):
        raise ValueError("catalog rows must be JSON objects")
    if (record.get("origin") != "legacy_markdown_research"
            or record.get("current_availability") != "UNKNOWN"
            or record.get("current_eligibility") != "UNKNOWN"
            or record.get("application_authorized") is not False):
        raise ValueError("archive record attempted to assert live availability or authorization")
    if not all(isinstance(record.get(name), str) and record[name].strip()
               for name in ("legacy_id", "source_file", "name", "category")):
        raise ValueError("archival record missing required identity or provenance")
    return record


def read_catalog(path: Path):
    if not path.is_file():
        raise ValueError(f"input file not found: {path}")
    with path.open(encoding="utf-8") as file:
        for number, line in enumerate(file, 1):
            if len(line.encode("utf-8")) > MAX_RECORD_BYTES:
                raise ValueError(f"oversize record at line {number}")
            if not line.strip():
                continue
            try:
                parsed = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"invalid catalog JSON at line {number}") from exc
            yield validate_record(parsed)


def batches(records, batch_size: int):
    batch = []
    for record in records:
        batch.append(record)
        if len(batch) == batch_size:
            yield batch
            batch = []
    if batch:
        yield batch


class RejectRedirects(HTTPRedirectHandler):
    """Never forward operator credentials or change an ingestion request's method."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def stage(batch: list[dict], endpoint: str, token: str) -> dict:
    payload = json.dumps({"leads": batch}, ensure_ascii=False).encode("utf-8")
    request = Request(
        endpoint,
        data=payload,
        method="POST",
        headers={"Content-Type": "application/json", "X-Admin-Token": token},
    )
    with build_opener(RejectRedirects()).open(request, timeout=30) as response:
        result = json.load(response)
    if (result.get("current_opportunities_verified") != 0
            or result.get("total") != len(batch)
            or result.get("inserted", 0) + result.get("unchanged", 0) != len(batch)):
        raise ValueError("gateway returned inconsistent historical lead accounting")
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="JSONL from legacy_catalog.py")
    parser.add_argument("--endpoint", default="http://127.0.0.1:3000/api/v1/admin/funding-swarm/legacy-leads")
    parser.add_argument("--batch-size", type=int, default=DEFAULT_BATCH)
    parser.add_argument("--execute", action="store_true", help="allow private gateway insertion")
    parser.add_argument("--confirm-stage", action="store_true", help="explicitly confirm private historical staging")
    args = parser.parse_args()

    if not 1 <= args.batch_size <= MAX_BATCH:
        parser.error("batch-size must be between 1 and 32")
    if args.execute != args.confirm_stage:
        parser.error("execution requires both --execute and --confirm-stage")
    try:
        endpoint = validate_endpoint(args.endpoint)
    except ValueError as exc:
        parser.error(str(exc))
    token = os.getenv("FUNDING_SWARM_ADMIN_TOKEN") if args.execute else None
    if args.execute and not token:
        parser.error("FUNDING_SWARM_ADMIN_TOKEN is required for private staging")
    count = inserted = unchanged = 0
    categories = Counter()
    try:
        for batch in batches(read_catalog(args.input), args.batch_size):
            count += len(batch)
            categories.update(record["category"] for record in batch)
            if args.execute:
                result = stage(batch, endpoint, token)
                inserted += result["inserted"]
                unchanged += result["unchanged"]
    except (ValueError, OSError) as exc:
        parser.exit(status=1, message=f"staging failed: {exc}\n")

    if not count:
        parser.exit(status=1, message="empty catalog; nothing staged\n")
    print(json.dumps({
        "mode": "PRIVATE_STAGED" if args.execute else "OFFLINE_PREVIEW",
        "historical_leads": count,
        "inserted": inserted if args.execute else None,
        "unchanged": unchanged if args.execute else None,
        "current_opportunities_verified": 0,
        "categories": dict(sorted(categories.items())),
        "outbound_applications": 0,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
