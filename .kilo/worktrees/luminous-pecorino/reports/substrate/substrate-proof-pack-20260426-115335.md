# X3 Substrate Proof Pack

- Generated: 2026-04-26T17:53:35Z
- Repository: /home/lojak/Desktop/X3_ATOMIC_STAR
- Commit: b3c54f471bae3d4c575b9849ca3919aa8ffa7935
- Heavy checks: 0

This report is evidence, not an external audit certificate. PASS means the listed command passed on this machine. SKIP means the prerequisite/tooling was not present or heavy mode was disabled.

| Gate | Status | Evidence |
| --- | --- | --- |
| substrate-toolchain-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/substrate-toolchain-inventory-20260426-115335.log) / sha256 a0add8f442da556e8736dc83961839579c8748ca74e115d6feb156cac29e0408 |
| try-runtime-command-wired | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/try-runtime-command-wired-20260426-115335.log) / sha256 d348bbb5b141be902e35f51c2255cb133e8f9e1157f76f5cace671d9b27452a5 |
| try-runtime-feature-compiles | SKIP | Set RUN_HEAVY_SUBSTRATE_PROOFS=1 to run cargo check -p x3-chain-node --features try-runtime. |
| node-benchmark-cli-present | SKIP | No built x3-chain-node binary found under target/release or target/debug. |
| placeholder-or-manual-weight-scan | WARN | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/placeholder-or-manual-weight-scan-20260426-115335.log) / sha256 2818670cadf35728beedc0bed1be1fd2f911c53d2183567b6d09a0129951753a |
| srtool-installed | SKIP | srtool not found in PATH. Install before claiming reproducible runtime Wasm proof. |
| subwasm-installed | SKIP | subwasm not found in PATH. Install before publishing runtime metadata diff proof. |
| zombienet-installed | SKIP | zombienet not found in PATH. Install before publishing local validator network proof. |
| chopsticks-installed | SKIP | chopsticks not found in PATH. Install before publishing fork/replay proof. |
| client-compatibility-source-inventory | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/client-compatibility-source-inventory-20260426-115335.log) / sha256 06bc03ae85bbca6126f21577a83ac3dd1063cd03516099fe2420b720f5a0a3bd |
| chain-spec-source-present | PASS | [log](/home/lojak/Desktop/X3_ATOMIC_STAR/launch-gates/evidence/substrate/chain-spec-source-present-20260426-115335.log) / sha256 0ddf86943191751cee6c164d4da97b1c7270d632a88c8bac3595da7b9deac93f |

## Summary

- PASS: 4
- WARN: 1
- FAIL: 0
- SKIP: 6

## Certificate-Like Labels Allowed After Evidence

- Substrate Runtime Upgrade Check: only claim PASS after try-runtime on-runtime-upgrade runs against a live/snapshot state.
- FRAME Weights: only claim generated/committed after benchmark output replaces manual placeholder weights.
- Runtime Wasm Reproducibility: only claim after srtool hash/proposal hash is published.
- Runtime Metadata Diff: only claim after subwasm diff is published.
- Local Network Smoke: only claim after Zombienet topology/test logs are published.
- Fork/Replay Suite: only claim after Chopsticks replay logs are published.

## Exit Policy

Result: PARTIAL. Do not present this as complete Substrate security proof.
