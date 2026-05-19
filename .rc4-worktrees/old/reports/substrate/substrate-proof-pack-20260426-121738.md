# X3 Substrate Proof Pack

- Generated: 2026-04-26T18:17:38Z
- Repository: /home/lojak/Desktop/X3_ATOMIC_STAR
- Commit: 446567e06f6ae75905659f08910e992b95c1f067
- Heavy checks: 0

This report is evidence, not an external audit certificate. PASS means the listed command passed on this machine. SKIP means the prerequisite/tooling was not present or heavy mode was disabled.

| Gate | Status | Evidence |
| --- | --- | --- |
| substrate-toolchain-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/substrate-toolchain-inventory-20260426-121738.log) / sha256 a0add8f442da556e8736dc83961839579c8748ca74e115d6feb156cac29e0408 |
| try-runtime-command-wired | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/try-runtime-command-wired-20260426-121738.log) / sha256 d348bbb5b141be902e35f51c2255cb133e8f9e1157f76f5cace671d9b27452a5 |
| try-runtime-feature-compiles | SKIP | Set RUN_HEAVY_SUBSTRATE_PROOFS=1 to run cargo check -p x3-chain-node --features try-runtime. |
| node-benchmark-cli-present | SKIP | No built x3-chain-node binary found under target/release or target/debug. |
| placeholder-or-manual-weight-scan | WARN | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/placeholder-or-manual-weight-scan-20260426-121738.log) / sha256 add497478fecd26d2469c77ea488611660dba0718f0b8fc663fa6e2764299589 |
| srtool-installed | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/srtool-installed-20260426-121738.log) / sha256 e236e8531f34e0c7d5f5b9343715fbd46696fc31a116d7b8a4c0ffb73440eff5 |
| subwasm-installed | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/subwasm-installed-20260426-121738.log) / sha256 0b14f6bf61a061135a260875bf12f2fafa4bd6761664331f8db34bd4d1a1125c |
| zombienet-installed | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/zombienet-installed-20260426-121738.log) / sha256 e025a950a827bf4ad4f28490dd122c59829a58c3a9d919eb32a677165f0c89c4 |
| chopsticks-installed | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/chopsticks-installed-20260426-121738.log) / sha256 0f15fd66e6addf2fd29d9f89e7aec7fe7674ab01e919a17779490088667f605e |
| client-compatibility-source-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/client-compatibility-source-inventory-20260426-121738.log) / sha256 a59fb49ea30028921144dec57f4349958e943b326e6b7e0672e9d78e0dd87edf |
| chain-spec-source-present | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/chain-spec-source-present-20260426-121738.log) / sha256 0ddf86943191751cee6c164d4da97b1c7270d632a88c8bac3595da7b9deac93f |

## Summary

- PASS: 8
- WARN: 1
- FAIL: 0
- SKIP: 2

## Certificate-Like Labels Allowed After Evidence

- Substrate Runtime Upgrade Check: only claim PASS after try-runtime on-runtime-upgrade runs against a live/snapshot state.
- FRAME Weights: only claim generated/committed after benchmark output replaces manual placeholder weights.
- Runtime Wasm Reproducibility: only claim after srtool hash/proposal hash is published.
- Runtime Metadata Diff: only claim after subwasm diff is published.
- Local Network Smoke: only claim after Zombienet topology/test logs are published.
- Fork/Replay Suite: only claim after Chopsticks replay logs are published.

## Exit Policy

Result: PARTIAL. Do not present this as complete Substrate security proof.
