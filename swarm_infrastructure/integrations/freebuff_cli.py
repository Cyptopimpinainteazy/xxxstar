"""Freebuff CLI Integration — subprocess wrapper for the freebuff coding agent.

This module wraps the freebuff CLI as a callable service for the X3 swarm.
It follows the same pattern as ``swarm/openspec_integration.py``:

*   Binary resolution from $FREEBUFF_BIN, $PATH, or well-known locations.
*   Dataclass result objects with status, output, and timing.
*   Optional in-memory cache with TTL.
*   Timeout-gated subprocess execution.
*   Conversation management (start new / continue existing).

Constraints:
    Only one freebuff (or codebuff) instance can run at a time.  The
    wrapper enforces this via an internal lock so callers don't
    accidentally spawn overlapping sessions.

Usage::

    from swarm.integrations.freebuff_cli import FreebuffCLI

    cli = FreebuffCLI(workspace_root="/path/to/repo")
    result = cli.run_prompt("Refactor the authentication module")
    if result.ok:
        print(result.output)

    # Continue a previous conversation
    result2 = cli.run_prompt("Now add unit tests", conversation_id=result.conversation_id)
"""

from __future__ import annotations

import os
import subprocess
import threading
import time
from dataclasses import dataclass, field
from typing import Callable, Dict, Optional, Tuple


# ---------------------------------------------------------------------------
# Binary resolution
# ---------------------------------------------------------------------------

def resolve_freebuff_bin() -> Optional[str]:
    """Resolve the freebuff CLI path.

    Checks (in order):
    1. ``FREEBUFF_BIN`` environment variable
    2. ``freebuff`` on ``$PATH``
    3. Well-known nvm global install path

    Returns the absolute path to the freebuff binary, or None.
    """
    env_bin = os.getenv("FREEBUFF_BIN")
    if env_bin and os.path.isfile(env_bin):
        return env_bin

    for path_dir in os.getenv("PATH", "").split(os.pathsep):
        candidate = os.path.join(path_dir, "freebuff")
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate

    # Well-known nvm global install paths (iterate versions)
    home = os.path.expanduser("~")
    nvm_versions = os.path.join(home, ".nvm", "versions", "node")
    if os.path.isdir(nvm_versions):
        for version_dir in sorted(os.listdir(nvm_versions), reverse=True):
            candidate = os.path.join(nvm_versions, version_dir, "bin", "freebuff")
            if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
                return candidate

    return None


# ---------------------------------------------------------------------------
# Result & config
# ---------------------------------------------------------------------------

@dataclass
class FreebuffResult:
    """Result of a single freebuff CLI invocation."""

    ok: bool
    output: str
    conversation_id: Optional[str] = None
    error: Optional[str] = None
    exit_code: int = -1
    duration_seconds: float = 0.0
    timestamp: float = field(default_factory=time.time)


# ---------------------------------------------------------------------------
# Subprocess runner (pluggable for testing)
# ---------------------------------------------------------------------------

def _default_runner(
    command: list[str],
    cwd: str,
    stdin_text: Optional[str] = None,
    timeout: float = 300.0,
) -> Tuple[int, str, str]:
    """Run a subprocess and return (exit_code, stdout, stderr)."""
    completed = subprocess.run(
        command,
        cwd=cwd,
        input=stdin_text,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    return completed.returncode, completed.stdout or "", completed.stderr or ""


# ---------------------------------------------------------------------------
# FreebuffCLI
# ---------------------------------------------------------------------------

class FreebuffCLI:
    """High-level wrapper around the freebuff CLI.

    Args:
        workspace_root: The repo/project directory to run freebuff within.
        freebuff_bin: Path to the freebuff binary (auto-resolved if None).
        cache_ttl_s: TTL in seconds for the in-memory result cache.
        default_timeout_s: Default subprocess timeout in seconds.
        runner: Pluggable subprocess runner (for testing).

    The wrapper is thread-safe: only one invocation runs at a time
    (freebuff enforces single-instance semantics).

    Conversation management:
        Each ``run_prompt()`` returns a ``FreebuffResult`` with a
        ``conversation_id``.  Pass this ID to the next call to continue
        the same conversation thread.
    """

    def __init__(
        self,
        workspace_root: Optional[str] = None,
        freebuff_bin: Optional[str] = None,
        cache_ttl_s: int = 300,
        default_timeout_s: float = 300.0,
        runner: Optional[
            Callable[[list[str], str, Optional[str], float], Tuple[int, str, str]]
        ] = None,
    ) -> None:
        self.workspace_root = os.path.abspath(
            workspace_root or os.getcwd()
        )
        self.freebuff_bin = freebuff_bin or resolve_freebuff_bin()
        self.cache_ttl_s = cache_ttl_s
        self.default_timeout_s = default_timeout_s
        self._runner = runner or _default_runner

        self._cache: Dict[str, FreebuffResult] = {}
        self._lock = threading.Lock()  # Single-instance guard

        if not self.freebuff_bin:
            raise RuntimeError(
                "freebuff binary not found. Set $FREEBUFF_BIN or ensure "
                "'freebuff' is on $PATH."
            )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def run_prompt(
        self,
        prompt: str,
        conversation_id: Optional[str] = None,
        timeout: Optional[float] = None,
        cache_key: Optional[str] = None,
    ) -> FreebuffResult:
        """Send a prompt to the freebuff CLI and collect the response.

        Args:
            prompt: The coding prompt to send.
            conversation_id: Continue an existing conversation.
            timeout: Subprocess timeout (defaults to ``default_timeout_s``).
            cache_key: If provided, cache the result and return cached
                       value on subsequent calls with the same key.

        Returns:
            A ``FreebuffResult`` with the CLI output.
        """
        # Check cache
        if cache_key:
            cached = self._cache.get(cache_key)
            if cached and (time.time() - cached.timestamp) < self.cache_ttl_s:
                return cached

        start = time.monotonic()
        command = [self.freebuff_bin, "--cwd", self.workspace_root]

        if conversation_id:
            command.extend(["--continue", conversation_id])

        timeout_s = timeout or self.default_timeout_s

        # Acquire the instance lock — only one freebuff process at a time
        acquired = self._lock.acquire(timeout=min(timeout_s + 3.0, 30.0))
        if not acquired:
            return FreebuffResult(
                ok=False,
                output="",
                conversation_id=conversation_id,
                error="Another freebuff invocation is already running",
                exit_code=-1,
                duration_seconds=time.monotonic() - start,
            )

        try:
            exit_code, stdout, stderr = self._runner(
                command,
                cwd=self.workspace_root,
                stdin_text=prompt,
                timeout=timeout_s,
            )

            ok = exit_code == 0
            output = stdout.strip()
            error = stderr.strip() if stderr else None
            cid = conversation_id

            # If this was a new conversation, try to extract the conversation ID
            # from the output or a sidecar file
            if not cid and ok:
                cid = self._extract_conversation_id(output)

            result = FreebuffResult(
                ok=ok,
                output=output,
                conversation_id=cid,
                error=error,
                exit_code=exit_code,
                duration_seconds=round(time.monotonic() - start, 3),
            )

            # Store in cache
            if cache_key:
                self._cache[cache_key] = result

            return result

        except subprocess.TimeoutExpired:
            duration = round(time.monotonic() - start, 3)
            return FreebuffResult(
                ok=False,
                output="",
                conversation_id=conversation_id,
                error=f"freebuff timed out after {timeout_s}s",
                exit_code=-1,
                duration_seconds=duration,
            )
        except Exception as exc:
            duration = round(time.monotonic() - start, 3)
            return FreebuffResult(
                ok=False,
                output="",
                conversation_id=conversation_id,
                error=str(exc),
                exit_code=-1,
                duration_seconds=duration,
            )
        finally:
            self._lock.release()

    def get_version(self) -> FreebuffResult:
        """Check the freebuff version and health.

        Returns a ``FreebuffResult`` with the version string.
        """
        command = [self.freebuff_bin, "--version"]

        start = time.monotonic()
        try:
            exit_code, stdout, stderr = self._runner(
                command,
                cwd=self.workspace_root,
                timeout=15.0,
            )
        except subprocess.TimeoutExpired:
            return FreebuffResult(
                ok=False,
                output="",
                error="freebuff --version timed out",
                exit_code=-1,
                duration_seconds=round(time.monotonic() - start, 3),
            )
        except Exception as exc:
            return FreebuffResult(
                ok=False,
                output="",
                error=str(exc),
                exit_code=-1,
                duration_seconds=round(time.monotonic() - start, 3),
            )

        return FreebuffResult(
            ok=exit_code == 0,
            output=stdout.strip(),
            error=stderr.strip() if stderr else None,
            exit_code=exit_code,
            duration_seconds=round(time.monotonic() - start, 3),
        )

    def get_status(self) -> Dict[str, object]:
        """Return current CLI wrapper status.

        Includes:
        * ``binary`` — resolved freebuff path
        * ``workspace`` — current working directory
        * ``version`` — freebuff version (cached for 60s)
        * ``conversations_cached`` — number of cached results
        * ``busy`` — whether the CLI is currently executing
        """
        # Version is cached separately with a short TTL
        version_result = self.get_version()

        return {
            "binary": self.freebuff_bin,
            "workspace": self.workspace_root,
            "version": version_result.output if version_result.ok else None,
            "version_ok": version_result.ok,
            "conversations_cached": len(self._cache),
            "busy": self._lock.locked(),
            "cache_ttl_s": self.cache_ttl_s,
            "default_timeout_s": self.default_timeout_s,
        }

    def clear_cache(self) -> int:
        """Clear the in-memory result cache.  Returns number of entries removed."""
        count = len(self._cache)
        self._cache.clear()
        return count

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _extract_conversation_id(self, output: str) -> Optional[str]:
        """Try to extract a conversation ID from the freebuff output.

        Freebuff may emit a conversation ID in the output or save it to
        a sidecar file.  This is a best-effort extraction.
        """
        if not output:
            return None

        # Look for known patterns in freebuff output
        for line in output.splitlines():
            line = line.strip()

            # Pattern: "Conversation ID: <id>" or "conversation: <id>"
            for prefix in ("Conversation ID:", "conversation_id:", "Conversation:"):
                if line.lower().startswith(prefix.lower()):
                    cid = line[len(prefix):].strip()
                    if cid:
                        return cid

        return None
