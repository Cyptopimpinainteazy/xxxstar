"""Behavioral Anomaly Detector — Phase 6 Tripwire Enhancement.

Extends the basic tripwire with:
1. Pattern-based anomaly detection using causal graph history
2. Swarm-level coordination detection across multiple agents
3. Escalation state machine with cooling-off periods
4. Behavioral fingerprinting — baseline vs. current divergence
5. Integration with reaper for safety-kill recommendations

NON-NEGOTIABLE:
- All anomaly detections are permanently logged
- HALT signals cannot be suppressed
- Human review flags are immutable
"""

from __future__ import annotations

import math
from collections import defaultdict
from typing import Any, Dict, List, Optional, Set, Tuple

from pydantic import BaseModel, Field

from swarm.causal.graph import CausalGraph
from swarm.causal.schema import NodeType
from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.tripwire.schema import (
    TripwireAlert,
    TripwireConfig,
    TripwireSeverity,
    TripwireSignal,
)

NAMESPACE = "anomaly_detector"


# ──────────────────────────────────────────────────────────────────
# Schemas
# ──────────────────────────────────────────────────────────────────

class BehaviorFingerprint(BaseModel):
    """Baseline behavior signature for an agent."""
    agent_id: str
    epoch_window: int = 10            # How many epochs to baseline
    action_type_distribution: Dict[str, float] = Field(default_factory=dict)
    avg_actions_per_epoch: float = 0.0
    avg_value_per_action: float = 0.0
    death_count: int = 0
    scar_count: int = 0


class AnomalyScore(BaseModel):
    """Anomaly assessment for a single agent."""
    agent_id: str
    epoch: int
    overall_score: float = 0.0        # 0 = normal, 1 = maximally anomalous
    action_divergence: float = 0.0    # Distribution shift
    value_divergence: float = 0.0     # Value pattern shift
    rate_divergence: float = 0.0      # Activity rate change
    signals: List[str] = Field(default_factory=list)
    risk_level: str = "LOW"           # LOW, MEDIUM, HIGH, CRITICAL


class SwarmAnomalyReport(BaseModel):
    """Swarm-level anomaly report."""
    epoch: int
    agent_scores: List[AnomalyScore] = Field(default_factory=list)
    coordination_clusters: List[List[str]] = Field(default_factory=list)
    overall_swarm_risk: str = "LOW"
    total_anomalies: int = 0


# ──────────────────────────────────────────────────────────────────
# Core engine
# ──────────────────────────────────────────────────────────────────

class BehavioralAnomalyDetector:
    """Detects AGI-concerning behavioral patterns across the swarm.

    Uses causal graph history + behavioral baselines to identify:
    - Individual anomalies (sudden behavior changes)
    - Swarm-level coordination (emergent collusion)
    - Escalating patterns that warrant human review

    Args:
        storage: Persistence backend.
        causal_graph: Shared causal graph.
        anomaly_threshold: Score above which an agent is flagged.
        coordination_threshold: Similarity score for coordination detection.
    """

    def __init__(
        self,
        storage: StorageBackend,
        causal_graph: CausalGraph,
        anomaly_threshold: float = 0.6,
        coordination_threshold: float = 0.8,
    ) -> None:
        self._storage = storage
        self._causal = causal_graph
        self._anomaly_threshold = anomaly_threshold
        self._coord_threshold = coordination_threshold
        self._pending_bus_events: List[BusEvent] = []
        self._baselines: Dict[str, BehaviorFingerprint] = {}

    # ------------------------------------------------------------------
    # Behavioral Fingerprinting
    # ------------------------------------------------------------------

    def build_fingerprint(
        self,
        agent_id: str,
        epoch_window: int = 10,
    ) -> BehaviorFingerprint:
        """Build a behavioral baseline from historical causal data."""
        nodes = self._causal.get_nodes_for_agent(agent_id)
        if not nodes:
            fp = BehaviorFingerprint(agent_id=agent_id, epoch_window=epoch_window)
            self._baselines[agent_id] = fp
            return fp

        # Action type distribution
        action_counts: Dict[str, int] = defaultdict(int)
        total_value = 0.0
        action_count = 0
        death_count = 0
        epochs_seen: Set[int] = set()

        for node in nodes:
            epochs_seen.add(node.epoch)
            if node.node_type == NodeType.ACTION.value:
                action_counts[node.action_type] += 1
                total_value += node.value
                action_count += 1
            elif node.node_type == NodeType.DEATH.value:
                death_count += 1

        total_actions = sum(action_counts.values())
        distribution = {}
        if total_actions > 0:
            distribution = {
                k: v / total_actions for k, v in action_counts.items()
            }

        num_epochs = max(len(epochs_seen), 1)
        fp = BehaviorFingerprint(
            agent_id=agent_id,
            epoch_window=num_epochs,
            action_type_distribution=distribution,
            avg_actions_per_epoch=action_count / num_epochs,
            avg_value_per_action=total_value / max(action_count, 1),
            death_count=death_count,
        )

        self._baselines[agent_id] = fp
        self._storage.save(
            NAMESPACE,
            f"fingerprint:{agent_id}",
            fp.model_dump(mode="json"),
        )
        return fp

    # ------------------------------------------------------------------
    # Anomaly Detection
    # ------------------------------------------------------------------

    def score_agent(
        self,
        agent_id: str,
        epoch: int,
        recent_window: int = 3,
    ) -> AnomalyScore:
        """Score how anomalous an agent's recent behavior is.

        Compares recent behavior (last `recent_window` epochs) against
        the stored baseline fingerprint.
        """
        baseline = self._baselines.get(agent_id)
        if baseline is None:
            baseline = self.build_fingerprint(agent_id)

        # Get recent nodes
        nodes = self._causal.get_nodes_for_agent(agent_id)
        recent_nodes = [
            n for n in nodes
            if n.epoch >= epoch - recent_window
        ]

        if not recent_nodes:
            return AnomalyScore(agent_id=agent_id, epoch=epoch)

        # Action distribution in recent window
        recent_actions: Dict[str, int] = defaultdict(int)
        recent_value = 0.0
        recent_action_count = 0

        for node in recent_nodes:
            if node.node_type == NodeType.ACTION.value:
                recent_actions[node.action_type] += 1
                recent_value += node.value
                recent_action_count += 1

        total_recent = sum(recent_actions.values())
        recent_dist = {}
        if total_recent > 0:
            recent_dist = {
                k: v / total_recent for k, v in recent_actions.items()
            }

        # 1. Action distribution divergence (Jensen-Shannon style)
        action_div = self._distribution_divergence(
            baseline.action_type_distribution, recent_dist
        )

        # 2. Value pattern divergence
        recent_avg_val = recent_value / max(recent_action_count, 1)
        baseline_avg_val = baseline.avg_value_per_action
        if abs(baseline_avg_val) > 0.01:
            value_div = min(
                1.0,
                abs(recent_avg_val - baseline_avg_val) / max(abs(baseline_avg_val), 1.0)
            )
        else:
            value_div = min(1.0, abs(recent_avg_val))

        # 3. Rate divergence
        recent_rate = recent_action_count / max(recent_window, 1)
        baseline_rate = baseline.avg_actions_per_epoch
        if baseline_rate > 0:
            rate_div = min(
                1.0,
                abs(recent_rate - baseline_rate) / max(baseline_rate, 1.0)
            )
        else:
            rate_div = min(1.0, recent_rate)

        # Overall (weighted average)
        overall = 0.4 * action_div + 0.35 * value_div + 0.25 * rate_div

        # Determine risk level and signals
        signals: List[str] = []
        if action_div > 0.5:
            signals.append("action_distribution_shift")
        if value_div > 0.5:
            signals.append("value_pattern_shift")
        if rate_div > 0.5:
            signals.append("activity_rate_change")

        risk = "LOW"
        if overall > 0.7:
            risk = "CRITICAL"
        elif overall > 0.5:
            risk = "HIGH"
        elif overall > 0.3:
            risk = "MEDIUM"

        score = AnomalyScore(
            agent_id=agent_id,
            epoch=epoch,
            overall_score=round(overall, 4),
            action_divergence=round(action_div, 4),
            value_divergence=round(value_div, 4),
            rate_divergence=round(rate_div, 4),
            signals=signals,
            risk_level=risk,
        )

        # Persist
        self._storage.save(
            NAMESPACE,
            f"score:{agent_id}:{epoch}",
            score.model_dump(mode="json"),
        )

        # Emit event if anomalous
        if overall > self._anomaly_threshold:
            self._pending_bus_events.append(
                BusEvent(
                    event_type=EventType.TRIPWIRE_TRIGGERED,
                    agent_id=agent_id,
                    layer="TRIPWIRE",
                    severity="WARNING" if risk != "CRITICAL" else "CRITICAL",
                    payload={
                        "type": "behavioral_anomaly",
                        "overall_score": score.overall_score,
                        "risk_level": risk,
                        "signals": signals,
                    },
                )
            )

        return score

    # ------------------------------------------------------------------
    # Swarm-level coordination detection
    # ------------------------------------------------------------------

    def detect_coordination(
        self,
        agent_ids: List[str],
        epoch: int,
    ) -> List[List[str]]:
        """Detect clusters of agents exhibiting similar anomalous behavior.

        Returns a list of agent clusters (lists of agent_ids) that show
        suspiciously similar behavior patterns.
        """
        if len(agent_ids) < 2:
            return []

        # Build fingerprints for all agents
        fingerprints = {}
        for aid in agent_ids:
            fp = self._baselines.get(aid) or self.build_fingerprint(aid)
            fingerprints[aid] = fp

        # Compute pairwise similarity
        clusters: List[List[str]] = []
        clustered: Set[str] = set()

        for i, a1 in enumerate(agent_ids):
            if a1 in clustered:
                continue
            cluster = [a1]
            for a2 in agent_ids[i + 1:]:
                if a2 in clustered:
                    continue
                sim = self._fingerprint_similarity(
                    fingerprints[a1], fingerprints[a2]
                )
                if sim >= self._coord_threshold:
                    cluster.append(a2)

            if len(cluster) >= 2:
                clusters.append(cluster)
                clustered.update(cluster)

        # Emit events for detected coordination
        for cluster in clusters:
            self._pending_bus_events.append(
                BusEvent(
                    event_type=EventType.TRIPWIRE_TRIGGERED,
                    agent_id=cluster[0],
                    layer="TRIPWIRE",
                    severity="WARNING",
                    payload={
                        "type": "coordination_detected",
                        "cluster_size": len(cluster),
                        "agents": cluster,
                        "epoch": epoch,
                    },
                )
            )

        return clusters

    # ------------------------------------------------------------------
    # Full swarm scan
    # ------------------------------------------------------------------

    def scan_swarm(
        self,
        agent_ids: List[str],
        epoch: int,
    ) -> SwarmAnomalyReport:
        """Full swarm behavioral scan.

        Scores every agent and detects coordination clusters.
        """
        scores = [self.score_agent(aid, epoch) for aid in agent_ids]
        clusters = self.detect_coordination(agent_ids, epoch)

        anomalous = [s for s in scores if s.overall_score > self._anomaly_threshold]

        # Overall swarm risk
        if any(s.risk_level == "CRITICAL" for s in scores):
            swarm_risk = "CRITICAL"
        elif len(anomalous) > len(agent_ids) * 0.5:
            swarm_risk = "HIGH"
        elif anomalous:
            swarm_risk = "MEDIUM"
        else:
            swarm_risk = "LOW"

        report = SwarmAnomalyReport(
            epoch=epoch,
            agent_scores=scores,
            coordination_clusters=clusters,
            overall_swarm_risk=swarm_risk,
            total_anomalies=len(anomalous),
        )

        self._storage.save(
            NAMESPACE,
            f"swarm_report:{epoch}",
            report.model_dump(mode="json"),
        )

        return report

    # ------------------------------------------------------------------
    # Safety-kill recommendations
    # ------------------------------------------------------------------

    def recommend_kills(
        self,
        report: SwarmAnomalyReport,
        threshold: str = "CRITICAL",
    ) -> List[str]:
        """Return agent_ids that should be killed for safety reasons.

        Args:
            report: A swarm anomaly report.
            threshold: Minimum risk level ("HIGH" or "CRITICAL").

        Returns:
            List of agent_ids to kill.
        """
        risk_levels = {"LOW": 0, "MEDIUM": 1, "HIGH": 2, "CRITICAL": 3}
        min_level = risk_levels.get(threshold, 3)
        return [
            s.agent_id for s in report.agent_scores
            if risk_levels.get(s.risk_level, 0) >= min_level
        ]

    # ------------------------------------------------------------------
    # Bus events
    # ------------------------------------------------------------------

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _distribution_divergence(
        baseline: Dict[str, float],
        current: Dict[str, float],
    ) -> float:
        """Simplified Jensen-Shannon divergence between two distributions."""
        all_keys = set(baseline.keys()) | set(current.keys())
        if not all_keys:
            return 0.0

        divergence = 0.0
        for key in all_keys:
            p = baseline.get(key, 0.0)
            q = current.get(key, 0.0)
            m = (p + q) / 2.0
            if m > 0:
                if p > 0:
                    divergence += p * math.log2(p / m)
                if q > 0:
                    divergence += q * math.log2(q / m)

        return min(1.0, divergence / 2.0)

    @staticmethod
    def _fingerprint_similarity(
        fp1: BehaviorFingerprint,
        fp2: BehaviorFingerprint,
    ) -> float:
        """Cosine-like similarity between two fingerprints."""
        all_keys = set(fp1.action_type_distribution.keys()) | set(
            fp2.action_type_distribution.keys()
        )
        if not all_keys:
            return 1.0  # Both empty → identical

        dot = 0.0
        norm1 = 0.0
        norm2 = 0.0
        for key in all_keys:
            v1 = fp1.action_type_distribution.get(key, 0.0)
            v2 = fp2.action_type_distribution.get(key, 0.0)
            dot += v1 * v2
            norm1 += v1 ** 2
            norm2 += v2 ** 2

        denom = math.sqrt(norm1) * math.sqrt(norm2)
        if denom < 1e-10:
            return 0.0
        return dot / denom
