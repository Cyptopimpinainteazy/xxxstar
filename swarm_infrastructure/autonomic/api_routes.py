"""Autonomic API Routes — HTTP endpoints for the control plane.

Registers all /api/autonomic/* routes on the swarm aiohttp app.
Designed to be called from SwarmAPIServer.setup_routes().
"""

from __future__ import annotations

import json
import logging
from typing import TYPE_CHECKING

from aiohttp import web

if TYPE_CHECKING:
    from .bootstrap import AutonomicControlPlane

log = logging.getLogger("autonomic.api")


def register_autonomic_routes(app: web.Application, acp: "AutonomicControlPlane") -> None:
    """Register all autonomic API endpoints on an aiohttp Application."""

    async def health(request: web.Request) -> web.Response:
        """GET /api/autonomic/health — compact health."""
        return web.json_response(acp.health_summary())

    async def status(request: web.Request) -> web.Response:
        """GET /api/autonomic/status — full snapshot."""
        return web.json_response(acp.snapshot())

    async def circuit_breakers(request: web.Request) -> web.Response:
        """GET /api/autonomic/circuit-breakers"""
        data = {
            name: {"state": cb.state.value, "failures": cb._failure_count}
            for name, cb in acp.breakers.all().items()
        }
        return web.json_response(data)

    async def state_machine(request: web.Request) -> web.Response:
        """GET /api/autonomic/state"""
        return web.json_response(acp.state_machine.snapshot())

    async def audit(request: web.Request) -> web.Response:
        """GET /api/autonomic/audit?n=50"""
        n = int(request.query.get("n", "50"))
        return web.json_response(acp.audit_recent(n))

    async def gpu_guard(request: web.Request) -> web.Response:
        """GET /api/autonomic/gpu"""
        return web.json_response(acp.gpu_guard.snapshot())

    async def resources(request: web.Request) -> web.Response:
        """GET /api/autonomic/resources"""
        return web.json_response(acp.resource_monitor.snapshot())

    async def logs(request: web.Request) -> web.Response:
        """GET /api/autonomic/logs"""
        return web.json_response(acp.log_watcher.snapshot())

    async def operators(request: web.Request) -> web.Response:
        """GET /api/autonomic/operators"""
        return web.json_response(acp.operators.snapshot())

    async def health_history(request: web.Request) -> web.Response:
        """GET /api/autonomic/health/history?n=60"""
        n = int(request.query.get("n", "60"))
        return web.json_response(acp.health_engine.history(n))

    async def metrics_snapshot(request: web.Request) -> web.Response:
        """GET /api/autonomic/metrics"""
        return web.json_response(acp.bus.snapshot())

    # ── Manual overrides ─────────────────────────────────────────

    async def force_state(request: web.Request) -> web.Response:
        """POST /api/autonomic/override/state {"state": "normal", "reason": "..."}"""
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "invalid JSON"}, status=400)

        state = body.get("state")
        reason = body.get("reason", "manual override")
        if not state:
            return web.json_response({"error": "missing 'state'"}, status=400)

        ok = acp.orchestrator.force_state(state, reason)
        if not ok:
            return web.json_response({"error": f"invalid state: {state}"}, status=400)
        return web.json_response({"ok": True, "state": state})

    async def force_playbook(request: web.Request) -> web.Response:
        """POST /api/autonomic/override/playbook {"name": "...", "reason": "..."}"""
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "invalid JSON"}, status=400)

        name = body.get("name")
        reason = body.get("reason", "manual trigger")
        if not name:
            return web.json_response({"error": "missing 'name'"}, status=400)

        results = await acp.orchestrator.force_playbook(name, reason)
        if results is None:
            return web.json_response({"error": f"playbook not found: {name}"}, status=404)
        return web.json_response({
            "ok": True,
            "results": [
                {"action": f"{r.operator}.{r.action}",
                 "target": r.target,
                 "result": r.result.value,
                 "detail": r.detail}
                for r in results
            ]
        })

    async def reset_breaker(request: web.Request) -> web.Response:
        """POST /api/autonomic/override/circuit-breaker {"name": "..."}"""
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "invalid JSON"}, status=400)

        name = body.get("name")
        if not name:
            return web.json_response({"error": "missing 'name'"}, status=400)

        ok = acp.orchestrator.reset_circuit_breaker(name)
        if not ok:
            return web.json_response({"error": f"breaker not found: {name}"}, status=404)
        return web.json_response({"ok": True, "name": name})

    # ── Register routes ──────────────────────────────────────────

    app.router.add_get("/api/autonomic/health", health)
    app.router.add_get("/api/autonomic/status", status)
    app.router.add_get("/api/autonomic/circuit-breakers", circuit_breakers)
    app.router.add_get("/api/autonomic/state", state_machine)
    app.router.add_get("/api/autonomic/audit", audit)
    app.router.add_get("/api/autonomic/gpu", gpu_guard)
    app.router.add_get("/api/autonomic/resources", resources)
    app.router.add_get("/api/autonomic/logs", logs)
    app.router.add_get("/api/autonomic/operators", operators)
    app.router.add_get("/api/autonomic/health/history", health_history)
    app.router.add_get("/api/autonomic/metrics", metrics_snapshot)

    # Overrides (require POST)
    app.router.add_post("/api/autonomic/override/state", force_state)
    app.router.add_post("/api/autonomic/override/playbook", force_playbook)
    app.router.add_post("/api/autonomic/override/circuit-breaker", reset_breaker)

    log.info("Registered %d autonomic API routes", 14)
