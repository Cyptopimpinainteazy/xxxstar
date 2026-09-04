# X3 Substrate Proof Pack

- Generated: 2026-04-26T18:16:48Z
- Repository: /home/lojak/Desktop/X3_ATOMIC_STAR
- Commit: 446567e06f6ae75905659f08910e992b95c1f067
- Heavy checks: 0

This report is evidence, not an external audit certificate. PASS means the listed command passed on this machine. SKIP means the prerequisite/tooling was not present or heavy mode was disabled.

| Gate | Status | Evidence |
| --- | --- | --- |
| substrate-toolchain-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/substrate-toolchain-inventory-20260426-121648.log) / sha256 a0add8f442da556e8736dc83961839579c8748ca74e115d6feb156cac29e0408 |
| try-runtime-command-wired | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/try-runtime-command-wired-20260426-121648.log) / sha256 d348bbb5b141be902e35f51c2255cb133e8f9e1157f76f5cace671d9b27452a5 |
| try-runtime-feature-compiles | SKIP | Set RUN_HEAVY_SUBSTRATE_PROOFS=1 to run cargo check -p x3-chain-node --features try-runtime. |
| node-benchmark-cli-present | SKIP | No built x3-chain-node binary found under target/release or target/debug. |
| placeholder-or-manual-weight-scan | WARN | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/placeholder-or-manual-weight-scan-20260426-121648.log) / sha256 f78961f8db5c8e79251f60be6722f9391d698bc16de0343bb135287ab6a97871 |
| srtool-installed | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/srtool-installed-20260426-121648.log) / sha256 e236e8531f34e0c7d5f5b9343715fbd46696fc31a116d7b8a4c0ffb73440eff5 |
| subwasm-installed | FAIL | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/subwasm-installed-20260426-121648.log) / sha256 1a42be0c297c4822ef9dcc09375c12b0ca50dfae05e9001916ca1c40e526f6f6 |
| zombienet-installed | SKIP | zombienet not found in PATH. Install before publishing local validator network proof. |
| chopsticks-installed | SKIP | chopsticks not found in PATH. Install before publishing fork/replay proof. |
| client-compatibility-source-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/client-compatibility-source-inventory-20260426-121648.log) / sha256 695366348208e2c20a0367334549d2c706a56343df4f59ad28ce0796991be3af |
| chain-spec-source-present | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/chain-spec-source-present-20260426-121648.log) / sha256 0ddf86943191751cee6c164d4da97b1c7270d632a88c8bac3595da7b9deac93f |

## Summary

- PASS: 5
- WARN: 1
- FAIL: 1
- SKIP: 4

## Certificate-Like Labels Allowed After Evidence

- Substrate Runtime Upgrade Check: only claim PASS after try-runtime on-runtime-upgrade runs against a live/snapshot state.
- FRAME Weights: only claim generated/committed after benchmark output replaces manual placeholder weights.
- Runtime Wasm Reproducibility: only claim after srtool hash/proposal hash is published.
- Runtime Metadata Diff: only claim after subwasm diff is published.
- Local Network Smoke: only claim after Zombienet topology/test logs are published.
- Fork/Replay Suite: only claim after Chopsticks replay logs are published.

## Exit Policy

Result: FAIL. At least one required quick proof failed.
