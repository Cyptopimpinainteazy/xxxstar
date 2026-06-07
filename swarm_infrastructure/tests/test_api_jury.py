import asyncio
import pytest
from aiohttp import web
from aiohttp.test_utils import TestClient, TestServer
from swarm.api_server import SwarmAPIServer


@pytest.fixture
async def client(loop, aiohttp_client):
    server = SwarmAPIServer(host='127.0.0.1', port=0, total_gpus=2)
    app = web.Application()
    server.setup_routes(app)
    ts = await aiohttp_client(app)
    return ts


@pytest.mark.asyncio
async def test_jury_flow(client: TestClient):
    # Create session
    resp = await client.post('/api/jury/session', json={'tasks': ['task-1']})
    data = await resp.json()
    assert data['success'] is True
    session_id = data['session_id']

    # Commit phase
    for juror in ['juror-1', 'juror-2', 'juror-3']:
        r = await client.post('/api/jury/vote', json={'type': 'commit', 'session_id': session_id, 'contributor_id': juror, 'commitment': 'c1'})
        assert (await r.json())['success'] is True

    # Advance to reveal
    r = await client.post('/api/jury/vote', json={'type': 'advance', 'session_id': session_id})
    assert (await r.json())['success'] is True

    # Reveal
    for juror in ['juror-1', 'juror-2', 'juror-3']:
        r = await client.post('/api/jury/vote', json={'type': 'reveal', 'session_id': session_id, 'contributor_id': juror, 'vote': True})
        assert (await r.json())['success'] is True

    # Aggregate
    r = await client.post('/api/jury/vote', json={'type': 'aggregate', 'session_id': session_id})
    res = await r.json()
    assert res['result'] is True
