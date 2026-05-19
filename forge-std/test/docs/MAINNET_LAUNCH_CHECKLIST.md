# MAINNET LAUNCH CHECKLIST

Target: v0.4 Internal-Only Mainnet RC

**Note:** This checklist is scoped to internal RC-1 build and gate readiness only. Public launch messaging should follow `docs/MAINNET_CANARY_PLAN.md`.

## Related Readiness Checklist
- `docs/MAINNET_READINESS_CHECKLIST.md` — focused mainnet readiness and public canary launch plan
- `docs/MAINNET_CANARY_PLAN.md` — public canary launch path and 30–90 day reveal plan

## Build Gates

- cargo fmt --check passes
- cargo check --workspace passes
- cargo build --release -p x3-chain-node passes
- cargo build --release -p x3-cli passes
- cargo build --release -p x3-proof passes

## Test Gates

- cargo test --workspace passes
- cargo test -p pallet-x3-cross-vm-router passes
- cargo test -p pallet-x3-supply-ledger passes
- cargo test -p pallet-x3-atomic-kernel passes
- cargo test -p x3-ixl passes
- cargo test -p x3-proof passes

## Atomic Gates

- rollback and replay rejection tests pass
- completion after timeout rejected
- duplicate completion and replay produce no state change

## Genesis Gates

- chain-specs/x3-mainnet-plain.json generated from current node
- chain-specs/x3-mainnet-raw.json generated from current node
- docs/MAINNET_GENESIS_REVIEW.md completed with evidence
- ExternalBridgesEnabled=false confirmed

## Governance Gates

- external bridges remain disabled
- invariant halt policy set to EventAndPause or RejectNewTransfers
- emergency recovery and resume authority validated

## Validator Gates

- production authorities configured
- bootnodes reachable
- no dev seed authorities or dev endowed accounts

## Monitoring Gates

- panic and unwrap audit report generated: reports/panic_unwrap_audit.md
- mainnet RC report generated: reports/mainnet_rc_report.md

## No-Launch Conditions

- any failing build or test gate
- any runtime-hook panic path in production code
- any user-triggerable unwrap or expect in extrinsic hot path
- external bridges enabled without completed documented audit gate
- unresolved supply invariant halt policy behavior
