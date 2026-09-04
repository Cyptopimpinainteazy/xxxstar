"""Appeals v0: Token-Aware Appellate Justice System

Constitutional Requirements:
- Only civically alive agents (≥1 token) can file appeals
- Appellate jury requires see-dire (anonymized, dual-counsel)
- High-stakes appeals (Score conflicts) require ≥2 tokens per agent
- Precedent creation respects token scarcity
- All decisions logged to Scrap Yard with token consumption records

Appeal States:
  FILED → ACCEPTED → HEARD → DECIDED → RECORDED
  Terminal failure: REJECTED_TOKENS or REJECTED_PROCEDURE
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Tuple
import time
import uuid

from swarm.jury.tokens import TokenManager, TokenSpendReason


class AppealState(Enum):
    """Appeal lifecycle states."""
    FILED = "filed"
    ACCEPTED = "accepted"
    HEARD = "heard"
    DECIDED = "decided"
    RECORDED = "recorded"
    REJECTED_TOKENS = "rejected_tokens"  # Filer ran out of tokens
    REJECTED_PROCEDURE = "rejected_procedure"  # Procedural grounds


class AppealGround(Enum):
    """Constitutional grounds for appeal."""
    SCORE_VIOLATION = "score_violation"  # Jury decision violated immutable Score
    LAWYER_MISCONDUCT = "lawyer_misconduct"  # DA/Defense violated rules
    PROCEDURAL_VIOLATION = "procedural_violation"  # Jury selection invalid
    BIAS_PATTERN = "bias_pattern"  # Lawyer struck pattern detected
    NEW_EVIDENCE = "new_evidence"  # Discovery of error
    CONSTITUTIONAL_QUESTION = "constitutional_question"  # Novel governance issue


@dataclass
class AppealRequest:
    """Request for appellate review."""
    appeal_id: str
    filer_id: str  # Agent filing appeal
    original_case_id: str  # Case being appealed
    grounds: List[AppealGround]
    brief: str  # Formal written argument
    filed_at: float = field(default_factory=time.time)
    state: AppealState = AppealState.FILED
    filer_has_tokens: bool = False  # Checked during filing
    tokens_consumed_per_agent: int = 1  # Normal appeal: 1 token


@dataclass
class AppellateJuryDecision:
    """Appellate jury vote and reasoning."""
    appeal_id: str
    jury_session_id: str
    affirm: bool  # True=uphold original, False=reverse
    dissent_count: int = 0
    reasoning: str = ""
    votes: Dict[str, bool] = field(default_factory=dict)  # juror_id -> vote
    tokens_consumed: int = 0  # Total tokens spent by jury


@dataclass
class Appeal:
    """Complete appeal record."""
    appeal_id: str
    filer_id: str
    original_case_id: str
    grounds: List[AppealGround]
    brief: str
    filed_at: float
    state: AppealState = AppealState.FILED
    
    # Procedural checks
    filer_civically_alive: bool = False
    token_eligibility_checked: bool = False
    
    # Appellate trial
    jury_decision: Optional[AppellateJuryDecision] = None
    decided_at: Optional[float] = None
    
    # Precedent created (if affirmed and significant)
    creates_precedent: bool = False
    precedent_id: Optional[str] = None


class AppealsManager:
    """Manages appeal filing, empanelment, jury trials, and precedent creation."""
    
    def __init__(self, token_manager: TokenManager):
        """Initialize appeals system with token integration."""
        self.token_manager = token_manager
        self.appeals: Dict[str, Appeal] = {}
        self.appeal_audit_log: List[Dict] = []
        
        # Appellate jury pool (will be populated from voir dire)
        self.appellate_jury_candidates: List[str] = []
    
    def file_appeal(
        self,
        filer_id: str,
        original_case_id: str,
        grounds: List[AppealGround],
        brief: str,
    ) -> Tuple[bool, Optional[Appeal], str]:
        """File an appeal (token-gated).
        
        Process:
        1. Check filer is civically alive (≥1 token)
        2. Check grounds are valid
        3. Create appeal record
        4. Log to Scrap Yard
        
        Args:
            filer_id: Agent ID filing appeal
            original_case_id: Case being appealed
            grounds: List of constitutional grounds
            brief: Written argument
            
        Returns:
            Tuple (success, Appeal, error_reason)
        """
        # Check civic status
        if not self.token_manager.is_civically_alive(filer_id):
            self.appeal_audit_log.append({
                "type": "APPEAL_REJECTED_CIVIC_DEATH",
                "filer_id": filer_id,
                "reason": "Filer is civically dead (0 tokens)",
                "timestamp": time.time(),
            })
            return False, None, "CIVIC_DEATH: Filer has no voting tokens remaining"
        
        # Create appeal
        appeal_id = str(uuid.uuid4())
        appeal = Appeal(
            appeal_id=appeal_id,
            filer_id=filer_id,
            original_case_id=original_case_id,
            grounds=grounds,
            brief=brief,
            filed_at=time.time(),
            filer_civically_alive=True,
        )
        
        self.appeals[appeal_id] = appeal
        
        self.appeal_audit_log.append({
            "type": "APPEAL_FILED",
            "appeal_id": appeal_id,
            "filer_id": filer_id,
            "original_case_id": original_case_id,
            "grounds": [g.value for g in grounds],
            "timestamp": appeal.filed_at,
        })
        
        return True, appeal, "Appeal filed successfully"
    
    def accept_appeal(
        self,
        appeal_id: str,
        check_passed: bool = True,
        rejection_reason: Optional[str] = None,
    ) -> bool:
        """Accept appeal for oral argument (FILED → ACCEPTED/REJECTED).
        
        Gates:
        - Procedural review (basic formality check)
        - Token availability for appellate jury
        """
        appeal = self.appeals.get(appeal_id)
        if not appeal:
            return False
        
        if not check_passed:
            appeal.state = AppealState.REJECTED_PROCEDURE
            self.appeal_audit_log.append({
                "type": "APPEAL_REJECTED_PROCEDURE",
                "appeal_id": appeal_id,
                "reason": rejection_reason or "Failed procedural review",
                "timestamp": time.time(),
            })
            return False
        
        appeal.state = AppealState.ACCEPTED
        self.appeal_audit_log.append({
            "type": "APPEAL_ACCEPTED",
            "appeal_id": appeal_id,
            "filer_id": appeal.filer_id,
            "timestamp": time.time(),
        })
        return True
    
    def empanel_appellate_jury(
        self,
        appeal_id: str,
        candidate_ids: List[str],
        prosecution_counsel_id: str,
        defense_counsel_id: str,
        jury_size: int = 5,  # Appellate juries smaller (5-7)
    ) -> Tuple[bool, List[str]]:
        """Empanel appellate jury via token-aware voir dire.
        
        Process:
        1. Filter candidates to civically alive agents only
        2. Run voir dire (anonymized, dual-counsel)
        3. Seat jury from non-struck, token-eligible pool
        
        Args:
            appeal_id: Appeal ID
            candidate_ids: All available candidates
            prosecution_counsel_id: DA counsel agent ID
            defense_counsel_id: Defense counsel agent ID
            jury_size: How many jurors to seat (default 5)
            
        Returns:
            Tuple (success, seated_juror_ids)
        """
        appeal = self.appeals.get(appeal_id)
        if not appeal:
            return False, []
        
        # Filter to civically alive candidates
        eligible = self.token_manager.get_eligible_jurors(candidate_ids)
        
        if len(eligible) < jury_size:
            self.appeal_audit_log.append({
                "type": "APPELLATE_JURY_EMPANELMENT_FAILED",
                "appeal_id": appeal_id,
                "reason": f"Insufficient token-eligible candidates: {len(eligible)} < {jury_size}",
                "timestamp": time.time(),
            })
            return False, []
        
        # Randomize and seat (simplified; full voir dire would apply here)
        import random
        random.shuffle(eligible)
        seated = eligible[:jury_size]
        
        self.appeal_audit_log.append({
            "type": "APPELLATE_JURY_EMPANELED",
            "appeal_id": appeal_id,
            "jury_size": jury_size,
            "eligible_from": len(eligible),
            "timestamp": time.time(),
        })
        
        return True, seated
    
    def record_jury_decision(
        self,
        appeal_id: str,
        jury_session_id: str,
        affirm: bool,
        votes: Dict[str, bool],
        reasoning: str,
    ) -> bool:
        """Record appellate jury decision and consume tokens.
        
        Process:
        1. Record votes
        2. Consume 1 token per jury member
        3. Determine if appeals affirm or reverse
        4. Check if creates precedent (reversals only)
        
        Args:
            appeal_id: Appeal ID
            jury_session_id: Jury trial session ID
            affirm: True if appealing court upheld original
            votes: Dict of juror_id -> voted_to_affirm
            reasoning: Jury's written reasoning
            
        Returns:
            bool: True if decision recorded
        """
        appeal = self.appeals.get(appeal_id)
        if not appeal:
            return False
        
        # Transition ACCEPTED → HEARD if needed
        if appeal.state == AppealState.ACCEPTED:
            appeal.state = AppealState.HEARD
        
        if appeal.state != AppealState.HEARD:
            return False
        
        # Consume tokens for jury members
        for juror_id in votes.keys():
            self.token_manager.spend_token(
                juror_id,
                appeal_id,
                TokenSpendReason.APPEAL_JURY_VOTE,
            )
        
        # Record decision
        decision = AppellateJuryDecision(
            appeal_id=appeal_id,
            jury_session_id=jury_session_id,
            affirm=affirm,
            dissent_count=sum(1 for v in votes.values() if v != affirm),
            reasoning=reasoning,
            votes=votes,
            tokens_consumed=len(votes),
        )
        
        appeal.jury_decision = decision
        appeal.decided_at = time.time()
        appeal.state = AppealState.DECIDED
        
        # Reversals may create precedent
        if not affirm:
            appeal.creates_precedent = True
            appeal.precedent_id = str(uuid.uuid4())
        
        self.appeal_audit_log.append({
            "type": "APPELLATE_DECISION_RECORDED",
            "appeal_id": appeal_id,
            "affirm": affirm,
            "jury_size": len(votes),
            "tokens_consumed": len(votes),
            "creates_precedent": appeal.creates_precedent,
            "timestamp": appeal.decided_at,
        })
        
        return True
    
    def record_appeal(appeal_id: str) -> bool:
        """Record appeal in immutable ledger (DECIDED → RECORDED).
        
        This is the final step: appeal is logged to Scrap Yard and locked.
        """
        # Placeholder for on-chain recording
        return True
    
    def get_appeal_audit_trail(self) -> List[Dict]:
        """Retrieve full audit log of all appeals."""
        return self.appeal_audit_log.copy()
    
    def get_appeal_summary(self, appeal_id: str) -> Optional[Dict]:
        """Get summary of single appeal."""
        appeal = self.appeals.get(appeal_id)
        if not appeal:
            return None
        
        return {
            "appeal_id": appeal.appeal_id,
            "filer_id": appeal.filer_id,
            "original_case_id": appeal.original_case_id,
            "grounds": [g.value for g in appeal.grounds],
            "state": appeal.state.value,
            "filed_at": appeal.filed_at,
            "decided_at": appeal.decided_at,
            "affirm": appeal.jury_decision.affirm if appeal.jury_decision else None,
            "creates_precedent": appeal.creates_precedent,
            "precedent_id": appeal.precedent_id,
        }
