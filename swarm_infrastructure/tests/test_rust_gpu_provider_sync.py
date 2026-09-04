import asyncio

from aiohttp import web
from aiohttp.test_utils import TestServer

from swarm.gpu_bridge.provider import RustGpuProvider
from swarm.gpu_bridge.schema import GpuTaskStatus
from swarm.tests.test_gpu_bridge import _make_task


async def _make_server():
    pending = ["pending-1", "pending-2"]
    submitted = {}

    async def submit(request):
        payload = await request.json()
        submitted[payload["task_id"]] = payload
        return web.json_response({"task_id": payload["task_id"]})

    async def poll(request):
        task_id = request.match_info["task_id"]
        if task_id == "pending-task":
            return web.json_response({"task_id": task_id, "status": GpuTaskStatus.PENDING.value})
        if task_id not in submitted:
            return web.Response(status=404)
        return web.json_response(
            {
                "task_id": task_id,
                "agent_id": submitted[task_id]["agent_id"],
                "status": GpuTaskStatus.COMPLETED.value,
                "result_data": {"ok": True},
                "result_hash": "abc123",
                "compute_units_used": 1,
            }
        )

    async def cancel(request):
        task_id = request.match_info["task_id"]
        return web.json_response({"cancelled": task_id in submitted})

    async def list_pending(request):
        return web.json_response({"task_ids": pending})

    app = web.Application()
    app.router.add_post("/api/v1/tasks", submit)
    app.router.add_get("/api/v1/tasks/pending", list_pending)
    app.router.add_get("/api/v1/tasks/{task_id}", poll)
    app.router.add_post("/api/v1/tasks/{task_id}/cancel", cancel)

    server = TestServer(app)
    await server.start_server()
    return server, submitted, pending


def test_rust_gpu_provider_submit_and_poll():
    async def _scenario():
        server, submitted, _ = await _make_server()
        provider = RustGpuProvider(coordinator_url=str(server.make_url("")).rstrip("/"))
        try:
            task = _make_task()
            task_id = await provider.submit(task)
            assert task_id == task.task_id
            assert task_id in submitted

            result = await provider.poll(task_id)
            assert result is not None
            assert result.succeeded
            assert result.result_data["ok"] is True
        finally:
            await provider.close()
            await server.close()

    asyncio.run(_scenario())


def test_rust_gpu_provider_pending_cancel_and_list():
    async def _scenario():
        server, submitted, pending = await _make_server()
        provider = RustGpuProvider(coordinator_url=str(server.make_url("")).rstrip("/"))
        try:
            assert await provider.poll("pending-task") is None

            task = _make_task(agent_id="a-cancel")
            await provider.submit(task)
            assert await provider.cancel(task.task_id) is True
            assert await provider.list_pending() == pending
            assert submitted[task.task_id]["agent_id"] == "a-cancel"
        finally:
            await provider.close()
            await server.close()

    asyncio.run(_scenario())
