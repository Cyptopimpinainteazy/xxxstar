"""CausalGraph — directed acyclic graph of agent cause → effect relationships.

The graph is the central data structure of the Causal Analysis Layer.
It stores nodes (actions, consequences, predictions, deaths, etc.)
and directed edges (cause → effect) between them.

Key operations:
- add_node / add_edge — build the graph as events happen
- ancestors / descendants — traverse the DAG
- get_chain — extract the full causal chain ending at a node
- subgraph — extract a subgraph for a time window or agent
- roots / leaves — find entry/exit points

Persistence: all nodes and edges are written to StorageBackend
so the graph survives process restarts.

NON-NEGOTIABLE:
- Edges are directed: cause_node_id → effect_node_id
- No cycles allowed (validated on add_edge)
- Nodes are append-only (never modified after creation)
"""

from __future__ import annotations

import logging
from collections import defaultdict, deque
from typing import Any, Dict, List, Optional, Set

from swarm.causal.schema import (
    CausalChain,
    CausalEdge,
    CausalNode,
    EdgeType,
    NodeType,
)
from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend

logger = logging.getLogger(__name__)

NODE_NS = "causal_nodes"
EDGE_NS = "causal_edges"


class CausalGraph:
    """Persistent directed acyclic graph of causal relationships.

    Args:
        storage: Persistence backend.
        agent_id: If set, scopes the graph to a single agent.
                  If None, this is a swarm-wide graph.
    """

    def __init__(
        self,
        storage: StorageBackend,
        agent_id: Optional[str] = None,
    ) -> None:
        self._storage = storage
        self._agent_id = agent_id

        # In-memory indexes for fast traversal
        self._nodes: Dict[str, CausalNode] = {}
        self._edges: Dict[str, CausalEdge] = {}

        # Adjacency lists
        self._outgoing: Dict[str, List[str]] = defaultdict(list)  # node → [edge_ids]
        self._incoming: Dict[str, List[str]] = defaultdict(list)  # node → [edge_ids]

        # Agent index
        self._agent_nodes: Dict[str, List[str]] = defaultdict(list)  # agent → [node_ids]

        # Epoch index
        self._epoch_nodes: Dict[int, List[str]] = defaultdict(list)  # epoch → [node_ids]

        # Pending bus events
        self._pending_bus_events: List[BusEvent] = []

        # Rebuild indexes from storage
        self._rebuild_indexes()

    # ------------------------------------------------------------------
    # Graph construction
    # ------------------------------------------------------------------

    def add_node(self, node: CausalNode) -> CausalNode:
        """Add a causal node to the graph.

        If the node_id already exists, returns the existing node.
        """
        if node.node_id in self._nodes:
            return self._nodes[node.node_id]

        # Persist
        self._storage.save(NODE_NS, node.node_id, node.model_dump(mode="json"))

        # Index
        self._nodes[node.node_id] = node
        self._agent_nodes[node.agent_id].append(node.node_id)
        self._epoch_nodes[node.epoch].append(node.node_id)

        logger.debug(
            "Causal node added: %s type=%s agent=%s epoch=%d",
            node.node_id[:8],
            node.node_type,
            node.agent_id,
            node.epoch,
        )

        return node

    def add_edge(
        self,
        cause_id: str,
        effect_id: str,
        edge_type: EdgeType = EdgeType.DIRECT,
        weight: float = 1.0,
        confidence: float = 1.0,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> CausalEdge:
        """Add a directed edge from cause → effect.

        Raises ValueError if:
        - Either node doesn't exist
        - Adding the edge would create a cycle
        - The edge already exists (duplicate)
        """
        if cause_id not in self._nodes:
            raise ValueError(f"Cause node {cause_id} not found in graph")
        if effect_id not in self._nodes:
            raise ValueError(f"Effect node {effect_id} not found in graph")

        # Check for duplicate
        for eid in self._outgoing.get(cause_id, []):
            edge = self._edges[eid]
            if edge.effect_node_id == effect_id:
                return edge  # Already exists

        # Cycle detection: would adding cause→effect create a path effect→cause?
        if self._has_path(effect_id, cause_id):
            raise ValueError(
                f"Adding edge {cause_id[:8]}→{effect_id[:8]} "
                f"would create a cycle"
            )

        cause_node = self._nodes[cause_id]
        effect_node = self._nodes[effect_id]
        lag = max(0, effect_node.epoch - cause_node.epoch)

        edge = CausalEdge(
            cause_node_id=cause_id,
            effect_node_id=effect_id,
            edge_type=edge_type,
            weight=weight,
            confidence=confidence,
            lag_epochs=lag,
            metadata=metadata or {},
        )

        # Persist
        self._storage.save(EDGE_NS, edge.edge_id, edge.model_dump(mode="json"))

        # Index
        self._edges[edge.edge_id] = edge
        self._outgoing[cause_id].append(edge.edge_id)
        self._incoming[effect_id].append(edge.edge_id)

        logger.debug(
            "Causal edge added: %s → %s type=%s weight=%.2f",
            cause_id[:8],
            effect_id[:8],
            edge_type,
            weight,
        )

        return edge

    # ------------------------------------------------------------------
    # Node retrieval
    # ------------------------------------------------------------------

    def get_node(self, node_id: str) -> Optional[CausalNode]:
        """Get a node by ID."""
        return self._nodes.get(node_id)

    def get_nodes_for_agent(self, agent_id: str) -> List[CausalNode]:
        """Get all nodes for a specific agent, chronologically."""
        node_ids = self._agent_nodes.get(agent_id, [])
        nodes = [self._nodes[nid] for nid in node_ids if nid in self._nodes]
        return sorted(nodes, key=lambda n: (n.epoch, n.timestamp))

    def get_nodes_for_epoch(self, epoch: int) -> List[CausalNode]:
        """Get all nodes in a specific epoch."""
        node_ids = self._epoch_nodes.get(epoch, [])
        return [self._nodes[nid] for nid in node_ids if nid in self._nodes]

    def get_nodes_by_type(
        self, node_type: NodeType, agent_id: Optional[str] = None
    ) -> List[CausalNode]:
        """Get all nodes of a given type, optionally filtered by agent."""
        candidates = (
            self.get_nodes_for_agent(agent_id)
            if agent_id
            else list(self._nodes.values())
        )
        return [n for n in candidates if n.node_type == node_type.value]

    @property
    def node_count(self) -> int:
        return len(self._nodes)

    @property
    def edge_count(self) -> int:
        return len(self._edges)

    # ------------------------------------------------------------------
    # Graph traversal
    # ------------------------------------------------------------------

    def ancestors(self, node_id: str, max_depth: int = 50) -> List[CausalNode]:
        """Get all ancestors (causes) of a node via BFS, ordered by depth."""
        if node_id not in self._nodes:
            return []

        visited: Set[str] = set()
        result: List[CausalNode] = []
        queue: deque = deque([(node_id, 0)])

        while queue:
            current_id, depth = queue.popleft()
            if depth > max_depth:
                continue

            for edge_id in self._incoming.get(current_id, []):
                edge = self._edges[edge_id]
                cause_id = edge.cause_node_id
                if cause_id not in visited:
                    visited.add(cause_id)
                    result.append(self._nodes[cause_id])
                    queue.append((cause_id, depth + 1))

        # Sort: deepest ancestors first (root causes)
        return list(reversed(result))

    def descendants(self, node_id: str, max_depth: int = 50) -> List[CausalNode]:
        """Get all descendants (effects) of a node via BFS."""
        if node_id not in self._nodes:
            return []

        visited: Set[str] = set()
        result: List[CausalNode] = []
        queue: deque = deque([(node_id, 0)])

        while queue:
            current_id, depth = queue.popleft()
            if depth > max_depth:
                continue

            for edge_id in self._outgoing.get(current_id, []):
                edge = self._edges[edge_id]
                effect_id = edge.effect_node_id
                if effect_id not in visited:
                    visited.add(effect_id)
                    result.append(self._nodes[effect_id])
                    queue.append((effect_id, depth + 1))

        return result

    def direct_causes(self, node_id: str) -> List[CausalNode]:
        """Get immediate parent nodes (direct causes)."""
        result = []
        for edge_id in self._incoming.get(node_id, []):
            edge = self._edges[edge_id]
            node = self._nodes.get(edge.cause_node_id)
            if node:
                result.append(node)
        return result

    def direct_effects(self, node_id: str) -> List[CausalNode]:
        """Get immediate child nodes (direct effects)."""
        result = []
        for edge_id in self._outgoing.get(node_id, []):
            edge = self._edges[edge_id]
            node = self._nodes.get(edge.effect_node_id)
            if node:
                result.append(node)
        return result

    def roots(self, agent_id: Optional[str] = None) -> List[CausalNode]:
        """Get root nodes (no incoming edges)."""
        candidates = (
            self.get_nodes_for_agent(agent_id)
            if agent_id
            else list(self._nodes.values())
        )
        return [n for n in candidates if not self._incoming.get(n.node_id)]

    def leaves(self, agent_id: Optional[str] = None) -> List[CausalNode]:
        """Get leaf nodes (no outgoing edges)."""
        candidates = (
            self.get_nodes_for_agent(agent_id)
            if agent_id
            else list(self._nodes.values())
        )
        return [n for n in candidates if not self._outgoing.get(n.node_id)]

    # ------------------------------------------------------------------
    # Chain extraction
    # ------------------------------------------------------------------

    def get_chain_to(self, target_node_id: str, max_depth: int = 20) -> CausalChain:
        """Extract the causal chain ending at target_node_id.

        Follows the highest-weight incoming edge at each step,
        producing the "most likely" causal chain.
        """
        if target_node_id not in self._nodes:
            target = self._nodes.get(target_node_id)
            agent_id = target.agent_id if target else "unknown"
            return CausalChain(agent_id=agent_id)

        target = self._nodes[target_node_id]
        chain_nodes: List[CausalNode] = [target]
        chain_edges: List[CausalEdge] = []
        total_weight = 1.0

        current_id = target_node_id
        visited: Set[str] = {current_id}

        for _ in range(max_depth):
            incoming = self._incoming.get(current_id, [])
            if not incoming:
                break

            # Follow the highest-weight incoming edge
            best_edge = max(
                (self._edges[eid] for eid in incoming),
                key=lambda e: e.weight * e.confidence,
            )

            cause_id = best_edge.cause_node_id
            if cause_id in visited:
                break  # Safety: prevent infinite loops
            visited.add(cause_id)

            chain_nodes.append(self._nodes[cause_id])
            chain_edges.append(best_edge)
            total_weight *= best_edge.weight
            current_id = cause_id

        # Reverse to get root-cause → effect order
        chain_nodes.reverse()
        chain_edges.reverse()

        return CausalChain(
            agent_id=target.agent_id,
            nodes=chain_nodes,
            edges=chain_edges,
            total_weight=total_weight,
            depth=len(chain_edges),
        )

    def get_all_chains_to(
        self, target_node_id: str, max_depth: int = 10, max_chains: int = 10
    ) -> List[CausalChain]:
        """Extract ALL causal chains ending at target_node_id (DFS).

        Returns up to max_chains chains, sorted by total_weight descending.
        """
        if target_node_id not in self._nodes:
            return []

        target = self._nodes[target_node_id]
        chains: List[CausalChain] = []

        def _dfs(
            node_id: str,
            path_nodes: List[CausalNode],
            path_edges: List[CausalEdge],
            weight: float,
            depth: int,
        ) -> None:
            if len(chains) >= max_chains:
                return
            if depth >= max_depth:
                return

            incoming = self._incoming.get(node_id, [])
            if not incoming:
                # Root reached — emit chain
                nodes = list(reversed(path_nodes))
                edges = list(reversed(path_edges))
                chains.append(
                    CausalChain(
                        agent_id=target.agent_id,
                        nodes=nodes,
                        edges=edges,
                        total_weight=weight,
                        depth=len(edges),
                    )
                )
                return

            for edge_id in incoming:
                edge = self._edges[edge_id]
                cause_id = edge.cause_node_id
                if cause_id in {n.node_id for n in path_nodes}:
                    continue  # Skip cycles

                cause_node = self._nodes[cause_id]
                _dfs(
                    cause_id,
                    path_nodes + [cause_node],
                    path_edges + [edge],
                    weight * edge.weight,
                    depth + 1,
                )

        _dfs(target_node_id, [target], [], 1.0, 0)

        # If no incoming edges found, return single-node chain
        if not chains:
            chains.append(
                CausalChain(
                    agent_id=target.agent_id,
                    nodes=[target],
                    edges=[],
                    total_weight=1.0,
                    depth=0,
                )
            )

        return sorted(chains, key=lambda c: c.total_weight, reverse=True)

    # ------------------------------------------------------------------
    # Subgraph extraction
    # ------------------------------------------------------------------

    def subgraph(
        self,
        agent_id: Optional[str] = None,
        epoch_start: Optional[int] = None,
        epoch_end: Optional[int] = None,
        node_types: Optional[List[NodeType]] = None,
    ) -> "CausalGraph":
        """Extract a subgraph filtered by agent, epoch range, and/or node type.

        Returns a new CausalGraph (in-memory only, not persisted).
        """
        from swarm.storage.backend import SqliteStorage
        sub_storage = SqliteStorage(":memory:")
        sub = CausalGraph(storage=sub_storage)

        # Filter nodes
        for node in self._nodes.values():
            if agent_id and node.agent_id != agent_id:
                continue
            if epoch_start is not None and node.epoch < epoch_start:
                continue
            if epoch_end is not None and node.epoch > epoch_end:
                continue
            if node_types:
                type_values = [nt.value if hasattr(nt, 'value') else nt for nt in node_types]
                if node.node_type not in type_values:
                    continue
            sub.add_node(node)

        # Add edges where both endpoints are in the subgraph
        for edge in self._edges.values():
            if (
                edge.cause_node_id in sub._nodes
                and edge.effect_node_id in sub._nodes
            ):
                sub._edges[edge.edge_id] = edge
                sub._outgoing[edge.cause_node_id].append(edge.edge_id)
                sub._incoming[edge.effect_node_id].append(edge.edge_id)

        return sub

    # ------------------------------------------------------------------
    # Metrics
    # ------------------------------------------------------------------

    def node_influence(self, node_id: str) -> float:
        """Compute the influence score of a node.

        Influence = number of descendants weighted by edge weights.
        Higher influence means this action had wider downstream effects.
        """
        if node_id not in self._nodes:
            return 0.0

        total = 0.0
        visited: Set[str] = set()
        queue: deque = deque([(node_id, 1.0)])

        while queue:
            current_id, weight = queue.popleft()
            for edge_id in self._outgoing.get(current_id, []):
                edge = self._edges[edge_id]
                effect_id = edge.effect_node_id
                if effect_id not in visited:
                    visited.add(effect_id)
                    propagated_weight = weight * edge.weight
                    total += propagated_weight
                    queue.append((effect_id, propagated_weight))

        return total

    def critical_path(self, agent_id: str) -> List[CausalNode]:
        """Find the highest-influence path through an agent's causal history.

        Returns the sequence of nodes that had the most downstream impact.
        """
        agent_nodes = self.get_nodes_for_agent(agent_id)
        if not agent_nodes:
            return []

        # Score each node by influence
        scored = [(n, self.node_influence(n.node_id)) for n in agent_nodes]
        scored.sort(key=lambda x: x[1], reverse=True)

        # Greedily build the critical path
        path: List[CausalNode] = []
        for node, _influence in scored:
            if not path or node.epoch >= path[-1].epoch:
                path.append(node)

        return sorted(path, key=lambda n: (n.epoch, n.timestamp))

    # ------------------------------------------------------------------
    # Bus events
    # ------------------------------------------------------------------

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _has_path(self, from_id: str, to_id: str) -> bool:
        """Check if there's any path from from_id to to_id (cycle detection)."""
        if from_id == to_id:
            return True

        visited: Set[str] = set()
        queue: deque = deque([from_id])

        while queue:
            current = queue.popleft()
            if current == to_id:
                return True
            if current in visited:
                continue
            visited.add(current)

            for edge_id in self._outgoing.get(current, []):
                edge = self._edges[edge_id]
                queue.append(edge.effect_node_id)

        return False

    def _rebuild_indexes(self) -> None:
        """Rebuild in-memory indexes from storage."""
        # Load all nodes
        node_keys = self._storage.list_keys(NODE_NS)
        for key in node_keys:
            data = self._storage.load(NODE_NS, key)
            if data:
                node = CausalNode.model_validate(data)
                self._nodes[node.node_id] = node
                self._agent_nodes[node.agent_id].append(node.node_id)
                self._epoch_nodes[node.epoch].append(node.node_id)

        # Load all edges
        edge_keys = self._storage.list_keys(EDGE_NS)
        for key in edge_keys:
            data = self._storage.load(EDGE_NS, key)
            if data:
                edge = CausalEdge.model_validate(data)
                self._edges[edge.edge_id] = edge
                self._outgoing[edge.cause_node_id].append(edge.edge_id)
                self._incoming[edge.effect_node_id].append(edge.edge_id)

        if node_keys or edge_keys:
            logger.info(
                "CausalGraph rebuilt: %d nodes, %d edges",
                len(self._nodes),
                len(self._edges),
            )
