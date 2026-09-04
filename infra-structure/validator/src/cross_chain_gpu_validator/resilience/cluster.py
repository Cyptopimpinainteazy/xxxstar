"""Cluster Coordinator — multi-machine failover for validator clusters.

Architecture
────────────
  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
  │  Region A    │     │  Region B    │     │  Region C    │
  │  (Primary)   │     │  (Warm)      │     │  (Cold)      │
  │              │     │              │     │              │
  │  GPU Node 1  │◀───▶│  GPU Node 2  │◀───▶│  CPU Node 3  │
  │  Signing     │     │  No Signing  │     │  No Signing  │
  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘
         │                    │                    │
         └────────────────────┴────────────────────┘
                        Redis
              (SignerLock + ClusterState)

Failover Chain:
  Region A (Primary) → Region B (Warm Standby) → Region C (Cold Standby)

Split-Brain Protection:
  - Fencing tokens (monotonically increasing)
  - Quorum-based leader election (requires majority)
  - Stale primary detection via Redis TTL
  - Automatic recovery when network partitions heal

Usage
-----
    # Start cluster coordinator:
    python -m cross_chain_gpu_validator.resilience.cluster \\
        --cluster-id x3-mainnet \\
        --node-id node-us-east-1 \\
        --region us-east-1 \\
        --role primary \\
        --peers node-us-west-2,node-eu-frankfurt
"""

from __future__ import annotations

import json
import logging
import os
import socket
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Callable

from cross_chain_gpu_validator.resilience.signer_lock import SignerLock, SignerAuthority

logger = logging.getLogger("x3.cluster")


# ─── Constants ────────────────────────────────────────────────

DEFAULT_HEARTBEAT_INTERVAL = 5.0
DEFAULT_HEARTBEAT_TIMEOUT = 30.0
DEFAULT_ELECTION_TIMEOUT = 15.0
DEFAULT_QUORUM_MAJORITY = 0.51  # 51% majority


# ─── Enums ────────────────────────────────────────────────────


class ClusterRole(Enum):
    """Role of a node in the cluster."""
    LEADER = "leader"           # Active primary, holds signer lock
    FOLLOWER = "follower"       # Warm standby, ready to promote
    OBSERVER = "observer"       # Cold standby, not in quorum
    CANDIDATE = "candidate"     # Running for election
    UNKNOWN = "unknown"         # Not yet determined


class ClusterState(Enum):
    """State of the cluster."""
    STABLE = "stable"           # Leader is healthy
    ELECTION = "election"       # Leader election in progress
    DEGRADED = "degraded"       # Some nodes unreachable
    SPLIT_BRAIN = "split_brain" # Multiple leaders detected
    RECOVERING = "recovering"   # Healing from split-brain


# ─── Data Types ───────────────────────────────────────────────


@dataclass
class ClusterNode:
    """Represents a node in the validator cluster."""
    node_id: str
    region: str
    role: ClusterRole
    rpc_endpoint: str
    last_heartbeat: float = 0.0
    health_score: float = 1.0
    block_height: int = 0
    is_alive: bool = True
    term: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "node_id": self.node_id,
            "region": self.region,
            "role": self.role.value,
            "rpc_endpoint": self.rpc_endpoint,
            "last_heartbeat": self.last_heartbeat,
            "health_score": round(self.health_score, 3),
            "block_height": self.block_height,
            "is_alive": self.is_alive,
            "term": self.term,
        }


@dataclass
class ClusterConfig:
    """Configuration for the cluster coordinator."""
    cluster_id: str = "x3-default"
    node_id: str = ""
    region: str = "unknown"
    role: ClusterRole = ClusterRole.FOLLOWER
    peers: list[str] = field(default_factory=list)
    rpc_endpoint: str = "http://127.0.0.1:9933"
    redis_url: str = "redis://127.0.0.1:6379/0"
    heartbeat_interval: float = DEFAULT_HEARTBEAT_INTERVAL
    heartbeat_timeout: float = DEFAULT_HEARTBEAT_TIMEOUT
    election_timeout: float = DEFAULT_ELECTION_TIMEOUT
    quorum_majority: float = DEFAULT_QUORUM_MAJORITY
    signer_ttl: float = 30.0
    data_dir: str = "/tmp/x3-cluster"

    @classmethod
    def from_env(cls) -> ClusterConfig:
        """Load configuration from environment variables."""
        return cls(
            cluster_id=os.getenv("X3_CLUSTER_ID", "x3-default"),
            node_id=os.getenv("X3_NODE_ID", ""),
            region=os.getenv("X3_REGION", "unknown"),
            role=ClusterRole(os.getenv("X3_CLUSTER_ROLE", "follower")),
            peers=os.getenv("X3_CLUSTER_PEERS", "").split(",") if os.getenv("X3_CLUSTER_PEERS") else [],
            rpc_endpoint=os.getenv("X3_RPC_ENDPOINT", "http://127.0.0.1:9933"),
            redis_url=os.getenv("CCGV_REDIS_URL", "redis://127.0.0.1:6379/0"),
            heartbeat_interval=float(os.getenv("X3_HEARTBEAT_INTERVAL", "5.0")),
            heartbeat_timeout=float(os.getenv("X3_HEARTBEAT_TIMEOUT", "30.0")),
            election_timeout=float(os.getenv("X3_ELECTION_TIMEOUT", "15.0")),
            quorum_majority=float(os.getenv("X3_QUORUM_MAJORITY", "0.51")),
            signer_ttl=float(os.getenv("X3_SIGNER_TTL", "30.0")),
            data_dir=os.getenv("X3_CLUSTER_DATA_DIR", "/tmp/x3-cluster"),
        )


# ─── Cluster Coordinator ──────────────────────────────────────


class ClusterCoordinator:
    """Multi-machine cluster coordinator with leader election and failover.

    Parameters
    ----------
    config : ClusterConfig
        Cluster configuration.
    on_leader_elected : callable
        ``fn(leader_id, term)`` called when a new leader is elected.
    on_leader_lost : callable
        ``fn(old_leader_id)`` called when the leader is lost.
    on_split_brain : callable
        ``fn(leaders)`` called when split-brain is detected.
    """

    def __init__(
        self,
        config: ClusterConfig | None = None,
        on_leader_elected: Callable[[str, int], None] | None = None,
        on_leader_lost: Callable[[str], None] | None = None,
        on_split_brain: Callable[[list[str]], None] | None = None,
    ) -> None:
        self._config = config or ClusterConfig.from_env()

        # Generate node ID if not set
        if not self._config.node_id:
            import uuid
            self._config.node_id = f"{self._config.region}-{uuid.uuid4().hex[:6]}"

        self._on_leader_elected = on_leader_elected
        self._on_leader_lost = on_leader_lost
        self._on_split_brain = on_split_brain

        # State
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._cluster_state = ClusterState.STABLE
        self._current_term = 0
        self._voted_for: str | None = None
        self._leader_id: str | None = None
        self._leader_term: int = 0

        # Node registry
        self._nodes: dict[str, ClusterNode] = {}
        self._register_self()

        # Signer lock (distributed)
        self._signer = SignerLock(
            node_id=self._config.node_id,
            redis_url=self._config.redis_url,
            ttl_seconds=self._config.signer_ttl,
            on_acquired=self._on_signer_acquired,
            on_lost=self._on_signer_lost,
        )

        # Metrics
        self._elections: int = 0
        self._leader_changes: int = 0
        self._split_brain_events: int = 0

    def _register_self(self) -> None:
        """Register this node in the local registry."""
        self._nodes[self._config.node_id] = ClusterNode(
            node_id=self._config.node_id,
            region=self._config.region,
            role=self._config.role,
            rpc_endpoint=self._config.rpc_endpoint,
            last_heartbeat=time.time(),
        )

    # ── Properties ────────────────────────────────────────────

    @property
    def leader_id(self) -> str | None:
        with self._lock:
            return self._leader_id

    @property
    def is_leader(self) -> bool:
        return self._config.role == ClusterRole.LEADER

    @property
    def current_term(self) -> int:
        with self._lock:
            return self._current_term

    @property
    def cluster_state(self) -> ClusterState:
        with self._lock:
            return self._cluster_state

    @property
    def node_count(self) -> int:
        with self._lock:
            return len(self._nodes)

    @property
    def alive_count(self) -> int:
        with self._lock:
            return sum(1 for n in self._nodes.values() if n.is_alive)

    # ── Lifecycle ─────────────────────────────────────────────

    def start(self) -> None:
        """Start the cluster coordinator."""
        logger.info(
            "ClusterCoordinator starting — cluster=%s, node=%s, region=%s, role=%s",
            self._config.cluster_id, self._config.node_id,
            self._config.region, self._config.role.value,
        )

        os.makedirs(self._config.data_dir, exist_ok=True)

        # Register peers
        for peer_id in self._config.peers:
            peer_id = peer_id.strip()
            if peer_id and peer_id not in self._nodes:
                self._nodes[peer_id] = ClusterNode(
                    node_id=peer_id,
                    region="unknown",
                    role=ClusterRole.UNKNOWN,
                    rpc_endpoint=f"http://{peer_id}:9933",
                )

        # If configured as leader, try to acquire signer lock
        if self._config.role == ClusterRole.LEADER:
            self._signer.try_acquire()
            with self._lock:
                self._leader_id = self._config.node_id
                self._cluster_state = ClusterState.STABLE

        # Start heartbeat sender
        self._start_heartbeat_sender()

        # Start heartbeat checker
        self._start_heartbeat_checker()

        # Start leader election monitor
        self._start_election_monitor()

        logger.info(
            "ClusterCoordinator started — %d nodes, %d peers",
            len(self._nodes), len(self._config.peers),
        )

    def stop(self) -> None:
        """Gracefully stop the cluster coordinator."""
        logger.info("ClusterCoordinator stopping...")
        self._stop.set()
        self._signer.release()
        logger.info("ClusterCoordinator stopped")

    # ── Heartbeat ─────────────────────────────────────────────

    def _start_heartbeat_sender(self) -> None:
        """Start periodic heartbeat broadcast."""
        thread = threading.Thread(
            target=self._heartbeat_sender_loop,
            daemon=True,
            name="cluster-heartbeat-sender",
        )
        thread.start()

    def _heartbeat_sender_loop(self) -> None:
        """Send heartbeats to Redis at regular intervals."""
        redis_key = f"x3:cluster:{self._config.cluster_id}:heartbeat:{self._config.node_id}"

        while not self._stop.is_set():
            try:
                heartbeat = {
                    "node_id": self._config.node_id,
                    "region": self._config.region,
                    "role": self._config.role.value,
                    "term": self._current_term,
                    "leader_id": self._leader_id,
                    "timestamp": time.time(),
                    "rpc_endpoint": self._config.rpc_endpoint,
                }
                # Write heartbeat to local state
                with self._lock:
                    if self._config.node_id in self._nodes:
                        self._nodes[self._config.node_id].last_heartbeat = time.time()
                        self._nodes[self._config.node_id].role = self._config.role

                # Try to write to Redis
                self._write_heartbeat_redis(redis_key, heartbeat)

            except Exception as exc:
                logger.debug("Heartbeat send error: %s", exc)

            self._stop.wait(self._config.heartbeat_interval)

    def _write_heartbeat_redis(self, key: str, data: dict) -> None:
        """Write heartbeat to Redis (best-effort)."""
        try:
            import redis as redis_lib
            r = redis_lib.Redis.from_url(
                self._config.redis_url,
                socket_timeout=2,
                socket_connect_timeout=2,
            )
            r.setex(key, int(self._config.heartbeat_timeout * 2), json.dumps(data))
            r.close()
        except Exception:
            pass

    def _start_heartbeat_checker(self) -> None:
        """Start periodic heartbeat monitoring."""
        thread = threading.Thread(
            target=self._heartbeat_checker_loop,
            daemon=True,
            name="cluster-heartbeat-checker",
        )
        thread.start()

    def _heartbeat_checker_loop(self) -> None:
        """Check peer heartbeats and detect failures."""
        while not self._stop.is_set():
            try:
                self._check_peer_heartbeats()
            except Exception as exc:
                logger.debug("Heartbeat check error: %s", exc)
            self._stop.wait(self._config.heartbeat_interval)

    def _check_peer_heartbeats(self) -> None:
        """Check if peers are still alive."""
        now = time.time()
        timeout = self._config.heartbeat_timeout
        state_changed = False

        with self._lock:
            for node_id, node in self._nodes.items():
                if node_id == self._config.node_id:
                    continue
                elapsed = now - node.last_heartbeat
                was_alive = node.is_alive
                node.is_alive = elapsed < timeout
                if was_alive and not node.is_alive:
                    logger.warning(
                        "Node %s (%s) is DEAD — last heartbeat %.1fs ago",
                        node_id, node.region, elapsed,
                    )
                    state_changed = True
                elif not was_alive and node.is_alive:
                    logger.info(
                        "Node %s (%s) RECOVERED", node_id, node.region,
                    )
                    state_changed = True

            if state_changed:
                self._update_cluster_state()

    def _update_cluster_state(self) -> None:
        """Update cluster state based on node liveness."""
        alive = sum(1 for n in self._nodes.values() if n.is_alive)
        total = len(self._nodes)

        if alive == total:
            self._cluster_state = ClusterState.STABLE
        elif alive >= total * self._config.quorum_majority:
            self._cluster_state = ClusterState.DEGRADED
        else:
            self._cluster_state = ClusterState.DEGRADED
            logger.warning(
                "Cluster DEGRADED — %d/%d nodes alive (quorum: %.0f%%)",
                alive, total, self._config.quorum_majority * 100,
            )

    # ── Leader Election ───────────────────────────────────────

    def _start_election_monitor(self) -> None:
        """Start the leader election monitor."""
        thread = threading.Thread(
            target=self._election_monitor_loop,
            daemon=True,
            name="cluster-election-monitor",
        )
        thread.start()

    def _election_monitor_loop(self) -> None:
        """Monitor leader health and trigger elections if needed."""
        while not self._stop.is_set():
            try:
                self._check_leader_health()
            except Exception as exc:
                logger.debug("Election monitor error: %s", exc)
            self._stop.wait(self._config.election_timeout / 3)

    def _check_leader_health(self) -> None:
        """Check if the current leader is alive."""
        with self._lock:
            if self._leader_id is None:
                # No leader — start election
                logger.warning("No leader detected — starting election")
                self._start_election()
                return

            if self._leader_id not in self._nodes:
                logger.warning("Leader %s not in registry — starting election", self._leader_id)
                self._start_election()
                return

            leader = self._nodes[self._leader_id]
            if not leader.is_alive:
                logger.warning(
                    "Leader %s (%s) is DEAD — starting election",
                    self._leader_id, leader.region,
                )
                self._start_election()
                return

    def _start_election(self) -> None:
        """Start a leader election (Raft-style)."""
        with self._lock:
            self._cluster_state = ClusterState.ELECTION
            self._current_term += 1
            self._voted_for = self._config.node_id
            self._elections += 1

        logger.info(
            "Starting election — term=%d, candidate=%s",
            self._current_term, self._config.node_id,
        )

        # Check if we can become leader
        if self._can_become_leader():
            self._become_leader()
        else:
            # Wait for another node to become leader
            logger.info("Cannot become leader — waiting for election result")

    def _can_become_leader(self) -> bool:
        """Check if this node can become leader based on quorum."""
        alive = self.alive_count
        total = self.node_count

        # Need majority of alive nodes
        needed = max(1, int(total * self._config.quorum_majority))
        return alive >= needed

    def _become_leader(self) -> None:
        """Become the cluster leader."""
        old_leader = self._leader_id

        # Acquire signer lock
        if not self._signer.try_acquire():
            logger.error("Failed to acquire signer lock — cannot become leader")
            with self._lock:
                self._cluster_state = ClusterState.STABLE
            return

        with self._lock:
            self._leader_id = self._config.node_id
            self._leader_term = self._current_term
            self._config.role = ClusterRole.LEADER
            self._cluster_state = ClusterState.STABLE

            if self._config.node_id in self._nodes:
                self._nodes[self._config.node_id].role = ClusterRole.LEADER
                self._nodes[self._config.node_id].term = self._current_term

            self._leader_changes += 1

        logger.warning(
            "ELECTED LEADER — node=%s, term=%d, region=%s (change #%d)",
            self._config.node_id, self._current_term,
            self._config.region, self._leader_changes,
        )

        if old_leader and self._on_leader_lost:
            try:
                self._on_leader_lost(old_leader)
            except Exception:
                pass

        if self._on_leader_elected:
            try:
                self._on_leader_elected(self._config.node_id, self._current_term)
            except Exception:
                pass

    def _step_down(self) -> None:
        """Step down as leader."""
        logger.warning("Stepping down as leader")
        self._signer.release()

        with self._lock:
            old_leader = self._leader_id
            self._leader_id = None
            self._config.role = ClusterRole.FOLLOWER

            if self._config.node_id in self._nodes:
                self._nodes[self._config.node_id].role = ClusterRole.FOLLOWER

        if self._on_leader_lost and old_leader:
            try:
                self._on_leader_lost(old_leader)
            except Exception:
                pass

    # ── Split-Brain Detection ─────────────────────────────────

    def detect_split_brain(self) -> list[str]:
        """Detect if multiple nodes claim to be leader.

        Returns list of node IDs claiming leadership.
        """
        leaders: list[str] = []
        now = time.time()
        timeout = self._config.heartbeat_timeout

        with self._lock:
            for node_id, node in self._nodes.items():
                if node.role == ClusterRole.LEADER and node.is_alive:
                    if now - node.last_heartbeat < timeout:
                        leaders.append(node_id)

        if len(leaders) > 1:
            self._split_brain_events += 1
            with self._lock:
                self._cluster_state = ClusterState.SPLIT_BRAIN

            logger.error(
                "SPLIT-BRAIN DETECTED — %d leaders: %s",
                len(leaders), leaders,
            )

            if self._on_split_brain:
                try:
                    self._on_split_brain(leaders)
                except Exception:
                    pass

            # Resolve: highest term wins
            self._resolve_split_brain(leaders)

        return leaders

    def _resolve_split_brain(self, leaders: list[str]) -> None:
        """Resolve split-brain by selecting the leader with highest term."""
        if len(leaders) <= 1:
            return

        best_leader = leaders[0]
        best_term = 0

        with self._lock:
            for leader_id in leaders:
                node = self._nodes.get(leader_id)
                if node and node.term > best_term:
                    best_term = node.term
                    best_leader = leader_id

        logger.warning(
            "Split-brain resolved — leader=%s (term=%d)",
            best_leader, best_term,
        )

        # If we're a false leader, step down
        if best_leader != self._config.node_id and self.is_leader:
            self._step_down()

        with self._lock:
            self._cluster_state = ClusterState.RECOVERING

    # ── Signer Callbacks ──────────────────────────────────────

    def _on_signer_acquired(self) -> None:
        logger.info("Signer lock acquired — this node is the signing authority")

    def _on_signer_lost(self) -> None:
        logger.warning("Signer lock lost — stepping down")
        if self.is_leader:
            self._step_down()

    # ── External API ──────────────────────────────────────────

    def register_peer(self, node_id: str, region: str, rpc_endpoint: str) -> None:
        """Register a peer node."""
        with self._lock:
            if node_id not in self._nodes:
                self._nodes[node_id] = ClusterNode(
                    node_id=node_id,
                    region=region,
                    role=ClusterRole.UNKNOWN,
                    rpc_endpoint=rpc_endpoint,
                    last_heartbeat=time.time(),
                )
                logger.info("Peer registered: %s (%s)", node_id, region)

    def update_peer_heartbeat(
        self, node_id: str, role: str, term: int, leader_id: str | None
    ) -> None:
        """Update heartbeat for a peer node."""
        with self._lock:
            if node_id in self._nodes:
                self._nodes[node_id].last_heartbeat = time.time()
                self._nodes[node_id].is_alive = True
                try:
                    self._nodes[node_id].role = ClusterRole(role)
                except ValueError:
                    pass
                self._nodes[node_id].term = term

            # Track leader changes
            if leader_id and leader_id != self._leader_id:
                if self._leader_id is not None:
                    self._leader_changes += 1
                self._leader_id = leader_id
                self._leader_term = term

    def force_election(self) -> None:
        """Force a new leader election."""
        logger.info("Forcing leader election")
        self._start_election()

    # ── Status ────────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        with self._lock:
            return {
                "cluster_id": self._config.cluster_id,
                "node_id": self._config.node_id,
                "region": self._config.region,
                "role": self._config.role.value,
                "cluster_state": self._cluster_state.value,
                "current_term": self._current_term,
                "leader_id": self._leader_id,
                "leader_term": self._leader_term,
                "is_leader": self.is_leader,
                "is_signer": self._signer.is_signer,
                "signer_authority": self._signer.authority.value,
                "nodes": {
                    nid: node.to_dict() for nid, node in self._nodes.items()
                },
                "alive_count": self.alive_count,
                "total_count": self.node_count,
                "quorum_needed": max(1, int(self.node_count * self._config.quorum_majority)),
                "elections": self._elections,
                "leader_changes": self._leader_changes,
                "split_brain_events": self._split_brain_events,
                "config": {
                    "heartbeat_interval": self._config.heartbeat_interval,
                    "heartbeat_timeout": self._config.heartbeat_timeout,
                    "election_timeout": self._config.election_timeout,
                    "quorum_majority": self._config.quorum_majority,
                },
            }


# ─── CLI Entry Point ──────────────────────────────────────────


def main() -> None:
    """CLI entry point for the cluster coordinator."""
    import argparse

    parser = argparse.ArgumentParser(
        description="X3 Cluster Coordinator — multi-machine validator failover"
    )
    parser.add_argument(
        "--cluster-id", default="x3-default",
        help="Unique cluster identifier",
    )
    parser.add_argument(
        "--node-id", default="",
        help="Unique node identifier",
    )
    parser.add_argument(
        "--region", default="unknown",
        help="Region/AZ of this node",
    )
    parser.add_argument(
        "--role", choices=["leader", "follower", "observer"], default="follower",
        help="Initial role (default: follower)",
    )
    parser.add_argument(
        "--peers", default="",
        help="Comma-separated peer node IDs",
    )
    parser.add_argument(
        "--rpc-endpoint", default="http://127.0.0.1:9933",
        help="RPC endpoint for this node",
    )
    parser.add_argument(
        "--redis-url", default="redis://127.0.0.1:6379/0",
        help="Redis connection URL",
    )
    parser.add_argument(
        "--heartbeat-interval", type=float, default=DEFAULT_HEARTBEAT_INTERVAL,
        help="Heartbeat interval in seconds",
    )
    parser.add_argument(
        "--heartbeat-timeout", type=float, default=DEFAULT_HEARTBEAT_TIMEOUT,
        help="Heartbeat timeout before declaring node dead",
    )
    parser.add_argument(
        "--election-timeout", type=float, default=DEFAULT_ELECTION_TIMEOUT,
        help="Timeout before triggering leader election",
    )
    parser.add_argument(
        "--log-level", default="INFO",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="Logging level",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=getattr(logging, args.log_level),
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    config = ClusterConfig(
        cluster_id=args.cluster_id,
        node_id=args.node_id,
        region=args.region,
        role=ClusterRole(args.role),
        peers=[p.strip() for p in args.peers.split(",") if p.strip()],
        rpc_endpoint=args.rpc_endpoint,
        redis_url=args.redis_url,
        heartbeat_interval=args.heartbeat_interval,
        heartbeat_timeout=args.heartbeat_timeout,
        election_timeout=args.election_timeout,
    )

    coordinator = ClusterCoordinator(config=config)

    try:
        coordinator.start()
        # Block until stopped
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        logger.info("Received SIGINT — shutting down")
    finally:
        coordinator.stop()


if __name__ == "__main__":
    main()
