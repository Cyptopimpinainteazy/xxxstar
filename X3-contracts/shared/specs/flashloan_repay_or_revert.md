# Flashloan: repay-or-revert (parity spec)

This spec is the canonical behavior contract for X3 flashloans across both
execution stacks. The vector file
`../test-vectors/flashloan_repay_or_revert.json` is the executable form.

## Invariants

- **I1 atomicity**: terminal pool balance must be `>= pre + fee`, else revert.
- **I2 no reentrancy**: a flashloan call cannot recursively borrow the same asset.
- **I3 fee monotonic**: `fee` is purely additive; protocol never owes borrower.
- **I4 round-up**: fee rounds up so 1-wei loops cannot drain.

## Vector schema

```jsonc
{
  "id": "flashloan/repay_or_revert/<n>",
  "asset": "MOCK",
  "amount": "<u128 as decimal string>",
  "fee_bps": <u16>,
  "borrower_kind": "honest" | "deadbeat" | "underpay",
  "expected": {
    "result": "ok" | "revert",
    "revert_reason": "CallbackFailed" | "NotRepaid" | null,
    "pool_delta": "<i128 decimal string>"  // signed change in pool balance
  }
}
```

## Required parity proofs

For each vector:
- EVM path: `forge test --match-test testParity_<id>`
- SVM path: `anchor test -- --filter parity_<id>`
- Outputs (`result`, `revert_reason`, `pool_delta`) MUST match exactly.

A mismatch is a hard launch blocker.
