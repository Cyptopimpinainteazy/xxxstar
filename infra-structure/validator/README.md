# Cross-Chain GPU Validator

Canonical Python validator stack for X3's cross-chain testnet operations.

## Capabilities

- Multi-chain registry for EVM, Cosmos, SVM, and ED25519-configured validator
  adapters. SR25519 Substrate accounts remain on native Substrate verification.
- GPU-first Secp256k1, Ed25519, and Keccak batch paths.
- CPU failover with parity checks; a GPU result that disagrees with CPU is
  rejected and the accelerator path is disabled.
- Atomic-swap orchestration backed by Redis.
- Failover, toll-booth access control, signer locking, and operator health
  primitives.

## Quick Start

```bash
python3 -m venv .venv
source .venv/bin/activate
python3 -m pip install -e '.[test]'
python3 -m pytest -q
```

Build optional CUDA kernels when `nvcc` and compatible libraries are present:

```bash
bash kernels/build.sh
```

Without CUDA, the verifier uses its CPU reference path and reports no GPU
acceleration. GPU availability never changes acceptance semantics: CPU and GPU
results must agree.

The current CUDA Ed25519 kernel accepts fixed 32-byte messages. Variable-length
SVM transaction messages therefore use the CPU verifier, whose acceptance
semantics are identical.

## Environment

- `CCGV_REQUIRE_GPU=true|false`
- `CCGV_GPU_PARITY_CHECK=true|false`
- `CCGV_KERNEL_DIR=/abs/path/to/kernels`
- `CCGV_REDIS_URL=redis://127.0.0.1:6379/0`
- `CCGV_SVM_RPC=http://127.0.0.1:8899`
- `CCGV_EVM_RPC=http://127.0.0.1:8545`

## Release Boundary

The package is testnet infrastructure. Production promotion still requires
real GPU hosts, live chain endpoints, credentialed Redis deployment, operator
failover drills, and the repository's release gates.
