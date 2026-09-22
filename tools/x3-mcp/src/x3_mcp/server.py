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
PROOF_LEDGER = Path(os.environ.get("X3_PROOF_LEDGER", REPO_ROOT / "audit-artifacts" / "x3vm-proof-ledger.json")).resolve()


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
        "state_getKeysPaged",
        "state_queryStorageAt",
        "rpc_methods",
        "grandpa_roundState",
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


def _inside_repo(path: Path) -> Path:
    resolved = path.resolve()
    if resolved != REPO_ROOT and REPO_ROOT not in resolved.parents:
        raise ValueError(f"path escapes X3 repository: {resolved}")
    return resolved


def _read_json(path: Path) -> Any:
    path = _inside_repo(path)
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise RuntimeError(f"evidence file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"invalid JSON evidence in {path}: {exc}") from exc


def _intent_records(ledger: dict[str, Any], intent_id: int) -> list[dict[str, Any]]:
    records = ledger.get("records")
    if not isinstance(records, list):
        raise RuntimeError("proof ledger has no records array")
    return [r for r in records if isinstance(r, dict) and r.get("intent_id") == intent_id]


@mcp.tool()
def x3_trace_atomic_swap(intent_id: int, ledger_path: str = "") -> dict[str, Any]:
    """Trace one intent from the durable X3 atomic-swap proof ledger.

    This does not infer missing lifecycle steps. It returns only persisted
    records/entries and reports missing success/refund evidence explicitly.
    """
    if intent_id < 0:
        raise ValueError("intent_id must be non-negative")
    path = Path(ledger_path) if ledger_path else PROOF_LEDGER
    ledger = _read_json(path)
    records = _intent_records(ledger, intent_id)
    if not records:
        raise RuntimeError(f"no persisted proof records for intent {intent_id}")

    entries = [e for r in records for e in r.get("entries", []) if isinstance(e, dict)]
    kinds = {
        str(e.get("proof_kind"))
        for e in entries
        if e.get("verified") is True and e.get("proof_kind") is not None
    }
    # serde's externally visible enum names are intentionally accepted as
    # evidence labels; no absent step is synthesized.
    success_required = {
        "SourceLock", "DestinationLock", "HashlockMatch", "TimeoutOrderValid",
        "FinalityVerified", "SecretReveal", "Claim", "Score",
    }
    refund_required = {
        "SourceLock", "DestinationLock", "TimeoutOrderValid", "Refund", "Score",
    }
    return {
        "intent_id": intent_id,
        "ledger_path": str(_inside_repo(path)),
        "records": records,
        "verified_kinds": sorted(kinds),
        "success_missing": sorted(success_required - kinds),
        "refund_missing": sorted(refund_required - kinds),
        "ledger_final_status": ledger.get("final_status"),
    }


@mcp.tool()
def x3_verify_proof(intent_id: int, proof_kind: str, ledger_path: str = "") -> dict[str, Any]:
    """Verify that exact persisted proof evidence exists for an intent.

    The MCP server does not cryptographically bless arbitrary bytes. It checks
    the durable ledger written by X3 and requires a verified entry with a tx
    hash, block number and non-empty raw proof data.
    """
    if intent_id < 0:
        raise ValueError("intent_id must be non-negative")
    path = Path(ledger_path) if ledger_path else PROOF_LEDGER
    ledger = _read_json(path)
    records = _intent_records(ledger, intent_id)
    matches = []
    for record in records:
        for entry in record.get("entries", []):
            if not isinstance(entry, dict) or str(entry.get("proof_kind")) != proof_kind:
                continue
            complete = (
                entry.get("verified") is True
                and bool(entry.get("tx_hash"))
                and entry.get("block_number") is not None
                and bool(entry.get("data"))
            )
            matches.append({"complete": complete, "entry": entry})
    if not matches:
        raise RuntimeError(f"no {proof_kind} proof for intent {intent_id}")
    if not any(m["complete"] for m in matches):
        raise RuntimeError(f"{proof_kind} evidence for intent {intent_id} is incomplete or unverified")
    return {
        "intent_id": intent_id,
        "proof_kind": proof_kind,
        "verified": True,
        "evidence": [m["entry"] for m in matches if m["complete"]],
    }


@mcp.tool()
def x3_verify_receipt(receipt_path: str, trusted_key_specs: str = "") -> dict[str, Any]:
    """Verify an X3Lang receipt, optionally requiring trusted signer attestation.

    trusted_key_specs is a comma-separated list of KEY_ID=64_HEX_PUBLIC_KEY.
    When supplied, x3c verifies the receipt against the caller's trust map; the
    public key embedded in the receipt is never sufficient by itself.
    """
    path = _inside_repo(Path(receipt_path))
    if not path.is_file():
        raise RuntimeError(f"receipt not found: {path}")
    manifest = REPO_ROOT / "x3-lang" / "Cargo.toml"
    argv = [
        "cargo", "run", "--quiet", "--manifest-path", str(manifest),
        "-p", "x3-tools", "--bin", "x3c", "--", "receipt", "verify", str(path),
    ]
    trusted = [spec.strip() for spec in trusted_key_specs.split(",") if spec.strip()]
    for spec in trusted:
        argv.extend(["--trusted", spec])
    result = _run(argv)
    return {
        "receipt_path": str(path),
        "verified": result["ok"],
        "trusted_attestation_required": bool(trusted),
        "scope": (
            "receipt hash + economic invariants + trusted signer attestation"
            if trusted else
            "receipt hash + structural/economic invariants; signer trust not requested"
        ),
        "command": result,
    }


def _hex32(value: str, name: str) -> str:
    if not value.startswith("0x") or len(value) != 66:
        raise ValueError(f"{name} must be a 32-byte 0x-prefixed hash")
    int(value[2:], 16)
    return value.lower()


def _storage_prefix(pallet: str, item: str) -> str:
    # Substrate storage prefixes are TwoX-128(pallet) ++ TwoX-128(item).
    # Python stdlib has no xxhash; ask the live node for metadata-driven keys
    # through state_getKeysPaged only after the exact prefix is supplied by the
    # caller or generated by a repository helper. Never substitute a guessed key.
    raise RuntimeError(
        f"storage-key derivation for {pallet}.{item} is not available in the "
        "Python MCP process; refusing to guess. Use runtime-backed RPC below."
    )


@mcp.tool()
def x3_inspect_intent(intent_id: str) -> dict[str, Any]:
    """Inspect a settlement intent through the node's runtime RPC surface.

    The current runtime declares GovernanceSettlementApi but does not wire a
    custom JSON-RPC endpoint for get_settlement. Until that is exposed, return
    live capability evidence and fail closed rather than derive SCALE storage
    keys incorrectly.
    """
    intent_id = _hex32(intent_id, "intent_id")
    methods = _rpc("rpc_methods")
    available = methods.get("methods", []) if isinstance(methods, dict) else []
    candidates = [
        m for m in available
        if "settlement" in m.lower() or "intent" in m.lower()
    ]
    if not candidates:
        raise RuntimeError(
            "live node exposes no settlement/intent JSON-RPC method; "
            "GovernanceSettlementApi is declared in the pallet but is not "
            "currently surfaced through node RPC"
        )
    return {"intent_id": intent_id, "available_runtime_methods": sorted(candidates)}


@mcp.tool()
def x3_atomic_state(intent_id: str) -> dict[str, Any]:
    """Return live atomic-settlement capability evidence for one intent.

    Refuses to report escrow/intent state until the runtime API is reachable
    from node RPC; this prevents guessed storage keys or stale ledger evidence
    from being mislabeled as live chain state.
    """
    intent_id = _hex32(intent_id, "intent_id")
    health = x3_chain_health()
    try:
        intent = x3_inspect_intent(intent_id)
    except Exception as exc:
        return {
            "intent_id": intent_id,
            "live_chain": health,
            "state_available": False,
            "error": str(exc),
        }
    return {
        "intent_id": intent_id,
        "live_chain": health,
        "state_available": True,
        "intent": intent,
    }


@mcp.tool()
def x3_validator_status() -> dict[str, Any]:
    """Return live validator/node health and GRANDPA capability evidence."""
    health = x3_chain_health()
    methods = _rpc("rpc_methods")
    available = methods.get("methods", []) if isinstance(methods, dict) else []
    grandpa = sorted(m for m in available if "grandpa" in m.lower())
    author = sorted(m for m in available if "author" in m.lower())
    return {
        "chain": health,
        "grandpa_methods": grandpa,
        "author_methods": author,
        "is_syncing": health["health"].get("isSyncing") if isinstance(health.get("health"), dict) else None,
        "peers": health["health"].get("peers") if isinstance(health.get("health"), dict) else None,
        "should_have_peers": health["health"].get("shouldHavePeers") if isinstance(health.get("health"), dict) else None,
    }


@mcp.tool()
def x3_consensus_status() -> dict[str, Any]:
    """Inspect finalized/best heads and GRANDPA round state when exposed."""
    health = x3_chain_health()
    methods = _rpc("rpc_methods")
    available = methods.get("methods", []) if isinstance(methods, dict) else []
    result: dict[str, Any] = {
        "best_header": health["best_header"],
        "finalized_hash": health["finalized_hash"],
        "finalized_header": health["finalized_header"],
        "grandpa_round_state_available": "grandpa_roundState" in available,
    }
    if "grandpa_roundState" in available:
        result["grandpa_round_state"] = _rpc("grandpa_roundState")
    return result


@mcp.tool()
def x3_run_failure_matrix(profile: str = "validator") -> dict[str, Any]:
    """Run an existing repository failure/adversarial gate.

    validator: scripts/local-ci.sh --failure
    atomic: x3-atomic-swap test suite
    """
    commands = {
        "validator": ["bash", "scripts/local-ci.sh", "--failure"],
        "atomic": ["cargo", "test", "-p", "x3-atomic-swap", "--features", "std"],
    }
    if profile not in commands:
        raise ValueError(f"unknown failure profile {profile!r}; choose {sorted(commands)}")
    result = _run(commands[profile])
    result["profile"] = profile
    result["fail_closed"] = True
    return result


def main() -> None:
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
