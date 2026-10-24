"""Signer-lease races and the atomic-swap kernel's curve order.

Both were found by the PR-Agent review of #677:
- the local lease was read, checked and rewritten without a lock, so two nodes
  could hold it at once;
- atomic_swap.cu carried a wrong secp256k1 order limb.
"""

from __future__ import annotations

import multiprocessing
import os
import re
import time
from pathlib import Path

import pytest

from cross_chain_gpu_validator.resilience.signer_lock import SignerLock

SECP256K1_N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
KERNEL = Path(__file__).resolve().parents[1] / "kernels" / "atomic_swap.cu"


@pytest.fixture(autouse=True)
def isolated_lock_dir(tmp_path, monkeypatch):
    monkeypatch.setenv("CCGV_DATA_DIR", str(tmp_path))
    return tmp_path


def _write_expired_lease(lock_dir: Path) -> None:
    (lock_dir / "x3_signer.lock").write_text(
        '{"node_id": "crashed", "expires_at": 0}', encoding="utf-8"
    )


def _contend(node_id, barrier, results):
    lock = SignerLock(node_id=node_id, redis_url=None, ttl_seconds=60)
    barrier.wait()
    results.put((node_id, lock.try_acquire()))


def test_only_one_node_takes_an_expired_lease(isolated_lock_dir):
    ctx = multiprocessing.get_context("fork")
    contenders = 8
    for round_no in range(15):
        _write_expired_lease(isolated_lock_dir)
        barrier = ctx.Barrier(contenders)
        results = ctx.Queue()
        procs = [
            ctx.Process(target=_contend, args=(f"n{round_no}-{i}", barrier, results))
            for i in range(contenders)
        ]
        for proc in procs:
            proc.start()
        outcomes = [results.get(timeout=30) for _ in procs]
        for proc in procs:
            proc.join(timeout=30)

        winners = [node for node, won in outcomes if won]
        assert len(winners) == 1, f"round {round_no}: {winners} all hold the lease"
        holder = SignerLock(node_id="probe", redis_url=None)._read_local_state()
        assert holder is not None and holder[0] == winners[0]
        os.unlink(isolated_lock_dir / "x3_signer.lock")


def test_renewal_after_takeover_does_not_overwrite_the_new_holder():
    old = SignerLock(node_id="old", redis_url=None, ttl_seconds=0.1)
    assert old.try_acquire()
    old._stop_renewal()
    time.sleep(0.2)

    new = SignerLock(node_id="new", redis_url=None, ttl_seconds=60)
    assert new.try_acquire()

    assert old.renew() is False
    assert new._read_local_state()[0] == "new"
    new.release()


def test_release_by_a_former_holder_leaves_the_new_lease():
    old = SignerLock(node_id="old", redis_url=None, ttl_seconds=0.1)
    assert old.try_acquire()
    old._stop_renewal()
    time.sleep(0.2)
    new = SignerLock(node_id="new", redis_url=None, ttl_seconds=60)
    assert new.try_acquire()

    old.release()
    assert new._read_local_state()[0] == "new"
    new.release()


def test_atomic_swap_kernel_uses_the_secp256k1_order():
    source = KERNEL.read_text(encoding="utf-8")
    block = re.search(r"const BigInt N = \{\s*\{([^}]*)\}", source)
    assert block, "curve order constant not found in atomic_swap.cu"
    limbs = [
        int(x.rstrip("uU"), 16) for x in re.findall(r"0x[0-9A-Fa-f]+[uU]?", block[1])
    ]
    assert len(limbs) == 8
    assert sum(limb << (32 * i) for i, limb in enumerate(limbs)) == SECP256K1_N
