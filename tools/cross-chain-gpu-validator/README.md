# Cross-Chain GPU Validator

Cross-chain GPU validator stack for Solana, EVM, Cosmos, and Substrate-style networks.

## What is here

- Multi-chain validator registry with generated chain metadata
- GPU-first secp256k1, ed25519, and keccak batch paths with CPU failover
- Atomic swap orchestration backed by Redis
- Operator dashboard and benchmark report viewer
- Resilience primitives for failover, toll-booth access control, and signer locking

## Quick start

1. Create a virtual environment:
   `python3 -m venv .venv && source .venv/bin/activate`
2. Install the package:
   `python3 -m pip install -e .`
3. Run the validator dashboard:
   `python3 -m cross_chain_gpu_validator.cli dashboard`
4. Run the local test suite:
   `python3 -m pytest -q`

## Key environment variables

- `CCGV_REQUIRE_GPU=true|false`
- `CCGV_GPU_PARITY_CHECK=true|false`
- `CCGV_KERNEL_DIR=/abs/path/to/kernels`
- `CCGV_REDIS_URL=redis://127.0.0.1:6379/0`
- `CCGV_SVM_RPC=http://127.0.0.1:8899`
- `CCGV_EVM_RPC=http://127.0.0.1:8545`

## Readiness notes

The codebase includes production-oriented failover and operator controls, but live go-live items such as real GPU node provisioning, real endpoint secrets, and external failover drills still need environment-specific validation before production launch.
