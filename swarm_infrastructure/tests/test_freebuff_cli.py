"""Tests for FreebuffCLI — subprocess wrapper for the freebuff coding agent."""

from __future__ import annotations

import os
import pytest
import time
from unittest.mock import patch, MagicMock

from swarm.integrations.freebuff_cli import (
    FreebuffCLI,
    FreebuffResult,
    resolve_freebuff_bin,
    _default_runner,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_runner(exit_code=0, stdout="ok", stderr=""):
    """Create a fake runner that returns the given values."""
    def runner(command, cwd, stdin_text=None, timeout=300.0):
        return exit_code, stdout, stderr
    return runner


# ---------------------------------------------------------------------------
# Binary resolution
# ---------------------------------------------------------------------------

class TestResolveFreebuffBin:
    """Test binary resolution with various environment states."""

    def test_env_var_takes_priority(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/custom/path/freebuff")
        with patch("os.path.isfile", return_value=True):
            result = resolve_freebuff_bin()
            assert result == "/custom/path/freebuff"

    def test_env_var_ignored_if_not_file(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/nonexistent/freebuff")
        monkeypatch.setenv("PATH", "/empty")
        # Also block the nvm fallback path
        with patch("os.path.isdir", return_value=False):
            with patch("os.path.isfile", return_value=False):
                result = resolve_freebuff_bin()
                assert result is None

    def test_path_resolution(self, monkeypatch):
        monkeypatch.delenv("FREEBUFF_BIN", raising=False)
        monkeypatch.setenv("PATH", "/fake/bin")
        with patch("os.path.isfile", side_effect=lambda p: p == "/fake/bin/freebuff"):
            with patch("os.access", return_value=True):
                result = resolve_freebuff_bin()
                assert result == "/fake/bin/freebuff"

    def test_returns_none_when_not_found(self, monkeypatch):
        monkeypatch.delenv("FREEBUFF_BIN", raising=False)
        monkeypatch.setenv("PATH", "/empty")
        with patch("os.path.isfile", return_value=False):
            result = resolve_freebuff_bin()
            assert result is None


# ---------------------------------------------------------------------------
# FreebuffResult
# ---------------------------------------------------------------------------

class TestFreebuffResult:
    """Test the result dataclass."""

    def test_defaults(self):
        result = FreebuffResult(ok=True, output="hello")
        assert result.ok is True
        assert result.output == "hello"
        assert result.conversation_id is None
        assert result.error is None
        assert result.exit_code == -1
        assert result.duration_seconds == 0.0
        assert result.timestamp > 0

    def test_with_all_fields(self):
        result = FreebuffResult(
            ok=False,
            output="",
            conversation_id="conv-123",
            error="something went wrong",
            exit_code=1,
            duration_seconds=2.5,
        )
        assert result.ok is False
        assert result.conversation_id == "conv-123"
        assert result.error == "something went wrong"
        assert result.exit_code == 1
        assert result.duration_seconds == 2.5


# ---------------------------------------------------------------------------
# FreebuffCLI construction
# ---------------------------------------------------------------------------

class TestFreebuffCLIConstruction:
    """Test CLI wrapper construction and configuration."""

    def test_raises_if_binary_not_found(self, monkeypatch):
        monkeypatch.delenv("FREEBUFF_BIN", raising=False)
        monkeypatch.setenv("PATH", "/empty")
        with patch("os.path.isfile", return_value=False):
            with pytest.raises(RuntimeError, match="freebuff binary not found"):
                FreebuffCLI(workspace_root="/tmp")

    def test_constructs_with_explicit_bin(self):
        cli = FreebuffCLI(
            workspace_root="/tmp",
            freebuff_bin="/fake/freebuff",
        )
        assert cli.freebuff_bin == "/fake/freebuff"
        assert cli.workspace_root == "/tmp"

    def test_constructs_with_env_var(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/env/freebuff")
        with patch("os.path.isfile", return_value=True):
            cli = FreebuffCLI(workspace_root="/tmp")
            assert "/env/freebuff" in cli.freebuff_bin

    def test_default_workspace_is_cwd(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/env/freebuff")
        with patch("os.path.isfile", return_value=True):
            cli = FreebuffCLI()
            assert cli.workspace_root == os.path.abspath(os.getcwd())


# ---------------------------------------------------------------------------
# Run prompt — basic
# ---------------------------------------------------------------------------

class TestRunPrompt:
    """Test run_prompt() behaviour."""

    @pytest.fixture
    def cli(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/fake/freebuff")
        with patch("os.path.isfile", return_value=True):
            return FreebuffCLI(workspace_root="/tmp/testproj")

    def test_successful_invocation(self, cli):
        cli._runner = _make_runner(
            exit_code=0,
            stdout="Conversation ID: conv-abc\n\nRefactored successfully.",
        )

        result = cli.run_prompt("Refactor auth module")
        assert result.ok is True
        assert "Refactored" in result.output
        assert result.conversation_id == "conv-abc"
        assert result.exit_code == 0
        assert result.duration_seconds >= 0.0

    def test_failed_invocation(self, cli):
        cli._runner = _make_runner(
            exit_code=1, stdout="", stderr="parse error at line 42"
        )

        result = cli.run_prompt("bad input")
        assert result.ok is False
        assert result.exit_code == 1
        assert "parse error" in (result.error or "")

    def test_continue_conversation(self, cli):
        cli._runner = _make_runner(
            exit_code=0, stdout="Continued conv-xyz: tests added.\n"
        )

        result = cli.run_prompt("Add unit tests", conversation_id="conv-xyz")
        assert result.ok is True
        assert result.conversation_id == "conv-xyz"
        assert "tests added" in result.output

    def test_timeout_handling(self, cli):
        import subprocess

        def slow_runner(command, cwd, stdin_text=None, timeout=300.0):
            raise subprocess.TimeoutExpired(cmd=command, timeout=timeout)

        cli._runner = slow_runner

        result = cli.run_prompt("do work", timeout=1.0)
        assert result.ok is False
        assert result.conversation_id is None
        assert "timed out" in (result.error or "")

    def test_generic_exception_handling(self, cli):
        def error_runner(command, cwd, stdin_text=None, timeout=300.0):
            raise OSError("disk full")

        cli._runner = error_runner

        result = cli.run_prompt("do work")
        assert result.ok is False
        assert "disk full" in (result.error or "")

    def test_result_caching(self, cli):
        call_count = [0]

        def counting_runner(command, cwd, stdin_text=None, timeout=300.0):
            call_count[0] += 1
            return 0, f"run {call_count[0]}", ""

        cli._runner = counting_runner

        r1 = cli.run_prompt("hi", cache_key="test-cache")
        r2 = cli.run_prompt("hi", cache_key="test-cache")

        assert call_count[0] == 1  # Only one real call
        assert r1.output == r2.output

    def test_cache_expiry(self, cli):
        call_count = [0]

        def counting_runner(command, cwd, stdin_text=None, timeout=300.0):
            call_count[0] += 1
            return 0, f"run {call_count[0]}", ""

        cli._runner = counting_runner
        cli.cache_ttl_s = 0  # Immediate expiry

        cli.run_prompt("hi", cache_key="ephemeral")
        cli.run_prompt("hi", cache_key="ephemeral")

        assert call_count[0] == 2  # Both calls hit the runner


# ---------------------------------------------------------------------------
# Version check
# ---------------------------------------------------------------------------

class TestGetVersion:
    """Test get_version()."""

    @pytest.fixture
    def cli(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/fake/freebuff")
        with patch("os.path.isfile", return_value=True):
            return FreebuffCLI(workspace_root="/tmp")

    def test_version_success(self, cli):
        cli._runner = _make_runner(exit_code=0, stdout="0.0.103")
        result = cli.get_version()
        assert result.ok is True
        assert result.output == "0.0.103"

    def test_version_timeout(self, cli):
        import subprocess

        def slow_runner(command, cwd, stdin_text=None, timeout=300.0):
            raise subprocess.TimeoutExpired(cmd=command, timeout=timeout)

        cli._runner = slow_runner
        result = cli.get_version()
        assert result.ok is False
        assert "timed out" in (result.error or "")


# ---------------------------------------------------------------------------
# Status & cache management
# ---------------------------------------------------------------------------

class TestStatusAndCache:
    """Test get_status() and clear_cache()."""

    @pytest.fixture
    def cli(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/fake/freebuff")
        with patch("os.path.isfile", return_value=True):
            c = FreebuffCLI(workspace_root="/tmp/testproj")
            c._runner = _make_runner(exit_code=0, stdout="0.0.103")
            return c

    def test_status_includes_all_keys(self, cli):
        status = cli.get_status()
        assert "binary" in status
        assert "workspace" in status
        assert "version" in status
        assert "version_ok" in status
        assert "conversations_cached" in status
        assert "busy" in status
        assert "cache_ttl_s" in status
        assert "default_timeout_s" in status

    def test_clear_cache(self, cli):
        cli._runner = _make_runner(exit_code=0, stdout="output")
        cli.run_prompt("a", cache_key="k1")
        cli.run_prompt("b", cache_key="k2")

        removed = cli.clear_cache()
        assert removed == 2

        status = cli.get_status()
        assert status["conversations_cached"] == 0


# ---------------------------------------------------------------------------
# Concurrency guard (single-instance enforcement)
# ---------------------------------------------------------------------------

class TestConcurrencyGuard:
    """Test that the lock prevents concurrent freebuff invocations."""

    @pytest.fixture
    def cli(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/fake/freebuff")
        with patch("os.path.isfile", return_value=True):
            return FreebuffCLI(workspace_root="/tmp")

    def test_lock_prevents_concurrent_calls(self, cli):
        # Acquire the lock externally to simulate a running invocation
        acquired = cli._lock.acquire(blocking=False)
        assert acquired is True

        cli._runner = _make_runner(exit_code=0, stdout="ok")

        result = cli.run_prompt("do work", timeout=0.1)
        assert result.ok is False
        assert "already running" in (result.error or "")
        assert result.exit_code == -1

        cli._lock.release()

    def test_lock_released_after_success(self, cli):
        cli._runner = _make_runner(exit_code=0, stdout="ok")
        cli.run_prompt("do work")

        # Lock should be released — another call should succeed
        acquired = cli._lock.acquire(blocking=False)
        assert acquired is True
        cli._lock.release()

    def test_lock_released_after_failure(self, cli):
        def error_runner(command, cwd, stdin_text=None, timeout=300.0):
            raise ValueError("boom")

        cli._runner = error_runner
        cli.run_prompt("do work")

        # Lock should still be released after an exception
        acquired = cli._lock.acquire(blocking=False)
        assert acquired is True
        cli._lock.release()


# ---------------------------------------------------------------------------
# Conversation ID extraction
# ---------------------------------------------------------------------------

class TestConversationIdExtraction:
    """Test the internal conversation ID extraction logic."""

    @pytest.fixture
    def cli(self, monkeypatch):
        monkeypatch.setenv("FREEBUFF_BIN", "/fake/freebuff")
        with patch("os.path.isfile", return_value=True):
            return FreebuffCLI(workspace_root="/tmp")

    def test_extracts_standard_format(self, cli):
        cid = cli._extract_conversation_id(
            "Conversation ID: abc-123-def\n\nSome other text"
        )
        assert cid == "abc-123-def"

    def test_extracts_lowercase_format(self, cli):
        cid = cli._extract_conversation_id(
            "conversation_id: xyz-789\nOutput..."
        )
        assert cid == "xyz-789"

    def test_returns_none_for_empty_output(self, cli):
        assert cli._extract_conversation_id("") is None
        assert cli._extract_conversation_id("Hello World") is None
