#!/usr/bin/env bash
# Physical-GPU gate. Runs on the GPU node itself; fails (nonzero) on any
# correctness failure or if no GPU is present. Evidence:
#   audit-artifacts/gpu-production/<commit>/{gpu-info.json,hash-parity.json,summary.json}
#
# What it covers today (wgpu/Vulkan; no CUDA toolkit is required):
#   - driver + GPU enumeration
#   - Keccak-256 / SHA-256 CPU-GPU byte parity on every card, determinism,
#     throughput vs 1 and all CPU threads, multi-GPU split with ordered merge
#   - the crate's GPU tests with X3_REQUIRE_GPU=1 (a missing GPU is a failure),
#     including secp256k1 field/scalar primitives vs a naive reference
#   - secp256k1 ECDSA verdicts vs CpuBackend (libsecp256k1) on ~8k valid,
#     invalid and edge-case jobs, on the default adapter and split across all
#     adapters (MultiDevice); throughput per adapter and for the split
# Not covered: ed25519 GPU verification (no kernel), CUDA (no nvcc).
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"
commit="$(git rev-parse HEAD)"
dirty="$(test -n "$(git status --porcelain)" && echo true || echo false)"
out="audit-artifacts/gpu-production/$commit"
mkdir -p "$out"
results=()
step() {  # step NAME CMD...
    local name="$1"; shift
    echo "== $name"
    if "$@"; then results+=("{\"step\":\"$name\",\"pass\":true}"); else results+=("{\"step\":\"$name\",\"pass\":false}"); fi
}

nvidia-smi --query-gpu=index,name,pci.bus_id,memory.total,compute_cap,driver_version,pcie.link.gen.current,pcie.link.width.current \
    --format=csv > "$out/gpu-info.csv"
step gpu_enumeration test "$(nvidia-smi -L | wc -l)" -ge 1
step crate_gpu_tests env X3_REQUIRE_GPU=1 cargo test -q -p x3-accel-wgpu
step hash_parity_all_gpus cargo run --release -q -p x3-accel-wgpu --example gpu_parity -- --out "$out/hash-parity.json"
step secp256k1_parity_vs_cpu env X3_REQUIRE_GPU=1 X3_REQUIRE_MULTI_GPU=1 cargo test --release -q -p x3-accel --features wgpu --test secp256k1_gpu_parity
step secp256k1_bench_all_gpus cargo run --release -q -p x3-accel --features wgpu --example secp256k1_gpu_bench -- --out "$out/secp256k1-bench.json"

pass=true
for r in "${results[@]}"; do [[ "$r" == *'"pass":false'* ]] && pass=false; done
cat > "$out/summary.json" <<JSON
{"label":"PHYSICAL","host":"$(hostname)","commit":"$commit","dirty":$dirty,"collected":"$(date -u +%FT%TZ)",
 "driver":"$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1)",
 "steps":[$(IFS=,; echo "${results[*]}")],"pass":$pass,
 "not_covered":["ed25519 GPU verification (no kernel)","CUDA backend (no nvcc)","multi-GPU parity needs 2+ adapters (required here via X3_REQUIRE_MULTI_GPU)"]}
JSON
echo "summary: $out/summary.json pass=$pass"
$pass
