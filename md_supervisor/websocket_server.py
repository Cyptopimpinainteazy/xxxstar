"""
WebSocket server for md_supervisor — streams live updates to the VS Code panel.
Handles rollback requests, change approval, and real-time status streaming.
"""
import asyncio
import json
import logging
from typing import Set

import websockets

from md_supervisor.schema import ChangeRequest, AuditEntry
from md_supervisor.patcher import apply_change, rollback, preview_diff
from md_supervisor.dedupe import deduplicate
from md_supervisor.gates import GatePipeline

logger = logging.getLogger("md_supervisor.ws")

clients: Set[websockets.WebSocketServerProtocol] = set()


async def notify_all(message: dict):
    """Broadcast a message to all connected clients."""
    if not clients:
        return
    msg = json.dumps(message, default=str)
    await asyncio.gather(
        *[client.send(msg) for client in clients if client.open],
        return_exceptions=True
    )


async def handle_client(ws: websockets.WebSocketServerProtocol):
    """Handle an individual WebSocket client connection."""
    clients.add(ws)
    try:
        async for raw in ws:
            try:
                data = json.loads(raw)
                await process_message(ws, data)
            except json.JSONDecodeError:
                await ws.send(json.dumps({"type": "error", "payload": "invalid JSON"}))
    except websockets.ConnectionClosed:
        pass
    finally:
        clients.discard(ws)


async def process_message(ws: websockets.WebSocketServerProtocol, data: dict):
    """Process an incoming message from the panel."""
    msg_type = data.get("type")
    payload = data.get("payload", {})

    if msg_type == "rollback_request":
        node_id = payload.get("nodeId")
        if node_id:
            req = ChangeRequest(id=node_id)
            try:
                audit = rollback(req)
                await notify_all({
                    "type": "rollback_update",
                    "payload": {"nodeId": node_id, "status": "rolled_back", "audit": audit}
                })
            except Exception as e:
                await ws.send(json.dumps({
                    "type": "error",
                    "payload": f"Rollback failed for {node_id}: {e}"
                }))

    elif msg_type == "preview":
        node_id = payload.get("nodeId")
        content = payload.get("content", "")
        req = ChangeRequest(id=node_id)
        diff = preview_diff(req) if content else "No content to preview"
        await ws.send(json.dumps({"type": "preview", "payload": {"nodeId": node_id, "diff": diff}}))

    elif msg_type == "ping":
        await ws.send(json.dumps({"type": "pong"}))


async def websocket_server(host: str = "localhost", port: int = 8765):
    """Start the WebSocket server."""
    server = await websockets.serve(handle_client, host, port)
    logger.info(f"WebSocket server running on ws://{host}:{port}")
    print(f"🛰 WebSocket server: ws://{host}:{port}")
    await server.wait_closed()