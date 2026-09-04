"""Async event bus for the X3 AGI substrate.

Design decisions:
- asyncio-native: all handlers are coroutines; fire-and-collect semantics.
- Typed subscriptions: consumers register for specific ``EventType`` values.
- Append-only persistence: every event is written to a JSONL log file for
  forensic analysis.  The log is never truncated at runtime.
- WebSocket broadcast hook: callers can register an async callback that
  receives every event for forwarding to the desktop UI.
- Back-pressure: handlers that raise are logged but never crash the bus.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import time
from collections import defaultdict
from pathlib import Path
from typing import Any, Awaitable, Callable, Dict, List, Optional, Set

from swarm.event_bus.events import BusEvent, EventType

logger = logging.getLogger(__name__)

# Type alias for an async event handler
EventHandler = Callable[[BusEvent], Awaitable[None]]


class AsyncEventBus:
    """Lightweight async pub/sub bus for AGI substrate events.

    Usage::

        bus = AsyncEventBus(log_dir="/var/log/x3-chain/events")
        bus.subscribe(EventType.AGENT_DEATH, my_handler)
        await bus.publish(some_event)
        await bus.shutdown()
    """

    def __init__(
        self,
        log_dir: Optional[str] = None,
        ws_broadcast: Optional[Callable[[BusEvent], Awaitable[None]]] = None,
    ) -> None:
        """Initialise the event bus.

        Args:
            log_dir: Directory for the append-only JSONL event log.
                     If ``None``, persistence is disabled (useful for tests).
            ws_broadcast: Optional async callback invoked for every event,
                          intended for WebSocket forwarding to the desktop UI.
        """
        self._handlers: Dict[EventType, List[EventHandler]] = defaultdict(list)
        self._global_handlers: List[EventHandler] = []
        self._ws_broadcast = ws_broadcast
        self._log_file: Optional[Any] = None
        self._log_dir = log_dir
        self._event_count: int = 0
        self._start_time: float = time.monotonic()
        self._lock = asyncio.Lock()

        if log_dir:
            Path(log_dir).mkdir(parents=True, exist_ok=True)
            log_path = Path(log_dir) / "events.jsonl"
            # Open in append mode; never truncate.
            self._log_file = open(log_path, "a", encoding="utf-8")
            logger.info("Event bus log: %s", log_path)

    # ------------------------------------------------------------------
    # Subscription
    # ------------------------------------------------------------------

    def subscribe(
        self,
        event_type: EventType,
        handler: EventHandler,
    ) -> None:
        """Register *handler* for events of *event_type*."""
        self._handlers[event_type].append(handler)

    def subscribe_all(self, handler: EventHandler) -> None:
        """Register *handler* for **all** event types (e.g. Tripwire)."""
        self._global_handlers.append(handler)

    def unsubscribe(
        self,
        event_type: EventType,
        handler: EventHandler,
    ) -> None:
        """Remove a previously registered handler."""
        handlers = self._handlers.get(event_type, [])
        if handler in handlers:
            handlers.remove(handler)

    # ------------------------------------------------------------------
    # Publishing
    # ------------------------------------------------------------------

    async def publish(self, event: BusEvent) -> None:
        """Publish *event* to all matching subscribers.

        1. Persist to append-only log.
        2. Invoke typed handlers.
        3. Invoke global (wildcard) handlers.
        4. Invoke optional WebSocket broadcast callback.
        """
        self._event_count += 1

        # 1. Persist
        await self._persist(event)

        # Resolve EventType from string value if needed
        try:
            et = EventType(event.event_type)
        except ValueError:
            et = None

        # 2. Typed handlers
        if et is not None:
            handlers = list(self._handlers.get(et, []))
            for handler in handlers:
                try:
                    await handler(event)
                except Exception:
                    logger.exception(
                        "Handler %s failed for event %s",
                        handler.__qualname__,
                        event.event_id,
                    )

        # 3. Global handlers
        for handler in list(self._global_handlers):
            try:
                await handler(event)
            except Exception:
                logger.exception(
                    "Global handler %s failed for event %s",
                    handler.__qualname__,
                    event.event_id,
                )

        # 4. WebSocket broadcast
        if self._ws_broadcast is not None:
            try:
                await self._ws_broadcast(event)
            except Exception:
                logger.exception("WebSocket broadcast failed for event %s", event.event_id)

    # ------------------------------------------------------------------
    # Internals
    # ------------------------------------------------------------------

    async def _persist(self, event: BusEvent) -> None:
        """Append event JSON to the log file (if configured)."""
        if self._log_file is None:
            return
        try:
            line = event.model_dump_json() + "\n"
            self._log_file.write(line)
            self._log_file.flush()
        except Exception:
            logger.exception("Failed to persist event %s", event.event_id)

    # ------------------------------------------------------------------
    # Metrics / lifecycle
    # ------------------------------------------------------------------

    @property
    def event_count(self) -> int:
        """Total events published since bus creation."""
        return self._event_count

    @property
    def throughput(self) -> float:
        """Events per second (lifetime average)."""
        elapsed = time.monotonic() - self._start_time
        if elapsed <= 0:
            return 0.0
        return self._event_count / elapsed

    async def shutdown(self) -> None:
        """Flush and close the event log."""
        if self._log_file is not None:
            self._log_file.flush()
            self._log_file.close()
            self._log_file = None
            logger.info("Event bus shut down. Total events: %d", self._event_count)
