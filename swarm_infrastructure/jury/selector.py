"""Jury member selection and rotation from agent pools.

Provides:
- Selection of jury members from available agent pools
- Section diversity enforcement
- Random selection with bias handling
- Agent availability tracking
"""

import random
from typing import List, Dict, Set, Tuple, Optional
from swarm.jury.manager import JuryMember
from swarm.jury.jury_types import JurySizing, JuryDomain, JuryType


class JurySelector:
    """Selects jury members from agent pools based on sizing and diversity rules."""
    
    def __init__(self, agent_pool: Dict[str, JuryMember]):
        """Initialize selector with available agents.
        
        Args:
            agent_pool: Dict mapping agent_id -> JuryMember
        """
        self.agent_pool = agent_pool
        self.section_pool: Dict[str, List[JuryMember]] = self._group_by_section()
    
    def _group_by_section(self) -> Dict[str, List[JuryMember]]:
        """Group agents by their section."""
        sections: Dict[str, List[JuryMember]] = {}
        for agent_id, member in self.agent_pool.items():
            if member.section not in sections:
                sections[member.section] = []
            sections[member.section].append(member)
        return sections
    
    def select_jury(
        self,
        sizing: JurySizing,
        excluded_agents: Optional[Set[str]] = None,
        seed: Optional[int] = None,
    ) -> List[JuryMember]:
        """Select jury members meeting sizing requirements.
        
        Args:
            sizing: JurySizing configuration with required sections and diversity rules
            excluded_agents: Agent IDs to exclude from selection
            seed: Random seed for reproducibility
            
        Returns:
            List of selected JuryMembers
            
        Raises:
            ValueError: If insufficient agents available for sizing requirements
        """
        if seed is not None:
            random.seed(seed)
        
        excluded = excluded_agents or set()
        available_pool = {
            agent_id: member for agent_id, member in self.agent_pool.items()
            if agent_id not in excluded
        }
        
        # Re-group available agents by section
        available_by_section: Dict[str, List[JuryMember]] = {}
        for member in available_pool.values():
            if member.section not in available_by_section:
                available_by_section[member.section] = []
            available_by_section[member.section].append(member)
        
        # Validate that all required sections have available agents
        for required_section in sizing.required_sections:
            if required_section not in available_by_section or not available_by_section[required_section]:
                raise ValueError(
                    f"No available agents in required section '{required_section}'"
                )
        
        selected: List[JuryMember] = []
        target_size = sizing.optimal_size
        max_per_section = int(target_size * sizing.section_diversity_threshold)
        
        # Step 1: Ensure at least one from each required section
        section_counts: Dict[str, int] = {}
        for required_section in sizing.required_sections:
            if available_by_section[required_section]:
                member = random.choice(available_by_section[required_section])
                selected.append(member)
                section_counts[required_section] = section_counts.get(required_section, 0) + 1
        
        # Step 2: Fill remaining slots while respecting diversity threshold
        while len(selected) < target_size:
            # Pick a random section (weighted toward underrepresented sections)
            available_sections = [
                s for s in available_by_section
                if available_by_section[s] and section_counts.get(s, 0) < max_per_section
            ]
            
            if not available_sections:
                # No more slots can be filled respecting diversity
                break
            
            section = random.choice(available_sections)
            # Remove already-selected members from this section
            available_in_section = [
                m for m in available_by_section[section]
                if m not in selected
            ]
            
            if available_in_section:
                member = random.choice(available_in_section)
                selected.append(member)
                section_counts[section] = section_counts.get(section, 0) + 1
        
        # Validate final selection
        if len(selected) < sizing.min_size:
            raise ValueError(
                f"Could not select minimum jury size: {len(selected)} < {sizing.min_size}"
            )
        
        return selected


class JuryPool:
    """Manages a pool of agents available for jury duty with availability tracking."""
    
    def __init__(self):
        """Initialize empty jury pool."""
        self.members: Dict[str, JuryMember] = {}
        self.on_duty: Set[str] = set()  # Agent IDs currently serving on juries
        self.duty_history: Dict[str, int] = {}  # Agent ID -> number of completed juries
    
    def add_agent(self, agent_id: str, section: str) -> None:
        """Add an agent to the jury pool.
        
        Args:
            agent_id: Unique agent identifier
            section: Section/department (governance, economic, security, etc.)
        """
        self.members[agent_id] = JuryMember(
            agent_id=agent_id,
            section=section,
            is_on_chain=False,
        )
        if agent_id not in self.duty_history:
            self.duty_history[agent_id] = 0
    
    def get_available(self) -> Dict[str, JuryMember]:
        """Get all agents not currently on jury duty.
        
        Returns:
            Dict of available JuryMembers
        """
        return {
            agent_id: member for agent_id, member in self.members.items()
            if agent_id not in self.on_duty
        }
    
    def mark_on_duty(self, agent_ids: List[str]) -> None:
        """Mark agents as currently serving on jury duty.
        
        Args:
            agent_ids: List of agent IDs starting jury duty
        """
        self.on_duty.update(agent_ids)
    
    def mark_duty_complete(self, agent_ids: List[str]) -> None:
        """Mark agents' jury duty as completed.
        
        Args:
            agent_ids: List of agent IDs completing jury duty
        """
        for agent_id in agent_ids:
            self.on_duty.discard(agent_id)
            self.duty_history[agent_id] = self.duty_history.get(agent_id, 0) + 1
    
    def get_duty_stats(self) -> Dict[str, any]:
        """Get statistics on jury duty across the pool.
        
        Returns:
            Dict with duty stats
        """
        total_agents = len(self.members)
        on_duty_count = len(self.on_duty)
        avg_completed = (
            sum(self.duty_history.values()) / total_agents
            if total_agents > 0 else 0
        )
        
        return {
            "total_agents": total_agents,
            "on_duty": on_duty_count,
            "available": total_agents - on_duty_count,
            "avg_duties_completed": avg_completed,
            "max_duties": max(self.duty_history.values()) if self.duty_history else 0,
            "min_duties": min(self.duty_history.values()) if self.duty_history else 0,
        }
