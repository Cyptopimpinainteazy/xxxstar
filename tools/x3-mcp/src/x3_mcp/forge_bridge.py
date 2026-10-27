"""Serena-free, fail-closed MCP adapters for the existing X3 Forge tools.

All agents must mount the same X3_REPO_ROOT and persistent .x3-forge directory
to share memory. No code is executed from a query, and nothing is silently indexed.
"""
from __future__ import annotations

import hashlib
import fcntl
import importlib.util
import json
import os
import subprocess
from pathlib import Path
from typing import Any


def _module(root: Path, name: str) -> Any:
    path = root / "tools" / "x3-forge" / (name + ".py")
    if not path.is_file():
        raise RuntimeError(f"X3 Forge module missing: {path}")
    spec = importlib.util.spec_from_file_location("x3_forge_" + name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load X3 Forge module: {name}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _head(root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True,
        capture_output=True, timeout=15, check=False,
    )
    if result.returncode != 0:
        raise RuntimeError("X3 Repo Brain requires a readable Git checkout")
    return result.stdout.strip()


def _index(root: Path) -> dict[str, Any]:
    path = root / ".x3-forge" / "index.json"
    if not path.is_file():
        raise RuntimeError("X3 Forge index missing; run: python3 tools/x3-forge/index.py build")
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or not isinstance(data.get("files"), dict):
        raise RuntimeError("X3 Forge index has invalid structure; rebuild it")
    if data.get("commit") != _head(root):
        raise RuntimeError("X3 Forge index is from a different HEAD; rebuild it before retrieval")
    return data


def _checked_package(root: Path, package: dict[str, Any]) -> dict[str, Any]:
    """Reject changed/deleted files and symlinks escaping the checkout."""
    for item in package["included"]:
        rel = item["path"]
        candidate = (root / rel).resolve()
        if candidate != root and root not in candidate.parents:
            raise RuntimeError(f"indexed file escapes repository: {rel}")
        if not candidate.is_file():
            raise RuntimeError(f"indexed file missing: {rel}; rebuild the index")
        digest = hashlib.sha256(candidate.read_bytes()).hexdigest()
        if digest != item["sha256"]:
            raise RuntimeError(f"indexed file changed: {rel}; rebuild the index")
    return package


def context(root: Path, query: str, budget_tokens: int = 12000,
            max_files: int = 8) -> dict[str, Any]:
    """Find relevant symbols/files with commit + content-hash provenance."""
    root = root.resolve()
    if not query.strip():
        raise ValueError("query must not be empty")
    if not 1 <= budget_tokens <= 60000 or not 1 <= max_files <= 25:
        raise ValueError("budget_tokens must be 1..60000 and max_files 1..25")
    index = _index(root)
    compiler = _module(root, "context")
    package = compiler.build_package(index, query, budget_tokens, max_files)
    package["retrieval"] = "lexical and symbol-based; not semantic embeddings"
    return _checked_package(root, package)


def _store(root: Path) -> Path:
    """Use one operator-configured memory file across isolated agent worktrees."""
    configured = os.getenv("X3_FORGE_SHARED_MEMORY", "").strip()
    if configured:
        path = Path(configured).expanduser()
        if not path.is_absolute():
            raise ValueError("X3_FORGE_SHARED_MEMORY must be an absolute path")
        return path.resolve()
    return root.resolve() / ".x3-forge" / "memory.jsonl"


def recall(root: Path, query: str, kind: str = "", limit: int = 10) -> dict[str, Any]:
    """Retrieve agent-supplied incident notes; never represent them as proof."""
    if not query.strip():
        raise ValueError("query must not be empty")
    if not 1 <= limit <= 20:
        raise ValueError("limit must be 1..20")
    memory = _module(root.resolve(), "failure_memory")
    if kind and kind not in memory.KINDS:
        raise ValueError("unknown memory kind")
    entries = memory.load(_store(root))
    return {
        "matches": memory.search(entries, query, kind=kind or None, limit=limit),
        "total_entries": len(entries),
        "trust": "unverified agent notes; validate against source and CI",
        "retrieval": "lexical, not vector search",
    }


def remember(root: Path, kind: str, component: str, observation: str,
             resolution: str = "", regression: str = "") -> dict[str, Any]:
    """Append a durable note, only after the operator enables write access."""
    if os.getenv("X3_FORGE_ALLOW_WRITES") != "1":
        raise PermissionError("writes disabled: set X3_FORGE_ALLOW_WRITES=1 explicitly")
    if kind not in ("failure", "success", "dead_end"):
        raise ValueError("kind must be failure, success or dead_end")
    if not component.strip() or not observation.strip():
        raise ValueError("component and observation are required")
    if any(len(value) > 4000 for value in (component, observation, resolution, regression)):
        raise ValueError("memory field exceeds 4000 characters")
    root = root.resolve()
    memory = _module(root, "failure_memory")
    store = _store(root)
    store.parent.mkdir(parents=True, exist_ok=True)
    lock_path = store.with_name(store.name + ".lock")
    # Guard concurrent workers writing the same memory log on Linux.
    with lock_path.open("a+b") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        try:
            saved = memory.record(
                store,
                {"kind": kind, "component": component, "error": observation,
                 "fix": resolution, "regression": regression, "commit": _head(root)},
            )
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
    return {"saved": saved, "trust": "agent-supplied; not verified evidence"}


def register(mcp: Any, root: Path) -> None:
    """Attach tools to the existing X3 FastMCP server used by OpenHands."""

    @mcp.tool()
    def x3_brain_context(query: str, budget_tokens: int = 12000,
                         max_files: int = 8) -> dict[str, Any]:
        """Locate relevant X3 symbols and files, rejecting stale source evidence."""
        return context(root, query, budget_tokens, max_files)

    @mcp.tool()
    def x3_brain_recall(query: str, kind: str = "", limit: int = 10) -> dict[str, Any]:
        """Search previous successes, failures and dead ends (unverified notes)."""
        return recall(root, query, kind, limit)

    @mcp.tool()
    def x3_brain_remember(kind: str, component: str, observation: str,
                          resolution: str = "", regression: str = "") -> dict[str, Any]:
        """Append shared agent memory when X3_FORGE_ALLOW_WRITES=1."""
        return remember(root, kind, component, observation, resolution, regression)
