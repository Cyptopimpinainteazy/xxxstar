from swarm.infra.gpu_manager import GPUManager, GPUCapabilities


def test_register_and_task_flow():
    gm = GPUManager(total_gpus=2)

    caps = GPUCapabilities(vendor='nvidia', device_name='gtx', vram_mb=8192, cuda=True, compute_score=10.0)
    gm.register('contrib-1', 'wallet1', caps)

    tid = gm.enqueue_task('general_compute', {'foo': 'bar'})
    assert tid

    res = gm.assign_task_to('contrib-1')
    assert res.task is not None

    ok = gm.submit_result('contrib-1', res.task.task_id, True, {'message': 'ok'}, None)
    assert ok
    t = gm.get_task(res.task.task_id)
    assert t.status == 'completed'
    assert t.result == {'message': 'ok'}


def test_rejects_wrong_contributor_result_submission():
    gm = GPUManager(total_gpus=2)
    caps = GPUCapabilities(vendor='nvidia', device_name='gtx', vram_mb=8192, cuda=True, compute_score=10.0)
    gm.register('contrib-1', 'wallet1', caps)
    gm.register('contrib-2', 'wallet2', caps)

    tid = gm.enqueue_task('general_compute', {'foo': 'bar'})
    res = gm.assign_task_to('contrib-1')
    assert res.task.task_id == tid

    assert gm.submit_result('contrib-2', tid, True, {'message': 'nope'}, None) is False
    assert gm.get_task(tid).status == 'assigned'


def test_skips_tasks_that_exceed_contributor_capabilities():
    gm = GPUManager(total_gpus=2)
    low_caps = GPUCapabilities(vendor='nvidia', device_name='gtx', vram_mb=4096, cuda=True, compute_score=2.0)
    high_caps = GPUCapabilities(vendor='nvidia', device_name='rtx', vram_mb=12288, cuda=True, compute_score=12.0)
    gm.register('low', 'wallet-low', low_caps)
    gm.register('high', 'wallet-high', high_caps)

    gm.enqueue_task('high_mem', {'foo': 'bar'}, required_vram_mb=8192, min_compute_score=8.0)
    gm.enqueue_task('general_compute', {'foo': 'baz'})

    low_result = gm.assign_task_to('low')
    assert low_result.task is not None
    assert low_result.task.workload_type == 'general_compute'

    high_result = gm.assign_task_to('high')
    assert high_result.task is not None
    assert high_result.task.workload_type == 'high_mem'
