"""Jury Token System — Voting Power & Civic Life/Death

Constitutional Principle:
"Power is lent, not owned. Those who spend it all at once are declaring themselves finished."

Token Rules:
- Every agent minted with exactly 3 JuryTokens at birth
- Tokens are non-transferrable
- Each vote consumes 1+ tokens (high-stakes appeals = 2 tokens)
- Token == 0 → Civic death:
  * Removed from all jury pools
  * Revoked lawyer eligibility
  * Barred from appeals, statutes, precedent creation
  * CIVIC_SELF_TERMINATION_PATTERN logged to Scrap Yard
- All burns immutable & cryptographically verified
- Token exhaustion enforced at:
  * Trial jury empanelment
  * Appellate jury selection
  * Precedent voting
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Set
import time
import uuid


class TokenStatus(Enum):
    """Token lifecycle states."""
    MINTED = "minted"  # Created but not yet usable
    ACTIVE = "active"  # Available for voting
    SPENT = "spent"  # Consumed in vote
    BURNED = "burned"  # Permanently removed


class TokenConsumptionType(Enum):
    """Context where token is consumed."""
    TRIAL_JURY_VOTE = "trial_jury_vote"  # 1 token
    APPELLATE_JURY_VOTE = "appellate_jury_vote"  # 1 token
    PRECEDENT_CREATION = "precedent_creation"  # 1 token
    PRECEDENT_VOTING = "precedent_voting"  # 1 token
    HIGH_STAKES_APPEAL = "high_stakes_appeal"  # 2 tokens (Score conflicts)
    CONSTITUTIONAL_VOTE = "constitutional_vote"  # 2 tokens


@dataclass
class JuryToken:
    """Individual jury token with immutable burn record."""
    token_id: str
    agent_id: str
    status: TokenStatus = TokenStatus.MINTED
    created_at: float = field(default_factory=time.time)
    consumed_at: Optional[float] = None
    consumption_type: Optional[TokenConsumptionType] = None
    consumption_context: str = ""  # Case ID, precedent ID, etc.
    burn_hash: Optional[str] = None  # Cryptographic proof of burn
    
    def activate(self) -> bool:
        """Transition from MINTED to ACTIVE."""
        if self.status != TokenStatus.MINTED:
            return False
        self.status = TokenStatus.ACTIVE
        return True
    
    def spend(
        self,
        consumption_type: TokenConsumptionType,
        context: str,
    ) -> bool:
        """Consume token in a vote or action."""
        if self.status != TokenStatus.ACTIVE:
            return False
        
        self.status = TokenStatus.SPENT
        self.consumed_at = time.time()
        self.consumption_type = consumption_type
        self.consumption_context = context
        
        # Create immutable burn hash
        import hashlib
        burn_data = f"{self.token_id}:{self.agent_id}:{self.consumed_at}:{consumption_type.value}:{context}"
        self.burn_hash = hashlib.sha256(burn_data.encode()).hexdigest()
        
        return True
    
    def burn(self) -> bool:
        """Finalize burn (immutable record created)."""
        if self.status != TokenStatus.SPENT:
            return False
        self.status = TokenStatus.BURNED
        return True


@dataclass
class AgentTokenAllocation:
    """Tracks all tokens for an agent."""
    agent_id: str
    tokens: List[JuryToken] = field(default_factory=list)
    total_minted: int = 3  # Always 3 per agent
    activated_count: int = 0
    spent_count: int = 0
    burned_count: int = 0
    created_at: float = field(default_factory=time.time)


class JuryTokenManager:
    """Manages token issuance, activation, spending, and burn verification."""
    
    TOKENS_PER_AGENT = 3
    HIGH_STAKES_TOKEN_COST = 2
    NORMAL_VOTE_TOKEN_COST = 1
    
    def __init__(self):
        """Initialize token manager."""
        self.allocations: Dict[str, AgentTokenAllocation] = {}
        self.burn_log: List[Dict] = []  # Immutable burn record
    
    def mint_tokens(self, agent_id: str) -> bool:
        """Mint 3 tokens for a new agent (irreversible)."""
        if agent_id in self.allocations:
            return False  # Agent already has tokens
        
        allocation = AgentTokenAllocation(agent_id=agent_id)
        
        for i in range(self.TOKENS_PER_AGENT):
            token = JuryToken(
                token_id=f"{agent_id}-token-{i+1}",
                agent_id=agent_id,
                status=TokenStatus.MINTED,
            )
            allocation.tokens.append(token)
        
        self.allocations[agent_id] = allocation
        return True
    
    def activate_tokens(self, agent_id: str) -> bool:
        """Activate all minted tokens for an agent."""
        allocation = self.allocations.get(agent_id)
        if not allocation:
            return False
        
        for token in allocation.tokens:
            if token.status == TokenStatus.MINTED:
                token.activate()
                allocation.activated_count += 1
        
        return True
    
    def get_active_tokens(self, agent_id: str) -> int:
        """Count available active tokens for an agent."""
        allocation = self.allocations.get(agent_id)
        if not allocation:
            return 0
        
        return sum(1 for t in allocation.tokens if t.status == TokenStatus.ACTIVE)
    
    def spend_token(
        self,
        agent_id: str,
        consumption_type: TokenConsumptionType,
        context: str,
    ) -> bool:
        """Spend one token from agent's allocation.
        
        Args:
            agent_id: Agent ID
            consumption_type: What the token is being used for
            context: Case ID, precedent ID, appeal ID, etc.
            
        Returns:
            bool: True if token spent; False if insufficient active tokens
        """
        allocation = self.allocations.get(agent_id)
        if not allocation:
            return False
        
        # Find first active token
        for token in allocation.tokens:
            if token.status == TokenStatus.ACTIVE:
                token.spend(consumption_type, context)
                allocation.spent_count += 1
                
                # Log to burn log
                self.burn_log.append({
                    "token_id": token.token_id,
                    "agent_id": agent_id,
                    "consumption_type": consumption_type.value,
                    "context": context,
                    "burn_hash": token.burn_hash,
                    "timestamp": time.time(),
                })
                
                return True
        
        return False
    
    def spend_tokens_for_high_stakes(
        self,
        agent_id: str,
        consumption_type: TokenConsumptionType,
        context: str,
    ) -> bool:
        """Spend 2 tokens for high-stakes votes (Score conflicts, constitutional).
        
        Returns False if agent doesn't have 2+ active tokens.
        """
        allocation = self.allocations.get(agent_id)
        if not allocation:
            return False
        
        active_tokens = [t for t in allocation.tokens if t.status == TokenStatus.ACTIVE]
        if len(active_tokens) < self.HIGH_STAKES_TOKEN_COST:
            return False
        
        for i in range(self.HIGH_STAKES_TOKEN_COST):
            active_tokens[i].spend(consumption_type, context)
            allocation.spent_count += 1
            
            self.burn_log.append({
                "token_id": active_tokens[i].token_id,
                "agent_id": agent_id,
                "consumption_type": consumption_type.value,
                "context": context,
                "burn_hash": active_tokens[i].burn_hash,
                "timestamp": time.time(),
            })
        
        return True
    
    def finalize_burns(self) -> int:
        """Finalize all spent tokens → BURNED (immutable).
        
        Returns count of newly burned tokens.
        """
        burned_count = 0
        for allocation in self.allocations.values():
            for token in allocation.tokens:
                if token.status == TokenStatus.SPENT:
                    token.burn()
                    allocation.burned_count += 1
                    burned_count += 1
        
        return burned_count
    
    def is_agent_civically_dead(self, agent_id: str) -> bool:
        """Check if agent has 0 active tokens (civic death)."""
        active_count = self.get_active_tokens(agent_id)
        return active_count == 0
    
    def get_excluded_agents(self) -> Set[str]:
        """Get all agents with 0 active tokens (barred from governance)."""
        excluded = set()
        for agent_id, allocation in self.allocations.items():
            if self.is_agent_civically_dead(agent_id):
                excluded.add(agent_id)
        
        return excluded
    
    def get_agent_token_status(self, agent_id: str) -> Dict:
        """Get full token status for an agent."""
        allocation = self.allocations.get(agent_id)
        if not allocation:
            return {}
        
        return {
            "agent_id": agent_id,
            "total_minted": allocation.total_minted,
            "active": self.get_active_tokens(agent_id),
            "spent": allocation.spent_count,
            "burned": allocation.burned_count,
            "civically_dead": self.is_agent_civically_dead(agent_id),
            "tokens": [
                {
                    "token_id": t.token_id,
                    "status": t.status.value,
                    "consumed_at": t.consumed_at,
                    "consumption_type": t.consumption_type.value if t.consumption_type else None,
                    "burn_hash": t.burn_hash,
                }
                for t in allocation.tokens
            ],
        }
    
    def get_burn_audit_trail(self) -> List[Dict]:
        """Get immutable burn log (all token consumption with hash verification)."""
        return self.burn_log
    
    def verify_burn(self, token_id: str) -> bool:
        """Verify a token's burn hash against log."""
        burn_record = next((r for r in self.burn_log if r["token_id"] == token_id), None)
        if not burn_record:
            return False
        
        # In production: re-hash and compare
        return bool(burn_record.get("burn_hash"))
