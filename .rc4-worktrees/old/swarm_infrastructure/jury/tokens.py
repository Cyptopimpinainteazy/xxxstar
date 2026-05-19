"""Jury Token Lifecycle System

Constitutional Rules:
- All agents mint 3 JuryTokens at birth
- Tokens are non-transferrable
- Votingconsumes 1+ tokens
- 0 tokens = civic death (removal from all governance)
- Token burns recorded immutably in Scrap Yard

Token States:
  MINTED → ACTIVE → SPENT (immutable)
  Terminal: BURNED (cryptographically verified)
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Set
import time
import hashlib


class TokenStatus(Enum):
    """Token lifecycle states."""
    MINTED = "minted"
    ACTIVE = "active"
    SPENT = "spent"
    BURNED = "burned"


class TokenSpendReason(Enum):
    """Why a token was consumed."""
    TRIAL_JURY_VOTE = "trial_jury_vote"
    APPEAL_JURY_VOTE = "appeal_jury_vote"
    PRECEDENT_CREATION = "precedent_creation"
    PRECEDENT_VOTING = "precedent_voting"
    HIGH_STAKES_APPEAL = "high_stakes_appeal"  # Requires 2 tokens


@dataclass
class JuryToken:
    """Individual jury token with audit trail."""
    token_id: str
    agent_id: str
    issued_at: float = field(default_factory=time.time)
    status: TokenStatus = TokenStatus.MINTED
    spent_at: Optional[float] = None
    spend_reason: Optional[TokenSpendReason] = None
    spend_case_id: Optional[str] = None
    burn_hash: Optional[str] = None  # SHA256 hash of burn record (immutable proof)
    
    def activate(self) -> bool:
        """Transition MINTED → ACTIVE."""
        if self.status != TokenStatus.MINTED:
            return False
        self.status = TokenStatus.ACTIVE
        return True
    
    def spend(self, case_id: str, reason: TokenSpendReason) -> bool:
        """Consume token in a specific case (ACTIVE → SPENT)."""
        if self.status != TokenStatus.ACTIVE:
            return False
        self.status = TokenStatus.SPENT
        self.spent_at = time.time()
        self.spend_case_id = case_id
        self.spend_reason = reason
        return True
    
    def burn(self) -> str:
        """Burn token permanently (SPENT → BURNED) with cryptographic hash."""
        if self.status != TokenStatus.SPENT:
            raise ValueError(f"Can only burn SPENT tokens; current status: {self.status.value}")
        
        burn_record = {
            "token_id": self.token_id,
            "agent_id": self.agent_id,
            "issued_at": self.issued_at,
            "spent_at": self.spent_at,
            "spend_reason": self.spend_reason.value if self.spend_reason else None,
            "spend_case_id": self.spend_case_id,
            "burned_at": time.time(),
        }
        
        burn_str = str(burn_record).encode()
        self.burn_hash = hashlib.sha256(burn_str).hexdigest()
        self.status = TokenStatus.BURNED
        return self.burn_hash


@dataclass
class TokenLedger:
    """Agent token ledger—tracks all 3 tokens."""
    agent_id: str
    tokens: List[JuryToken] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)
    burn_hashes: List[str] = field(default_factory=list)  # Immutable burn record
    
    def __post_init__(self):
        """Mint 3 tokens at ledger creation."""
        if not self.tokens:
            for i in range(3):
                token = JuryToken(
                    token_id=f"{self.agent_id}:token:{i+1}",
                    agent_id=self.agent_id,
                )
                token.activate()  # Activate immediately
                self.tokens.append(token)
    
    def active_tokens(self) -> int:
        """Count remaining active tokens."""
        return sum(1 for t in self.tokens if t.status == TokenStatus.ACTIVE)
    
    def spend_token(self, case_id: str, reason: TokenSpendReason) -> bool:
        """Spend one active token."""
        for token in self.tokens:
            if token.status == TokenStatus.ACTIVE:
                return token.spend(case_id, reason)
        return False  # No active tokens
    
    def civic_death(self) -> bool:
        """Check if agent is civically dead (0 active tokens)."""
        return self.active_tokens() == 0
    
    def burn_all_spent_tokens(self) -> List[str]:
        """Burn all SPENT tokens and return immutable burn hashes."""
        hashes = []
        for token in self.tokens:
            if token.status == TokenStatus.SPENT:
                burn_hash = token.burn()
                hashes.append(burn_hash)
                self.burn_hashes.append(burn_hash)
        return hashes
    
    def get_token_summary(self) -> Dict:
        """Get ledger summary."""
        return {
            "agent_id": self.agent_id,
            "total_tokens": len(self.tokens),
            "active_tokens": self.active_tokens(),
            "spent_tokens": sum(1 for t in self.tokens if t.status == TokenStatus.SPENT),
            "burned_tokens": sum(1 for t in self.tokens if t.status == TokenStatus.BURNED),
            "civic_death": self.civic_death(),
            "burn_hashes": self.burn_hashes,
        }


class TokenManager:
    """Manages token issuance, spending, and civic death tracking."""
    
    TOKENS_PER_AGENT = 3
    
    def __init__(self):
        """Initialize token manager."""
        self.ledgers: Dict[str, TokenLedger] = {}
        self.civic_death_log: List[Dict] = []  # CIVIC_SELF_TERMINATION_PATTERN
        self.token_spend_log: List[Dict] = []  # All spending events
    
    def mint_agent(self, agent_id: str) -> TokenLedger:
        """Mint 3 tokens for new agent."""
        if agent_id in self.ledgers:
            return self.ledgers[agent_id]
        
        ledger = TokenLedger(agent_id=agent_id)
        self.ledgers[agent_id] = ledger
        return ledger
    
    def get_ledger(self, agent_id: str) -> Optional[TokenLedger]:
        """Retrieve agent's token ledger."""
        return self.ledgers.get(agent_id)
    
    def spend_token(
        self,
        agent_id: str,
        case_id: str,
        reason: TokenSpendReason,
    ) -> bool:
        """Spend one token from agent's ledger."""
        ledger = self.get_ledger(agent_id)
        if not ledger:
            return False
        
        if not ledger.spend_token(case_id, reason):
            return False
        
        # Log the spend
        self.token_spend_log.append({
            "agent_id": agent_id,
            "case_id": case_id,
            "reason": reason.value,
            "remaining_tokens": ledger.active_tokens(),
            "timestamp": time.time(),
        })
        
        # Check for civic death
        if ledger.civic_death():
            self._record_civic_death(agent_id)
        
        return True
    
    def _record_civic_death(self, agent_id: str):
        """Record CIVIC_SELF_TERMINATION_PATTERN in log."""
        self.civic_death_log.append({
            "agent_id": agent_id,
            "civic_death_at": time.time(),
            "pattern": "CIVIC_SELF_TERMINATION",
            "tokens_spent": self.TOKENS_PER_AGENT,
        })
    
    def is_civically_alive(self, agent_id: str) -> bool:
        """Check if agent has voting rights (≥1 active token)."""
        ledger = self.get_ledger(agent_id)
        if not ledger:
            return False
        return not ledger.civic_death()
    
    def get_eligible_jurors(self, candidate_ids: List[str]) -> List[str]:
        """Filter candidates to only civically alive agents."""
        return [
            cid for cid in candidate_ids
            if self.is_civically_alive(cid)
        ]
    
    def burn_spent_tokens(self, agent_id: str) -> List[str]:
        """Burn all spent tokens for an agent (immutable)."""
        ledger = self.get_ledger(agent_id)
        if not ledger:
            return []
        return ledger.burn_all_spent_tokens()
    
    def get_civic_death_log(self) -> List[Dict]:
        """Retrieve all civic death events."""
        return self.civic_death_log.copy()
    
    def get_token_spend_log(self) -> List[Dict]:
        """Retrieve all token spending events."""
        return self.token_spend_log.copy()
    
    def get_system_summary(self) -> Dict:
        """Get snapshot of token system state."""
        total_agents = len(self.ledgers)
        civically_alive = sum(1 for ledger in self.ledgers.values() if not ledger.civic_death())
        civically_dead = total_agents - civically_alive
        
        return {
            "total_agents": total_agents,
            "civically_alive": civically_alive,
            "civically_dead": civically_dead,
            "total_tokens_minted": total_agents * self.TOKENS_PER_AGENT,
            "total_tokens_spent": sum(1 for t in self.token_spend_log),
            "civic_death_events": len(self.civic_death_log),
            "civic_death_percentage": civically_dead / total_agents if total_agents > 0 else 0,
        }
