#!/usr/bin/env python3
"""
Simple live smoke test for per-connection RPC rate limiting.

It opens two separate persistent HTTP connections to the node:
1) Floods connection A with a method guarded by `enforce_rpc_rate_limit`.
2) Sends a request on connection B and verifies it is not rate-limited.
"""

from __future__ import annotations

import argparse
import http.client
import json
import sys
from typing import Any

RATE_LIMIT_ERROR_CODE = -32098


def rpc_call(
    conn: http.client.HTTPConnection,
    method: str,
    params: Any,
    request_id: int,
) -> tuple[bool, int | None, str | None, Any]:
    body = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        }
    )
    conn.request(
        "POST",
        "/",
        body=body,
        headers={
            "Content-Type": "application/json",
            "Connection": "keep-alive",
        },
    )
    response = conn.getresponse()
    raw = response.read().decode("utf-8", errors="replace")

    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        return False, None, f"non-JSON response: {raw[:200]}", None

    if "error" in payload:
        err = payload["error"]
        return False, err.get("code"), err.get("message"), None

    return True, None, None, payload.get("result")


def run(host: str, port: int, method: str, params: Any, burst_requests: int) -> int:
    conn_a = http.client.HTTPConnection(host, port, timeout=10)
    conn_b = http.client.HTTPConnection(host, port, timeout=10)

    rate_limited_a = 0
    other_errors_a = 0

    try:
        for i in range(burst_requests):
            ok, code, _message, _ = rpc_call(conn_a, method, params, i + 1)
            if not ok:
                if code == RATE_LIMIT_ERROR_CODE:
                    rate_limited_a += 1
                else:
                    other_errors_a += 1

        # One more on A after burst to make sure limiter remains active.
        ok_a_final, code_a_final, message_a_final, _ = rpc_call(
            conn_a, method, params, burst_requests + 1
        )

        ok_b, code_b, message_b, _ = rpc_call(conn_b, method, params, 999_001)

        summary: dict[str, Any] = {
            "connection_a": {
                "burst_requests": burst_requests,
                "rate_limited_count": rate_limited_a,
                "other_error_count": other_errors_a,
                "final_ok": ok_a_final,
                "final_error_code": code_a_final,
                "final_error_message": message_a_final,
            },
            "connection_b": {
                "ok": ok_b,
                "error_code": code_b,
                "error_message": message_b,
            },
        }
        print(json.dumps(summary, indent=2))

        isolated = code_b != RATE_LIMIT_ERROR_CODE
        limited_on_a = (
            rate_limited_a > 0
            or code_a_final == RATE_LIMIT_ERROR_CODE
            or (not ok_a_final and message_a_final and "Rate limit exceeded" in message_a_final)
        )

        if limited_on_a and isolated:
            print("PASS: connection-scoped rate limiting observed.")
            return 0

        print("FAIL: expected A to be limited and B to remain un-limited.")
        return 1
    finally:
        conn_a.close()
        conn_b.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=9944)
    parser.add_argument("--method", default="atomicTrade_estimateCost")
    parser.add_argument("--burst-requests", type=int, default=220)
    parser.add_argument(
        "--params-json",
        default="[1,[0]]",
        help="JSON value for RPC params array",
    )
    args = parser.parse_args()

    try:
        params = json.loads(args.params_json)
    except json.JSONDecodeError as exc:
        print(f"Invalid --params-json: {exc}", file=sys.stderr)
        return 2

    return run(args.host, args.port, args.method, params, args.burst_requests)


if __name__ == "__main__":
    raise SystemExit(main())
