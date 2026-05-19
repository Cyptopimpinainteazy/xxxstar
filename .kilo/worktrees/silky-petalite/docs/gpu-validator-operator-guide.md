# X3 Chain — Multi-Chain X3 GPU Validator: Operator Guide

## Overview

The X3 Chain multi-chain validator uses GPU-accelerated cryptographic verification across four chain families (EVM, SVM, Cosmos, Substrate) on commodity NVIDIA hardware. This guide covers deployment, configuration, monitoring, and troubleshooting.

## Hardware Requirements

| Component | Minimum | Recommended (Pilot) |
|-----------|---------|---------------------|
| CPU | 8-core x86_64 | AMD Threadripper (32+ cores) |
| RAM | 32 GB | 64 GB+ |
| GPU | 1× GTX 1070 (8 GB) | 3× GTX 1070+ |
| Storage | 500 GB SSD | 1 TB NVMe |
| Network | 1 Gbps | 10 Gbps |

### GPU Support Matrix

| GPU | VRAM | CUDA Arch | Supported |
|-----|------|-----------|-----------|
| GTX 1070 | 8 GB | sm_61 | ✅ Primary target |
| GTX 1080 Ti | 11 GB | sm_61 | ✅ |
| RTX 2080 | 8 GB | sm_75 | ✅ (recompile with `--arch sm_75`) |
| RTX 3090 | 24 GB | sm_86 | ✅ (recompile with `--arch sm_86`) |
| A100 | 40/80 GB | sm_80 | ✅ (recompile with `--arch sm_80`) |

## Quick Start

### 1. Build CUDA Kernels

```bash
cd cross-chain-gpu-validator/kernels
bash build.sh
```

Produces 5 kernel libraries in `build/`:
- `libsecp256k1_batch.so` — Jacobian + Shamir's trick ECDSA verify (EVM/Cosmos)
- `libkeccak256_batch.so` — Keccak-256 hash batch (EVM)
- `libsha256_batch.so` — SHA-256 hash batch (SVM/Cosmos/Substrate)
- `libed25519_batch.so` — Ed25519 signature verify (SVM/Substrate)
- `libstream_pipeline.so` — PoH verification pipeline (SVM)

### 2. Configure Node

Edit `deployment/pilot/threadripper.toml`:
- Set GPU device indices and chain family assignments
- Configure VRAM headroom (default 512 MB per GPU)
- Set chain list and RPC endpoint

### 3. Deploy

```bash
# Local deployment
cd deployment/pilot
bash deploy.sh

# Docker Compose (with NVIDIA runtime)
docker compose -f docker-compose.pilot.yml up -d
```

### 4. Verify

```bash
# Check GPU detection
nvidia-smi

# Run benchmark
python3 tests/p4_benchmarks/crypto_bench.py

# Check dashboard
open http://localhost:8080
```

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  X3 VM (Rust)                                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ 0xD0-D3  │  │ 0xD4-D5  │  │  0xD6    │  │  0xD7    │        │
│  │ SHA/Ed/  │  │ DevCount │  │ Keccak   │  │ secp256k1│        │
│  │ PoH/Str  │  │ Bench    │  │ Batch    │  │ Verify   │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
│       │              │              │              │              │
│  ┌────▼──────────────▼──────────────▼──────────────▼─────┐      │
│  │              gpu_hostcalls.rs (FFI bridge)             │      │
│  └────┬──────────────┬──────────────┬──────────────┬─────┘      │
└───────┼──────────────┼──────────────┼──────────────┼─────────────┘
        │              │              │              │
   ┌────▼────┐   ┌────▼────┐   ┌────▼────┐   ┌────▼────┐
   │ SHA-256 │   │ Ed25519 │   │Keccak256│   │secp256k1│
   │  .so    │   │  .so    │   │  .so    │   │  .so    │
   └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘
        │              │              │              │
   ┌────▼──────────────▼──────────────▼──────────────▼─────┐
   │              CUDA Runtime (multi-GPU)                  │
   │         GPU 0          GPU 1          GPU 2            │
   └────────────────────────────────────────────────────────┘
```

## Chain Family → Kernel Mapping

| Family | Signature | Hash | Extra | Gas Cost |
|--------|-----------|------|-------|----------|
| **EVM** | secp256k1 (0xD7, 600 gas) | Keccak-256 (0xD6, 500 gas) | — | 1,100 |
| **SVM** | Ed25519 (0xD1, 500 gas) | SHA-256 (0xD0, 500 gas) | PoH (0xD2, 1000 gas) | 2,000 |
| **Cosmos** | secp256k1 (0xD7, 600 gas) | SHA-256 (0xD0, 500 gas) | — | 1,100 |
| **Substrate** | Ed25519 (0xD1, 500 gas) | SHA-256 (0xD0, 500 gas) | — | 1,000 |

## GPU Scheduler

The multi-GPU scheduler (`multi_gpu_scheduler.py`) dynamically assigns chains to GPUs:

1. **Priority-first**: High-priority chains get dedicated GPUs
2. **VRAM-aware**: Never exceeds GPU VRAM minus headroom
3. **Family grouping**: Co-locates chains using same kernels
4. **Swarm fallback**: Idle GPUs are offered to swarm compute tasks

Configuration in `threadripper.toml`:
```toml
[scheduler]
rebalance_interval_sec = 30
vram_headroom_mb = 512
idle_threshold_sec = 5
preempt_latency_ms = 50
```

## Monitoring

### Dashboard (port 8080)

Live metrics including:
- Per-chain TPS across all 4 families
- GPU utilization bars with VRAM tracking
- Crypto ops/sec per kernel type
- Gas savings (GPU vs CPU)
- Atomic swap success rate and rollbacks
- Swarm compute task status

### Prometheus (port 9090)

Scrape targets configured in `deployment/pilot/prometheus.yml`. Grafana available on port 3000 (default password: `x3-pilot`).

### JSON Metrics

Real-time metrics at `http://localhost:8080/metrics.json`.

## Benchmark Results (GTX 1070)

| Primitive | CPU | GPU | Speedup |
|-----------|-----|-----|---------|
| SHA-256 | 1.71M/s | 10.1M/s | 5.9× |
| Ed25519 | 14.4K/s | 79.8K/s | 5.5× |
| PoH | 1.71M/s | 532.8M/s | 312× |
| Keccak-256 | 1.06M/s | 50.0M/s | 47× |
| secp256k1 | 2.5K/s | 115.6K/s | 45.6× |

## Troubleshooting

### Kernel libraries not found
```
Error: GPU secp256k1 library not loaded
```
Ensure `LD_LIBRARY_PATH` includes `cross-chain-gpu-validator/kernels/build/`.

### CUDA out of memory
Reduce `max_batch` in config or increase `vram_headroom_mb`.

### Low secp256k1 throughput
The optimized kernel uses Jacobian projective coordinates. If throughput < 50K/s:
1. Check `nvidia-smi` for thermal throttling
2. Verify the optimized `.so` is loaded (not old naive kernel)
3. Increase batch size (default 8192)

### Docker GPU access
Ensure NVIDIA Container Toolkit is installed:
```bash
sudo apt install nvidia-container-toolkit
sudo systemctl restart docker
```
