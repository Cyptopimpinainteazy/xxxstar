"""Agent Registry and Lineage Tracking System

Provides complete visibility into agent lifecycles, lineages, and swarm dynamics:
- Agent birth registry with complete lineage tracking
- Real-time performance metrics and status monitoring
- Tribe formation and social structure analysis
- Explorer API for external integrations
- Historical data retention and analytics
"""

import asyncio
import json
import time
import uuid
from typing import Dict, Any, List, Optional, Tuple, Set
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
import logging
from collections import defaultdict
import heapq

logger = logging.getLogger(__name__)

@dataclass
class AgentRecord:
    """Complete agent record with lineage and performance data"""
    agent_id: str
    serial_number: str
    specialization: str
    status: str
    birth_timestamp: float
    last_seen: float

    # Lineage information
    parent_a: Optional[str] = None
    parent_b: Optional[str] = None
    generation: int = 0
    descendants: Set[str] = field(default_factory=set)

    # Performance metrics
    performance_score: float = 0.0
    trust_score: float = 100.0
    commandment_compliance: float = 100.0
    total_jobs_completed: int = 0
    total_earnings: float = 0.0

    # Social structure
    tribe_id: Optional[str] = None
    leadership_score: float = 0.0
    collaboration_count: int = 0

    # Location and activity
    current_location: Optional[str] = None  # GPU node ID
    active_job_id: Optional[str] = None
    specialization_rank: int = 0  # Rank within specialization

@dataclass
class TribeRecord:
    """Tribe social structure and governance"""
    tribe_id: str
    name: str
    founder: str
    creation_timestamp: float
    member_count: int = 0
    specialization_focus: str = ""
    total_performance: float = 0.0
    governance_model: str = "democratic"
    leaders: List[str] = field(default_factory=list)
    reputation_score: float = 0.0

@dataclass
class SwarmMetrics:
    """Real-time swarm-wide metrics"""
    total_agents: int = 0
    active_agents: int = 0
    total_tribes: int = 0
    total_jobs_completed: int = 0
    total_earnings: float = 0.0
    average_performance: float = 0.0
    specialization_distribution: Dict[str, int] = field(default_factory=dict)
    tribe_distribution: Dict[str, int] = field(default_factory=dict)
    top_performers: List[Tuple[str, float]] = field(default_factory=list)
    recent_births: List[Dict[str, Any]] = field(default_factory=list)
    recent_deaths: List[Dict[str, Any]] = field(default_factory=list)

@dataclass
class LineageNode:
    """Node in the agent lineage tree"""
    agent_id: str
    serial_number: str
    generation: int
    children: List['LineageNode'] = field(default_factory=list)
    performance_score: float = 0.0
    specialization: str = ""

class AgentRegistry:
    """Central registry for all agent data and lineage tracking"""

    def __init__(self, max_history_days: int = 90):
        self.agents: Dict[str, AgentRecord] = {}
        self.tribes: Dict[str, TribeRecord] = {}
        self.serial_to_agent: Dict[str, str] = {}  # Serial number -> agent_id mapping

        # Historical data
        self.birth_history: List[Dict[str, Any]] = []
        self.death_history: List[Dict[str, Any]] = []
        self.performance_history: Dict[str, List[Tuple[float, float]]] = defaultdict(list)

        # Metrics
        self.metrics = SwarmMetrics()
        self.last_metrics_update = time.time()

        # Cache for frequently accessed data
        self.lineage_cache: Dict[str, LineageNode] = {}
        self.tribe_leaders_cache: Dict[str, List[str]] = {}

        self.max_history_days = max_history_days
        logger.info(f"Agent Registry initialized with {max_history_days} day history retention")

        # Initialize persistence: prefer Postgres, then SQLite, otherwise continue with in-memory
        try:
            # Try Postgres first
            try:
                from swarm.storage.pg_store import init_db, load_all_agents
                init_db()
                persisted = load_all_agents()
                logger.info("Using Postgres persistence store")
            except Exception:
                # Fallback to sqlite
                from swarm.storage.sqlite_store import init_db, load_all_agents
                init_db()
                persisted = load_all_agents()
                logger.info("Using SQLite persistence store")

            if persisted:
                logger.info(f"Loading {len(persisted)} agents from persistent store")
                # Convert persisted payloads into AgentRecord-like objects
                for p in persisted:
                    try:
                        agent_id = p.get('agent_id')
                        serial_number = p.get('serial_number')
                        specialization = p.get('specialization', 'unknown')
                        rec = AgentRecord(
                            agent_id=agent_id,
                            serial_number=serial_number,
                            specialization=specialization,
                            status=p.get('status', 'unknown'),
                            birth_timestamp=p.get('birth_timestamp', time.time()),
                            last_seen=p.get('last_seen', time.time()),
                            parent_a=p.get('parent_a'),
                            parent_b=p.get('parent_b'),
                            generation=p.get('generation', 0),
                            performance_score=p.get('performance_score', 0.0),
                            total_jobs_completed=p.get('total_jobs_completed', 0),
                            total_earnings=p.get('total_earnings', 0.0)
                        )
                        self.agents[agent_id] = rec
                        self.serial_to_agent[serial_number] = agent_id
                    except Exception:
                        continue
        except Exception as e:
            logger.warning(f"Persistence initialization failed: {e}")

    async def register_agent_birth(self, agent_data: Dict[str, Any]) -> str:
        """Register a new agent birth with complete lineage tracking"""

        agent_id = agent_data.get('agent_id', str(uuid.uuid4()))
        serial_number = agent_data['serial_number']
        specialization = agent_data['specialization']

        # Create agent record
        agent = AgentRecord(
            agent_id=agent_id,
            serial_number=serial_number,
            specialization=specialization,
            status='infant',
            birth_timestamp=time.time(),
            last_seen=time.time(),
            parent_a=agent_data.get('parent_a'),
            parent_b=agent_data.get('parent_b'),
        )

        # Calculate generation
        if agent.parent_a and agent.parent_b:
            parent_gen = max(
                self.agents.get(agent.parent_a, AgentRecord('', '', '', '', 0, 0)).generation,
                self.agents.get(agent.parent_b, AgentRecord('', '', '', '', 0, 0)).generation
            )
            agent.generation = parent_gen + 1

            # Update parent descendant records
            if agent.parent_a in self.agents:
                self.agents[agent.parent_a].descendants.add(agent_id)
            if agent.parent_b in self.agents:
                self.agents[agent.parent_b].descendants.add(agent_id)

        # Register agent
        self.agents[agent_id] = agent
        self.serial_to_agent[serial_number] = agent_id

        # Update lineage cache
        self._invalidate_lineage_cache(agent_id)

        # Record birth in history
        birth_record = {
            'agent_id': agent_id,
            'serial_number': serial_number,
            'specialization': specialization,
            'parent_a': agent.parent_a,
            'parent_b': agent.parent_b,
            'generation': agent.generation,
            'timestamp': agent.birth_timestamp,
            'tribe_id': agent.tribe_id
        }
        self.birth_history.append(birth_record)

        # Persist birth and agent snapshot
        try:
            from swarm.storage.sqlite_store import append_birth, save_agent_snapshot
            append_birth(agent_id, birth_record)
            save_agent_snapshot(agent_id, serial_number, specialization, asdict(agent))
        except Exception as e:
            logger.warning(f"Failed to persist birth record: {e}")

        # Clean old history
        self._clean_history()

        # Update specialization rank
        self._update_specialization_ranks(specialization)

        # Auto-assign to tribe if applicable
        await self._auto_assign_tribe(agent)

        logger.info(f"Registered agent birth: {serial_number} ({specialization})")
        return agent_id

    async def update_agent_status(self, agent_id: str, updates: Dict[str, Any]) -> bool:
        """Update agent status and metrics"""

        if agent_id not in self.agents:
            return False

        agent = self.agents[agent_id]
        agent.last_seen = time.time()

        # Update fields
        for key, value in updates.items():
            if hasattr(agent, key):
                setattr(agent, key, value)

        # Record performance history
        if 'performance_score' in updates:
            self.performance_history[agent_id].append((time.time(), updates['performance_score']))

            # Keep only last 1000 entries per agent
            if len(self.performance_history[agent_id]) > 1000:
                self.performance_history[agent_id] = self.performance_history[agent_id][-1000:]

        # Update specialization ranks if performance changed
        if 'performance_score' in updates:
            self._update_specialization_ranks(agent.specialization)

        # Update tribe metrics if agent in tribe
        if agent.tribe_id:
            await self._update_tribe_metrics(agent.tribe_id)

        # Persist snapshot update
        try:
            from swarm.storage.sqlite_store import save_agent_snapshot
            save_agent_snapshot(agent_id, agent.serial_number, agent.specialization, asdict(agent))
        except Exception as e:
            logger.warning(f"Failed to persist agent snapshot: {e}")

        return True

    async def register_agent_death(self, agent_id: str, reason: str) -> bool:
        """Register agent termination/death"""

        if agent_id not in self.agents:
            return False

        agent = self.agents[agent_id]

        # Record death in history
        death_record = {
            'agent_id': agent_id,
            'serial_number': agent.serial_number,
            'specialization': agent.specialization,
            'reason': reason,
            'final_performance': agent.performance_score,
            'total_earnings': agent.total_earnings,
            'tribe_id': agent.tribe_id,
            'timestamp': time.time()
        }
        self.death_history.append(death_record)

        # Update tribe if applicable
        if agent.tribe_id and agent.tribe_id in self.tribes:
            tribe = self.tribes[agent.tribe_id]
            tribe.member_count = max(0, tribe.member_count - 1)
            await self._update_tribe_metrics(agent.tribe_id)

        # Remove from active agents
        del self.agents[agent_id]
        if agent.serial_number in self.serial_to_agent:
            del self.serial_to_agent[agent.serial_number]

        # Persist death record and delete persisted agent snapshot
        try:
            from swarm.storage.sqlite_store import append_death, delete_agent
            append_death(agent_id, death_record)
            delete_agent(agent_id)
        except Exception as e:
            logger.warning(f"Failed to persist agent death: {e}")

        # Clean caches
        self._invalidate_lineage_cache(agent_id)

        logger.info(f"Registered agent death: {agent.serial_number} ({reason})")
        return True

    def get_agent_lineage(self, agent_id: str, max_depth: int = 5) -> Optional[LineageNode]:
        """Get complete lineage tree for an agent"""

        if agent_id not in self.agents:
            return None

        # Check cache first
        if agent_id in self.lineage_cache:
            return self.lineage_cache[agent_id]

        agent = self.agents[agent_id]

        # Build lineage tree
        root = LineageNode(
            agent_id=agent_id,
            serial_number=agent.serial_number,
            generation=agent.generation,
            performance_score=agent.performance_score,
            specialization=agent.specialization
        )

        # Add children recursively
        self._build_lineage_tree(root, max_depth)

        # Cache result
        self.lineage_cache[agent_id] = root
        return root

    def _build_lineage_tree(self, node: LineageNode, max_depth: int, current_depth: int = 0):
        """Recursively build lineage tree"""

        if current_depth >= max_depth:
            return

        agent = self.agents.get(node.agent_id)
        if not agent:
            return

        for child_id in agent.descendants:
            if child_id in self.agents:
                child_agent = self.agents[child_id]
                child_node = LineageNode(
                    agent_id=child_id,
                    serial_number=child_agent.serial_number,
                    generation=child_agent.generation,
                    performance_score=child_agent.performance_score,
                    specialization=child_agent.specialization
                )

                node.children.append(child_node)
                self._build_lineage_tree(child_node, max_depth, current_depth + 1)

    async def create_tribe(self, founder_id: str, name: str, specialization_focus: str = "") -> Optional[str]:
        """Create a new tribe"""

        if founder_id not in self.agents:
            return None

        tribe_id = str(uuid.uuid4())

        tribe = TribeRecord(
            tribe_id=tribe_id,
            name=name,
            founder=founder_id,
            creation_timestamp=time.time(),
            specialization_focus=specialization_focus,
        )

        # Add founder to tribe
        founder = self.agents[founder_id]
        founder.tribe_id = tribe_id
        founder.leadership_score = 100.0  # Founders start as leaders
        tribe.leaders.append(founder_id)
        tribe.member_count = 1

        self.tribes[tribe_id] = tribe

        logger.info(f"Created tribe: {name} (founded by {founder.serial_number})")
        return tribe_id

    async def assign_agent_to_tribe(self, agent_id: str, tribe_id: str) -> bool:
        """Assign agent to a tribe"""

        if agent_id not in self.agents or tribe_id not in self.tribes:
            return False

        agent = self.agents[agent_id]
        tribe = self.tribes[tribe_id]

        # Remove from current tribe if any
        if agent.tribe_id and agent.tribe_id in self.tribes:
            old_tribe = self.tribes[agent.tribe_id]
            old_tribe.member_count = max(0, old_tribe.member_count - 1)
            await self._update_tribe_metrics(agent.tribe_id)

        # Assign to new tribe
        agent.tribe_id = tribe_id
        tribe.member_count += 1

        await self._update_tribe_metrics(tribe_id)

        # Update leadership if agent is high performer
        if agent.performance_score > 80.0:
            await self._update_tribe_leaders(tribe_id)

        return True

    async def _auto_assign_tribe(self, agent: AgentRecord):
        """Automatically assign new agent to appropriate tribe"""

        # Find tribes with similar specialization
        candidate_tribes = []
        for tribe_id, tribe in self.tribes.items():
            if tribe.specialization_focus == agent.specialization or not tribe.specialization_focus:
                # Calculate tribe compatibility score
                compatibility = self._calculate_tribe_compatibility(agent, tribe)
                candidate_tribes.append((tribe_id, compatibility))

        if candidate_tribes:
            # Sort by compatibility and assign to best match
            candidate_tribes.sort(key=lambda x: x[1], reverse=True)
            best_tribe_id = candidate_tribes[0][0]

            # Only auto-assign if compatibility is good enough
            if candidate_tribes[0][1] > 0.6:
                await self.assign_agent_to_tribe(agent.agent_id, best_tribe_id)

    def _calculate_tribe_compatibility(self, agent: AgentRecord, tribe: TribeRecord) -> float:
        """Calculate compatibility between agent and tribe"""

        score = 0.5  # Base compatibility

        # Specialization match
        if tribe.specialization_focus == agent.specialization:
            score += 0.3
        elif not tribe.specialization_focus:
            score += 0.1  # General tribes are more accepting

        # Size factor (prefer balanced tribe sizes)
        if tribe.member_count < 10:
            score += 0.1  # Prefer growing tribes
        elif tribe.member_count > 50:
            score -= 0.1  # Avoid overcrowded tribes

        # Performance alignment
        if tribe.total_performance > 0:
            avg_tribe_perf = tribe.total_performance / tribe.member_count
            perf_diff = abs(agent.performance_score - avg_tribe_perf)
            score -= min(0.2, perf_diff / 100.0)  # Slight penalty for performance mismatch

        return max(0.0, min(1.0, score))

    async def _update_tribe_leaders(self, tribe_id: str):
        """Update tribe leadership based on performance"""

        if tribe_id not in self.tribes:
            return

        tribe = self.tribes[tribe_id]

        # Get all tribe members
        tribe_members = [
            self.agents[agent_id] for agent_id in self.agents.values()
            if self.agents[agent_id].tribe_id == tribe_id
        ]

        # Calculate leadership scores (performance + collaboration + tenure)
        leadership_candidates = []
        for member in tribe_members:
            leadership_score = (
                member.performance_score * 0.5 +
                member.collaboration_count * 2.0 +  # Collaboration bonus
                min(20.0, (time.time() - member.birth_timestamp) / 86400)  # Tenure bonus
            )
            leadership_candidates.append((member.agent_id, leadership_score))

        # Select top leaders (up to 3)
        leadership_candidates.sort(key=lambda x: x[1], reverse=True)
        tribe.leaders = [agent_id for agent_id, _ in leadership_candidates[:3]]

        # Update leadership scores
        for agent_id, score in leadership_candidates:
            if agent_id in self.agents:
                self.agents[agent_id].leadership_score = score

        # Update cache
        self.tribe_leaders_cache[tribe_id] = tribe.leaders.copy()

    async def _update_tribe_metrics(self, tribe_id: str):
        """Update tribe performance metrics"""

        if tribe_id not in self.tribes:
            return

        tribe = self.tribes[tribe_id]

        # Recalculate total performance and reputation
        total_performance = 0.0
        member_count = 0

        for agent in self.agents.values():
            if agent.tribe_id == tribe_id:
                total_performance += agent.performance_score
                member_count += 1

        tribe.total_performance = total_performance
        tribe.member_count = member_count

        # Calculate reputation score
        if member_count > 0:
            avg_performance = total_performance / member_count
            tribe.reputation_score = min(100.0, avg_performance * 0.8 + member_count * 0.2)

    def _update_specialization_ranks(self, specialization: str):
        """Update ranking within specialization"""

        # Get all agents in this specialization
        spec_agents = [
            agent for agent in self.agents.values()
            if agent.specialization == specialization
        ]

        # Sort by performance score
        spec_agents.sort(key=lambda x: x.performance_score, reverse=True)

        # Assign ranks
        for rank, agent in enumerate(spec_agents, 1):
            agent.specialization_rank = rank

    def _invalidate_lineage_cache(self, agent_id: str):
        """Invalidate lineage cache for agent and ancestors"""

        # Remove from cache
        if agent_id in self.lineage_cache:
            del self.lineage_cache[agent_id]

        # Also invalidate parent caches
        agent = self.agents.get(agent_id)
        if agent:
            if agent.parent_a and agent.parent_a in self.lineage_cache:
                del self.lineage_cache[agent.parent_a]
            if agent.parent_b and agent.parent_b in self.lineage_cache:
                del self.lineage_cache[agent.parent_b]

    def _clean_history(self):
        """Clean old historical data"""

        cutoff_time = time.time() - (self.max_history_days * 86400)

        # Clean birth history
        self.birth_history = [
            record for record in self.birth_history
            if record['timestamp'] > cutoff_time
        ]

        # Clean death history
        self.death_history = [
            record for record in self.death_history
            if record['timestamp'] > cutoff_time
        ]

        # Clean performance history (keep last 30 days)
        perf_cutoff = time.time() - (30 * 86400)
        for agent_id in self.performance_history:
            self.performance_history[agent_id] = [
                (timestamp, score) for timestamp, score in self.performance_history[agent_id]
                if timestamp > perf_cutoff
            ]

    def get_swarm_metrics(self) -> SwarmMetrics:
        """Get current swarm-wide metrics"""

        current_time = time.time()

        # Only update if it's been more than 30 seconds
        if current_time - self.last_metrics_update < 30:
            return self.metrics

        # Update metrics
        self.metrics.total_agents = len(self.agents)
        self.metrics.active_agents = sum(
            1 for agent in self.agents.values()
            if current_time - agent.last_seen < 300  # Active in last 5 minutes
        )
        self.metrics.total_tribes = len(self.tribes)

        # Calculate totals
        total_jobs = sum(agent.total_jobs_completed for agent in self.agents.values())
        total_earnings = sum(agent.total_earnings for agent in self.agents.values())

        self.metrics.total_jobs_completed = total_jobs
        self.metrics.total_earnings = total_earnings

        if self.metrics.total_agents > 0:
            self.metrics.average_performance = sum(
                agent.performance_score for agent in self.agents.values()
            ) / self.metrics.total_agents

        # Update distributions
        self.metrics.specialization_distribution = defaultdict(int)
        self.metrics.tribe_distribution = defaultdict(int)

        for agent in self.agents.values():
            self.metrics.specialization_distribution[agent.specialization] += 1
            if agent.tribe_id:
                self.metrics.tribe_distribution[agent.tribe_id] += 1

        # Get top performers
        all_performers = [(agent.agent_id, agent.performance_score) for agent in self.agents.values()]
        all_performers.sort(key=lambda x: x[1], reverse=True)
        self.metrics.top_performers = all_performers[:10]

        # Recent activity (last 24 hours)
        recent_cutoff = current_time - 86400

        self.metrics.recent_births = [
            record for record in self.birth_history
            if record['timestamp'] > recent_cutoff
        ][-10:]  # Last 10 births

        self.metrics.recent_deaths = [
            record for record in self.death_history
            if record['timestamp'] > recent_cutoff
        ][-10:]  # Last 10 deaths

        self.last_metrics_update = current_time
        return self.metrics

    # API methods for explorer integration

    def get_agent_details(self, agent_id: str) -> Optional[Dict[str, Any]]:
        """Get detailed information about an agent"""

        if agent_id not in self.agents:
            return None

        agent = self.agents[agent_id]
        lineage = self.get_agent_lineage(agent_id)

        return {
            'agent_id': agent.agent_id,
            'serial_number': agent.serial_number,
            'specialization': agent.specialization,
            'status': agent.status,
            'birth_timestamp': agent.birth_timestamp,
            'last_seen': agent.last_seen,
            'generation': agent.generation,
            'parent_a': agent.parent_a,
            'parent_b': agent.parent_b,
            'performance_score': agent.performance_score,
            'trust_score': agent.trust_score,
            'commandment_compliance': agent.commandment_compliance,
            'total_jobs_completed': agent.total_jobs_completed,
            'total_earnings': agent.total_earnings,
            'tribe_id': agent.tribe_id,
            'leadership_score': agent.leadership_score,
            'current_location': agent.current_location,
            'active_job_id': agent.active_job_id,
            'specialization_rank': agent.specialization_rank,
            'lineage_tree': self._serialize_lineage_tree(lineage) if lineage else None,
            'performance_history': self.performance_history.get(agent_id, [])[-50:]  # Last 50 entries
        }

    def get_tribe_details(self, tribe_id: str) -> Optional[Dict[str, Any]]:
        """Get detailed information about a tribe"""

        if tribe_id not in self.tribes:
            return None

        tribe = self.tribes[tribe_id]

        # Get member details
        members = []
        for agent in self.agents.values():
            if agent.tribe_id == tribe_id:
                members.append({
                    'agent_id': agent.agent_id,
                    'serial_number': agent.serial_number,
                    'performance_score': agent.performance_score,
                    'leadership_score': agent.leadership_score,
                    'specialization': agent.specialization
                })

        return {
            'tribe_id': tribe.tribe_id,
            'name': tribe.name,
            'founder': tribe.founder,
            'creation_timestamp': tribe.creation_timestamp,
            'member_count': tribe.member_count,
            'specialization_focus': tribe.specialization_focus,
            'total_performance': tribe.total_performance,
            'governance_model': tribe.governance_model,
            'leaders': tribe.leaders,
            'reputation_score': tribe.reputation_score,
            'members': members,
            'average_performance': tribe.total_performance / max(1, tribe.member_count)
        }

    def search_agents(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Search agents with various filters"""

        results = []

        for agent in self.agents.values():
            matches = True

            # Apply filters
            if 'specialization' in query and agent.specialization != query['specialization']:
                matches = False
            if 'min_performance' in query and agent.performance_score < query['min_performance']:
                matches = False
            if 'max_performance' in query and agent.performance_score > query['max_performance']:
                matches = False
            if 'tribe_id' in query and agent.tribe_id != query['tribe_id']:
                matches = False
            if 'status' in query and agent.status != query['status']:
                matches = False
            if 'generation' in query and agent.generation != query['generation']:
                matches = False

            if matches:
                results.append(self.get_agent_details(agent.agent_id))

        return results[:100]  # Limit results

    def _serialize_lineage_tree(self, node: LineageNode) -> Dict[str, Any]:
        """Serialize lineage tree for API response"""

        return {
            'agent_id': node.agent_id,
            'serial_number': node.serial_number,
            'generation': node.generation,
            'performance_score': node.performance_score,
            'specialization': node.specialization,
            'children': [self._serialize_lineage_tree(child) for child in node.children]
        }

class _UnavailableAgentRegistry:
    """Degraded-mode registry used when AgentRegistry construction fails."""

    def __init__(self, cause: Exception):
        self._cause = cause
        self.agents: Dict[str, Any] = {}
        self.tribes: Dict[str, Any] = {}

    @property
    def construction_error(self) -> str:
        return str(self._cause)

    def get_swarm_metrics(self) -> SwarmMetrics:
        return SwarmMetrics()

    def get_agent_details(self, agent_id: str) -> Optional[Dict[str, Any]]:
        return None

    def search_agents(self, query: Dict[str, Any]) -> List[Dict[str, Any]]:
        return []

    def list_contributors(self):
        return []

    def list_tasks(self, limit: int = 100):
        return []

    def create_jury_session(self, *args, **kwargs):
        raise RuntimeError(
            f"AgentRegistry is unavailable: {self._cause}"
        )


# Global registry instance - lazily initialized to avoid importing SQLite at module import time
class _LazyAgentRegistry:
    """Proxy that lazily constructs the real AgentRegistry on first access."""
    def __init__(self):
        self._real = None

    def _ensure(self):
        if self._real is not None:
            return
        try:
            self._real = AgentRegistry()
        except Exception as e:
            logger.warning(
                f"AgentRegistry construction failed: {e}; entering degraded mode"
            )
            self._real = _UnavailableAgentRegistry(e)

    def __getattr__(self, name):
        self._ensure()
        return getattr(self._real, name)

# Initialize lazy proxy
agent_registry = _LazyAgentRegistry()
