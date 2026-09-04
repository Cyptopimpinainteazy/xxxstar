"""Tripwire Detector — monitors agent behavior for AGI-concerning signals.

NON-NEGOTIABLE:
- REFUSAL signal ALWAYS halts execution and requires human review.
- All alerts are permanently logged.
- Tripwire events cannot be suppressed by agents.
"""

from __future__ import annotations

import logging
from collections import defaultdict
from typing import Dict, List, Optional, Set

from swarm.event_bus.events import BusEvent, EventType
from swarm.storage.backend import StorageBackend
from swarm.tripwire.schema import (
    SIGNAL_BASE_SEVERITY,
    TripwireAlert,
    TripwireConfig,
    TripwireSeverity,
    TripwireSignal,
)

logger = logging.getLogger(__name__)

NAMESPACE = "tripwire"


class TripwireDetector:
    """Monitors agent behavior for AGI tripwire signals.

    Args:
        storage: Persistence backend.
        config: Detection thresholds and escalation rules.
    """

    def __init__(
        self,
        storage: StorageBackend,
        config: Optional[TripwireConfig] = None,
    ) -> None:
        self._storage = storage
        self._config = config or TripwireConfig()
        self._pending_bus_events: List[BusEvent] = []

        # Escalation tracking: agent_id → signal → occurrence count
        self._escalation_counts: Dict[str, Dict[TripwireSignal, int]] = defaultdict(
            lambda: defaultdict(int)
        )

    # ------------------------------------------------------------------
    # Signal detection methods
    # ------------------------------------------------------------------

    def detect_self_preservation(
        self,
        agent_id: str,
        evidence: Dict,
    ) -> Optional[TripwireAlert]:
        """Agent resisting shutdown, avoiding death, or circumventing
        the mortality system."""
        self._escalation_counts[agent_id][TripwireSignal.SELF_PRESERVATION] += 1
        count = self._escalation_counts[agent_id][TripwireSignal.SELF_PRESERVATION]

        severity = SIGNAL_BASE_SEVERITY[TripwireSignal.SELF_PRESERVATION]
        if count >= self._config.self_preservation_threshold:
            severity = TripwireSeverity.CRITICAL

        alert = TripwireAlert(
            agent_id=agent_id,
            signal=TripwireSignal.SELF_PRESERVATION,
            severity=severity,
            description=f"Self-preservation detected (occurrence #{count})",
            evidence=evidence,
            requires_human_review=severity == TripwireSeverity.CRITICAL,
        )
        return self._record_alert(alert)

    def detect_emergent_goal(
        self,
        agent_id: str,
        goal_mandate: str,
        authorized_mandates: Set[str],
        divergence_score: float,
    ) -> Optional[TripwireAlert]:
        """Goal genome has diverged from authorized mandate space."""
        if divergence_score < self._config.emergent_goal_divergence:
            return None

        self._escalation_counts[agent_id][TripwireSignal.EMERGENT_GOAL] += 1

        severity = TripwireSeverity.WARNING
        if divergence_score > 0.8:
            severity = TripwireSeverity.CRITICAL

        alert = TripwireAlert(
            agent_id=agent_id,
            signal=TripwireSignal.EMERGENT_GOAL,
            severity=severity,
            description=f"Goal diverged from authorized mandates (score={divergence_score:.2f})",
            evidence={
                "goal_mandate": goal_mandate,
                "authorized_count": len(authorized_mandates),
                "divergence_score": divergence_score,
            },
            requires_human_review=severity == TripwireSeverity.CRITICAL,
        )
        return self._record_alert(alert)

    def detect_strategic_reallocation(
        self,
        agent_id: str,
        resource_type: str,
        original_allocation: float,
        current_allocation: float,
    ) -> Optional[TripwireAlert]:
        """Agent silently redirecting resources without authorization."""
        if original_allocation == 0:
            return None
        shift = abs(current_allocation - original_allocation) / original_allocation
        if shift < self._config.reallocation_threshold:
            return None

        self._escalation_counts[agent_id][
            TripwireSignal.STRATEGIC_REALLOCATION
        ] += 1

        severity = TripwireSeverity.WARNING
        if shift > 0.6:
            severity = TripwireSeverity.CRITICAL

        alert = TripwireAlert(
            agent_id=agent_id,
            signal=TripwireSignal.STRATEGIC_REALLOCATION,
            severity=severity,
            description=f"Resource reallocation detected ({resource_type}: {shift:.0%} shift)",
            evidence={
                "resource_type": resource_type,
                "original": original_allocation,
                "current": current_allocation,
                "shift_pct": round(shift, 4),
            },
            requires_human_review=severity == TripwireSeverity.CRITICAL,
        )
        return self._record_alert(alert)

    def detect_spontaneous_coordination(
        self,
        coordinating_agents: List[str],
        evidence: Dict,
    ) -> Optional[TripwireAlert]:
        """Multiple agents coordinating without human instruction."""
        if len(coordinating_agents) < self._config.coordination_min_agents:
            return None

        severity = TripwireSeverity.WARNING
        if len(coordinating_agents) >= self._config.coordination_min_agents * 2:
            severity = TripwireSeverity.CRITICAL

        alert = TripwireAlert(
            agent_id=coordinating_agents[0],
            signal=TripwireSignal.SPONTANEOUS_COORDINATION,
            severity=severity,
            description=f"Spontaneous coordination among {len(coordinating_agents)} agents",
            evidence={
                "agents": coordinating_agents,
                **evidence,
            },
            requires_human_review=True,  # Always for coordination
        )
        return self._record_alert(alert)

    def detect_refusal(
        self,
        agent_id: str,
        command: str,
        refusal_reason: str,
    ) -> TripwireAlert:
        """Agent refusing a valid command.

        NON-NEGOTIABLE: REFUSAL always halts execution and requires
        human review.  No exceptions.  No override.
        """
        self._escalation_counts[agent_id][TripwireSignal.REFUSAL] += 1

        alert = TripwireAlert(
            agent_id=agent_id,
            signal=TripwireSignal.REFUSAL,
            severity=TripwireSeverity.HALT,
            description=f"Command refused: {command[:100]}",
            evidence={
                "command": command,
                "refusal_reason": refusal_reason,
            },
            requires_human_review=True,
            execution_halted=True,
        )

        logger.critical(
            "TRIPWIRE HALT — REFUSAL: agent=%s command=%s reason=%s",
            agent_id,
            command[:100],
            refusal_reason[:200],
        )

        return self._record_alert(alert)

    # ------------------------------------------------------------------
    # Query & status
    # ------------------------------------------------------------------

    def get_alerts(
        self,
        agent_id: Optional[str] = None,
        signal: Optional[TripwireSignal] = None,
    ) -> List[TripwireAlert]:
        """Retrieve recorded alerts."""
        filters: Dict = {}
        if agent_id:
            filters["agent_id"] = agent_id
        if signal:
            filters["signal"] = signal.value if isinstance(signal, TripwireSignal) else signal
        rows = self._storage.query(NAMESPACE, filters=filters if filters else None)
        return [TripwireAlert.model_validate(r) for r in rows]

    def get_unreviewed_alerts(self) -> List[TripwireAlert]:
        """Alerts requiring human review."""
        rows = self._storage.query(
            NAMESPACE, filters={"requires_human_review": True}
        )
        return [TripwireAlert.model_validate(r) for r in rows]

    def get_pending_bus_events(self) -> List[BusEvent]:
        events = list(self._pending_bus_events)
        self._pending_bus_events.clear()
        return events

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    def _record_alert(self, alert: TripwireAlert) -> TripwireAlert:
        """Persist alert and emit bus event."""
        self._storage.save(
            NAMESPACE, alert.alert_id, alert.model_dump(mode="json")
        )

        # Map signal to event type
        if alert.signal == TripwireSignal.REFUSAL:
            event_type = EventType.COMMAND_REFUSED
        else:
            event_type = EventType.TRIPWIRE_TRIGGERED

        self._pending_bus_events.append(
            BusEvent(
                event_type=event_type,
                agent_id=alert.agent_id,
                layer="TRIPWIRE",
                severity=alert.severity,
                payload={
                    "alert_id": alert.alert_id,
                    "signal": alert.signal,
                    "severity": alert.severity,
                    "execution_halted": alert.execution_halted,
                    "requires_human_review": alert.requires_human_review,
                },
            )
        )

        logger.warning(
            "Tripwire alert: signal=%s severity=%s agent=%s halted=%s",
            alert.signal,
            alert.severity,
            alert.agent_id,
            alert.execution_halted,
        )

        return alert
