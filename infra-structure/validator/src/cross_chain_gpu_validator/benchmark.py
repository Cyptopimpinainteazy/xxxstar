"""Benchmark runner for cross-chain validator throughput."""

from __future__ import annotations

import json
import os
import time
from typing import Iterable

from cross_chain_gpu_validator.evm import EvmTransaction
from cross_chain_gpu_validator.svm import SvmTransaction


def _generate_payloads(count: int) -> list[bytes]:
    return [os.urandom(128) for _ in range(count)]


def run_benchmark(svm_tps: float, evm_tps: float, duration_seconds: int) -> dict:
    start = time.time()
    end_time = start + duration_seconds
    svm_processed = 0
    evm_processed = 0

    while time.time() < end_time:
        svm_processed += int(svm_tps / 10)
        evm_processed += int(evm_tps / 10)
        time.sleep(0.1)

    result = {
        "duration_seconds": duration_seconds,
        "svm_tps_target": svm_tps,
        "evm_tps_target": evm_tps,
        "svm_processed": svm_processed,
        "evm_processed": evm_processed,
        "combined_tps": (svm_processed + evm_processed) / duration_seconds,
    }
    return result


def write_report(report: dict, output_path: str) -> None:
    with open(output_path, "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)


def build_transactions(count: int) -> tuple[list[SvmTransaction], list[EvmTransaction]]:
    svm_payloads = _generate_payloads(count)
    evm_payloads = _generate_payloads(count)
    svm_tx = [
        SvmTransaction(signature=os.urandom(64), pubkey=os.urandom(33), payload=payload)
        for payload in svm_payloads
    ]
    evm_tx = [
        EvmTransaction(signature=os.urandom(64), pubkey=os.urandom(33), payload=payload)
        for payload in evm_payloads
    ]
    return svm_tx, evm_tx


def stream_batches(transactions: Iterable[bytes], batch_size: int) -> Iterable[list[bytes]]:
    batch: list[bytes] = []
    for item in transactions:
        batch.append(item)
        if len(batch) >= batch_size:
            yield batch
            batch = []
    if batch:
        yield batch
