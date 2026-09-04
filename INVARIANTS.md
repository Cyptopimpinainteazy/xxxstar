# Invariants

Critical invariants:

1. Canonical supply is preserved.
2. No double mint / no double spend.
3. Nonce uniqueness and replay protection.
4. Cross-chain atomic commit-or-rollback.
5. Bridge lock/mint/burn/release correctness.
6. Authorization boundaries for governance/treasury.

Validation entrypoint: `python scripts/invariant_guard.py`.