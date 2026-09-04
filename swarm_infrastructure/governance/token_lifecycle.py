"""Jury Token Lifecycle System

Constitutional Rule:
"Power is lent, not owned. Those who spend it all at once are declaring themselves finished."

Token Mechanics:
- 3 JuryTokens per agent at birth
- Tokens are non-transferrable, non-tradeable
- Voting = token consumption
- Token == 0 → civic death (removed from all jury/appeal/precedent pools)
- Token burns are immutable and cryptographically verified
- Used in: trial juries, appellate juries, precedent voting

Token Usage Rules:
- Trial jury voting: 1 token per agent per jury session
- Appellate jury voting: depends on stakes (1 token low, 2 tokens high)
- Precedent creation/voting: 1 token per agent
- Token exhaustion triggers CIVIC_SELF_TERMINATION_PATTERN logging
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional
import time
import hashlib


class TokenStatus(Enum):
    """Token lifecycle states."""
    MINTED = "minted"
    ACTIVE = "active"
    BURNED = "burned"
    REVOKED = "revoked"  # Admin revocation (rare)


class TokenConsumptionContext(Enum):
    """Contexts where tokens are consumed."""
    TRIAL_JURY_VOTE = "trial_jury_vote"
    APPELLATE_JURY_VOTE_LOW_STAKES = "appellate_jury_vote_low_stakes"
    APPELLATE_JURY_VOTE_HIGH_STAKES = "appellate_jury_vote_high_stakes"
    PRECEDENT_VOTING = "precedent_voting"
    PRECEDENT_CREATION = "precedent_creation"


@dataclass
class JuryToken:
    """Single jury token representing voting power."""
    token_id: str  # UUID
    agent_id: str
    status: TokenStatus = TokenStatus.MINTED
    created_at: float = field(default_factory=time.time)
    burned_at: Optional[float] = None
    burn_context: Optional[TokenConsumptionContext] = None
    burn_evidence: Optional[Dict] = None  # Immutable record of burn event
    burn_signature: str = ""  # Cryptographic signature of burn


@dataclass
class AgentTokenLedger:
    """Per-agent token ledger (immutable record of all token activity)."""
    agent_id: str
    tokens: List[JuryToken] = field(default_factory=lambda: [])  # All tokens (minted, burned, etc.)
    total_minted: int = 3  # Always 3 at birth
    total_burned: int = 0
    remaining_active: int = 3  # Starts at 3, decreases with each burn
    ledger_created_at: float = field(default_factory=time.time)
    burn_history: List[Dict] = field(default_factory=list)  # Audit log of burns
    civic_status: str = "active"  # "active" or "terminated" (0 tokens remaining)


@dataclass
class TokenBurnRecord:
    """Immutable record of token burn (forensic artifact)."""
    burn_id: str
    agent_id: str
    token_id: str
    context: TokenConsumptionContext
    case_id: str  # Trial, appeal, precedent case
    burned_at: float = field(default_factory=time.time)
    evidence: Dict = field(default_factory=dict)  # Full context of burn
    signature: str = ""  # Cryptographic proof of burn
    scrap_yard_hash: str = ""  # Link to Scrap Yard immutable log


class JuryTokenManager:
    """Manages token lifecycle: minting, consumption, tracking, enforcement."""
    
    TOKENS_PER_AGENT = 3
    MAX_TOKENS_PER_JURY_SESSION = 1
    MAX_TOKENS_APPELLATE_LOW_STAKES = 1
    MAX_TOKENS_APPELLATE_HIGH_STAKES = 2
    MAX_TOKENS_PRECEDENT_VOTING = 1
    
    def __init__(self):
        """Initialize token manager."""
        self.agent_ledgers: Dict[str, AgentTokenLedger] = {}
        self.all_tokens: Dict[str, JuryToken] = {}  # token_id -> token
        self.burn_records: List[TokenBurnRecord] = []
        self.scrap_yard_links: Dict[str, str] = {}  # burn_id -> scrap_yard_hash
    
    def mint_tokens_for_agent(self, agent_id: str) -> AgentTokenLedger:
        """Create initial token allocation for new agent (3 tokens).
        
        Args:
            agent_id: Agent ID
            
        Returns:
            AgentTokenLedger: Ledger for agent with 3 minted tokens
        """
        if agent_id in self.agent_ledgers:
            # Agent already has tokens
            return self.agent_ledgers[agent_id]
        
        ledger = AgentTokenLedger(agent_id=agent_id)
        
        # Mint 3 tokens
        for i in range(self.TOKENS_PER_AGENT):
            token = JuryToken(
                token_id=f"{agent_id}-token-{i+1}",
                agent_id=agent_id,
                status=TokenStatus.ACTIVE,  # Start as ACTIVE, not MINTED
            )
            ledger.tokens.append(token)
            self.all_tokens[token.token_id] = token
        
        self.agent_ledgers[agent_id] = ledger
        return ledger
    
    def get_agent_ledger(self, agent_id: str) -> Optional[AgentTokenLedger]:
        """Retrieve agent's token ledger."""
        return self.agent_ledgers.get(agent_id)
    
    def get_remaining_tokens(self, agent_id: str) -> int:
        """Get count of remaining active tokens for agent."""
        ledger = self.agent_ledgers.get(agent_id)
        if not ledger:
            return 0
        return ledger.remaining_active
    
    def is_agent_civic_alive(self, agent_id: str) -> bool:
        """Check if agent has remaining tokens (is civically active)."""
        return self.get_remaining_tokens(agent_id) > 0
    
    def burn_token(
        self,
        agent_id: str,
        context: TokenConsumptionContext,
        case_id: str,
        evidence: Optional[Dict] = None,
    ) -> tuple[bool, str]:
        """Consume a token (burn it) in a specific context.
        
        Args:
            agent_id: Agent burning token
            context: TokenConsumptionContext (trial vote, appeal, precedent, etc.)
            case_id: Case/trial/appeal/precedent ID
            evidence: Full context data (jury_session_id, vote, etc.)
            
        Returns:
            Tuple (success, token_id_burned or error_message)
        """
        ledger = self.agent_ledgers.get(agent_id)
        if not ledger:
            return False, f"Agent {agent_id} has no token ledger"
        
        if ledger.remaining_active <= 0:
            return False, f"Agent {agent_id} has no remaining tokens (civic death)"
        
        # Find first active token
        active_token = None
        for token in ledger.tokens:
            if token.status == TokenStatus.ACTIVE:
                active_token = token
                break
        
        if not active_token:
            return False, f"Agent {agent_id} has no remaining tokens (civic death)"
        
        # Burn token
        burn_time = time.time()
        active_token.status = TokenStatus.BURNED
        active_token.burned_at = burn_time
        active_token.burn_context = context
        active_token.burn_evidence = evidence or {}
        
        # Create immutable burn record
        burn_record = TokenBurnRecord(
            burn_id=f"burn-{agent_id}-{int(burn_time*1000)}",
            agent_id=agent_id,
            token_id=active_token.token_id,
            context=context,
            case_id=case_id,
            evidence={
                **active_token.burn_evidence,
                "timestamp": burn_time,
                "case_id": case_id,
                "context": context.value,
            },
        )
        
        # Generate cryptographic signature (simplified: SHA256 of burn data)
        burn_data = f"{burn_record.agent_id}:{burn_record.token_id}:{burn_record.context.value}:{burn_record.case_id}:{burn_time}"
        burn_record.signature = hashlib.sha256(burn_data.encode()).hexdigest()
        active_token.burn_signature = burn_record.signature
        
        self.burn_records.append(burn_record)
        
        # Update ledger
        ledger.total_burned += 1
        ledger.remaining_active -= 1
        ledger.burn_history.append({
            "burn_id": burn_record.burn_id,
            "context": context.value,
            "case_id": case_id,
            "timestamp": burn_time,
            "remaining_after": ledger.remaining_active,
        })
        
        # Check civic status
        if ledger.remaining_active == 0:
            ledger.civic_status = "terminated"
            self._log_civic_termination(agent_id, burn_record)
        
        return True, active_token.token_id
    
    def _log_civic_termination(self, agent_id: str, final_burn: TokenBurnRecord):
        """Log CIVIC_SELF_TERMINATION_PATTERN when agent runs out of tokens."""
        termination_event = {
            "type": "CIVIC_SELF_TERMINATION_PATTERN",
            "agent_id": agent_id,
            "final_token_burn": final_burn.burn_id,
            "final_context": final_burn.context.value,
            "final_case_id": final_burn.case_id,
            "terminated_at": time.time(),
            "enforcement": {
                "removed_from_jury_pools": True,
                "revoked_lawyer_eligibility": True,
                "barred_from_appeals": True,
                "barred_from_precedent_creation": True,
                "agents_with_zero_tokens_governance_weight": 0,
            },
        }
        # This would be logged to Scrap Yard in production
        return termination_event
    
    def check_jury_eligibility(self, agent_ids: List[str]) -> Dict[str, bool]:
        """Check if agents are eligible for jury duty (have remaining tokens).
        
        Args:
            agent_ids: List of candidate agent IDs
            
        Returns:
            Dict mapping agent_id -> eligible (True/False)
        """
        return {
            agent_id: self.is_agent_civic_alive(agent_id)
            for agent_id in agent_ids
        }
    
    def filter_jury_eligible_agents(self, candidate_ids: List[str]) -> List[str]:
        """Filter candidate list to only civically-alive agents with tokens."""
        return [
            agent_id for agent_id in candidate_ids
            if self.is_agent_civic_alive(agent_id)
        ]
    
    def get_token_audit_trail(self, agent_id: str) -> Optional[Dict]:
        """Get full audit trail of token lifecycle for agent."""
        ledger = self.agent_ledgers.get(agent_id)
        if not ledger:
            return None
        
        return {
            "agent_id": agent_id,
            "total_minted": ledger.total_minted,
            "total_burned": ledger.total_burned,
            "remaining_active": ledger.remaining_active,
            "civic_status": ledger.civic_status,
            "burn_history": ledger.burn_history,
            "tokens": [
                {
                    "token_id": t.token_id,
                    "status": t.status.value,
                    "created_at": t.created_at,
                    "burned_at": t.burned_at,
                    "burn_context": t.burn_context.value if t.burn_context else None,
                }
                for t in ledger.tokens
            ],
        }
    
    def get_burn_records_for_case(self, case_id: str) -> List[TokenBurnRecord]:
        """Get all token burns for a specific case."""
        return [r for r in self.burn_records if r.case_id == case_id]
    
    def analyze_token_distribution(self) -> Dict:
        """System-wide token analysis."""
        total_agents = len(self.agent_ledgers)
        agents_alive = sum(1 for ledger in self.agent_ledgers.values() if ledger.civic_status == "active")
        agents_terminated = total_agents - agents_alive
        total_tokens_minted = total_agents * self.TOKENS_PER_AGENT
        total_tokens_burned = sum(r.agent_id for r in self.burn_records)
        
        return {
            "total_agents": total_agents,
            "agents_civically_alive": agents_alive,
            "agents_terminated": agents_terminated,
            "total_tokens_minted": total_tokens_minted,
            "total_tokens_burned": len(self.burn_records),
            "avg_tokens_per_agent": (self.TOKENS_PER_AGENT * total_agents - len(self.burn_records)) / max(total_agents, 1),
            "termination_rate": agents_terminated / max(total_agents, 1),
        }
