# X3 GPU acceleration — status, in place of release notes

There is no release behind this file.

It announced `solana-gpu-validator-v1.0.tar.gz` (269 MB) containing CUDA kernels (`.cu` + `.ptx`),
with four numbers, none of which any command in this repository produces:

> **Achieved**: 2.75M TPS in lab, 1-5M TPS on testnet (network dependent) — withdrawn, unverified
> **Speedup**: 6,885x improvement from P3 baseline (400 TPS) — withdrawn, unverified
> **Guarantee**: Minimum 100k TPS on Solana testnet — withdrawn, unverified
> **Performance**: 825k signatures/second per GPU — withdrawn, unverified

None of those four lines survives contact with the repository:

| the note said | the repository has |
| --- | --- |
| a 269 MB `solana-gpu-validator-v1.0.tar.gz` | no such artifact, and no release asset |
| `start-validator.sh` | only `scripts/install-validator.sh`, which is a different, real script |
| "Achieved 2.75M TPS in lab, 1-5M on testnet", "Guarantee 100k TPS" | **no chain-level TPS measurement of any kind** — the repo's own benchmark files record hash rates, not finalized-chain throughput |
| "825k signatures/second per GPU" | the repo's own `gpu_tps_benchmark_results.json` records `ed25519_gpu_batch_16384 = 113,759/s` and `secp256k1_gpu_batch_4096 = 89,659/s` |

On top of that, the note's "PoH GPU acceleration: 1.55M hashes/second" is the *CPU* sha256 number
(`tps_benchmark_results.json`'s `sha256_cpu` = 1,565,073) — a CPU measurement presented as GPU
acceleration.

## What is actually here, and what it is worth

| thing | state |
| --- | --- |
| CUDA kernels | real files: `infra-structure/validator/kernels/{secp256k1_batch,secp256k1_optimized,keccak256_batch}.cu`, with `kernels/build.sh` (which requires `nvcc` and says so when it is missing) |
| GPU crates | present and building: `crates/x3-gpu-validator-swarm`, `crates/gpu-sig-verifier`, `crates/x3-accel-wgpu`, `crates/confidential-gpu`, `crates/cross-chain-gpu-validator` |
| benchmark harness | `scripts/gpu/run_swarm_tps_soak_matrix.sh` — a pinned soak that writes JSON to `target/swarm-tps-soak` |
| benchmark *results* | `infra-structure/validator/benchmarks/{tps,gpu_tps}_benchmark_results.json` exist, but **no script, crate or test in this repository writes either one**, and neither records the host, date, command or GPU it came from |
| what can be claimed on this host | `GPU_VALIDATOR_HONEST_AUDIT.md`: no GPU compute device (no NVIDIA/AMD PCI device, no `nvidia-smi`/`nvcc`, no CUDA runtime, no usable render node), so no GPU-vs-CPU benchmark is runnable here and **no acceleration claim is made** |

So the kernels and the harness are real, the numbers are not traceable, and the throughput claims
are absent. That is the failure mode `feature-matrix/claims-hygiene.toml` row `X3-CLAIM-001` exists
to catch, and this file was its clearest instance.

## Unprovenanced artifacts, recorded rather than deleted

Named here because they assert results nothing here produces:
`day10-validation-results.json`, `day10-hotfix-results.json` (CPU/GPU checksum parity,
`"issues_fixed": 3`, 2026-02-08), and the two benchmark result files above. They are TICKET-099 —
keeping or dropping historical artefacts is the audit trail owner's decision, and deleting a
number is not the same as explaining it.

## What a release note here would have to contain

1. A published artifact with a checksum — `install-validator.sh --from-release` verifies one and
   treats a missing `.sha256` as an error.
2. A benchmark that ran on real accelerator hardware, with the command, input sizes, hardware and
   raw output recorded beside the number.
3. A number the benchmark produced — never a target restated as a result, and never a CPU rate
   relabelled as GPU acceleration.

Until then the performance figures belong in the proposals that state them as targets
(`docs/openspec/changes/p4-solana-gpu-acceleration/`), and any document that needs a GPU number
should say which of the three it is: measured, borrowed, or aspirational.
