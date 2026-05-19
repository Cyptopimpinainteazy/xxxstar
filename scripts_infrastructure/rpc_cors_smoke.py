#!/usr/bin/env python3
"""
Live smoke test for RPC CORS policy.

Checks:
1) Allowed origin receives a valid CORS allow-origin header.
2) Blocked origin does not receive wildcard/echoed allow-origin.
"""

from __future__ import annotations

import argparse
import http.client
import json
from typing import Any


def rpc_post(
    host: str,
    port: int,
    origin: str,
    method: str = "system_chain",
) -> tuple[int, str | None, str]:
    conn = http.client.HTTPConnection(host, port, timeout=10)
    try:
        body = json.dumps(
            {"jsonrpc": "2.0", "id": 1, "method": method, "params": []},
            separators=(",", ":"),
        )
        conn.request(
            "POST",
            "/",
            body=body,
            headers={
                "Content-Type": "application/json",
                "Origin": origin,
                "Connection": "close",
            },
        )
        response = conn.getresponse()
        status = response.status
        allow_origin = response.getheader("access-control-allow-origin")
        payload = response.read().decode("utf-8", errors="replace")
        return status, allow_origin, payload
    finally:
        conn.close()


def run(host: str, port: int, allowed_origin: str, blocked_origin: str) -> int:
    allowed_status, allowed_header, allowed_payload = rpc_post(
        host, port, allowed_origin
    )
    blocked_status, blocked_header, blocked_payload = rpc_post(
        host, port, blocked_origin
    )

    summary: dict[str, Any] = {
        "allowed_origin": {
            "origin": allowed_origin,
            "status": allowed_status,
            "allow_origin_header": allowed_header,
            "payload_preview": allowed_payload[:160],
        },
        "blocked_origin": {
            "origin": blocked_origin,
            "status": blocked_status,
            "allow_origin_header": blocked_header,
            "payload_preview": blocked_payload[:160],
        },
    }
    print(json.dumps(summary, indent=2))

    allowed_ok = allowed_status < 400 and allowed_header in {allowed_origin, "*"}
    blocked_ok = blocked_header not in {"*", blocked_origin}

    if allowed_ok and blocked_ok:
        print("PASS: CORS policy blocks untrusted origin without wildcard exposure.")
        return 0

    print("FAIL: CORS policy did not match expected allow/block behavior.")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=9944)
    parser.add_argument("--allowed-origin", default="http://localhost:3000")  # nosemgrep: py-no-localhost-endpoints
    parser.add_argument("--blocked-origin", default="http://evil.com")
    args = parser.parse_args()
    return run(args.host, args.port, args.allowed_origin, args.blocked_origin)


if __name__ == "__main__":
    raise SystemExit(main())
