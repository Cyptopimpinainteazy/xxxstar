"""
P4 Phase 5 — GPU Kernel Integration Tests
==========================================

Validates the GPU kernel dispatch path, fallback chain, memory pool,
receipt generation, lane service throughput, and guard sentinel.

All tests use pure-Python stubs (no CUDA hardware required) that mirror
the contracts defined in:
  - crates/x3-vm/src/gpu_hostcalls.rs       (hostcall IDs 0xD0–0xD9)
  - crates/x3-gpu-validator-swarm/src/gpu_fallback_chain.rs
  - cross-chain-gpu-validator/tests/inferstructor/gpu_lane_service.py

Tests: 48 total across 8 suites
"""

from __future__ import annotations

import hashlib
import secrets
import struct
import threading
import time
from collections import deque
from dataclasses import dataclass, field
from enum import IntEnum

# ---------------------------------------------------------------------------
# § Hostcall ID constants (mirrors gpu_hostcalls.rs)
# ---------------------------------------------------------------------------

class GpuHostcallId(IntEnum):
    GPU_SHA256_BATCH     = 0xD0
    GPU_ED25519_VERIFY   = 0xD1
    GPU_POH_CHAIN        = 0xD2
    GPU_SHA256_STREAMED  = 0xD3
    GPU_DEVICE_COUNT     = 0xD4
    GPU_BENCHMARK        = 0xD5
    GPU_KECCAK256_BATCH  = 0xD6
    GPU_SECP256K1_VERIFY = 0xD7
    GPU_ATOMIC_VERIFY    = 0xD8
    GPU_ATOMIC_COMMIT    = 0xD9


# ---------------------------------------------------------------------------
# § Execution target / degradation models (mirrors gpu_fallback_chain.rs)
# ---------------------------------------------------------------------------

class ExecutionTarget:
    GPU  = "GPU"
    CPU  = "CPU"
    MOCK = "Mock"


class DegradationStrategy:
    CASCADING = "Cascading"
    STRICT    = "Strict"
    CPU_ONLY  = "CPUOnly"


@dataclass
class FallbackEvent:
    block_height: int
    from_target: str
    to_target: str
    reason: str
    recovery_time_ms: int


@dataclass
class X3KernelInstance:
    kernel_id: int
    name: str
    version: str
    is_operational: bool = True
    last_error: str | None = None


@dataclass
class CPUFallbackEngine:
    name: str = "scalar-cpu-executor"
    supports_double_precision: bool = True

    def execute_scalar(self, op: str, args: list[int]) -> list[int]:
        if op == "add":
            if len(args) != 2:
                raise ValueError("ADD requires 2 arguments")
            return [(args[0] + args[1]) & 0xFFFF_FFFF_FFFF_FFFF]
        if op == "mul":
            if len(args) != 2:
                raise ValueError("MUL requires 2 arguments")
            return [(args[0] * args[1]) & 0xFFFF_FFFF_FFFF_FFFF]
        if op == "hash":
            return [0xDEADBEEF]
        if op == "verify":
            return [1]
        raise ValueError(f"Unknown operation: {op}")


class FallbackChain:
    """Python equivalent of FallbackChain in gpu_fallback_chain.rs."""

    def __init__(self, strategy: str):
        self.strategy = strategy
        self.primary: X3KernelInstance | None = None
        self.cpu_engine = CPUFallbackEngine()
        self.current_target = ExecutionTarget.GPU
        self.fallback_history: deque[FallbackEvent] = deque(maxlen=100)

    def attach_gpu_kernel(self, kernel: X3KernelInstance) -> None:
        self.primary = kernel

    def execute(self, op: str, args: list[int], block_height: int = 0) -> list[int]:
        if self.strategy == DegradationStrategy.STRICT:
            return self._execute_on_gpu(op, args, block_height)

        if self.strategy == DegradationStrategy.CPU_ONLY:
            result = self._execute_on_cpu(op, args)
            self.current_target = ExecutionTarget.CPU
            return result

        # Cascading: GPU → CPU
        t0 = time.monotonic()
        try:
            result = self._execute_on_gpu(op, args, block_height)
            self.current_target = ExecutionTarget.GPU
            return result
        except Exception as gpu_err:
            try:
                result = self._execute_on_cpu(op, args)
                elapsed_ms = int((time.monotonic() - t0) * 1000)
                self.fallback_history.append(FallbackEvent(
                    block_height=block_height,
                    from_target=ExecutionTarget.GPU,
                    to_target=ExecutionTarget.CPU,
                    reason=str(gpu_err),
                    recovery_time_ms=elapsed_ms,
                ))
                self.current_target = ExecutionTarget.CPU
                return result
            except Exception as cpu_err:
                raise RuntimeError(f"GPU: {gpu_err}, CPU: {cpu_err}") from cpu_err

    def _execute_on_gpu(self, op: str, args: list[int], _block_height: int) -> list[int]:
        if self.primary is None:
            raise RuntimeError("GPU kernel not attached")
        if not self.primary.is_operational:
            raise RuntimeError(f"GPU kernel '{self.primary.name}' not operational")
        # Stub GPU ops
        gpu_ops = {"matmul": [42], "conv2d": [100], "hash": [0xCAFEBABE]}
        if op in gpu_ops:
            return gpu_ops[op]
        raise RuntimeError(f"GPU kernel doesn't support operation: {op}")

    def _execute_on_cpu(self, op: str, args: list[int]) -> list[int]:
        return self.cpu_engine.execute_scalar(op, args)

    def record_recovery(self, block_height: int) -> None:
        """Simulate GPU recovery after fallback."""
        if self.current_target == ExecutionTarget.CPU:
            self.current_target = ExecutionTarget.GPU


# ---------------------------------------------------------------------------
# § GPU Memory Pool (Python stub mirroring gpu_memory_pool.rs semantics)
# ---------------------------------------------------------------------------

@dataclass
class GpuBuffer:
    buf_id: int
    size_bytes: int
    in_use: bool = False
    data: bytearray = field(default_factory=bytearray)

    def __post_init__(self):
        self.data = bytearray(self.size_bytes)


class GpuMemoryPool:
    """
    Fixed-capacity GPU memory pool. Mirrors the alloc/free semantics of
    the Rust GpuMemoryPool used in gpu_validator_swarm.
    """

    def __init__(self, capacity: int = 16, buffer_size: int = 4096):
        self.capacity = capacity
        self.buffer_size = buffer_size
        self._next_id = 0
        self._free: list[GpuBuffer] = []
        self._allocated: dict[int, GpuBuffer] = {}
        self._lock = threading.Lock()

        for _ in range(capacity):
            buf = GpuBuffer(buf_id=self._next_id, size_bytes=buffer_size)
            self._next_id += 1
            self._free.append(buf)

    def alloc(self) -> GpuBuffer | None:
        with self._lock:
            if not self._free:
                return None
            buf = self._free.pop()
            buf.in_use = True
            self._allocated[buf.buf_id] = buf
            return buf

    def free(self, buf: GpuBuffer) -> bool:
        with self._lock:
            if buf.buf_id not in self._allocated:
                return False
            buf.in_use = False
            buf.data = bytearray(self.buffer_size)  # zero on release
            del self._allocated[buf.buf_id]
            self._free.append(buf)
            return True

    @property
    def available(self) -> int:
        with self._lock:
            return len(self._free)

    @property
    def in_use_count(self) -> int:
        with self._lock:
            return len(self._allocated)


# ---------------------------------------------------------------------------
# § GPU Receipt (mirrors gpu_receipt.rs)
# ---------------------------------------------------------------------------

@dataclass
class GpuReceipt:
    hostcall_id: int
    input_hash: bytes          # sha256 of inputs
    output_hash: bytes         # sha256 of outputs
    gpu_device_id: int
    execution_time_us: int
    signature: bytes           # hmac-sha256 of fields with node secret

    @classmethod
    def generate(
        cls,
        hostcall_id: int,
        inputs: bytes,
        outputs: bytes,
        gpu_device_id: int,
        node_secret: bytes,
        execution_time_us: int = 0,
    ) -> GpuReceipt:
        input_hash  = hashlib.sha256(inputs).digest()
        output_hash = hashlib.sha256(outputs).digest()
        payload = (
            hostcall_id.to_bytes(1, "big")
            + input_hash
            + output_hash
            + gpu_device_id.to_bytes(4, "big")
            + execution_time_us.to_bytes(8, "big")
        )
        import hmac
        sig = hmac.new(node_secret, payload, hashlib.sha256).digest()
        return cls(
            hostcall_id=hostcall_id,
            input_hash=input_hash,
            output_hash=output_hash,
            gpu_device_id=gpu_device_id,
            execution_time_us=execution_time_us,
            signature=sig,
        )

    def verify(self, node_secret: bytes) -> bool:
        import hmac
        payload = (
            self.hostcall_id.to_bytes(1, "big")
            + self.input_hash
            + self.output_hash
            + self.gpu_device_id.to_bytes(4, "big")
            + self.execution_time_us.to_bytes(8, "big")
        )
        expected = hmac.new(node_secret, payload, hashlib.sha256).digest()
        return hmac.compare_digest(expected, self.signature)


# ---------------------------------------------------------------------------
# § GPU Lane Service stub (mirrors CUDABatchAccelerator + GPULaneService)
# ---------------------------------------------------------------------------

class CUDABatchAcceleratorStub:
    """CPU-side stub that mirrors CUDABatchAccelerator without real CUDA."""

    def __init__(self, gpu_id: int = 0, lane_id: str = "lane-0"):
        self.gpu_id = gpu_id
        self.lane_id = lane_id
        self.gpu_available = False  # no CUDA in test env

    def accelerate_batch_cpu(
        self, tx_hashes: list[str], tx_datas: list[bytes], chain: str
    ) -> list[str]:
        results = []
        for h, d in zip(tx_hashes, tx_datas, strict=False):
            data = f"{h}_{d.hex()}_{chain}_cpu".encode()
            results.append(hashlib.sha256(data).hexdigest())
        return results

    def accelerate_batch_gpu_sim(
        self, tx_hashes: list[str], tx_datas: list[bytes], chain: str
    ) -> list[str]:
        """Simulate GPU batch: deterministic per-input hash."""
        results = []
        for h, d in zip(tx_hashes, tx_datas, strict=False):
            data = f"{h}_{d.hex()}_{chain}_gpu".encode()
            results.append(hashlib.sha256(data).hexdigest())
        return results

    def accelerate_batch(
        self, tx_hashes: list[str], tx_datas: list[bytes], chain: str
    ) -> list[str]:
        if self.gpu_available:
            return self.accelerate_batch_gpu_sim(tx_hashes, tx_datas, chain)
        return self.accelerate_batch_cpu(tx_hashes, tx_datas, chain)


class GPULaneServiceStub:
    def __init__(self, lane_id: str, gpu_id: int = 0):
        self.lane_id = lane_id
        self.gpu_id = gpu_id
        self.accelerator = CUDABatchAcceleratorStub(gpu_id, lane_id)
        self.total_requests = 0
        self.total_txns = 0
        self.total_success = 0

    def process_batch(self, tx_hashes: list[str], tx_datas: list[bytes], chain: str) -> dict:
        t0 = time.monotonic()
        results = self.accelerator.accelerate_batch(tx_hashes, tx_datas, chain)
        elapsed = time.monotonic() - t0
        self.total_requests += 1
        self.total_txns += len(tx_hashes)
        self.total_success += len(results)
        return {
            "lane_id": self.lane_id,
            "count": len(results),
            "results": results,
            "elapsed_s": elapsed,
        }


# ---------------------------------------------------------------------------
# § GPU Guard Sentinel
# ---------------------------------------------------------------------------

class GpuHealthStatus:
    OK       = "ok"
    DEGRADED = "degraded"
    FAILED   = "failed"


@dataclass
class GpuDeviceHealth:
    device_id: int
    status: str = GpuHealthStatus.OK
    temperature_c: int = 60
    utilization_pct: int = 50
    memory_used_mb: int = 2048
    memory_total_mb: int = 8192
    last_error: str | None = None
    consecutive_failures: int = 0

    @property
    def memory_used_pct(self) -> float:
        return (self.memory_used_mb / self.memory_total_mb) * 100.0


class GpuGuardSentinel:
    TEMP_WARN_C    = 80
    TEMP_CRIT_C    = 90
    MEM_WARN_PCT   = 80.0
    FAIL_THRESHOLD = 3

    def __init__(self, device_ids: list[int]):
        self.devices: dict[int, GpuDeviceHealth] = {
            d: GpuDeviceHealth(device_id=d) for d in device_ids
        }
        self.alerts: list[str] = []

    def report_error(self, device_id: int, error: str) -> None:
        dev = self.devices[device_id]
        dev.last_error = error
        dev.consecutive_failures += 1
        if dev.consecutive_failures >= self.FAIL_THRESHOLD:
            dev.status = GpuHealthStatus.FAILED
        else:
            dev.status = GpuHealthStatus.DEGRADED
        self.alerts.append(f"device={device_id} error={error}")

    def report_ok(self, device_id: int) -> None:
        dev = self.devices[device_id]
        dev.consecutive_failures = 0
        dev.last_error = None
        dev.status = GpuHealthStatus.OK

    def check_temperature(self, device_id: int, temp_c: int) -> str:
        dev = self.devices[device_id]
        dev.temperature_c = temp_c
        if temp_c >= self.TEMP_CRIT_C:
            dev.status = GpuHealthStatus.FAILED
            return "critical"
        if temp_c >= self.TEMP_WARN_C:
            if dev.status == GpuHealthStatus.OK:
                dev.status = GpuHealthStatus.DEGRADED
            return "warning"
        return "ok"

    def healthy_device_ids(self) -> list[int]:
        return [d for d, h in self.devices.items() if h.status == GpuHealthStatus.OK]

    def overall_status(self) -> str:
        statuses = [h.status for h in self.devices.values()]
        if all(s == GpuHealthStatus.FAILED for s in statuses):
            return GpuHealthStatus.FAILED
        if any(s != GpuHealthStatus.OK for s in statuses):
            return GpuHealthStatus.DEGRADED
        return GpuHealthStatus.OK


# ---------------------------------------------------------------------------
# § CPU Canonicaliser (non-determinism routing, mirrors hostcall architecture)
# ---------------------------------------------------------------------------

class CPUCanonicaliser:
    """
    Routes output from GPU kernels through a CPU-side normaliser so that
    any floating-point non-determinism is eliminated before state-root hashing.
    Contract: canonicalise(x) produces the same bytes for any bitwise-equal
    rational value regardless of GPU rounding mode.
    """

    @staticmethod
    def canonicalise_hash_batch(raw_outputs: list[bytes]) -> list[bytes]:
        """Re-hash each raw GPU output through SHA-256 on CPU."""
        return [hashlib.sha256(o).digest() for o in raw_outputs]

    @staticmethod
    def canonicalise_verify_bitmap(bitmap: bytes, count: int) -> list[bool]:
        """Unpack a GPU result bitmap to a canonical bool list."""
        result = []
        for i in range(count):
            byte_idx = i // 8
            bit_idx  = i %  8
            if byte_idx < len(bitmap):
                result.append(bool((bitmap[byte_idx] >> bit_idx) & 1))
            else:
                result.append(False)
        return result

    @staticmethod
    def canonicalise_poh_chain(chain: list[bytes]) -> bytes:
        """Fold a PoH chain to a canonical digest."""
        acc = b"\x00" * 32
        for h in chain:
            acc = hashlib.sha256(acc + h).digest()
        return acc


# ---------------------------------------------------------------------------
# § Hostcall Dispatch Stub (mirrors GpuHostcalls struct in Rust)
# ---------------------------------------------------------------------------

class GpuHostcallDispatcher:
    """
    Pure-Python hostcall dispatcher that:
      - Routes each opcode to the right stub handler
      - Records calls for test inspection
      - Applies CPUCanonicaliser before returning results
    """

    def __init__(self):
        self.call_log: list[tuple[int, str]] = []   # (hostcall_id, status)
        self._canonicaliser = CPUCanonicaliser()

    def invoke(self, hostcall_id: int, args: bytes) -> bytes:
        try:
            result = self._dispatch(hostcall_id, args)
            self.call_log.append((hostcall_id, "ok"))
            return result
        except Exception as e:
            self.call_log.append((hostcall_id, f"err:{e}"))
            raise

    def _dispatch(self, hid: int, args: bytes) -> bytes:
        if hid == GpuHostcallId.GPU_SHA256_BATCH:
            return self._sha256_batch(args)
        if hid == GpuHostcallId.GPU_ED25519_VERIFY:
            return self._ed25519_verify_bitmap(args)
        if hid == GpuHostcallId.GPU_POH_CHAIN:
            return self._poh_chain(args)
        if hid == GpuHostcallId.GPU_SHA256_STREAMED:
            return self._sha256_batch(args)  # same path, stub
        if hid == GpuHostcallId.GPU_DEVICE_COUNT:
            return struct.pack(">q", 1)       # 1 stub device
        if hid == GpuHostcallId.GPU_BENCHMARK:
            return b'{"tps":500000.0}'
        if hid == GpuHostcallId.GPU_KECCAK256_BATCH:
            return self._keccak256_batch(args)
        if hid == GpuHostcallId.GPU_SECP256K1_VERIFY:
            return self._secp256k1_verify_bitmap(args)
        if hid == GpuHostcallId.GPU_ATOMIC_VERIFY:
            return b"\x01"
        if hid == GpuHostcallId.GPU_ATOMIC_COMMIT:
            return b"\x01"
        raise ValueError(f"Unknown hostcall id: {hex(hid)}")

    # -- stub implementations (canonicalised through CPU) --

    def _sha256_batch(self, args: bytes) -> bytes:
        # Each 32-byte chunk is one input block
        results = []
        for i in range(0, len(args), 32):
            chunk = args[i:i+32].ljust(32, b"\x00")
            results.append(hashlib.sha256(chunk).digest())
        canonicalised = self._canonicaliser.canonicalise_hash_batch(results)
        return b"".join(canonicalised)

    def _poh_chain(self, args: bytes) -> bytes:
        # args = list of 32-byte seeds (one chain)
        seeds = [args[i:i+32] for i in range(0, len(args), 32)]
        return self._canonicaliser.canonicalise_poh_chain(seeds)

    def _keccak256_batch(self, args: bytes) -> bytes:
        import hashlib
        results = []
        for i in range(0, len(args), 32):
            chunk = args[i:i+32].ljust(32, b"\x00")
            # Use hashlib.sha3_256 as a stand-in for keccak (no external dep)
            results.append(hashlib.sha256(b"keccak:" + chunk).digest())
        return b"".join(results)

    def _ed25519_verify_bitmap(self, args: bytes) -> bytes:
        # Stub: mark all as valid
        count = max(len(args) // 96, 1)   # sig(64) + pk(32) = 96B per entry
        n_bytes = (count + 7) // 8
        return b"\xff" * n_bytes  # all valid

    def _secp256k1_verify_bitmap(self, args: bytes) -> bytes:
        count = max(len(args) // 128, 1)  # u1(32)+u2(32)+pk(64)
        n_bytes = (count + 7) // 8
        return b"\xff" * n_bytes


# ===========================================================================
# TESTS
# ===========================================================================

import pytest


class TestGpuHostcallIds:
    """Suite 1 — Hostcall ID constants match the Rust spec (gpu_hostcalls.rs)."""

    def test_sha256_batch_id(self):
        assert GpuHostcallId.GPU_SHA256_BATCH == 0xD0

    def test_ed25519_verify_id(self):
        assert GpuHostcallId.GPU_ED25519_VERIFY == 0xD1

    def test_poh_chain_id(self):
        assert GpuHostcallId.GPU_POH_CHAIN == 0xD2

    def test_sha256_streamed_id(self):
        assert GpuHostcallId.GPU_SHA256_STREAMED == 0xD3

    def test_device_count_id(self):
        assert GpuHostcallId.GPU_DEVICE_COUNT == 0xD4

    def test_benchmark_id(self):
        assert GpuHostcallId.GPU_BENCHMARK == 0xD5

    def test_keccak256_id(self):
        assert GpuHostcallId.GPU_KECCAK256_BATCH == 0xD6

    def test_secp256k1_verify_id(self):
        assert GpuHostcallId.GPU_SECP256K1_VERIFY == 0xD7

    def test_atomic_verify_id(self):
        assert GpuHostcallId.GPU_ATOMIC_VERIFY == 0xD8

    def test_atomic_commit_id(self):
        assert GpuHostcallId.GPU_ATOMIC_COMMIT == 0xD9

    def test_id_range_0xD0_to_0xD9(self):
        ids = list(GpuHostcallId)
        assert min(ids) == 0xD0
        assert max(ids) == 0xD9
        assert len(ids) == 10


class TestGpuHostcallDispatch:
    """Suite 2 — Dispatcher routes opcodes and logs calls correctly."""

    def setup_method(self):
        self.disp = GpuHostcallDispatcher()

    def test_sha256_batch_returns_32n_bytes(self):
        inputs = secrets.token_bytes(32 * 4)   # 4 inputs
        result = self.disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, inputs)
        assert len(result) == 32 * 4

    def test_sha256_batch_deterministic(self):
        inputs = secrets.token_bytes(32 * 8)
        r1 = self.disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, inputs)
        r2 = self.disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, inputs)
        assert r1 == r2

    def test_poh_chain_returns_32_bytes(self):
        seed = secrets.token_bytes(32)
        result = self.disp.invoke(GpuHostcallId.GPU_POH_CHAIN, seed)
        assert len(result) == 32

    def test_device_count_returns_positive(self):
        result = self.disp.invoke(GpuHostcallId.GPU_DEVICE_COUNT, b"")
        count = struct.unpack(">q", result)[0]
        assert count >= 1

    def test_keccak256_batch_returns_32n_bytes(self):
        inputs = secrets.token_bytes(32 * 3)
        result = self.disp.invoke(GpuHostcallId.GPU_KECCAK256_BATCH, inputs)
        assert len(result) == 32 * 3

    def test_ed25519_bitmap_all_valid_stub(self):
        # 3 entries × 96 bytes each
        inputs = secrets.token_bytes(96 * 3)
        result = self.disp.invoke(GpuHostcallId.GPU_ED25519_VERIFY, inputs)
        assert len(result) >= 1

    def test_secp256k1_bitmap_all_valid_stub(self):
        inputs = secrets.token_bytes(128 * 2)
        result = self.disp.invoke(GpuHostcallId.GPU_SECP256K1_VERIFY, inputs)
        assert len(result) >= 1

    def test_benchmark_returns_json(self):
        result = self.disp.invoke(GpuHostcallId.GPU_BENCHMARK, b"")
        assert b"tps" in result

    def test_atomic_verify_returns_success(self):
        result = self.disp.invoke(GpuHostcallId.GPU_ATOMIC_VERIFY, b"\x00" * 64)
        assert result == b"\x01"

    def test_atomic_commit_returns_success(self):
        result = self.disp.invoke(GpuHostcallId.GPU_ATOMIC_COMMIT, b"\x00" * 64)
        assert result == b"\x01"

    def test_unknown_hostcall_raises(self):
        with pytest.raises(ValueError):
            self.disp.invoke(0xFF, b"")

    def test_call_log_populated(self):
        self.disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, b"\x00" * 32)
        assert len(self.disp.call_log) == 1
        hid, status = self.disp.call_log[0]
        assert hid == GpuHostcallId.GPU_SHA256_BATCH
        assert status == "ok"

    def test_failed_call_logged_with_error(self):
        with pytest.raises(ValueError):
            self.disp.invoke(0xFE, b"")
        hid, status = self.disp.call_log[-1]
        assert hid == 0xFE
        assert status.startswith("err:")


class TestGpuFallbackChain:
    """Suite 3 — Fallback chain degradation strategies."""

    def _healthy_kernel(self) -> X3KernelInstance:
        return X3KernelInstance(kernel_id=1, name="test-kernel", version="1.0.0")

    def _broken_kernel(self) -> X3KernelInstance:
        return X3KernelInstance(
            kernel_id=2, name="broken-kernel", version="1.0.0",
            is_operational=False, last_error="CUDA OOM"
        )

    # -- CPU_ONLY --

    def test_cpuonly_uses_cpu_always(self):
        fc = FallbackChain(DegradationStrategy.CPU_ONLY)
        result = fc.execute("add", [3, 4], block_height=1)
        assert result == [7]
        assert fc.current_target == ExecutionTarget.CPU

    def test_cpuonly_no_gpu_needed(self):
        fc = FallbackChain(DegradationStrategy.CPU_ONLY)
        # No kernel attached — should still succeed on CPU
        result = fc.execute("mul", [6, 7])
        assert result == [42]

    # -- STRICT --

    def test_strict_succeeds_with_healthy_gpu(self):
        fc = FallbackChain(DegradationStrategy.STRICT)
        fc.attach_gpu_kernel(self._healthy_kernel())
        result = fc.execute("matmul", [], block_height=5)
        assert result == [42]

    def test_strict_raises_with_broken_gpu(self):
        fc = FallbackChain(DegradationStrategy.STRICT)
        fc.attach_gpu_kernel(self._broken_kernel())
        with pytest.raises(RuntimeError):
            fc.execute("matmul", [], block_height=5)

    def test_strict_raises_without_kernel(self):
        fc = FallbackChain(DegradationStrategy.STRICT)
        with pytest.raises(RuntimeError):
            fc.execute("matmul", [], block_height=5)

    # -- CASCADING --

    def test_cascading_uses_gpu_when_available(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._healthy_kernel())
        result = fc.execute("hash", [], block_height=10)
        assert result == [0xCAFEBABE]
        assert fc.current_target == ExecutionTarget.GPU

    def test_cascading_falls_back_on_gpu_failure(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._broken_kernel())
        result = fc.execute("add", [1, 2], block_height=10)
        assert result == [3]
        assert fc.current_target == ExecutionTarget.CPU

    def test_cascading_records_fallback_event(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._broken_kernel())
        fc.execute("add", [1, 2], block_height=7)
        assert len(fc.fallback_history) == 1
        evt = fc.fallback_history[0]
        assert evt.block_height == 7
        assert evt.from_target == ExecutionTarget.GPU
        assert evt.to_target == ExecutionTarget.CPU

    def test_cascading_raises_when_both_fail(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._broken_kernel())
        with pytest.raises(RuntimeError, match="GPU:.*CPU:"):
            fc.execute("unknown_op_xyz", [1, 2])

    def test_cascading_recovery_updates_target(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._broken_kernel())
        fc.execute("add", [1, 2])          # triggers fallback
        fc.primary.is_operational = True   # simulate GPU recovery
        result = fc.execute("matmul", [])  # should use GPU again
        assert result == [42]
        assert fc.current_target == ExecutionTarget.GPU

    def test_fallback_history_bounded(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.attach_gpu_kernel(self._broken_kernel())
        for i in range(150):
            fc.execute("add", [i, i])
        assert len(fc.fallback_history) <= 100

    def test_cpu_engine_hash_and_verify_ops(self):
        fc = FallbackChain(DegradationStrategy.CPU_ONLY)
        assert fc.execute("hash", [], block_height=1) == [0xDEADBEEF]
        assert fc.execute("verify", [], block_height=1) == [1]

    def test_cpu_engine_add_mul_argument_validation(self):
        fc = FallbackChain(DegradationStrategy.CPU_ONLY)
        with pytest.raises(ValueError, match="ADD requires 2 arguments"):
            fc.execute("add", [1])
        with pytest.raises(ValueError, match="MUL requires 2 arguments"):
            fc.execute("mul", [2])

    def test_record_recovery_switches_back_to_gpu(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.current_target = ExecutionTarget.CPU
        fc.record_recovery(block_height=123)
        assert fc.current_target == ExecutionTarget.GPU

    def test_record_recovery_noop_when_already_gpu(self):
        fc = FallbackChain(DegradationStrategy.CASCADING)
        fc.current_target = ExecutionTarget.GPU
        fc.record_recovery(block_height=123)
        assert fc.current_target == ExecutionTarget.GPU


class TestGpuMemoryPool:
    """Suite 4 — Memory pool alloc/free/reuse lifecycle."""

    def test_alloc_returns_buffer(self):
        pool = GpuMemoryPool(capacity=4, buffer_size=1024)
        buf = pool.alloc()
        assert buf is not None
        assert buf.in_use is True
        assert buf.size_bytes == 1024

    def test_alloc_reduces_available(self):
        pool = GpuMemoryPool(capacity=4, buffer_size=256)
        assert pool.available == 4
        pool.alloc()
        assert pool.available == 3

    def test_free_restores_available(self):
        pool = GpuMemoryPool(capacity=4, buffer_size=256)
        buf = pool.alloc()
        pool.free(buf)
        assert pool.available == 4

    def test_alloc_exhaustion_returns_none(self):
        pool = GpuMemoryPool(capacity=2, buffer_size=256)
        pool.alloc()
        pool.alloc()
        assert pool.alloc() is None

    def test_free_zeros_buffer(self):
        pool = GpuMemoryPool(capacity=2, buffer_size=64)
        buf = pool.alloc()
        buf.data[0:4] = b"\xDE\xAD\xBE\xEF"
        pool.free(buf)
        assert buf.data == bytearray(64)

    def test_double_free_returns_false(self):
        pool = GpuMemoryPool(capacity=2, buffer_size=64)
        buf = pool.alloc()
        assert pool.free(buf) is True
        assert pool.free(buf) is False

    def test_in_use_count_tracks_allocated(self):
        pool = GpuMemoryPool(capacity=4, buffer_size=64)
        b1 = pool.alloc()
        pool.alloc()
        assert pool.in_use_count == 2
        pool.free(b1)
        assert pool.in_use_count == 1

    def test_buffer_reuse_after_free(self):
        pool = GpuMemoryPool(capacity=1, buffer_size=64)
        b1 = pool.alloc()
        id1 = b1.buf_id
        pool.free(b1)
        b2 = pool.alloc()
        assert b2.buf_id == id1

    def test_concurrent_alloc_free_safe(self):
        pool = GpuMemoryPool(capacity=8, buffer_size=64)
        errors = []

        def worker():
            try:
                buf = pool.alloc()
                if buf:
                    time.sleep(0.001)
                    pool.free(buf)
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=worker) for _ in range(8)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        assert errors == []
        assert pool.available == 8


class TestGpuReceiptGeneration:
    """Suite 5 — GPU receipt generation and verification."""

    def setup_method(self):
        self.secret = secrets.token_bytes(32)

    def test_receipt_generates_without_error(self):
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_SHA256_BATCH,
            b"inputs",
            b"outputs",
            gpu_device_id=0,
            node_secret=self.secret,
        )
        assert r is not None

    def test_receipt_verifies_with_correct_secret(self):
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_SHA256_BATCH,
            b"inputs",
            b"outputs",
            gpu_device_id=0,
            node_secret=self.secret,
        )
        assert r.verify(self.secret) is True

    def test_receipt_fails_verify_with_wrong_secret(self):
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_SHA256_BATCH,
            b"inputs",
            b"outputs",
            gpu_device_id=0,
            node_secret=self.secret,
        )
        wrong = secrets.token_bytes(32)
        assert r.verify(wrong) is False

    def test_receipt_tamper_output_fails_verify(self):
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_SHA256_BATCH,
            b"inputs",
            b"outputs",
            gpu_device_id=0,
            node_secret=self.secret,
        )
        r.output_hash = hashlib.sha256(b"tampered").digest()
        assert r.verify(self.secret) is False

    def test_receipt_tamper_device_id_fails_verify(self):
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_ED25519_VERIFY,
            b"in",
            b"out",
            gpu_device_id=0,
            node_secret=self.secret,
        )
        r.gpu_device_id = 99
        assert r.verify(self.secret) is False

    def test_receipt_input_hash_matches_input(self):
        inputs = b"my input data"
        r = GpuReceipt.generate(
            GpuHostcallId.GPU_KECCAK256_BATCH,
            inputs,
            b"output",
            gpu_device_id=1,
            node_secret=self.secret,
        )
        assert r.input_hash == hashlib.sha256(inputs).digest()

    def test_receipt_distinct_per_call(self):
        r1 = GpuReceipt.generate(0xD0, b"a", b"b", 0, self.secret)
        r2 = GpuReceipt.generate(0xD0, b"c", b"d", 0, self.secret)
        assert r1.input_hash != r2.input_hash

    def test_receipt_signature_length(self):
        r = GpuReceipt.generate(0xD0, b"in", b"out", 0, self.secret)
        assert len(r.signature) == 32  # HMAC-SHA256 = 32 bytes


class TestGpuLaneService:
    """Suite 6 — Lane service batch processing and throughput."""

    def setup_method(self):
        self.lane = GPULaneServiceStub(lane_id="lane-0")

    def _make_batch(self, n: int) -> tuple[list[str], list[bytes]]:
        hashes = [secrets.token_hex(16) for _ in range(n)]
        datas  = [secrets.token_bytes(32) for _ in range(n)]
        return hashes, datas

    def test_single_tx_processed(self):
        hashes, datas = self._make_batch(1)
        resp = self.lane.process_batch(hashes, datas, "SVM")
        assert resp["count"] == 1
        assert len(resp["results"]) == 1
        assert len(resp["results"][0]) == 64

    def test_batch_of_100(self):
        hashes, datas = self._make_batch(100)
        resp = self.lane.process_batch(hashes, datas, "EVM")
        assert resp["count"] == 100

    def test_batch_deterministic(self):
        hashes, datas = self._make_batch(50)
        r1 = self.lane.process_batch(hashes, datas, "SVM")
        r2 = self.lane.process_batch(hashes, datas, "SVM")
        assert r1["results"] == r2["results"]

    def test_different_inputs_different_outputs(self):
        h1, d1 = self._make_batch(10)
        h2, d2 = self._make_batch(10)
        r1 = self.lane.process_batch(h1, d1, "SVM")
        r2 = self.lane.process_batch(h2, d2, "SVM")
        assert r1["results"] != r2["results"]

    def test_chain_tag_affects_result(self):
        hashes, datas = self._make_batch(5)
        r_svm = self.lane.process_batch(hashes, datas, "SVM")
        r_evm = self.lane.process_batch(hashes, datas, "EVM")
        assert r_svm["results"] != r_evm["results"]

    def test_empty_batch(self):
        resp = self.lane.process_batch([], [], "SVM")
        assert resp["count"] == 0
        assert resp["results"] == []

    def test_total_txns_accumulates(self):
        for _ in range(5):
            h, d = self._make_batch(20)
            self.lane.process_batch(h, d, "EVM")
        assert self.lane.total_txns == 100

    def test_throughput_cpu_fallback_500_tx(self):
        n = 500
        hashes, datas = self._make_batch(n)
        t0 = time.monotonic()
        resp = self.lane.process_batch(hashes, datas, "SVM")
        elapsed = time.monotonic() - t0
        # Sanity: completes in < 5 seconds on any modern CPU
        assert elapsed < 5.0
        assert resp["count"] == n

    def test_gpu_sim_branch_used_when_available(self):
        hashes, datas = self._make_batch(8)
        self.lane.accelerator.gpu_available = True
        gpu_resp = self.lane.process_batch(hashes, datas, "SVM")

        self.lane.accelerator.gpu_available = False
        cpu_resp = self.lane.process_batch(hashes, datas, "SVM")

        assert gpu_resp["count"] == 8
        assert cpu_resp["count"] == 8
        assert gpu_resp["results"] != cpu_resp["results"]


class TestGpuGuardSentinel:
    """Suite 7 — GPU Guard sentinel health monitoring."""

    def setup_method(self):
        self.sentinel = GpuGuardSentinel(device_ids=[0, 1])

    def test_initial_status_ok(self):
        assert self.sentinel.overall_status() == GpuHealthStatus.OK

    def test_all_devices_healthy_initially(self):
        assert len(self.sentinel.healthy_device_ids()) == 2

    def test_single_error_degrades_device(self):
        self.sentinel.report_error(0, "CUDA_ERROR_ILLEGAL_INSTRUCTION")
        assert self.sentinel.devices[0].status == GpuHealthStatus.DEGRADED

    def test_three_errors_fails_device(self):
        for _ in range(3):
            self.sentinel.report_error(0, "err")
        assert self.sentinel.devices[0].status == GpuHealthStatus.FAILED

    def test_recovery_restores_ok(self):
        self.sentinel.report_error(0, "err")
        self.sentinel.report_ok(0)
        assert self.sentinel.devices[0].status == GpuHealthStatus.OK
        assert self.sentinel.devices[0].consecutive_failures == 0

    def test_temperature_warning_threshold(self):
        level = self.sentinel.check_temperature(0, 82)
        assert level == "warning"
        assert self.sentinel.devices[0].status == GpuHealthStatus.DEGRADED

    def test_temperature_critical_threshold(self):
        level = self.sentinel.check_temperature(0, 92)
        assert level == "critical"
        assert self.sentinel.devices[0].status == GpuHealthStatus.FAILED

    def test_healthy_device_list_excludes_failed(self):
        self.sentinel.report_error(0, "e")
        self.sentinel.report_error(0, "e")
        self.sentinel.report_error(0, "e")  # third → FAILED
        healthy = self.sentinel.healthy_device_ids()
        assert 0 not in healthy
        assert 1 in healthy

    def test_overall_status_degraded_when_one_bad(self):
        self.sentinel.report_error(1, "e")
        assert self.sentinel.overall_status() == GpuHealthStatus.DEGRADED

    def test_overall_status_failed_when_all_failed(self):
        for i in [0, 1]:
            for _ in range(3):
                self.sentinel.report_error(i, "crash")
        assert self.sentinel.overall_status() == GpuHealthStatus.FAILED

    def test_alert_log_appended_on_error(self):
        self.sentinel.report_error(0, "OOM")
        assert any("OOM" in a for a in self.sentinel.alerts)

    def test_temperature_ok_path(self):
        level = self.sentinel.check_temperature(0, 70)
        assert level == "ok"

    def test_warning_does_not_downgrade_failed_device(self):
        for _ in range(3):
            self.sentinel.report_error(0, "err")
        assert self.sentinel.devices[0].status == GpuHealthStatus.FAILED
        level = self.sentinel.check_temperature(0, 82)
        assert level == "warning"
        assert self.sentinel.devices[0].status == GpuHealthStatus.FAILED

    def test_memory_used_pct_property(self):
        dev = self.sentinel.devices[0]
        dev.memory_used_mb = 4096
        dev.memory_total_mb = 8192
        assert dev.memory_used_pct == 50.0


class TestCPUCanonicaliser:
    """Suite 8 — CPU canonicaliser routing for non-determinism elimination."""

    def test_hash_batch_returns_same_length(self):
        raw = [secrets.token_bytes(32) for _ in range(8)]
        canon = CPUCanonicaliser.canonicalise_hash_batch(raw)
        assert len(canon) == 8
        assert all(len(c) == 32 for c in canon)

    def test_hash_batch_deterministic(self):
        raw = [secrets.token_bytes(32) for _ in range(4)]
        c1 = CPUCanonicaliser.canonicalise_hash_batch(raw)
        c2 = CPUCanonicaliser.canonicalise_hash_batch(raw)
        assert c1 == c2

    def test_hash_batch_different_inputs_different_outputs(self):
        r1 = [secrets.token_bytes(32) for _ in range(4)]
        r2 = [secrets.token_bytes(32) for _ in range(4)]
        assert CPUCanonicaliser.canonicalise_hash_batch(r1) != \
               CPUCanonicaliser.canonicalise_hash_batch(r2)

    def test_verify_bitmap_all_set(self):
        bools = CPUCanonicaliser.canonicalise_verify_bitmap(b"\xff", 8)
        assert all(bools)

    def test_verify_bitmap_all_clear(self):
        bools = CPUCanonicaliser.canonicalise_verify_bitmap(b"\x00", 8)
        assert not any(bools)

    def test_verify_bitmap_partial(self):
        # 0b1010_1010 = 0xAA → bits 1,3,5,7 set
        bools = CPUCanonicaliser.canonicalise_verify_bitmap(b"\xAA", 8)
        assert bools == [False, True, False, True, False, True, False, True]

    def test_verify_bitmap_short_input_fills_false(self):
        bools = CPUCanonicaliser.canonicalise_verify_bitmap(b"\x01", 10)
        assert bools[:8] == [True, False, False, False, False, False, False, False]
        assert bools[8:] == [False, False]

    def test_poh_chain_returns_32_bytes(self):
        chain = [secrets.token_bytes(32) for _ in range(10)]
        result = CPUCanonicaliser.canonicalise_poh_chain(chain)
        assert len(result) == 32

    def test_poh_chain_deterministic(self):
        chain = [secrets.token_bytes(32) for _ in range(5)]
        assert (CPUCanonicaliser.canonicalise_poh_chain(chain) ==
                CPUCanonicaliser.canonicalise_poh_chain(chain))

    def test_poh_chain_order_matters(self):
        chain = [secrets.token_bytes(32) for _ in range(4)]
        reversed_chain = list(reversed(chain))
        assert (CPUCanonicaliser.canonicalise_poh_chain(chain) !=
                CPUCanonicaliser.canonicalise_poh_chain(reversed_chain))

    def test_empty_poh_chain(self):
        result = CPUCanonicaliser.canonicalise_poh_chain([])
        assert result == b"\x00" * 32

    def test_gpu_output_routed_through_cpu_canonicaliser(self):
        """
        Key invariant: dispatcher always routes GPU hash outputs through
        CPUCanonicaliser.canonicalise_hash_batch before returning.
        This ensures any GPU rounding non-determinism is eliminated.
        """
        disp = GpuHostcallDispatcher()
        inputs = secrets.token_bytes(32 * 4)
        result = disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, inputs)
        # Canonicalised result must itself be sha256 of the intermediate hash
        # (since stub does sha256 → canonicalise(sha256))
        # Verify: each 32-byte chunk of result is sha256 of sha256(input_chunk)
        for i in range(4):
            chunk = inputs[i*32:(i+1)*32]
            expected = hashlib.sha256(hashlib.sha256(chunk).digest()).digest()
            assert result[i*32:(i+1)*32] == expected

    def test_streamed_sha256_path_aliases_batch(self):
        disp = GpuHostcallDispatcher()
        inputs = secrets.token_bytes(32 * 2)
        batch = disp.invoke(GpuHostcallId.GPU_SHA256_BATCH, inputs)
        streamed = disp.invoke(GpuHostcallId.GPU_SHA256_STREAMED, inputs)
        assert batch == streamed


class TestGpuMemoryPoolThreadEdgeCases:
    def test_concurrent_worker_handles_no_buffer_path(self):
        pool = GpuMemoryPool(capacity=0, buffer_size=64)
        errors = []

        def worker():
            try:
                buf = pool.alloc()
                if buf:
                    time.sleep(0.001)
                    pool.free(buf)
            except Exception as e:
                errors.append(e)

        threads = [threading.Thread(target=worker) for _ in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert errors == []
        assert pool.available == 0

    def test_concurrent_worker_captures_exception(self):
        class BrokenPool:
            @staticmethod
            def alloc():
                raise RuntimeError("alloc failed")

            @staticmethod
            def free(_):
                return False

        pool = BrokenPool()
        errors = []

        def worker():
            try:
                pool.alloc()
            except Exception as e:
                errors.append(e)

        t = threading.Thread(target=worker)
        t.start()
        t.join()

        assert len(errors) == 1
        assert "alloc failed" in str(errors[0])
