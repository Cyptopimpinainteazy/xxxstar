"""
Multi-agent arbitration — Planner, Auditor, Executor agents vote on changes.
Each agent independently evaluates. Consensus required for approval.
"""
from typing import Dict, List, Tuple
from md_supervisor.schema import ChangeRequest, Vote, VoteValue, ArbitrationResult


class PlannerAgent:
    """Evaluates strategic alignment — does this change fit the architecture?"""
    
    def vote(self, req: ChangeRequest) -> Vote:
        # Check for obvious structural issues
        issues = 0
        for ft in req.files:
            if "node_modules" in ft.path or ".git" in ft.path:
                issues += 1
            if ft.language not in ("rs", "py", "ts", "js", "sol", "go", "md", "json", "toml", "yaml"):
                issues += 0  # unknown languages are fine
        
        confidence = max(0.3, 1.0 - (issues * 0.2))
        value = VoteValue.APPROVE if issues < 3 else VoteValue.REJECT
        return Vote("planner", "Planner", value, confidence, f"issues={issues}")


class AuditorAgent:
    """Audits for security, correctness, and policy compliance."""
    
    def vote(self, req: ChangeRequest) -> Vote:
        issues = 0
        for ft in req.files:
            content_lower = ft.proposed_content.lower()
            if any(kw in content_lower for kw in ("secret_key", "password=", "api_key=", "private_key")):
                issues += 3
            if "exec(" in ft.proposed_content or "eval(" in ft.proposed_content:
                issues += 2
            if "../" in ft.path:
                issues += 1
        
        confidence = max(0.2, 1.0 - (issues * 0.25))
        if issues >= 3:
            return Vote("auditor", "Auditor", VoteValue.REJECT, confidence, f"security_issues={issues}")
        if issues > 0:
            return Vote("auditor", "Auditor", VoteValue.APPROVE, confidence, f"minor_issues={issues}")
        return Vote("auditor", "Auditor", VoteValue.APPROVE, 0.9, "clean")


class ExecutorAgent:
    """Validates feasibility — can this change be applied cleanly?"""
    
    def vote(self, req: ChangeRequest) -> Vote:
        issues = 0
        for ft in req.files:
            if not ft.path:
                issues += 1
            if len(ft.proposed_content) > 1_000_000:  # >1MB
                issues += 1
            if not ft.language:
                issues += 1
        
        confidence = max(0.4, 1.0 - (issues * 0.3))
        value = VoteValue.APPROVE if issues < 2 else VoteValue.ABSTAIN
        return Vote("executor", "Executor", value, confidence, f="apply_issues={issues}")


class MultiAgentArbitration:
    """Coordinates multi-agent voting with configurable agent set."""
    
    def __init__(self, agents: Dict[str, object]):
        self.agents = agents
    
    def vote(self, req: ChangeRequest) -> Tuple[bool, Dict[str, bool], float]:
        """Run arbitration. Returns (approved, vote_map, consensus_score)."""
        votes: Dict[str, bool] = {}
        scores: List[float] = []
        
        for name, agent in self.agents.items():
            result = agent.vote(req)
            approved = result.value in (VoteValue.APPROVE,)
            votes[name] = approved
            scores.append(result.confidence if approved else 0.0)
        
        consensus_score = sum(scores) / len(scores) if scores else 0.0
        # All agents must approve
        approved = all(votes.values())
        
        return approved, votes, consensus_score


def get_default_arbitration() -> MultiAgentArbitration:
    """Create arbitration with default agent set."""
    return MultiAgentArbitration({
        "planner": PlannerAgent(),
        "auditor": AuditorAgent(),
        "executor": ExecutorAgent(),
    })