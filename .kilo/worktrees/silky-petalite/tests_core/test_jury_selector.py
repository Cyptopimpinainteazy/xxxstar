"""Tests for jury member selection and pool management."""

import pytest

from swarm.jury.jury_types import JuryDomain, JurySizer, JuryType
from swarm.jury.manager import JuryMember
from swarm.jury.selector import JuryPool, JurySelector


class TestJurySelector:
    """Test jury member selection from agent pools."""

    @pytest.fixture
    def agent_pool(self):
        """Create a diverse pool of test agents."""
        pool = {}
        sections = ["governance", "economic", "security", "technical", "general"]

        # Create 20 agents, 4 per section
        for i in range(20):
            section_idx = i % len(sections)
            agent_id = f"agent_{i:02d}"
            pool[agent_id] = JuryMember(
                agent_id=agent_id,
                section=sections[section_idx],
                is_on_chain=False,
            )

        return pool

    def test_select_jury_petit_general(self, agent_pool):
        """Test selecting a small petit jury for general domain."""
        selector = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.PETIT, JuryDomain.GENERAL)

        jury = selector.select_jury(sizing)

        assert len(jury) >= sizing.min_size
        assert len(jury) <= sizing.max_size
        # Check all required sections are represented
        sections_in_jury = {member.section for member in jury}
        assert sizing.required_sections <= sections_in_jury

    def test_select_jury_grand_security(self, agent_pool):
        """Test selecting a large grand jury for security domain."""
        selector = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.GRAND, JuryDomain.SECURITY)

        jury = selector.select_jury(sizing)

        assert len(jury) >= sizing.min_size
        assert len(jury) <= sizing.max_size
        sections_in_jury = {member.section for member in jury}
        assert sizing.required_sections <= sections_in_jury

    def test_select_jury_specialized_market(self, agent_pool):
        """Test selecting specialized jury for market domain."""
        selector = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.SPECIALIZED, JuryDomain.MARKET)

        jury = selector.select_jury(sizing)

        assert len(jury) >= sizing.min_size
        assert len(jury) <= sizing.max_size
        sections_in_jury = {member.section for member in jury}
        assert sizing.required_sections <= sections_in_jury

    def test_diversity_threshold_respected(self, agent_pool):
        """Test that section diversity threshold is enforced."""
        selector = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.GRAND, JuryDomain.GENERAL)

        jury = selector.select_jury(sizing, seed=42)

        # Count members per section
        section_counts = {}
        for member in jury:
            section_counts[member.section] = section_counts.get(member.section, 0) + 1

        # Each section should not exceed diversity threshold
        max_per_section = int(len(jury) * sizing.section_diversity_threshold)
        for section, count in section_counts.items():
            assert count <= max_per_section, (
                f"Section '{section}' has {count} members, exceeds max {max_per_section}"
            )

    def test_exclude_agents(self, agent_pool):
        """Test excluding specific agents from selection."""
        selector = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.PETIT, JuryDomain.GENERAL)

        excluded = {"agent_00", "agent_01", "agent_02"}
        jury = selector.select_jury(sizing, excluded_agents=excluded)

        jury_ids = {member.agent_id for member in jury}
        assert jury_ids.isdisjoint(excluded)

    def test_insufficient_agents_raises(self, agent_pool):
        """Test that ValueError is raised when insufficient agents available."""
        # Keep only 2 agents
        small_pool = dict(list(agent_pool.items())[:2])
        selector = JurySelector(small_pool)
        sizing = JurySizer.get_sizing(JuryType.GRAND, JuryDomain.GENERAL)

        with pytest.raises(ValueError):
            selector.select_jury(sizing)

    def test_reproducible_with_seed(self, agent_pool):
        """Test that seed produces reproducible selections."""
        selector1 = JurySelector(agent_pool)
        selector2 = JurySelector(agent_pool)
        sizing = JurySizer.get_sizing(JuryType.PETIT, JuryDomain.MARKET)

        jury1 = selector1.select_jury(sizing, seed=12345)
        jury2 = selector2.select_jury(sizing, seed=12345)

        ids1 = [m.agent_id for m in jury1]
        ids2 = [m.agent_id for m in jury2]
        assert ids1 == ids2


class TestJuryPool:
    """Test jury pool management and availability tracking."""

    @pytest.fixture
    def pool(self):
        """Create a test jury pool."""
        return JuryPool()

    def test_add_agents(self, pool):
        """Test adding agents to pool."""
        pool.add_agent("agent_001", "governance")
        pool.add_agent("agent_002", "economic")

        assert len(pool.members) == 2
        assert pool.members["agent_001"].section == "governance"
        assert pool.members["agent_002"].section == "economic"

    def test_get_available(self, pool):
        """Test retrieving available agents."""
        pool.add_agent("agent_001", "governance")
        pool.add_agent("agent_002", "economic")

        available = pool.get_available()
        assert len(available) == 2

        # Mark one as on duty
        pool.mark_on_duty(["agent_001"])
        available = pool.get_available()

        assert len(available) == 1
        assert "agent_002" in available
        assert "agent_001" not in available

    def test_duty_cycle(self, pool):
        """Test marking agents on duty and completing duty."""
        pool.add_agent("agent_001", "governance")
        pool.add_agent("agent_002", "economic")

        assert pool.duty_history["agent_001"] == 0
        assert pool.duty_history["agent_002"] == 0

        # Start duty
        pool.mark_on_duty(["agent_001"])
        assert "agent_001" in pool.on_duty

        # Complete duty
        pool.mark_duty_complete(["agent_001"])
        assert "agent_001" not in pool.on_duty
        assert pool.duty_history["agent_001"] == 1

    def test_duty_stats(self, pool):
        """Test pool statistics calculation."""
        for i in range(5):
            pool.add_agent(f"agent_{i:03d}", "governance")

        # Complete some duties
        pool.mark_on_duty(["agent_000"])
        pool.mark_on_duty(["agent_001"])

        stats = pool.get_duty_stats()

        assert stats["total_agents"] == 5
        assert stats["on_duty"] == 2
        assert stats["available"] == 3
        assert stats["avg_duties_completed"] == 0

        # Complete duties
        pool.mark_duty_complete(["agent_000", "agent_001"])
        stats = pool.get_duty_stats()

        assert stats["available"] == 5
        assert stats["avg_duties_completed"] == 0.4
        assert stats["max_duties"] == 1

    def test_multiple_duty_cycles(self, pool):
        """Test agent serving multiple juries."""
        pool.add_agent("agent_001", "governance")

        # Multiple duty cycles
        for _cycle in range(3):
            pool.mark_on_duty(["agent_001"])
            pool.mark_duty_complete(["agent_001"])

        assert pool.duty_history["agent_001"] == 3
        assert "agent_001" not in pool.on_duty


class TestIntegration:
    """Integration tests for selector + pool together."""

    def test_jury_selection_from_pool(self):
        """Test selecting jury from pool with availability tracking."""
        pool = JuryPool()

        # Build large pool
        sections = ["governance", "economic", "security", "technical"]
        for i in range(32):
            pool.add_agent(f"agent_{i:02d}", sections[i % len(sections)])

        # Select jury
        selector = JurySelector(pool.get_available())
        sizing = JurySizer.get_sizing(JuryType.GRAND, JuryDomain.CAPABILITY)

        jury = selector.select_jury(sizing)
        jury_ids = [m.agent_id for m in jury]

        # Mark them on duty
        pool.mark_on_duty(jury_ids)

        # Verify they're unavailable
        available = pool.get_available()
        assert len(available) == 32 - len(jury)
        for agent_id in jury_ids:
            assert agent_id not in available

    def test_rotate_juries(self):
        """Test rotating between multiple juries with same pool."""
        pool = JuryPool()

        # Small pool - 12 agents
        sections = ["governance", "economic", "security"]
        for i in range(12):
            pool.add_agent(f"agent_{i:02d}", sections[i % len(sections)])

        sizing_petit = JurySizer.get_sizing(JuryType.PETIT, JuryDomain.GENERAL)

        # Select and complete multiple juries
        for _jury_num in range(3):
            selector = JurySelector(pool.get_available())
            jury = selector.select_jury(sizing_petit)
            jury_ids = [m.agent_id for m in jury]

            pool.mark_on_duty(jury_ids)
            pool.mark_duty_complete(jury_ids)

        # All agents should have served multiple times
        stats = pool.get_duty_stats()
        assert stats["avg_duties_completed"] > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
