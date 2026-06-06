import asyncio

from aiohttp import web
from aiohttp.test_utils import TestClient, TestServer

from swarm.api_server import SwarmAPIServer


async def _make_client() -> TestClient:
    server = SwarmAPIServer(host='127.0.0.1', port=0, total_gpus=2)
    app = web.Application()
    server.setup_routes(app)
    test_server = TestServer(app)
    client = TestClient(test_server)
    await client.start_server()
    return client


def test_gpu_register_and_task_flow():
    async def _scenario():
        client = await _make_client()
        try:
            r = await client.post('/api/gpu/register', json={'contributor_id': 'test-contrib', 'gpuInfo': {'vendor': 'nvidia', 'model': 'gtx', 'vram': 8192, 'cuda': True, 'computeScore': 10.0}})
            data = await r.json()
            assert data['success'] is True

            r = await client.post('/api/tasks/submit', json={'workload_type': 'general_compute', 'payload': {'foo': 'bar'}})
            td = await r.json()
            assert td['success'] is True
            task_id = td['task_id']

            r = await client.post('/api/tasks/request', json={'contributor_id': 'test-contrib'})
            req = await r.json()
            assert req['success'] is True
            assert req['task']['task_id'] == task_id

            r = await client.post(f'/api/tasks/{task_id}/result', json={'contributor_id': 'test-contrib', 'success': True, 'result': {'message': 'done'}})
            res = await r.json()
            assert res['success'] is True

            r = await client.get(f'/api/tasks/{task_id}/status')
            s = await r.json()
            assert s['status'] == 'completed'

            r = await client.get(f'/api/tasks/{task_id}')
            task = await r.json()
            assert task['task_id'] == task_id
            assert task['status'] == 'completed'
            assert task['result'] == {'message': 'done'}

            r = await client.get('/api/status')
            status = await r.json()
            assert status['swarm']['queue_stats']['completed'] >= 1

            r = await client.get('/api/jobs/distribution')
            distribution = await r.json()
            assert 'queue_depth' in distribution
        finally:
            await client.close()

    asyncio.run(_scenario())


def test_task_result_rejects_wrong_contributor():
    async def _scenario():
        client = await _make_client()
        try:
            await client.post('/api/gpu/register', json={'contributor_id': 'worker-a', 'gpuInfo': {'vendor': 'nvidia', 'model': 'gtx', 'vram': 8192, 'cuda': True, 'computeScore': 10.0}})
            await client.post('/api/gpu/register', json={'contributor_id': 'worker-b', 'gpuInfo': {'vendor': 'nvidia', 'model': 'gtx', 'vram': 8192, 'cuda': True, 'computeScore': 10.0}})

            submitted = await client.post('/api/tasks/submit', json={'workload_type': 'general_compute', 'payload': {'foo': 'bar'}})
            task_id = (await submitted.json())['task_id']

            claimed = await client.post('/api/tasks/request', json={'contributor_id': 'worker-a'})
            claim_data = await claimed.json()
            assert claim_data['success'] is True

            wrong_submit = await client.post(
                f'/api/tasks/{task_id}/result',
                json={'contributor_id': 'worker-b', 'success': True, 'result': {'message': 'bad actor'}},
            )
            wrong_submit_data = await wrong_submit.json()
            assert wrong_submit_data['success'] is False

            status = await client.get(f'/api/tasks/{task_id}')
            task = await status.json()
            assert task['status'] == 'assigned'
        finally:
            await client.close()

    asyncio.run(_scenario())
