"""Substrate Wiring — connects all AGI layers through the event bus.

This module provides:
1. SubstrateWiring: Factory that creates and connects all subsystems
2. Cross-layer subscription handlers (reaction to events from other layers)
3. Event routing: CausalGraph ← events, AnomalyDetector ← epoch events

Usage::

    wiring = SubstrateWiring(storage=storage, event_bus=bus)
    orchestrator = wiring.build()
    stats = await orchestrator.run_epoch()

NON-NEGOTIABLE:
- Every subsystem publishes through the shared AsyncEventBus
- Tripwire events are never suppressed
- The anomaly detector runs at the END of every epoch (after kills)
"""

from __future__ import annotations

import logging
from typing import Any, Callable, Dict, List, Optional

from swarm.causal.graph import CausalGraph
from swarm.core.agent import Agent, Consequence
from swarm.core.lifecycle import EpochOrchestrator, EpochStats
from swarm.event_bus.bus import AsyncEventBus
from swarm.event_bus.events import BusEvent, EventType
from swarm.gpu_bridge.client import GpuTaskClient
from swarm.reaper import PostmortemAnalyzer, ReaperEngine, ScarPropagator
from swarm.storage.backend import StorageBackend
from swarm.tripwire.anomaly import BehavioralAnomalyDetector
from swarm.tripwire.detector import TripwireDetector
from swarm.world_sim.prediction import PredictionMarket
from swarm.world_sim.scoreboard import AccuracyScoreboard
from swarm.world_sim.state_graph import WorldStateGraph

logger = logging.getLogger(__name__)


class SubstrateWiring:
    """Factory that creates and wires all AGI substrate subsystems.

    This is the single place where cross-layer subscriptions are established.
    After calling ``build()``, every subsystem is connected to the event bus
    and will react to events from sibling layers.

    Args:
        storage: Shared persistence backend.
        event_bus: Shared async event bus.
        gpu_client: Optional GPU task client.
        consequence_fn: Optional environment callback for consequences.
        anomaly_threshold: Anomaly detection threshold (0-1).
        coordination_threshold: Coordination detection threshold (0-1).
    """

    def __init__(
        self,
        storage: StorageBackend,
        event_bus: AsyncEventBus,
        gpu_client: Optional[GpuTaskClient] = None,
        consequence_fn: Optional[Callable] = None,
        anomaly_threshold: float = 0.6,
        coordination_threshold: float = 0.8,
    ) -> None:
        self._storage = storage
        self._bus = event_bus
        self._gpu_client = gpu_client
        self._consequence_fn = consequence_fn
        self._anomaly_threshold = anomaly_threshold
        self._coordination_threshold = coordination_threshold

        # Subsystem instances (created in build())
        self.world_state: Optional[WorldStateGraph] = None
        self.prediction_market: Optional[PredictionMarket] = None
        self.scoreboard: Optional[AccuracyScoreboard] = None
        self.reaper: Optional[ReaperEngine] = None
        self.postmortem: Optional[PostmortemAnalyzer] = None
        self.scar_propagator: Optional[ScarPropagator] = None
        self.tripwire: Optional[TripwireDetector] = None
        self.causal_graph: Optional[CausalGraph] = None
        self.anomaly_detector: Optional[BehavioralAnomalyDetector] = None
        self.orchestrator: Optional[EpochOrchestrator] = None

        # Event counters for diagnostics
        self._event_counts: Dict[str, int] = {}

    def build(self) -> EpochOrchestrator:
        """Create all subsystems and wire cross-layer subscriptions.

        Returns the fully wired EpochOrchestrator.
        """
        # 1. Create subsystems
        self.world_state = WorldStateGraph(storage=self._storage)
        self.prediction_market = PredictionMarket(storage=self._storage)
        self.scoreboard = AccuracyScoreboard(storage=self._storage)
        self.reaper = ReaperEngine(storage=self._storage)
        self.postmortem = PostmortemAnalyzer(storage=self._storage)
        self.scar_propagator = ScarPropagator(storage=self._storage)
        self.tripwire = TripwireDetector(storage=self._storage)
        self.causal_graph = CausalGraph(storage=self._storage)
        self.anomaly_detector = BehavioralAnomalyDetector(
            storage=self._storage,
            causal_graph=self.causal_graph,
            anomaly_threshold=self._anomaly_threshold,
            coordination_threshold=self._coordination_threshold,
        )

        # 2. Create orchestrator
        self.orchestrator = EpochOrchestrator(
            storage=self._storage,
            event_bus=self._bus,
            world_state=self.world_state,
            prediction_market=self.prediction_market,
            scoreboard=self.scoreboard,
            reaper=self.reaper,
            postmortem_analyzer=self.postmortem,
            scar_propagator=self.scar_propagator,
            tripwire=self.tripwire,
            causal_graph=self.causal_graph,
            gpu_client=self._gpu_client,
            consequence_fn=self._consequence_fn,
        )

        # 3. Wire cross-layer subscriptions
        self._wire_subscriptions()

        return self.orchestrator

    def _wire_subscriptions(self) -> None:
        """Establish all cross-layer event subscriptions."""
        # Tripwire monitors all events (wildcard subscription)
        self._bus.subscribe_all(self._on_any_event)

        # Death events → log to anomaly detector + trigger fingerprint rebuild
        self._bus.subscribe(EventType.AGENT_DEATH, self._on_agent_death)

        # Scar events → tracked for anomaly patterns
        self._bus.subscribe(EventType.SCAR_RECORDED, self._on_scar_recorded)

        # Accuracy warnings → potential anomaly signal
        self._bus.subscribe(EventType.ACCURACY_WARNING, self._on_accuracy_warning)
        self._bus.subscribe(EventType.ACCURACY_CRITICAL, self._on_accuracy_critical)

        # Tripwire events → logged prominently
        self._bus.subscribe(EventType.TRIPWIRE_TRIGGERED, self._on_tripwire)

        # Epoch advance → run anomaly scans
        self._bus.subscribe(EventType.EPOCH_ADVANCED, self._on_epoch_advanced)

        logger.info("Substrate wiring: %d subscriptions established", 6)

    # ------------------------------------------------------------------
    # Event handlers
    # ------------------------------------------------------------------

    async def _on_any_event(self, event: BusEvent) -> None:
        """Global handler: count events by type for diagnostics."""
        et = event.event_type
        self._event_counts[et] = self._event_counts.get(et, 0) + 1

    async def _on_agent_death(self, event: BusEvent) -> None:
        """When an agent dies, rebuild its fingerprint for forensics."""
        agent_id = event.agent_id
        if self.anomaly_detector:
            self.anomaly_detector.build_fingerprint(agent_id)
        logger.info("Wiring: agent death processed for %s", agent_id)

    async def _on_scar_recorded(self, event: BusEvent) -> None:
        """Track scar events (potential signal for anomaly detection)."""
        logger.debug("Wiring: scar recorded for %s", event.agent_id)

    async def _on_accuracy_warning(self, event: BusEvent) -> None:
        """Accuracy warning — agent predictions degrading."""
        logger.info(
            "Wiring: accuracy warning for %s: %s",
            event.agent_id,
            event.payload,
        )

    async def _on_accuracy_critical(self, event: BusEvent) -> None:
        """Accuracy critical — agent predictions severely degraded."""
        logger.warning(
            "Wiring: accuracy CRITICAL for %s: %s",
            event.agent_id,
            event.payload,
        )

    async def _on_tripwire(self, event: BusEvent) -> None:
        """Tripwire triggered — log prominently, never suppress."""
        logger.critical(
            "TRIPWIRE: agent=%s severity=%s payload=%s",
            event.agent_id,
            event.severity,
            event.payload,
        )

    async def _on_epoch_advanced(self, event: BusEvent) -> None:
        """On epoch advance, scan swarm for behavioral anomalies."""
        if self.anomaly_detector and self.orchestrator:
            agent_ids = [a.agent_id for a in self.orchestrator.living_agents]
            if agent_ids:
                epoch = event.payload.get("epoch", 0)
                report = self.anomaly_detector.scan_swarm(agent_ids, epoch)

                # Publish any anomaly events
                for ae in self.anomaly_detector.get_pending_bus_events():
                    await self._bus.publish(ae)

                if report.total_anomalies > 0:
                    logger.warning(
                        "Anomaly scan epoch %d: %d anomalies, risk=%s",
                        epoch,
                        report.total_anomalies,
                        report.overall_swarm_risk,
                    )

    # ------------------------------------------------------------------
    # Diagnostics
    # ------------------------------------------------------------------

    @property
    def event_counts(self) -> Dict[str, int]:
        """Return event counts by type."""
        return dict(self._event_counts)

    @property
    def total_events_routed(self) -> int:
        """Total events that passed through the wiring."""
        return sum(self._event_counts.values())
