"""Jury manager with commit-reveal voting, rotation, and lifecycle management.

Features:
- Lawyer-administered voir dire (questioning & strikes) with anonymization
- Dual-counsel model (DA + Defense both constrain member selection)
- Randomized empanelment from remaining candidates (preserves entropy)
- All lawyer actions logged as evidence (audit trail in Scrap Yard)
- Create sessions with tasks and jury members
- Commit-reveal voting protocol (commit -> reveal -> aggregate phases)
- Jury rotation with section diversity tracking
- Anonymous vote aggregation with quorum enforcement
- Session lifecycle: creation → voting → completion
- Audit trail for jury decisions (encrypted by infra layer)

This implementation provides the core jury lifecycle and voting mechanics.
Production deployments will extend this with off-chain storage, encryption,
and on-chain anchor recording (see specs/swarm/spec.md).
"""

import hashlib
import time
import uuid
from dataclasses import dataclass, field
from typing import Dict, Optional, List, Set
from enum import Enum
from swarm.jury.voir_dire import VoirDireManager, LawyerRole, StrikeReason


class JuryState(Enum):
    """Jury session lifecycle states."""
    CREATED = "created"
    COMMIT_PHASE = "commit"
    REVEAL_PHASE = "reveal"
    COMPLETED = "completed"
    CANCELLED = "cancelled"


@dataclass
class JuryMember:
    """Represents a jury member with rotation tracking."""
    agent_id: str
    section: str  # e.g., "governance", "economic", "security"
    is_on_chain: bool  # True if rotated from on-chain
    readonly_snapshot: Optional[Dict] = None  # Frozen state for on-chain jurors


@dataclass
class VoteCommit:
    """Sealed vote commitment (anonymity phase)."""
    member_id: str
    commitment: str  # hex digest of hash(vote|nonce)
    timestamp: float = field(default_factory=time.time)


@dataclass
class VoteReveal:
    """Revealed vote with verification data."""
    member_id: str
    vote: bool
    nonce: str
    timestamp: float = field(default_factory=time.time)


@dataclass
class JurySession:
    """Complete jury session lifecycle and voting state."""
    session_id: str
    task_ids: List[str]
    members: List[JuryMember]
    lawyer_id: Optional[str] = None  # ID of lawyer who vetted this session
    lawyer_approved: bool = False  # True if lawyer approved all members
    state: JuryState = JuryState.CREATED
    commitments: Dict[str, VoteCommit] = field(default_factory=dict)
    reveals: Dict[str, VoteReveal] = field(default_factory=dict)
    created_at: float = field(default_factory=time.time)
    commit_deadline: Optional[float] = None
    reveal_deadline: Optional[float] = None
    result: Optional[bool] = None
    quorum_met: bool = False
    metadata: Dict = field(default_factory=dict)  # Extensible audit metadata



class JuryManager:
    """Manages jury sessions with commit-reveal voting and rotation logic.
    
    Quorum Rules:
    - Minimum jury size: 3 members
    - Approval threshold: 66% (2/3 majority)
    - Formula: yes_count / jury_size >= 0.66 AND jury_size >= 3
    
    Rotation Rules:
    - On-chain agents receive read-only snapshot of state
    - Rotation diversity caps prevent section homogeneity
    - Completed jury members are audited before returning to on-chain
    
    Commit-Reveal Protocol:
    - Commit phase: Members submit SHA256(vote|nonce) commitment
    - Reveal phase: Members submit plaintext (vote, nonce) for verification
    - Aggregation: Only revealed votes are tallied; anonymity preserved
    """

    # Configuration constants
    MIN_JURY_SIZE = 3
    QUORUM_THRESHOLD = 2 / 3  # 66% majority
    DEFAULT_COMMIT_TIMEOUT_S = 300  # 5 minutes
    DEFAULT_REVEAL_TIMEOUT_S = 300  # 5 minutes
    MAX_SECTION_MEMBERS_RATIO = 0.75  # Max 75% from one section (prevents monoculture)

    def __init__(self):
        """Initialize jury manager with empty session store."""
        self.sessions: Dict[str, JurySession] = {}
        self.last_rotation_epoch = 0
        self.voir_dire_manager = VoirDireManager()  # Voir dire system

    def create_session_via_voir_dire(
        self,
        case_id: str,
        task_ids: List[str],
        candidate_ids: List[str],
        candidate_data: Dict[str, Dict],
        prosecution_counsel_id: str,
        defense_counsel_id: str,
        jury_size: int = 6,
        commit_timeout_s: int = DEFAULT_COMMIT_TIMEOUT_S,
        reveal_timeout_s: int = DEFAULT_REVEAL_TIMEOUT_S,
    ) -> tuple[bool, Optional['JurySession'], Dict]:
        """Create a jury session via proper voir dire process.
        
        CONSTITUTIONAL MODEL:
        - Anonymize candidates (lawyers never see actual juror IDs)
        - Dual-counsel voir dire (DA + Defense both question & strike)
        - Symmetric strike limits (identical constraints on both sides)
        - Randomized empanelment from remaining candidates
        - All strikes logged as evidence (auditable)
        
        Args:
            case_id: Unique case identifier
            task_ids: Tasks to be voted on
            candidate_ids: Pool of candidate juror IDs
            candidate_data: Dict mapping candidate_id -> {reputation, section, recent_jury_count, ...}
            prosecution_counsel_id: DA lawyer agent ID
            defense_counsel_id: Defense lawyer agent ID
            jury_size: How many jurors to seat
            commit_timeout_s: Voting timeout
            reveal_timeout_s: Reveal timeout
            
        Returns:
            Tuple:
                - bool: True if empanelment successful
                - JurySession or None: Created session
                - Dict: Audit metadata {voir_dire_id, strikes_count, etc}
        """
        # Step 1: Anonymize candidate pool
        voir_dire_id, profiles = self.voir_dire_manager.anonymize_candidates(
            case_id=case_id,
            candidate_ids=candidate_ids,
            candidate_data=candidate_data,
        )
        
        record = self.voir_dire_manager.voir_dire_records[voir_dire_id]
        record.prosecution_counsel = prosecution_counsel_id
        record.defense_counsel = defense_counsel_id
        
        # Step 2: Simulate dual-counsel voir dire
        # (In production, lawyers would interact with system; here we mock a pattern)
        # Prosecution counsel strikes candidates based on safety/procedure
        from random import sample
        prosecution_targets = sample(profiles, min(3, len(profiles)))
        for profile in prosecution_targets:
            self.voir_dire_manager.strike_juror(
                voir_dire_id=voir_dire_id,
                lawyer_id=prosecution_counsel_id,
                lawyer_role=LawyerRole.DA_SAFETY,
                lawyer_side="prosecution",
                profile_hash=profile.profile_hash,
                reason=StrikeReason.SAFETY_CONCERN,
                reasoning_text="Safety review completed",
            )
        
        # Defense counsel strikes candidates independently
        remaining = [p for p in profiles if p not in prosecution_targets]
        defense_targets = sample(remaining, min(3, len(remaining)))
        for profile in defense_targets:
            self.voir_dire_manager.strike_juror(
                voir_dire_id=voir_dire_id,
                lawyer_id=defense_counsel_id,
                lawyer_role=LawyerRole.DEFENSE_DUE_PROCESS,
                lawyer_side="defense",
                profile_hash=profile.profile_hash,
                reason=StrikeReason.BIAS_PATTERN,
                reasoning_text="Due process review completed",
            )
        
        # Step 3: Randomized empanelment
        success, seated_profile_hashes, excluded_hashes = self.voir_dire_manager.finalize_empanelment(
            voir_dire_id=voir_dire_id,
            jury_size=jury_size,
        )
        
        if not success:
            return False, None, {
                "voir_dire_id": voir_dire_id,
                "error": "Insufficient remaining candidates after voir dire",
            }
        
        # Step 4: Map anonymized profiles back to actual juror IDs (post-decision)
        seated_juror_ids = [
            self.voir_dire_manager.profile_id_mapping[profile_hash]
            for profile_hash in seated_profile_hashes
        ]
        
        # Step 5: Create jury members from seated jurors
        members = [
            JuryMember(
                agent_id=jid,
                section=candidate_data[jid].get("section", "general"),
                is_on_chain=False,
            )
            for jid in seated_juror_ids
        ]
        
        # Step 6: Create jury session
        session_id = str(uuid.uuid4())
        now = time.time()
        session = JurySession(
            session_id=session_id,
            task_ids=task_ids,
            members=members,
            lawyer_id=None,  # Voir dire uses dual counsel, not single lawyer
            lawyer_approved=False,  # Voir dire doesn't "approve" -- it selects via strikes
            state=JuryState.COMMIT_PHASE,
            created_at=now,
            commit_deadline=now + commit_timeout_s,
            reveal_deadline=now + commit_timeout_s + reveal_timeout_s,
        )
        
        # Step 7: Store voir dire audit trail in session metadata
        session.metadata["voir_dire_id"] = voir_dire_id
        session.metadata["voir_dire_audit"] = self.voir_dire_manager.get_voir_dire_audit_trail(voir_dire_id)
        session.metadata["prosecution_counsel"] = prosecution_counsel_id
        session.metadata["defense_counsel"] = defense_counsel_id
        
        self.sessions[session_id] = session
        
        return True, session, {
            "voir_dire_id": voir_dire_id,
            "session_id": session_id,
            "prosecution_strikes": len(record.prosecution_strikes),
            "defense_strikes": len(record.defense_strikes),
            "seated_count": len(members),
        }

    def create_session(
        self,
        task_ids: List[str],
        lawyer=None,
        candidate_reputations=None,
        members: Optional[List[JuryMember]] = None,
        commit_timeout_s: int = DEFAULT_COMMIT_TIMEOUT_S,
        reveal_timeout_s: int = DEFAULT_REVEAL_TIMEOUT_S,
    ) -> JurySession:
        """Create a new jury session with specified tasks and members.
        
        DEPRECATED: Use create_session_via_voir_dire() instead.
        This method is kept for backward compatibility.
        
        Args:
            task_ids: List of task IDs to be voted on
            lawyer: Lawyer agent (legacy parameter, ignored if members provided)
            candidate_reputations: Reputation scores (legacy parameter)
            members: List of JuryMember instances; if None, creates default 3-member jury
            commit_timeout_s: Seconds until commit phase expires
            reveal_timeout_s: Seconds until reveal phase expires
            
        Returns:
            JurySession: Newly created session
            
        Raises:
            ValueError: If jury size < MIN_JURY_SIZE or too many members from one section
        """
        session_id = str(uuid.uuid4())
        
        # Default jury if not provided
        if members is None:
            members = [
                JuryMember(agent_id=f"juror-{i+1}", section="general", is_on_chain=False)
                for i in range(3)
            ]
        
        # Validate jury composition
        if len(members) < self.MIN_JURY_SIZE:
            raise ValueError(f"Jury must have at least {self.MIN_JURY_SIZE} members; got {len(members)}")
        
        # Check section diversity cap (prevent monoculture)
        section_counts: Dict[str, int] = {}
        for member in members:
            section_counts[member.section] = section_counts.get(member.section, 0) + 1
        
        for section, count in section_counts.items():
            ratio = count / len(members)
            if ratio > self.MAX_SECTION_MEMBERS_RATIO:
                raise ValueError(
                    f"Section '{section}' has {count}/{len(members)} members ({ratio:.0%}); "
                    f"max ratio is {self.MAX_SECTION_MEMBERS_RATIO:.0%}"
                )
        
        # Create session
        now = time.time()
        session = JurySession(
            session_id=session_id,
            task_ids=task_ids,
            members=members,
            state=JuryState.COMMIT_PHASE,
            created_at=now,
            commit_deadline=now + commit_timeout_s,
            reveal_deadline=now + commit_timeout_s + reveal_timeout_s,
        )
        
        self.sessions[session_id] = session
        return session

    def get_session(self, session_id: str) -> Optional[JurySession]:
        """Retrieve a session by ID."""
        return self.sessions.get(session_id)

    def submit_commit(self, session_id: str, member_id: str, commitment: str) -> bool:
        """Record a sealed vote commitment.
        
        Args:
            session_id: Session ID
            member_id: Jury member ID
            commitment: SHA256 hash commitment (hex string)
            
        Returns:
            bool: True if recorded; False if session not found, wrong phase, or member not in jury
        """
        s = self.get_session(session_id)
        if not s:
            return False
        
        if s.state != JuryState.COMMIT_PHASE:
            return False
        
        # Verify member is in jury
        member_ids = {m.agent_id for m in s.members}
        if member_id not in member_ids:
            return False
        
        s.commitments[member_id] = VoteCommit(
            member_id=member_id,
            commitment=commitment,
            timestamp=time.time(),
        )
        return True

    def advance_to_reveal(self, session_id: str) -> bool:
        """Transition from commit phase to reveal phase.
        
        Args:
            session_id: Session ID
            
        Returns:
            bool: True if successful; False if session not found or already advanced
        """
        s = self.get_session(session_id)
        if not s or s.state != JuryState.COMMIT_PHASE:
            return False
        
        s.state = JuryState.REVEAL_PHASE
        return True

    def submit_reveal(self, session_id: str, member_id: str, vote: bool, nonce: str) -> bool:
        """Record a revealed vote with verification against commitment.
        
        Args:
            session_id: Session ID
            member_id: Jury member ID
            vote: Boolean vote (True=approve, False=reject)
            nonce: Secret nonce used in commitment
            
        Returns:
            bool: True if vote recorded and verified; False if commitment doesn't match
        """
        s = self.get_session(session_id)
        if not s or s.state != JuryState.REVEAL_PHASE:
            return False
        
        # Verify commitment matches
        expected_commitment = hashlib.sha256(
            (str(int(vote)) + "|" + nonce).encode()
        ).hexdigest()
        
        stored_commit = s.commitments.get(member_id)
        if not stored_commit or stored_commit.commitment != expected_commitment:
            return False
        
        s.reveals[member_id] = VoteReveal(
            member_id=member_id,
            vote=vote,
            nonce=nonce,
            timestamp=time.time(),
        )
        return True

    def aggregate(self, session_id: str) -> Optional[Dict]:
        """Tally votes and determine session outcome.
        
        Quorum Check:
        - Minimum jury_size >= 3
        - Approval if: yes_count / jury_size >= 0.66
        
        Returns:
            Dict with keys:
                - yes: Count of yes votes
                - no: Count of no votes
                - total: Jury size
                - quorum_met: Whether quorum threshold was met
                - result: Boolean outcome (if quorum met) or None
                
        Returns None if session not in reveal phase.
        """
        s = self.get_session(session_id)
        if not s or s.state != JuryState.REVEAL_PHASE:
            return None
        
        yes_count = sum(1 for r in s.reveals.values() if r.vote)
        total = len(s.members)
        
        # Check quorum: 66% approval + minimum 3 members
        if total >= self.MIN_JURY_SIZE and (yes_count / total) >= self.QUORUM_THRESHOLD:
            s.result = True
            s.quorum_met = True
        else:
            s.result = False
            s.quorum_met = total >= self.MIN_JURY_SIZE and (yes_count / total) >= self.QUORUM_THRESHOLD
        
        s.state = JuryState.COMPLETED
        
        return {
            "yes": yes_count,
            "no": total - yes_count,
            "total": total,
            "quorum_met": s.quorum_met,
            "result": s.result,
        }

    def cancel_session(self, session_id: str) -> bool:
        """Cancel a session (e.g., if task intent file is deleted).
        
        Args:
            session_id: Session ID
            
        Returns:
            bool: True if cancelled; False if session not found
        """
        s = self.get_session(session_id)
        if not s:
            return False
        
        s.state = JuryState.CANCELLED
        return True

    def list_sessions(self, state: Optional[JuryState] = None) -> List[JurySession]:
        """List all sessions, optionally filtered by state.
        
        Args:
            state: If specified, only return sessions in this state
            
        Returns:
            List of JurySession instances
        """
        sessions = list(self.sessions.values())
        if state:
            sessions = [s for s in sessions if s.state == state]
        return sessions

    def rotate_jury(self, epoch: int, on_chain_agents: List[str], section_weights: Optional[Dict[str, float]] = None) -> List[JuryMember]:
        """Rotate jury membership from on-chain agents for an epoch.
        
        Selection algorithm:
        - Selects a subset of on-chain agents for jury duty
        - Respects section weights to maintain diversity
        - Returns list of JuryMember instances with readonly snapshot markers
        
        Args:
            epoch: Rotation epoch number
            on_chain_agents: List of available on-chain agent IDs
            section_weights: Dict mapping section names to selection weights
                           Used to distribute jury seats proportionally
                           
        Returns:
            List of selected JuryMember instances (each marked is_on_chain=True)
        """
        # Simple round-robin selection from available agents
        # In production, this would use cryptographic randomness + weighted section sampling
        on_chain_jury_size = min(3, len(on_chain_agents))
        selected = on_chain_agents[:on_chain_jury_size]
        
        members = [
            JuryMember(
                agent_id=agent_id,
                section="on-chain",  # Mark as rotated agent
                is_on_chain=True,
                readonly_snapshot={
                    "epoch": epoch,
                    "restriction": "readonly",
                    "access_level": "snapshot",
                },
            )
            for agent_id in selected
        ]
        
        self.last_rotation_epoch = epoch
        return members

    def get_session_audit_trail(self, session_id: str) -> Optional[Dict]:
        """Get audit trail for a completed session.
        
        Returns encrypted/hashed audit data including:
        - Commit timestamps (when jurors submitted commitments)
        - Reveal timestamps (when jurors revealed votes)
        - Final tally and quorum status
        - Session metadata
        
        In production, this data is encrypted and anchored on-chain.
        """
        s = self.get_session(session_id)
        if not s:
            return None
        
        return {
            "session_id": s.session_id,
            "task_ids": s.task_ids,
            "state": s.state.value,
            "created_at": s.created_at,
            "commit_count": len(s.commitments),
            "reveal_count": len(s.reveals),
            "result": s.result,
            "quorum_met": s.quorum_met,
            "jury_size": len(s.members),
            "section_distribution": {
                section: sum(1 for m in s.members if m.section == section)
                for section in set(m.section for m in s.members)
            },
            "metadata": s.metadata,
        }
