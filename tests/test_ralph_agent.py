import threading
import time

import requests

from swarm.api_server import SwarmAPIServer


def start_server_in_thread():
    server = SwarmAPIServer(host='127.0.0.1', port=8081)
    import asyncio
    loop = asyncio.new_event_loop()

    def run():
        asyncio.set_event_loop(loop)
        loop.run_until_complete(server.start())

    t = threading.Thread(target=run, daemon=True)
    t.start()
    time.sleep(1)
    return server, t, loop


def test_ralph_register_and_request_task(tmp_path):
    # Start server
    _server, _t, loop = start_server_in_thread()

    # Submit a task to queue
    r = requests.post('http://127.0.0.1:8081/api/tasks/submit', json={'workload_type': 'general_compute', 'payload': {'cmd': 'echo hi'}})
    assert r.status_code == 200 and r.json().get('success') is True
    r.json().get('task_id')

    # Register ralph
    r = requests.post('http://127.0.0.1:8081/api/gpu/register', json={'contributor_id': 'ralph-test', 'gpuInfo': {'vendor': 'none', 'model': 'cpu', 'vram': 0}})
    assert r.status_code == 200 and r.json().get('success') is True

    # Ralph requests a task
    r = requests.post('http://127.0.0.1:8081/api/tasks/request', json={'contributor_id': 'ralph-test'})
    payload = r.json()
    assert payload.get('success') is True and payload.get('task') is not None

    # Submit result
    task = payload['task']
    r = requests.post(f"http://127.0.0.1:8081/api/tasks/{task['task_id']}/result", json={'contributor_id': 'ralph-test', 'success': True, 'result': {'ok': True}})
    assert r.json().get('success') is True

    # stop server
    loop.call_soon_threadsafe(loop.stop)
