"""X3 MCP server.

Read-only by default. Tools return explicit evidence and fail closed when the
node or repository cannot prove a requested fact.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

import httpx
from mcp.server.fastmcp import FastMCP

mcp = FastMCP("x3")

REPO_ROOT = Path(os.environ.get("X3_REPO_ROOT", Path(__file__).resolve().parents[3])).resolve()
RPC_URL = os.environ.get("X3_RPC_URL", "http://127.0.0.1:9944")
COMMAND_TIMEOUT = int(os.environ.get("X3_MCP_COMMAND_TIMEOUT", "900"))


def _rpc(method: str, params: list[Any] | None = None) -> Any:
    payload = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []}
    try:
        response = httpx.post(RPC_URL, json=payload, timeout=15.0)
        response.raise_for_status()
        body = response.json()
    except Exception as exc:
        raise RuntimeError(f"X3 RPC unavailable at {RPC_URL}: {exc}") from exc
    if "error" in body:
        raise RuntimeError(f"X3 RPC {method} failed: {body['error']}")
    if "result" not in body:
        raise RuntimeError(f"X3 RPC {method} returned no result")
    return body["result"]


def _run(argv: list[str], timeout: int = COMMAND_TIMEOUT) -> dict[str, Any]:
    """Run an allow-listed local command without invoking a shell."""
    proc = subprocess.run(
        argv,
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )
    return {
        "argv": argv,
        "exit_code": proc.returncode,
        "stdout": proc.stdout[-20000:],
        "stderr": proc.stderr[-20000:],
        "ok": proc.returncode == 0,
    }


@mcp.tool()
def x3_chain_health() -> dict[str, Any]:
    """Return live node health plus finalized and best heads."""
    health = _rpc("system_health")
    finalized_hash = _rpc("chain_getFinalizedHead")
    finalized_header = _rpc("chain_getHeader", [finalized_hash])
    best_header = _rpc("chain_getHeader")
    return {
        "rpc_url": RPC_URL,
        "health": health,
        "finalized_hash": finalized_hash,
        "finalized_header": finalized_header,
        "best_header": best_header,
    }


@mcp.tool()
def x3_get_finalized_head() -> dict[str, Any]:
    """Return the node's finalized block hash and header."""
    block_hash = _rpc("chain_getFinalizedHead")
    header = _rpc("chain_getHeader", [block_hash])
    return {"hash": block_hash, "header": header}


@mcp.tool()
def x3_inspect_block(block_hash: str) -> dict[str, Any]:
    """Fetch one exact block and header by hash. Never substitutes best head."""
    if not block_hash.startswith("0x") or len(block_hash) != 66:
        raise ValueError("block_hash must be a 32-byte 0x-prefixed hash")
    block = _rpc("chain_getBlock", [block_hash])
    header = _rpc("chain_getHeader", [block_hash])
    if block is None or header is None:
        raise RuntimeError(f"block not found: {block_hash}")
    return {"hash": block_hash, "header": header, "block": block}


@mcp.tool()
def x3_rpc(method: str, params_json: str = "[]") -> dict[str, Any]:
    """Call a read-only X3 JSON-RPC method from a strict allowlist."""
    allowed = {
        "system_health",
        "system_chain",
        "system_name",
        "system_version",
        "chain_getFinalizedHead",
        "chain_getHeader",
        "chain_getBlock",
        "chain_getBlockHash",
        "state_getRuntimeVersion",
        "state_getMetadata",
        "state_getStorage",
    }
    if method not in allowed:
        raise ValueError(f"RPC method is not allow-listed: {method}")
    params = json.loads(params_json)
    if not isinstance(params, list):
        raise ValueError("params_json must decode to a JSON array")
    return {"method": method, "result": _rpc(method, params)}


@mcp.tool()
def x3_repo_status() -> dict[str, Any]:
    """Return exact local Git branch, HEAD and working-tree status."""
    return {
        "root": str(REPO_ROOT),
        "branch": _run(["git", "branch", "--show-current"], 30),
        "head": _run(["git", "rev-parse", "HEAD"], 30),
        "status": _run(["git", "status", "--short"], 30),
    }


@mcp.tool()
def x3_run_readiness_gate(profile: str = "focused") -> dict[str, Any]:
    """Run repository verification without weakening or skipping gates.

    focused: cargo check --workspace
    tests: cargo test --workspace
    local-ci: scripts/local-ci.sh
    """
    commands = {
        "focused": ["cargo", "check", "--workspace"],
        "tests": ["cargo", "test", "--workspace"],
        "local-ci": ["bash", "scripts/local-ci.sh"],
    }
    if profile not in commands:
        raise ValueError(f"unknown profile {profile!r}; choose {sorted(commands)}")
    result = _run(commands[profile])
    result["profile"] = profile
    return result


@mcp.tool()
def x3_run_atomic_tests() -> dict[str, Any]:
    """Run the real x3-atomic-swap crate tests."""
    return _run(["cargo", "test", "-p", "x3-atomic-swap", "--features", "std"])


@mcp.tool()
def x3_collect_evidence() -> dict[str, Any]:
    """Collect reproducible local repository and live-node evidence."""
    evidence: dict[str, Any] = {"repo": x3_repo_status()}
    try:
        evidence["chain"] = x3_chain_health()
    except Exception as exc:
        evidence["chain"] = {"ok": False, "error": str(exc)}
    return evidence


def main() -> None:
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
