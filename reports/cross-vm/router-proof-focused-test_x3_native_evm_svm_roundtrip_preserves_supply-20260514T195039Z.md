# X3 Cross-VM Router Proof

## Verdict

- Result: PASS
- Mode: focused
- Test: test_x3_native_evm_svm_roundtrip_preserves_supply
- Docker image: rust:1.90-slim-bookworm
- Exit code: 0
- Passed count: 1
- Failed count: 0

## Proof Surface

This proof runs the `pallet-x3-cross-vm-router` Substrate pallet test harness. The focused default test executes the router and supply-ledger path for X3Native -> X3Evm, X3Evm -> X3Svm, and X3Svm -> X3Native, then verifies canonical supply conservation, pending supply drain, and VM-adapter origin enforcement.

## Command

```bash
cargo test -p pallet-x3-cross-vm-router --lib test_x3_native_evm_svm_roundtrip_preserves_supply -- --nocapture
```

## Evidence

- Log: /home/lojak/Desktop/X3_ATOMIC_STAR/reports/cross-vm/router-proof-focused-test_x3_native_evm_svm_roundtrip_preserves_supply-20260514T195039Z.log
- JSON: /home/lojak/Desktop/X3_ATOMIC_STAR/reports/cross-vm/router-proof-focused-test_x3_native_evm_svm_roundtrip_preserves_supply-20260514T195039Z.json

## Known Constraint

The local host Rust toolchains have shown rustc SIGSEGV/ICE failures while compiling third-party dependencies before X3 code. This script intentionally uses a clean Docker Rust environment so the proof is not dependent on the broken host compiler state.
