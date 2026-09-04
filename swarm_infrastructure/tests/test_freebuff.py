"""Tests for FreebuffAgent — GPU buffer/memory management + CLI integration."""

import pytest
from unittest.mock import MagicMock

from swarm.agents.freebuff import (
    FreebuffAgent,
    FreebuffAgentConfig,
    BufferRecord,
    GpuMemorySnapshot,
)
from swarm.core.enums import Domain, Outcome
from swarm.core.agent import Consequence
from swarm.integrations.freebuff_cli import FreebuffCLI, FreebuffResult
from swarm.storage.backend import SqliteStorage


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def storage():
    """In-memory SQLite storage that works with SelfModelLedger."""
    return SqliteStorage(":memory:")


@pytest.fixture
def freebuff_config():
    return FreebuffAgentConfig(
        agent_id="freebuff-001",
        initial_budget=2000.0,
        idle_buffer_ttl_epochs=5,
        max_reclaim_per_epoch=10,
        fragmentation_rebalance_threshold=0.6,
        target_gpu_nodes=["gpu-0", "gpu-1"],
    )


@pytest.fixture
def freebuff_agent(freebuff_config, storage):
    """Create a FreebuffAgent with real in-memory storage."""
    agent = FreebuffAgent(
        config=freebuff_config,
        storage=storage,
    )
    return agent


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

class TestFreebuffAgentConfig:
    """Test FreebuffAgentConfig defaults and overrides."""

    def test_defaults(self):
        config = FreebuffAgentConfig()
        assert config.domain == Domain.FREE_BUFF
        assert config.initial_budget == 2000.0
        assert config.idle_buffer_ttl_epochs == 30
        assert config.max_reclaim_per_epoch == 50
        assert config.fragmentation_rebalance_threshold == 0.7
        assert config.target_gpu_nodes == []

    def test_cli_config_defaults(self):
        config = FreebuffAgentConfig()
        assert config.cli_enabled is True
        assert config.cli_timeout_s == 120.0
        assert config.cli_throttle_epochs == 1
        assert config.cli_max_prompt_length == 4096

    def test_default_mandates(self):
        config = FreebuffAgentConfig()
        assert "reclaim_idle_buffers" in config.initial_mandates
        assert "defragment_vram" in config.initial_mandates
        assert "predict_memory_pressure" in config.initial_mandates
        assert "dispatch_coding_tasks" in config.initial_mandates

    def test_custom_config(self):
        config = FreebuffAgentConfig(
            agent_id="fb-1",
            initial_budget=5000.0,
            idle_buffer_ttl_epochs=10,
            max_reclaim_per_epoch=200,
            fragmentation_rebalance_threshold=0.85,
            target_gpu_nodes=["node-a"],
            initial_mandates=["custom_mandate"],
        )
        assert config.agent_id == "fb-1"
        assert config.initial_budget == 5000.0
        assert config.idle_buffer_ttl_epochs == 10
        assert config.max_reclaim_per_epoch == 200
        assert config.fragmentation_rebalance_threshold == 0.85
        assert config.target_gpu_nodes == ["node-a"]
        assert config.initial_mandates == ["custom_mandate"]


# ---------------------------------------------------------------------------
# Agent creation
# ---------------------------------------------------------------------------

class TestFreebuffAgentCreation:
    """Test FreebuffAgent instantiation and properties."""

    def test_created_alive(self, freebuff_agent):
        assert freebuff_agent.is_alive is True
        assert freebuff_agent.agent_id == "freebuff-001"
        assert freebuff_agent.specialization == "freebuff"

    def test_domain_is_freebuff(self, freebuff_agent):
        assert freebuff_agent._config.domain == Domain.FREE_BUFF

    def test_empty_buffer_registry_at_birth(self, freebuff_agent):
        assert freebuff_agent.buffer_registry == {}
        assert freebuff_agent.gpu_snapshots == {}

    def test_resource_budget_initialized(self, freebuff_agent):
        assert freebuff_agent.resource_budget == 2000.0

    def test_coding_queue_empty_at_birth(self, freebuff_agent):
        assert freebuff_agent.coding_queue_depth == 0
        assert freebuff_agent.active_conversation_id is None
        assert freebuff_agent.cli_results == []

    def test_agent_with_cli_injected(self, freebuff_config, storage):
        """Agent accepts an optional FreebuffCLI instance."""
        mock_cli = MagicMock(spec=FreebuffCLI)
        agent = FreebuffAgent(
            config=freebuff_config,
            storage=storage,
            freebuff_cli=mock_cli,
        )
        assert agent._cli is mock_cli


# ---------------------------------------------------------------------------
# Buffer management
# ---------------------------------------------------------------------------

class TestBufferOperations:
    """Test buffer registration, touch, and tracking."""

    def test_register_buffer(self, freebuff_agent):
        buf = BufferRecord(
            buffer_id="buf-001",
            gpu_node_id="gpu-0",
            size_bytes=1024 * 1024 * 100,  # 100 MB
            allocated_at=10.0,
            last_accessed_at=10.0,
        )
        freebuff_agent.register_buffer(buf)
        assert "buf-001" in freebuff_agent.buffer_registry
        assert freebuff_agent.buffer_registry["buf-001"].size_bytes == 104857600

    def test_register_multiple_buffers(self, freebuff_agent):
        for i in range(5):
            buf = BufferRecord(
                buffer_id=f"buf-{i:03d}",
                gpu_node_id="gpu-0",
                size_bytes=1024,
                allocated_at=float(i),
                last_accessed_at=float(i),
            )
            freebuff_agent.register_buffer(buf)
        assert len(freebuff_agent.buffer_registry) == 5

    def test_touch_buffer_updates_access_time(self, freebuff_agent):
        buf = BufferRecord(
            buffer_id="buf-001",
            gpu_node_id="gpu-0",
            size_bytes=1024,
            allocated_at=5.0,
            last_accessed_at=5.0,
        )
        freebuff_agent.register_buffer(buf)
        result = freebuff_agent.touch_buffer("buf-001", epoch=20)
        assert result is True
        assert freebuff_agent.buffer_registry["buf-001"].last_accessed_at == 20.0

    def test_touch_nonexistent_buffer(self, freebuff_agent):
        result = freebuff_agent.touch_buffer("ghost", epoch=1)
        assert result is False


# ---------------------------------------------------------------------------
# GPU snapshots
# ---------------------------------------------------------------------------

class TestGpuSnapshots:
    """Test GPU memory snapshot management."""

    def test_update_gpu_snapshot_adds_new(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024 * 1024 * 1024,  # 8 GB
            used_vram_bytes=4 * 1024 * 1024 * 1024,   # 4 GB
            free_vram_bytes=4 * 1024 * 1024 * 1024,   # 4 GB
            fragment_count=3,
            idle_buffer_count=2,
            reclaimable_bytes=500 * 1024 * 1024,
        )
        freebuff_agent.update_gpu_snapshot(snap)
        assert "gpu-0" in freebuff_agent.gpu_snapshots
        assert freebuff_agent.gpu_snapshots["gpu-0"].total_vram_bytes == 8 * 1024**3

    def test_update_gpu_snapshot_overwrites_existing(self, freebuff_agent):
        snap1 = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=2 * 1024**3,
            free_vram_bytes=6 * 1024**3,
            fragment_count=1,
            idle_buffer_count=1,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap1)

        snap2 = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=6 * 1024**3,
            free_vram_bytes=2 * 1024**3,
            fragment_count=5,
            idle_buffer_count=10,
            reclaimable_bytes=1 * 1024**3,
        )
        freebuff_agent.update_gpu_snapshot(snap2)

        snap = freebuff_agent.gpu_snapshots["gpu-0"]
        assert snap.used_vram_bytes == 6 * 1024**3
        assert snap.fragment_count == 5
        assert snap.idle_buffer_count == 10

    def test_multiple_gpu_nodes(self, freebuff_agent):
        for i in range(4):
            snap = GpuMemorySnapshot(
                gpu_node_id=f"gpu-{i}",
                total_vram_bytes=8 * 1024**3,
                used_vram_bytes=2 * 1024**3,
                free_vram_bytes=6 * 1024**3,
                fragment_count=i,
                idle_buffer_count=0,
                reclaimable_bytes=0,
            )
            freebuff_agent.update_gpu_snapshot(snap)
        assert len(freebuff_agent.gpu_snapshots) == 4


# ---------------------------------------------------------------------------
# Idle buffer scanning
# ---------------------------------------------------------------------------

class TestIdleBufferScanning:
    """Test idle buffer detection logic."""

    def test_scan_finds_idle_buffers(self, freebuff_agent):
        for i in range(5):
            buf = BufferRecord(
                buffer_id=f"buf-{i}",
                gpu_node_id="gpu-0",
                size_bytes=1024,
                allocated_at=0.0,
                last_accessed_at=0.0,
            )
            freebuff_agent.register_buffer(buf)

        idle = freebuff_agent._scan_idle_buffers(current_epoch=10)
        assert len(idle) == 5

    def test_scan_ignores_recently_accessed(self, freebuff_agent):
        buf = BufferRecord(
            buffer_id="buf-recent",
            gpu_node_id="gpu-0",
            size_bytes=1024,
            allocated_at=0.0,
            last_accessed_at=0.0,
        )
        freebuff_agent.register_buffer(buf)
        freebuff_agent.touch_buffer("buf-recent", epoch=8)

        idle = freebuff_agent._scan_idle_buffers(current_epoch=10)
        assert len(idle) == 0

    def test_scan_ignores_non_active_buffers(self, freebuff_agent):
        buf = BufferRecord(
            buffer_id="buf-dead",
            gpu_node_id="gpu-0",
            size_bytes=1024,
            allocated_at=0.0,
            last_accessed_at=0.0,
            status="freed",
        )
        freebuff_agent.register_buffer(buf)
        idle = freebuff_agent._scan_idle_buffers(current_epoch=50)
        assert len(idle) == 0


# ---------------------------------------------------------------------------
# Buffer reclamation
# ---------------------------------------------------------------------------

class TestBufferReclamation:
    """Test buffer reclamation logic."""

    def test_reclaim_buffers_changes_status(self, freebuff_agent):
        buffers = []
        for i in range(3):
            buf = BufferRecord(
                buffer_id=f"buf-{i}",
                gpu_node_id="gpu-0",
                size_bytes=1024,
                allocated_at=0.0,
                last_accessed_at=0.0,
            )
            freebuff_agent.register_buffer(buf)
            buffers.append(buf)

        reclaimed, failed = freebuff_agent._reclaim_buffers(buffers, current_epoch=10)
        assert reclaimed == 3
        assert failed == 0
        for buf in buffers:
            assert freebuff_agent.buffer_registry[buf.buffer_id].status == "freed"

    def test_reclaim_respects_max_per_epoch(self, freebuff_agent):
        buffers = []
        for i in range(25):
            buf = BufferRecord(
                buffer_id=f"buf-{i}",
                gpu_node_id="gpu-0",
                size_bytes=1024,
                allocated_at=0.0,
                last_accessed_at=0.0,
            )
            freebuff_agent.register_buffer(buf)
            buffers.append(buf)

        reclaimed, _ = freebuff_agent._reclaim_buffers(buffers, current_epoch=10)
        assert reclaimed == 10

    def test_reclaim_updates_snapshot_vram(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=1024 * 1024,
            used_vram_bytes=100 * 1024,
            free_vram_bytes=924 * 1024,
            fragment_count=0,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)

        buf = BufferRecord(
            buffer_id="buf-big",
            gpu_node_id="gpu-0",
            size_bytes=50 * 1024,
            allocated_at=0.0,
            last_accessed_at=0.0,
        )
        freebuff_agent.register_buffer(buf)
        freebuff_agent._reclaim_buffers([buf], current_epoch=10)
        updated = freebuff_agent.gpu_snapshots["gpu-0"]
        assert updated.used_vram_bytes == 50 * 1024


# ---------------------------------------------------------------------------
# Fragmentation scoring
# ---------------------------------------------------------------------------

class TestFragmentationScoring:
    """Test VRAM fragmentation score computation."""

    def test_perfectly_clean_node(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=2 * 1024**3,
            free_vram_bytes=6 * 1024**3,
            fragment_count=1,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)
        scores = freebuff_agent._score_fragmentation()
        assert scores["gpu-0"] < 0.01

    def test_fragmented_node(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=4 * 1024**3,
            free_vram_bytes=4 * 1024**3,
            fragment_count=2000,
            idle_buffer_count=50,
            reclaimable_bytes=1 * 1024**3,
        )
        freebuff_agent.update_gpu_snapshot(snap)
        scores = freebuff_agent._score_fragmentation()
        assert scores["gpu-0"] > 0.4

    def test_fully_used_node(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=8 * 1024**3,
            free_vram_bytes=0,
            fragment_count=0,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)
        scores = freebuff_agent._score_fragmentation()
        assert scores["gpu-0"] == 1.0


# ---------------------------------------------------------------------------
# Memory pressure prediction
# ---------------------------------------------------------------------------

class TestMemoryPressurePrediction:
    """Test OOM risk prediction."""

    def test_no_snapshots_returns_zero(self, freebuff_agent):
        pressure = freebuff_agent._predict_memory_pressure()
        assert pressure == 0.0

    def test_low_usage_low_frag(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=1 * 1024**3,
            free_vram_bytes=7 * 1024**3,
            fragment_count=1,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)
        pressure = freebuff_agent._predict_memory_pressure()
        assert pressure < 0.2

    def test_high_usage_high_frag(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=7 * 1024**3,  # Note: int, not float
            free_vram_bytes=1 * 1024**3,
            fragment_count=500,
            idle_buffer_count=100,
            reclaimable_bytes=200 * 1024**2,
        )
        # Pre-set fragmentation score for the pressure test
        snap.fragmentation_score = 0.90
        freebuff_agent.update_gpu_snapshot(snap)
        pressure = freebuff_agent._predict_memory_pressure()
        assert pressure > 0.8

    def test_multi_node_averaging(self, freebuff_agent):
        for i, (used, frag) in enumerate([(2, 0.1), (6, 0.8)]):
            snap = GpuMemorySnapshot(
                gpu_node_id=f"gpu-{i}",
                total_vram_bytes=8 * 1024**3,
                used_vram_bytes=used * 1024**3,
                free_vram_bytes=(8 - used) * 1024**3,
                fragment_count=10,
                idle_buffer_count=0,
                reclaimable_bytes=0,
            )
            snap.fragmentation_score = frag
            freebuff_agent.update_gpu_snapshot(snap)

        pressure = freebuff_agent._predict_memory_pressure()
        assert 0.4 < pressure < 0.6


# ---------------------------------------------------------------------------
# Epoch action (act)
# ---------------------------------------------------------------------------

class TestFreebuffAct:
    """Test the FreebuffAgent's act() method behavior."""

    def test_act_reclaims_idle_buffers(self, freebuff_agent):
        for i in range(5):
            buf = BufferRecord(
                buffer_id=f"buf-{i}",
                gpu_node_id="gpu-0",
                size_bytes=1024 * 1024,  # 1 MB each
                allocated_at=0.0,
                last_accessed_at=0.0,
            )
            freebuff_agent.register_buffer(buf)

        result = freebuff_agent.act(epoch=10)
        assert result is not None
        assert result.action_type == "freebuff:reclaim"
        assert result.outcome == Outcome.SUCCESS
        assert result.details["reclaimed"] == 5

    def test_act_monitors_when_fragmentation_high(self, freebuff_agent):
        """Monitor mode triggers when fragmentation exceeds threshold 0.6."""
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=6 * 1024**3,
            free_vram_bytes=2 * 1024**3,
            fragment_count=2000,  # High fragment count → frag > 0.6
            idle_buffer_count=5,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)

        result = freebuff_agent.act(epoch=5)
        assert result is not None
        assert result.action_type == "freebuff:monitor"
        assert "high_frag_nodes" in result.details
        assert len(result.details["high_frag_nodes"]) > 0

    def test_act_passes_through_when_clean(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=2 * 1024**3,
            free_vram_bytes=6 * 1024**3,
            fragment_count=1,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        freebuff_agent.update_gpu_snapshot(snap)

        result = freebuff_agent.act(epoch=1)
        # Clean state → falls through to base act() → no goals → None
        assert result is None


# ---------------------------------------------------------------------------
# Memory report
# ---------------------------------------------------------------------------

class TestMemoryReport:
    """Test get_memory_report() output."""

    def test_report_includes_all_sections(self, freebuff_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=4 * 1024**3,
            free_vram_bytes=4 * 1024**3,
            fragment_count=3,
            idle_buffer_count=1,
            reclaimable_bytes=100 * 1024**2,
        )
        freebuff_agent.update_gpu_snapshot(snap)

        report = freebuff_agent.get_memory_report()
        assert report["agent_id"] == "freebuff-001"
        assert report["domain"] == "FREE_BUFF"
        assert "buffer_stats" in report
        assert "reclaim_stats" in report
        assert "memory_pressure" in report
        assert "nodes" in report
        assert "gpu-0" in report["nodes"]

    def test_report_buffer_stats(self, freebuff_agent):
        buf = BufferRecord(
            buffer_id="buf-1",
            gpu_node_id="gpu-0",
            size_bytes=1024,
            allocated_at=0.0,
            last_accessed_at=0.0,
        )
        freebuff_agent.register_buffer(buf)
        report = freebuff_agent.get_memory_report()
        assert report["buffer_stats"]["total"] == 1
        assert report["buffer_stats"]["active"] == 1
        assert report["buffer_stats"]["freed"] == 0


# ---------------------------------------------------------------------------
# Consequence handling (inherited from Agent)
# ---------------------------------------------------------------------------

class TestFreebuffConsequences:
    """Test that FreebuffAgent handles consequences correctly."""

    def test_energy_drain_reduces_budget(self, freebuff_agent):
        original = freebuff_agent.resource_budget
        freebuff_agent.receive_consequence(
            Consequence("ENERGY_DRAIN", 500.0, "MEMORY_PRESSURE")
        )
        assert freebuff_agent.resource_budget == original - 500.0

    def test_reward_increases_total(self, freebuff_agent):
        freebuff_agent.receive_consequence(
            Consequence("REWARD", 100.0, "BUFFER_RECLAIMED")
        )
        assert freebuff_agent.is_alive

    def test_consequences_on_dead_agent_are_ignored(self, freebuff_agent):
        freebuff_agent.die("test death")
        original = freebuff_agent.resource_budget
        freebuff_agent.receive_consequence(
            Consequence("ENERGY_DRAIN", 9999.0, "MEMORY_PRESSURE")
        )
        assert freebuff_agent.resource_budget == original


# ---------------------------------------------------------------------------
# CLI coding prompt queue
# ---------------------------------------------------------------------------

class TestCodingPromptQueue:
    """Test enqueue_coding_prompt() behaviour."""

    @pytest.fixture
    def cli_agent(self, freebuff_config, storage):
        """Agent with a mock CLI injected."""
        mock_cli = MagicMock(spec=FreebuffCLI)
        mock_cli.run_prompt.return_value = FreebuffResult(
            ok=True,
            output="Conversation ID: cli-conv-1\nDone.",
            conversation_id="cli-conv-1",
        )
        return FreebuffAgent(
            config=freebuff_config,
            storage=storage,
            freebuff_cli=mock_cli,
        )

    def test_enqueue_accepts_valid_prompt(self, cli_agent):
        result = cli_agent.enqueue_coding_prompt("Refactor auth module")
        assert result is True
        assert cli_agent.coding_queue_depth == 1

    def test_enqueue_rejects_too_long_prompt(self, cli_agent):
        long_prompt = "x" * 5000
        result = cli_agent.enqueue_coding_prompt(long_prompt)
        assert result is False
        assert cli_agent.coding_queue_depth == 0

    def test_enqueue_respects_max_length(self, cli_agent):
        cli_agent._freebuff_config.cli_max_prompt_length = 10
        result = cli_agent.enqueue_coding_prompt("12345678901")
        assert result is False

    def test_enqueue_caps_at_100(self, cli_agent):
        cli_agent._freebuff_config.cli_max_prompt_length = 1000
        for i in range(101):
            cli_agent.enqueue_coding_prompt(f"prompt {i}")
        assert cli_agent.coding_queue_depth == 100

    def test_enqueue_strips_whitespace(self, cli_agent):
        cli_agent.enqueue_coding_prompt("  hello world  ")
        prompt, _ = cli_agent._coding_queue[0]
        assert prompt == "hello world"


# ---------------------------------------------------------------------------
# CLI action path (act with freebuff:code)
# ---------------------------------------------------------------------------

class TestFreebuffCodeAction:
    """Test the freebuff:code action path in act()."""

    @pytest.fixture
    def cli_agent(self, freebuff_config, storage):
        mock_cli = MagicMock(spec=FreebuffCLI)
        mock_cli.run_prompt.return_value = FreebuffResult(
            ok=True,
            output="Conversation ID: conv-xyz\nRefactored auth module.",
            conversation_id="conv-xyz",
            duration_seconds=1.5,
        )
        return FreebuffAgent(
            config=freebuff_config,
            storage=storage,
            freebuff_cli=mock_cli,
        )

    def test_act_dispatches_queued_prompt(self, cli_agent):
        cli_agent.enqueue_coding_prompt("Refactor auth", epoch=5)

        result = cli_agent.act(epoch=5)
        assert result is not None
        assert result.action_type == "freebuff:code"
        assert result.outcome == Outcome.SUCCESS
        assert result.details["ok"] is True
        assert result.details["conversation_id"] == "conv-xyz"
        assert cli_agent.coding_queue_depth == 0

    def test_act_sets_conversation_id(self, cli_agent):
        cli_agent.enqueue_coding_prompt("Refactor auth", epoch=5)
        cli_agent.act(epoch=5)
        assert cli_agent.active_conversation_id == "conv-xyz"

    def test_act_handles_cli_failure(self, cli_agent):
        cli_agent._cli.run_prompt.return_value = FreebuffResult(
            ok=False,
            output="",
            error="timeout",
            duration_seconds=30.0,
        )
        cli_agent.enqueue_coding_prompt("Do work", epoch=5)

        result = cli_agent.act(epoch=5)
        assert result.action_type == "freebuff:code"
        assert result.outcome == Outcome.FAILURE
        assert result.details["ok"] is False

    def test_act_throttles_cli_dispatch(self, cli_agent):
        cli_agent._freebuff_config.cli_throttle_epochs = 5
        cli_agent.enqueue_coding_prompt("First task", epoch=1)
        cli_agent.act(epoch=1)

        # Queue another prompt at epoch 2 (within throttle window)
        cli_agent.enqueue_coding_prompt("Second task", epoch=2)
        cli_agent.act(epoch=2)

        # CLI was throttled — second prompt should still be queued
        # but we may fall through to monitor/base path
        # The mock CLI should only have been called once
        assert cli_agent._cli.run_prompt.call_count == 1

    def test_act_passes_through_when_clean_and_empty_queue(self, cli_agent):
        snap = GpuMemorySnapshot(
            gpu_node_id="gpu-0",
            total_vram_bytes=8 * 1024**3,
            used_vram_bytes=2 * 1024**3,
            free_vram_bytes=6 * 1024**3,
            fragment_count=1,
            idle_buffer_count=0,
            reclaimable_bytes=0,
        )
        cli_agent.update_gpu_snapshot(snap)

        result = cli_agent.act(epoch=1)
        assert result is None  # Falls through to base act()


# ---------------------------------------------------------------------------
# CLI stats in report
# ---------------------------------------------------------------------------

class TestCliReport:
    @pytest.fixture
    def cli_agent(self, freebuff_config, storage):
        mock_cli = MagicMock(spec=FreebuffCLI)
        mock_cli.run_prompt.return_value = FreebuffResult(
            ok=True,
            output="ok",
            conversation_id="c1",
        )
        return FreebuffAgent(
            config=freebuff_config,
            storage=storage,
            freebuff_cli=mock_cli,
        )

    def test_report_includes_cli_section(self, cli_agent):
        report = cli_agent.get_memory_report()
        assert "cli" in report
        assert report["cli"]["connected"] is True
        assert report["cli"]["queue_depth"] == 0
        assert report["cli"]["total_results"] == 0

    def test_report_reflects_queue_depth(self, cli_agent):
        cli_agent.enqueue_coding_prompt("a")
        cli_agent.enqueue_coding_prompt("b")
        report = cli_agent.get_memory_report()
        assert report["cli"]["queue_depth"] == 2

    def test_report_reflects_conversation(self, cli_agent):
        cli_agent.enqueue_coding_prompt("test", epoch=5)
        cli_agent.act(epoch=5)
        report = cli_agent.get_memory_report()
        assert report["cli"]["conversation_id"] == "c1"
        assert report["cli"]["total_results"] == 1
