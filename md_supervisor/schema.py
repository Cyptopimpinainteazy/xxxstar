"""
md_supervisor — Typed data models for the change-control pipeline.

Normalized, traceable, deterministic data structures for chat ingestion,
change deduplication, patch application, and audit logging.
"""
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional
from uuid import uuid4
from datetime import datetime


class ChangeIntent(Enum):
    CODE = auto()
    DOC = auto()
    CONFIG = auto()
    TEST = auto()


class ChatSource(Enum):
    VSCODE_FILE = auto()
    MCP_SERVER = auto()
    API = auto()
    MANUAL = auto()


class VoteValue(Enum):
    APPROVE = auto()
    REJECT = auto()
    ABSTAIN = auto()


@dataclass
class FileTarget:
    path: str
    language: str
    original_hash: str
    proposed_content: str


@dataclass
class ChangeRequest:
    """Core data model — every change is a first-class object."""
    id: str = field(default_factory=lambda: uuid4().hex)
    source: ChatSource = ChatSource.MANUAL
    timestamp: datetime = field(default_factory=datetime.utcnow)
    files: list[FileTarget] = field(default_factory=list)
    intent: ChangeIntent = ChangeIntent.CODE
    content_hash: str = ""
    semantic_hash: str = ""
    priority: int = 0
    supersedes: list[str] = field(default_factory=list)
    superseded_by: Optional[str] = None
    is_applied: bool = False
    is_rolled_back: bool = False

    def __hash__(self):
        return hash(self.id)

    def __eq__(self, other):
        if isinstance(other, ChangeRequest):
            return self.id == other.id
        return False


@dataclass
class Vote:
    agent_id: str
    agent_role: str
    value: VoteValue
    confidence: float = 0.5
    rationale: str = ""


@dataclass
class ArbitrationResult:
    change_id: str
    votes: list[Vote]
    approved: bool
    consensus_score: float
    timestamp: datetime = field(default_factory=datetime.utcnow)


@dataclass
class AuditEntry:
    timestamp: datetime = field(default_factory=datetime.utcnow)
    agent_id: str = ""
    intent: str = ""
    files: list[str] = field(default_factory=list)
    outcome: str = ""
    hash: str = ""