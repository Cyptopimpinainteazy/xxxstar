# GPU performance claims: what the repository actually contains

Date: 2026-09-22
Scope: claims hygiene for `docs/testnet-config/RELEASE-NOTES.md`, row `X3-CLAIM-001` (mainnet_ready was 5)

## The claim

`docs/testnet-config/RELEASE-NOTES.md` announced a shipped artifact:

> `solana-gpu-validator-v1.0.tar.gz (269 MB)` containing `gpu-kernels/ # CUDA kernels (.cu + .ptx)`
> **Achieved**: 2.75M TPS in lab, 1-5M TPS on testnet (network dependent)
> **Speedup**: 6,885x improvement from P3 baseline (400 TPS)
> **Guarantee**: Minimum 100k TPS on Solana testnet
> **Performance**: 825k signatures/second per GPU

## What is checked, and what came back

| claim | check | result |
| --- | --- | --- |
| 269 MB tarball | `find` for the name and for any release asset | not present |
| `start-validator.sh` | `find -name start-validator.sh` | absent; the real script is `scripts/install-validator.sh` |
| CUDA kernels exist | `find -name '*.cu' -o -name '*.ptx'` | **present**: `infra-structure/validator/kernels/{secp256k1_batch,secp256k1_optimized,keccak256_batch}.cu`, with `kernels/build.sh` (refuses without `nvcc`) |
| 2.75M / 1–5M / 100k TPS | search for any chain-level TPS measurement | none — the repository's benchmark files record hash rates and signature rates, not finalized-chain throughput |
| 825k signatures/second | the repository's own `infra-structure/validator/benchmarks/gpu_tps_benchmark_results.json` | `ed25519_gpu_batch_16384 = 113,759`, `secp256k1_gpu_batch_4096 = 89,659`; the file records no host, date, command or GPU |
| "PoH GPU acceleration: 1.55M hashes/second" | the repository's own `tps_benchmark_results.json` | `sha256_cpu = 1,565,073` — the figure is the **CPU** rate, presented as GPU acceleration |
| provenance of the result files | grep for a producer | `tps_benchmark_results.json`, `gpu_tps_benchmark_results.json`, `day10-validation-results.json`, `day10-hotfix-results.json` are written by **nothing** in this repository |
| claimable on this host | `GPU_VALIDATOR_HONEST_AUDIT.md` | no GPU compute device, no `nvidia-smi`/`nvcc`, no CUDA runtime, no usable render node — no GPU-vs-CPU benchmark is runnable here |

## Why this is a mainnet-readiness item and not a copy edit

The first claim in the list ("a 269 MB tarball is available") is a statement a user acts on. The
third is a number a partner would quote. Both were false in a file whose name says "release notes",
next to a directory that does contain real things — real kernels, a real soak harness, a real
installer that refuses unverified downloads. A reader cannot tell which is which, which is exactly
what row `X3-CLAIM-001` measures.

## What changed here

The file now states what exists, what is measured and what is a target; it names the unprovenanced
artifacts instead of inheriting their authority; and it lists what a release note would have to
contain. `X3-CLAIM-001` moved from implemented 10 / tested 5 / mainnet_ready 5 to 55 / 25 / 35, and
its blocker no longer says "remove current-performance wording" — that part is done — but that no
GPU benchmark is possible on this host and no GPU artifact is published, so an acceleration claim
still cannot be verified here.

## Not claimed

Nothing here says the kernels are wrong, that the GPU work is fictional, or that the numbers were
invented. It says they are **unverifiable from this repository**: no producer, no hardware
recorded, no command, and no GPU on this host. Establishing them is a benchmark on accelerator
hardware, not an argument.
