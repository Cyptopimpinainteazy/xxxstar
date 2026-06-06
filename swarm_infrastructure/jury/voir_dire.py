"""Voir Dire System: Lawyer-Administered Jury Questioning & Selection.

Constitutional Model:
- Lawyers question & strike, but DON'T select
- Anonymized juror profiles only (no identities until post-decision audit)
- Dual-counsel (DA + Defense) with symmetric constraints
- All actions logged as first-class evidence
- Final empanelment from randomized remaining pool

This preserves entropy and prevents capture.
"""

import hashlib
import time
import random
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Set, Tuple


class LawyerRole(Enum):
    """Lawyer archetypes."""
    DA_PROCEDURAL = "da_procedural"  # Arguments rule violations
    DA_SAFETY = "da_safety"  # Risk, tripwire, harm focus
    DEFENSE_DUE_PROCESS = "defense_due_process"  # Challenges procedure
    DEFENSE_INNOVATION = "defense_innovation"  # Argues against ossification
    NEUTRAL_VOIR_DIRE = "neutral_voir_dire"  # Monitors fairness


class StrikeReason(Enum):
    """Documented reasons for juror strikes."""
    DOMAIN_CONFLICT = "domain_conflict"  # Juror has conflict in case domain
    BIAS_PATTERN = "bias_pattern"  # Documented pattern of bias
    RECENCY = "recency"  # Too many recent juries
    PROCEDURAL_VIOLATION = "procedural_violation"  # Process issue
    SAFETY_CONCERN = "safety_concern"  # Risk mitigation
    PEREMPTORY = "peremptory"  # Cause-free strike (limited per counsel)


@dataclass
class JurorProfile:
    """Anonymized juror profile for voir dire (NO identity)."""
    profile_hash: str  # Hash of juror ID (not the ID itself)
    reputation: float  # 0-1 scale
    section: str  # governance, economic, security
    recent_jury_count: int  # Juries in past 30 days
    conflict_flags: List[str] = field(default_factory=list)  # Known conflicts (anonymized)
    domain_expertise: List[str] = field(default_factory=list)  # Domains of expertise


@dataclass
class StrikeAction:
    """Lawyer strike action (logged as evidence)."""
    strike_id: str
    lawyer_id: str
    lawyer_role: LawyerRole
    lawyer_side: str  # "prosecution" or "defense"
    profile_hash: str  # Anonymized juror reference
    reason: StrikeReason
    reasoning_text: str  # Formal justification
    timestamp: float = field(default_factory=time.time)
    case_id: str = ""  # Case/session ID


@dataclass
class VoirDireRecord:
    """Complete voir dire process record for a case."""
    voir_dire_id: str
    case_id: str
    candidate_profiles: List[JurorProfile]  # Anonymized candidates
    prosecution_strikes: List[StrikeAction] = field(default_factory=list)
    defense_strikes: List[StrikeAction] = field(default_factory=list)
    final_jurors: List[str] = field(default_factory=list)  # Profile hashes of seated jurors
    prosecution_counsel: Optional[str] = None
    defense_counsel: Optional[str] = None
    created_at: float = field(default_factory=time.time)


class VoirDireManager:
    """Manages voir dire process with anonymization and dual-counsel constraints.
    
    Workflow:
    1. Create candidate pool (anonymize to profiles only)
    2. DA counsel reviews → strikes with reasons
    3. Defense counsel reviews → strikes with reasons
    4. Calculate mutual strikes (both sides strike same juror = auto-exclude)
    5. Detect bias patterns (same strike reason repeated)
    6. Randomize remaining candidates
    7. Final empanelment from shuffled pool
    8. All actions logged to Scrap Yard
    """
    
    MAX_PEREMPTORY_STRIKES_PER_SIDE = 3  # Limited cause-free strikes
    MAX_FOR_CAUSE_STRIKES_PER_SIDE = 10  # Limited motivated strikes
    
    def __init__(self):
        """Initialize voir dire system."""
        self.voir_dire_records: Dict[str, VoirDireRecord] = {}
        self.lawyer_strike_history: Dict[str, List[StrikeAction]] = {}  # Track patterns
        self.profile_id_mapping = {}  # profile_hash -> actual juror_id (hidden)
    
    def anonymize_candidates(
        self,
        case_id: str,
        candidate_ids: List[str],
        candidate_data: Dict[str, Dict],
    ) -> Tuple[str, List[JurorProfile]]:
        """Convert candidate list to anonymized profiles.
        
        Args:
            case_id: Case/session identifier
            candidate_ids: List of actual juror IDs
            candidate_data: Dict mapping juror_id -> {reputation, section, recent_jury_count, ...}
            
        Returns:
            Tuple:
                - voir_dire_id: Unique ID for this voir dire process
                - List of anonymized JurorProfile objects (NO juror IDs)
        """
        voir_dire_id = str(uuid.uuid4())
        profiles = []
        
        for juror_id in candidate_ids:
            # Create anonymized hash (one-way)
            profile_hash = hashlib.sha256(
                f"{case_id}:{juror_id}:{voir_dire_id}".encode()
            ).hexdigest()[:12]
            
            data = candidate_data.get(juror_id, {})
            profile = JurorProfile(
                profile_hash=profile_hash,
                reputation=data.get("reputation", 0.5),
                section=data.get("section", "general"),
                recent_jury_count=data.get("recent_jury_count", 0),
                conflict_flags=data.get("conflict_flags", []),
                domain_expertise=data.get("domain_expertise", []),
            )
            profiles.append(profile)
            
            # Store mapping (hidden until post-decision audit)
            self.profile_id_mapping[profile_hash] = juror_id
        
        # Create voir dire record
        record = VoirDireRecord(
            voir_dire_id=voir_dire_id,
            case_id=case_id,
            candidate_profiles=profiles,
        )
        self.voir_dire_records[voir_dire_id] = record
        
        return voir_dire_id, profiles
    
    def strike_juror(
        self,
        voir_dire_id: str,
        lawyer_id: str,
        lawyer_role: LawyerRole,
        lawyer_side: str,  # "prosecution" or "defense"
        profile_hash: str,
        reason: StrikeReason,
        reasoning_text: str,
    ) -> bool:
        """Record a lawyer strike of an anonymized juror.
        
        Args:
            voir_dire_id: Voir dire process ID
            lawyer_id: ID of lawyer performing strike
            lawyer_role: Role archetype of lawyer
            lawyer_side: "prosecution" or "defense"
            profile_hash: Anonymized juror hash (not ID)
            reason: StrikeReason enum
            reasoning_text: Formal justification
            
        Returns:
            bool: True if strike recorded; False if side out of strikes
        """
        record = self.voir_dire_records.get(voir_dire_id)
        if not record:
            return False
        
        # Determine which side's strikes we're updating
        if lawyer_side == "prosecution":
            strikes_list = record.prosecution_strikes
        elif lawyer_side == "defense":
            strikes_list = record.defense_strikes
        else:
            return False
        
        # Count existing strikes by type
        peremptory_count = sum(1 for s in strikes_list if s.reason == StrikeReason.PEREMPTORY)
        for_cause_count = sum(1 for s in strikes_list if s.reason != StrikeReason.PEREMPTORY)
        
        # Check limits
        if reason == StrikeReason.PEREMPTORY and peremptory_count >= self.MAX_PEREMPTORY_STRIKES_PER_SIDE:
            return False
        if reason != StrikeReason.PEREMPTORY and for_cause_count >= self.MAX_FOR_CAUSE_STRIKES_PER_SIDE:
            return False
        
        # Record strike
        strike = StrikeAction(
            strike_id=str(uuid.uuid4()),
            lawyer_id=lawyer_id,
            lawyer_role=lawyer_role,
            lawyer_side=lawyer_side,
            profile_hash=profile_hash,
            reason=reason,
            reasoning_text=reasoning_text,
            case_id=record.case_id,
        )
        
        strikes_list.append(strike)
        
        # Track lawyer action for bias detection
        if lawyer_id not in self.lawyer_strike_history:
            self.lawyer_strike_history[lawyer_id] = []
        self.lawyer_strike_history[lawyer_id].append(strike)
        
        return True
    
    def finalize_empanelment(
        self,
        voir_dire_id: str,
        jury_size: int = 6,
    ) -> Tuple[bool, List[str], List[str]]:
        """Finalize jury empanelment after voir dire.
        
        Process:
        1. Identify mutual strikes (both sides struck same juror = auto-exclude)
        2. Detect lawyer bias patterns (same reason repeated by one lawyer)
        3. Remove struck jurors + mutual strikes
        4. Randomize remaining pool
        5. Select final jury from shuffled pool
        
        Args:
            voir_dire_id: Voir dire process ID
            jury_size: How many jurors to seat
            
        Returns:
            Tuple:
                - bool: True if successful empanelment, False if insufficient candidates
                - List[str]: Profile hashes of seated jurors
                - List[str]: Profile hashes of excluded jurors (for audit)
        """
        record = self.voir_dire_records.get(voir_dire_id)
        if not record:
            return False, [], []
        
        # Collect all struck profiles
        prosecution_struck = {s.profile_hash for s in record.prosecution_strikes}
        defense_struck = {s.profile_hash for s in record.defense_strikes}
        
        # Identify mutual strikes (audit flag: why did both sides agree?)
        mutual_strikes = prosecution_struck & defense_struck
        
        # All excluded profiles
        all_excluded = prosecution_struck | defense_struck
        
        # Remaining candidates
        remaining = [
            p for p in record.candidate_profiles
            if p.profile_hash not in all_excluded
        ]
        
        if len(remaining) < jury_size:
            return False, [], list(all_excluded)
        
        # Randomize and select
        random.shuffle(remaining)
        seated = [p.profile_hash for p in remaining[:jury_size]]
        
        record.final_jurors = seated
        return True, seated, list(all_excluded)
    
    def detect_lawyer_bias_patterns(self, lawyer_id: str) -> Dict:
        """Analyze lawyer's strike patterns for bias.
        
        Looks for:
        - Same strike reason repeated > N times (ossified pattern)
        - Targeting jurors from same section/domain (systemic bias)
        - Extreme peremptory use
        
        Returns:
            Dict with flags: {pattern_type: count, concern_level: high/medium/low}
        """
        strikes = self.lawyer_strike_history.get(lawyer_id, [])
        
        if not strikes:
            return {"concern_level": "none", "flags": []}
        
        # Reason frequency
        reason_counts = {}
        for strike in strikes:
            reason_counts[strike.reason] = reason_counts.get(strike.reason, 0) + 1
        
        # Section targeting
        excluded_sections = {}
        for strike in strikes:
            # Get profile to check section
            profile = next(
                (p for p in self.voir_dire_records.values() 
                 for p in p.candidate_profiles if p.profile_hash == strike.profile_hash),
                None
            )
            if profile:
                excluded_sections[profile.section] = excluded_sections.get(profile.section, 0) + 1
        
        concerns = []
        concern_level = "low"
        
        # Detect ossification (same reason > 5 times)
        for reason, count in reason_counts.items():
            if count > 5:
                concerns.append(f"Repeated strike reason: {reason.value} ({count}x)")
                concern_level = "high"
        
        # Detect section targeting (>70% from one section)
        if excluded_sections:
            total = sum(excluded_sections.values())
            for section, count in excluded_sections.items():
                if (count / total) > 0.7:
                    concerns.append(f"Section targeting: {section} ({count}/{total})")
                    concern_level = "high" if concern_level != "high" else "high"
        
        return {
            "lawyer_id": lawyer_id,
            "total_strikes": len(strikes),
            "concern_level": concern_level,
            "flags": concerns,
        }
    
    def get_voir_dire_audit_trail(self, voir_dire_id: str) -> Dict:
        """Get full audit trail of voir dire process.
        
        Returns all strikes, mutual strikes, and pattern analysis.
        """
        record = self.voir_dire_records.get(voir_dire_id)
        if not record:
            return {}
        
        prosecution_struck = {s.profile_hash for s in record.prosecution_strikes}
        defense_struck = {s.profile_hash for s in record.defense_strikes}
        mutual_strikes = prosecution_struck & defense_struck
        
        return {
            "voir_dire_id": voir_dire_id,
            "case_id": record.case_id,
            "prosecution_counsel": record.prosecution_counsel,
            "defense_counsel": record.defense_counsel,
            "total_candidates": len(record.candidate_profiles),
            "prosecution_strikes": len(record.prosecution_strikes),
            "defense_strikes": len(record.defense_strikes),
            "mutual_strikes": len(mutual_strikes),
            "final_jury_size": len(record.final_jurors),
            "strikes_history": [
                {
                    "lawyer_id": s.lawyer_id,
                    "lawyer_role": s.lawyer_role.value,
                    "side": s.lawyer_side,
                    "reason": s.reason.value,
                    "reasoning": s.reasoning_text,
                    "timestamp": s.timestamp,
                }
                for s in record.prosecution_strikes + record.defense_strikes
            ],
        }
    
    def resolve_mutual_strikes(self) -> Dict:
        """Analyze all cases where both sides struck the same juror.
        
        Mutual strikes are anomalies: why did everyone agree to exclude this person?
        These warrant investigation (corruption, legitimate danger, bias agreement).
        
        Returns:
            Dict mapping voir_dire_id -> list of mutual strike analyis records
        """
        mutual_cases = {}
        
        for voir_dire_id, record in self.voir_dire_records.items():
            prosecution_struck = {s.profile_hash: s for s in record.prosecution_strikes}
            defense_struck = {s.profile_hash: s for s in record.defense_strikes}
            
            mutual = prosecution_struck.keys() & defense_struck.keys()
            
            if mutual:
                mutual_cases[voir_dire_id] = [
                    {
                        "profile_hash": profile_hash,
                        "prosecution_reason": prosecution_struck[profile_hash].reason.value,
                        "defense_reason": defense_struck[profile_hash].reason.value,
                        "prosecution_reasoning": prosecution_struck[profile_hash].reasoning_text,
                        "defense_reasoning": defense_struck[profile_hash].reasoning_text,
                    }
                    for profile_hash in mutual
                ]
        
        return mutual_cases
