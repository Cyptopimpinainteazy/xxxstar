# Cross-Chain GPU Validator

End-to-end validator stack for Solana + Ethereum testnets with GPU acceleration
and atomic swap orchestration.

## Quick Start

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .

ccgv --help
```

## Environment Variables

- `CCGV_REQUIRE_GPU` (default: true) - enforce GPU availability
- `CCGV_GPU_PARITY_CHECK` (default: true) - compare GPU output to CPU reference
- `CCGV_KERNEL_DIR` - path to CUDA kernel directory
- `CCGV_SVM_RPC` / `CCGV_EVM_RPC` - testnet RPC endpoints

## Layout

- `src/` Python service modules
- `kernels/` CUDA kernels and build script
- `dashboard/` static dashboard assets
- `deployment/` deployment scripts and configs
- `docs/` architecture and operational docs

## All-Chains TPS Benchmarks

Generate max TPS benchmarks for all configured chains and feed them to the dashboard:

```bash
cd cross-chain-gpu-validator
./.venv/bin/python scripts/benchmark_all_chains_tps.py \
  --output benchmarks/all_chains_tps.json \
  --levels 25000,100000 \
  --our-chain-id solana
```

The dashboard reads `benchmarks/all_chains_tps.json` from:

- `GET /chain-benchmarks.json` (served by `ccgv dashboard`)

This powers:

- `Our Chain Max TPS`
- `Global Max TPS`
- `Chain TPS Rankings` (top benchmarked chains)

## Crypto Notes

- secp256k1 GPU path uses an MIT-licensed ECC helper in `third_party/`.
- Public keys are expected as 64-byte uncompressed (x||y) values.
