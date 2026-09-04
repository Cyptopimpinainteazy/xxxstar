# Documentation Index (Hub Fee Workstream)

## Canonical Docs

- `DEPLOYMENT_PACKAGE_INDEX.md`
- `DEPLOYMENT_READINESS_REPORT.md`
- `DEPLOYMENT_QUICK_REFERENCE.md`
- `TESTNET_DEPLOYMENT_GUIDE.md`
- `DEPLOYMENT_VERIFICATION_CHECKLIST_HUB_FEE.md`
- `EXECUTION_PLAYBOOK.md`
- `HUB_FEE_DEPLOYMENT_STATUS.md`

## Production Stub Status (2026-06-14)

The following production stubs have been eliminated or feature-gated:

- **Bridge adapters** (`crates/x3-bridge-adapters/src/`): Ethereum/Solana/Bitcoin now have real RPC-based implementations with chain-specific header validation (RLP decode, base58 decode, fail-closed BTC feature gate).
- **Validator RPC** (`crates/x3-rpc/src/validator_rpc.rs`): Wired to live runtime API via `ProvideRuntimeApi` + `HeaderBackend`. Queries live Aura/GRANDPA authority set and x3-kernel authorized accounts.
- **ZK proof verification** (`crates/x3-bridge/src/cross_chain_proofs.rs`): Feature-gated behind `#[cfg(feature = "zk-proofs")]`. Production builds cannot accidentally route ZK proofs through unverified path.
- **Register allocator** (`x3-lang/compiler/src/regalloc.rs`): Linear-scan with 16 physical registers and spill-to-stack support. Deterministic output for GPU replay invariant.
- **Bytecode checksum** (`crates/x3-backend/src/bc_format_helpers.rs` + `crates/x3-vm/src/verifier.rs`): CRC32 computed over bytecode body, validated on VM load.
- **Swarm policy** (`crates/x3-swarm-core/src/policy.rs`): Full `ApprovalContext` chain including `human_approval_token`, `security_quorum_sig`, `governance_proposal_id`.
- **Risk engine** (`crates/x3-gateway-risk-engine/src/lib.rs`): `has_low_anti_rug_score` and `has_high_volatility` documented with wiring instructions to x3-foundry-auditor and x3-oracle pallets. `RateLimiter::should_limit` implements sliding-window counter.
- **Pow opcode** (`crates/x3-backend/src/mir_lower.rs`): Replaced `emit_mul_i` placeholder with `emit_call` to built-in Pow handler.
- **X3_BACKEND selector** (`x3-lang/vm/src/bridge.rs`): `resolve_bridge_backend()` function with environment-variable-based backend selection.
- **E2E tests** (`x3-lang/tests/e2e/`): `.x3` source files for simple transfer, atomic swap, and cross-chain bridge step.
- **Executor authorization** (`pallets/x3-settlement-engine/src/lib.rs` lines 1949-1959, `pallets/x3-cross-vm-router/src/lib.rs`): Settlement and routing extrinsics gated through `pallet_x3_kernel::AuthorizedAccounts`. Secure-by-default: empty registry rejects all callers.

## Updated Documentation (2026-06-14)

- `docs/_autodocs/PENDING_SYNC.md` — Replaced generic placeholder with a real module index covering bridge adapters, validator RPC, settlement engine, cross-VM router, register allocator, and bytecode format.
- `x3-lang/PLAN.md` — Consolidated completed milestones section (opcode alignment, intent semantics, production backend, compiler bridge, E2E tests, executor authorization, ZK feature gate). Pending work re-scoped to dev tooling and gas model.
- `TODO.md` — Funding Swarm Step 7 (smoke test) marked complete; all 7 steps now verified done. Pending: live DB integration test with Postgres container.
- `DOCUMENTATION_INDEX.md` — This file. Added executor authorization entry and updated-documentation section.

## Why this index exists

This index was created to correct documentation drift and ensure a single, root-level source of truth for this workstream.