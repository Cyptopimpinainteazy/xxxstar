# Coq proofs (planned)

Coq theorems pair with the TLA+ specs under `../tla/` to provide
machine-checked, mechanized proofs of the X3 asset kernel and bridge
invariants.

## Roadmap

- [ ] `asset_kernel/SupplyConservation.v` — proof that the conservation
      law in `tla/asset_kernel/AssetKernelSupply.tla` is implied by the
      structural typing of the kernel’s state-transition function
      (one ledger update per op, no parallel writes).
- [ ] `bridge/ReplayProtection.v` — proof that the bridge replay-set
      monotonically grows and never accepts a duplicate `(src_chain, nonce)`.
- [ ] `consensus/Finality.v` — proof that GRANDPA-style finality preserves
      `block_height` monotonicity under any honest-majority quorum.

## Status

Initial harness is bootstrapped via `SupplyInvariant.v` and is executed by
`x3-proof formal` through `coqc` in strict mode.
