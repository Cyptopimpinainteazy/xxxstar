# RFC t5-5..t5-7: Invariant Governance Controls & Emergency Authority APIs

**Status:** DRAFT — requires governance/consensus review before merging  
**Scope:** `pallets/x3-invariants/src/lib.rs` lines ~337–381  
**Risk:** HIGH — pallet changes affect on-chain governance; must not enable chain-halting by default

---

## Problem

Scanner flagged invariant-related governance controls and emergency authority APIs.
These paths previously used `panic!` which has been converted to returning errors,
but the broader design — who can trigger emergency authority, under what conditions,
with what on-chain audit trail — has not been fully specified.

## Current State (t5-5)

- `check_constitution_hash` returns an error on mismatch but does not emit an
  on-chain event; silent failure is undetectable by off-chain observers.
- Authority expiry logic exists but is not tested for the boundary case where
  `authority_expiry_block == current_block`.

## Current State (t5-6)

- Emergency pause path is gated behind `ensure_root` but no timelock or
  multi-sig quorum is required; a single compromised sudo key can halt the chain.

## Current State (t5-7)

- Invariant violations accumulate in storage but are never pruned; under sustained
  attack this can grow unbounded.

## Proposed Changes

1. **t5-5 — Events**: Emit `InvalidConstitutionHash { expected, got, block }` and
   `AuthorityExpired { authority_id, expired_at }` events on error paths so
   indexers can alert operators.
2. **t5-5 — Boundary test**: Add unit test for `authority_expiry_block == current_block`
   (should be treated as expired, not active).
3. **t5-6 — Timelock**: Wrap emergency pause in a timelock (e.g. 6-hour delay)
   with the ability for governance to fast-track via supermajority vote.
4. **t5-7 — Pruning**: Add a bounded storage map for violation records with
   `MaxViolations` config constant; prune oldest entries on insert when full.
5. **Tests**: Unit tests for each of the above; mock `current_block` via
   `frame_system::Pallet::<T>::set_block_number`.

## Migration / Governance

- All three changes require a runtime upgrade.
- t5-6 timelock change is the highest-risk item; deploy on testnet and get
  sign-off from security committee before mainnet.
- Do NOT set `chain_halt_on_invariant_violation = true` by default.

## Open Questions

- What is the correct timelock duration for emergency pause? (6h / 12h / 24h)
- Should authority expiry be a hard error or a degraded-mode warning?
- Should `MaxViolations` be a governance-settable parameter or a compile-time constant?
