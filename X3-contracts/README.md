# X3-contracts

Dual-stack contracts workspace for X3.

```
X3-contracts/
├── evm/        # Foundry workspace (Solidity)
├── svm/        # Anchor workspace (Solana programs)
├── shared/     # Cross-stack specs, schemas, ABIs, parity test vectors
└── proof/      # Parity receipts and reports (consumed by ProofForge)
```

## Why this layout

Two execution stacks, one truth.

- `evm/` and `svm/` implement the same primitives (flashloan repay-or-revert,
  governance-gated upgrade, smart-account onboarding) using their native
  toolchains.
- `shared/test-vectors/*.json` are the **single source of truth** for behavior.
  Every parity-critical claim must be expressed as a vector here.
- `shared/specs/*.md` describe the invariants. If a spec disagrees with a
  vector, the vector wins (vectors are executable).
- `proof/receipts/` holds machine-generated receipts produced by ProofForge's
  parity harness — never edit by hand.

## Quick commands

```bash
# EVM
(cd evm && forge build && forge test -vvv)

# SVM
(cd svm && anchor build && anchor test)

# Parity (run from repo root, after building proof-forge)
./target/release/x3-proof verify x3.contracts.evm_svm_parity --strict
```

## Status

This is a launch-critical workspace. Empty subtrees are intentional placeholders
when a parity slot has been declared but not yet implemented. A non-empty slot
without a corresponding receipt under `proof/receipts/` is a release blocker.
