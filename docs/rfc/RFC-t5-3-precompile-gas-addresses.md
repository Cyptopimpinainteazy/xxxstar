# RFC t5-3: Precompile Gas Accounting & Address Validation

**Status:** DRAFT — requires governance/consensus review before merging  
**Scope:** `runtime/src/precompiles.rs`  
**Risk:** HIGH — runtime changes affect all nodes; requires coordinated upgrade

---

## Problem

Scanner flagged TODO markers in the precompile setup around gas accounting and
address checks. Incorrect gas values cause under/over-charging for EVM
precompile calls; missing address validation can allow unexpected call targets.

## Current State

- Gas costs are hard-coded without reference to benchmarks.
- Address range checks may not reject out-of-range precompile addresses.
- TODO comments indicate this was deferred pending benchmarking.

## Proposed Changes

1. **Gas benchmarks**: Run `frame-benchmarking` on each precompile to derive
   actual weights; replace hard-coded values with `T::WeightToFee` conversions.
2. **Address validation**: Add explicit `if !is_precompile_address(addr)` guard
   at the dispatch entry point returning `PrecompileFailure::Error` for unknown
   addresses rather than falling through to undefined behavior.
3. **Tests**: Add unit tests covering (a) correct gas deduction for each
   precompile, (b) rejection of out-of-range addresses, (c) no regression on
   existing precompile outputs.

## Migration / Governance

- Requires runtime upgrade extrinsic (spec version bump).
- Changes must be reviewed by at least 2 core runtime maintainers.
- Deploy on testnet first; soak for ≥ 1 epoch before mainnet proposal.

## Open Questions

- Which benchmarking hardware profile should set the reference gas cost?
- Should unknown addresses hard-fail or silently no-op?
