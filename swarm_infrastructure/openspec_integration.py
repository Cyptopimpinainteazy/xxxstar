"""OpenSpec integration helpers for swarm (Orchestra) operations."""

from __future__ import annotations

import os
import subprocess
import time
from dataclasses import dataclass
from typing import Callable, Dict, Optional, Tuple


@dataclass
class OpenSpecValidationResult:
    change_id: str
    ok: bool
    output: str
    timestamp: float


def resolve_openspec_bin() -> Optional[str]:
    """Resolve the OpenSpec CLI path using OPENSPEC_BIN or PATH."""
    env_bin = os.getenv("OPENSPEC_BIN")
    if env_bin:
        return env_bin

    for path_dir in os.getenv("PATH", "").split(os.pathsep):
        candidate = os.path.join(path_dir, "openspec")
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate

    return None


def resolve_workspace_root(start_path: Optional[str] = None) -> str:
    """Resolve the workspace root by walking up to find .git or openspec/."""
    current = os.path.abspath(start_path or os.getcwd())
    while True:
        if os.path.isdir(os.path.join(current, ".git")):
            return current
        if os.path.isdir(os.path.join(current, "openspec")):
            return current
        parent = os.path.dirname(current)
        if parent == current:
            return os.path.abspath(start_path or os.getcwd())
        current = parent


def _default_runner(command: list[str], cwd: str) -> Tuple[int, str]:
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    output = (completed.stdout or "") + (completed.stderr or "")
    return completed.returncode, output.strip()


class OpenSpecValidator:
    def __init__(
        self,
        openspec_bin: Optional[str] = None,
        workspace_root: Optional[str] = None,
        cache_ttl_s: int = 300,
        runner: Optional[Callable[[list[str], str], Tuple[int, str]]] = None,
    ) -> None:
        self.openspec_bin = openspec_bin or resolve_openspec_bin()
        self.workspace_root = workspace_root or resolve_workspace_root()
        self.cache_ttl_s = cache_ttl_s
        self._runner = runner or _default_runner
        self._cache: Dict[str, OpenSpecValidationResult] = {}

    def validate_change(self, change_id: str) -> OpenSpecValidationResult:
        if not change_id:
            return OpenSpecValidationResult(change_id=change_id, ok=False, output="missing change_id", timestamp=time.time())

        cached = self._cache.get(change_id)
        if cached and (time.time() - cached.timestamp) < self.cache_ttl_s:
            return cached

        if not self.openspec_bin:
            result = OpenSpecValidationResult(
                change_id=change_id,
                ok=False,
                output="openspec binary not found",
                timestamp=time.time(),
            )
            self._cache[change_id] = result
            return result

        command = [self.openspec_bin, "validate", change_id, "--strict"]
        code, output = self._runner(command, self.workspace_root)
        result = OpenSpecValidationResult(
            change_id=change_id,
            ok=code == 0,
            output=output,
            timestamp=time.time(),
        )
        self._cache[change_id] = result
        return result

    def get_status(self, change_id: str) -> Optional[OpenSpecValidationResult]:
        return self._cache.get(change_id)


def create_change_skeleton(
    change_id: str,
    capability: str,
    workspace_root: Optional[str] = None,
) -> Dict[str, str]:
    """Create a minimal OpenSpec change skeleton on disk.

    Returns a mapping of artifact names to file paths.
    """
    root = workspace_root or resolve_workspace_root()
    change_root = os.path.join(root, "openspec", "changes", change_id)
    specs_root = os.path.join(change_root, "specs", capability)

    os.makedirs(specs_root, exist_ok=True)

    proposal_path = os.path.join(change_root, "proposal.md")
    tasks_path = os.path.join(change_root, "tasks.md")
    design_path = os.path.join(change_root, "design.md")
    spec_path = os.path.join(specs_root, "spec.md")

    if not os.path.exists(proposal_path):
        with open(proposal_path, "w", encoding="utf-8") as f:
            f.write(
                f"# Change: {change_id}\n\n"
                "## Why\n"
                "Describe the motivation and problem statement.\n\n"
                "## What Changes\n"
                "- Describe the intended changes.\n\n"
                "## Impact\n"
                f"- Affected specs: `specs/{capability}/spec.md`\n"
            )

    if not os.path.exists(tasks_path):
        with open(tasks_path, "w", encoding="utf-8") as f:
            f.write(
                "## 1. Implementation\n"
                "- [ ] 1.1 Describe task\n"
            )

    if not os.path.exists(design_path):
        with open(design_path, "w", encoding="utf-8") as f:
            f.write(
                "## Context\n"
                "Describe relevant system context and constraints.\n\n"
                "## Goals / Non-Goals\n"
                "- Goals:\n"
                "  - ...\n"
                "- Non-Goals:\n"
                "  - ...\n"
            )

    if not os.path.exists(spec_path):
        with open(spec_path, "w", encoding="utf-8") as f:
            f.write(
                "## ADDED Requirements\n"
                "### Requirement: Placeholder\n"
                "The system SHALL ...\n\n"
                "#### Scenario: Placeholder scenario\n"
                "- **WHEN** ...\n"
                "- **THEN** ...\n"
            )

    return {
        "proposal": proposal_path,
        "tasks": tasks_path,
        "design": design_path,
        "spec": spec_path,
    }
