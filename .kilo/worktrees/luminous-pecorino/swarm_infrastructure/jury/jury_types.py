"""Jury types and sizing logic for different task domains and severities.

Provides:
- JuryType enum (PETIT, GRAND, SPECIALIZED)
- Domain-specific jury configurations
- Jury sizing based on task severity and domain
- Section diversity enforcement across jury types
"""

from enum import Enum
from dataclasses import dataclass
from typing import Dict, List, Set, Tuple


class JuryType(Enum):
    """Types of juries for different decision-making contexts."""
    PETIT = "petit"          # 6-12 members, routine decisions
    GRAND = "grand"          # 16-23 members, major decisions
    SPECIALIZED = "specialized"  # Domain-specific composition


class JuryDomain(Enum):
    """Domains that require specialized jury composition."""
    MARKET = "market"           # Economic predictions, stake resolution
    SECURITY = "security"       # Safety, integrity, governance violations
    CAPABILITY = "capability"   # Self-improvement proposals, resource allocation
    COORDINATION = "coordination"  # Multi-agent behavior, emergent goals
    GENERAL = "general"         # Default for unspecified domains


@dataclass
class JurySizing:
    """Jury size configuration for a given type and domain."""
    jury_type: JuryType
    domain: JuryDomain
    min_size: int
    max_size: int
    optimal_size: int
    required_sections: Set[str]  # e.g., {"governance", "economic", "security"}
    section_diversity_threshold: float  # Max ratio per section (0.0-1.0)


class JurySizer:
    """Determines jury composition based on task severity, domain, and history."""
    
    # Define jury configurations for each type × domain combination
    CONFIGS: Dict[Tuple[JuryType, JuryDomain], JurySizing] = {
        # PETIT JURIES (6-12 members)
        (JuryType.PETIT, JuryDomain.MARKET): JurySizing(
            jury_type=JuryType.PETIT,
            domain=JuryDomain.MARKET,
            min_size=6,
            max_size=9,
            optimal_size=7,
            required_sections={"economic", "governance"},
            section_diversity_threshold=0.75,
        ),
        (JuryType.PETIT, JuryDomain.SECURITY): JurySizing(
            jury_type=JuryType.PETIT,
            domain=JuryDomain.SECURITY,
            min_size=7,
            max_size=10,
            optimal_size=8,
            required_sections={"security", "governance"},
            section_diversity_threshold=0.70,
        ),
        (JuryType.PETIT, JuryDomain.CAPABILITY): JurySizing(
            jury_type=JuryType.PETIT,
            domain=JuryDomain.CAPABILITY,
            min_size=6,
            max_size=9,
            optimal_size=7,
            required_sections={"economic", "technical"},
            section_diversity_threshold=0.75,
        ),
        (JuryType.PETIT, JuryDomain.COORDINATION): JurySizing(
            jury_type=JuryType.PETIT,
            domain=JuryDomain.COORDINATION,
            min_size=6,
            max_size=8,
            optimal_size=7,
            required_sections={"governance", "social"},
            section_diversity_threshold=0.80,
        ),
        (JuryType.PETIT, JuryDomain.GENERAL): JurySizing(
            jury_type=JuryType.PETIT,
            domain=JuryDomain.GENERAL,
            min_size=6,
            max_size=12,
            optimal_size=9,
            required_sections={"governance", "economic", "security"},
            section_diversity_threshold=0.75,
        ),
        
        # GRAND JURIES (16-23 members)
        (JuryType.GRAND, JuryDomain.MARKET): JurySizing(
            jury_type=JuryType.GRAND,
            domain=JuryDomain.MARKET,
            min_size=16,
            max_size=20,
            optimal_size=18,
            required_sections={"economic", "governance", "auditor"},
            section_diversity_threshold=0.60,
        ),
        (JuryType.GRAND, JuryDomain.SECURITY): JurySizing(
            jury_type=JuryType.GRAND,
            domain=JuryDomain.SECURITY,
            min_size=18,
            max_size=23,
            optimal_size=21,
            required_sections={"security", "governance", "forensic"},
            section_diversity_threshold=0.55,
        ),
        (JuryType.GRAND, JuryDomain.CAPABILITY): JurySizing(
            jury_type=JuryType.GRAND,
            domain=JuryDomain.CAPABILITY,
            min_size=16,
            max_size=22,
            optimal_size=19,
            required_sections={"economic", "technical", "governance", "research"},
            section_diversity_threshold=0.60,
        ),
        (JuryType.GRAND, JuryDomain.COORDINATION): JurySizing(
            jury_type=JuryType.GRAND,
            domain=JuryDomain.COORDINATION,
            min_size=16,
            max_size=21,
            optimal_size=19,
            required_sections={"governance", "social", "economic"},
            section_diversity_threshold=0.65,
        ),
        (JuryType.GRAND, JuryDomain.GENERAL): JurySizing(
            jury_type=JuryType.GRAND,
            domain=JuryDomain.GENERAL,
            min_size=16,
            max_size=23,
            optimal_size=20,
            required_sections={"governance", "economic", "security", "technical"},
            section_diversity_threshold=0.60,
        ),
        
        # SPECIALIZED JURIES
        (JuryType.SPECIALIZED, JuryDomain.MARKET): JurySizing(
            jury_type=JuryType.SPECIALIZED,
            domain=JuryDomain.MARKET,
            min_size=8,
            max_size=14,
            optimal_size=11,
            required_sections={"economic", "auditor", "market-analyst"},
            section_diversity_threshold=0.70,
        ),
        (JuryType.SPECIALIZED, JuryDomain.SECURITY): JurySizing(
            jury_type=JuryType.SPECIALIZED,
            domain=JuryDomain.SECURITY,
            min_size=9,
            max_size=16,
            optimal_size=12,
            required_sections={"security", "forensic", "audit"},
            section_diversity_threshold=0.65,
        ),
        (JuryType.SPECIALIZED, JuryDomain.CAPABILITY): JurySizing(
            jury_type=JuryType.SPECIALIZED,
            domain=JuryDomain.CAPABILITY,
            min_size=8,
            max_size=14,
            optimal_size=11,
            required_sections={"technical", "research", "economic"},
            section_diversity_threshold=0.70,
        ),
        (JuryType.SPECIALIZED, JuryDomain.COORDINATION): JurySizing(
            jury_type=JuryType.SPECIALIZED,
            domain=JuryDomain.COORDINATION,
            min_size=8,
            max_size=13,
            optimal_size=10,
            required_sections={"social", "governance", "behavioral"},
            section_diversity_threshold=0.75,
        ),
        (JuryType.SPECIALIZED, JuryDomain.GENERAL): JurySizing(
            jury_type=JuryType.SPECIALIZED,
            domain=JuryDomain.GENERAL,
            min_size=8,
            max_size=14,
            optimal_size=11,
            required_sections={"governance", "economic", "technical"},
            section_diversity_threshold=0.70,
        ),
    }

    @classmethod
    def get_sizing(
        cls,
        jury_type: JuryType = JuryType.PETIT,
        domain: JuryDomain = JuryDomain.GENERAL,
    ) -> JurySizing:
        """Get jury sizing configuration for a given type and domain.
        
        Args:
            jury_type: Type of jury (PETIT, GRAND, SPECIALIZED)
            domain: Domain requiring jury decision
            
        Returns:
            JurySizing configuration with size and composition rules
        """
        key = (jury_type, domain)
        if key not in cls.CONFIGS:
            # Fallback to GENERAL domain if specific combination not found
            key = (jury_type, JuryDomain.GENERAL)
        return cls.CONFIGS[key]

    @classmethod
    def determine_jury_type(
        cls,
        task_severity: str,
        domain: JuryDomain = JuryDomain.GENERAL,
        escalation_level: int = 0,
    ) -> JuryType:
        """Determine appropriate jury type based on task severity and domain.
        
        Args:
            task_severity: "minor", "major", or "critical"
            domain: Problem domain
            escalation_level: How many times this task has been escalated (0-2)
            
        Returns:
            Recommended JuryType
        """
        # Minor tasks → Petit
        if task_severity == "minor":
            return JuryType.PETIT
        
        # Major tasks with escalation → Grand
        if task_severity == "major" and escalation_level > 0:
            return JuryType.GRAND
        
        # Major tasks → Specialized (domain-specific review)
        if task_severity == "major":
            return JuryType.SPECIALIZED
        
        # Critical tasks always → Grand (maximum deliberation)
        if task_severity == "critical":
            return JuryType.GRAND
        
        # Default
        return JuryType.PETIT

    @classmethod
    def validate_jury_composition(
        cls,
        members: List,  # List[JuryMember]
        sizing: JurySizing,
    ) -> Tuple[bool, str]:
        """Validate that jury composition meets sizing requirements.
        
        Args:
            members: List of JuryMember instances
            sizing: JurySizing configuration
            
        Returns:
            (is_valid, error_message)
        """
        # Check size constraints
        if len(members) < sizing.min_size:
            return False, f"Jury too small: {len(members)} < {sizing.min_size}}"
        if len(members) > sizing.max_size:
            return False, f"Jury too large: {len(members)} > {sizing.max_size}}"
        
        # Check required sections present
        member_sections = {m.section for m in members}
        missing_sections = sizing.required_sections - member_sections
        if missing_sections:
            return False, f"Missing required sections: {missing_sections}"
        
        # Check section diversity threshold
        section_counts: Dict[str, int] = {}
        for member in members:
            section_counts[member.section] = section_counts.get(member.section, 0) + 1
        
        for section, count in section_counts.items():
            ratio = count / len(members)
            if ratio > sizing.section_diversity_threshold:
                return False, (
                    f"Section '{section}' exceeds diversity threshold: "
                    f"{ratio:.0%} > {sizing.section_diversity_threshold:.0%}"
                )
        
        return True, ""
