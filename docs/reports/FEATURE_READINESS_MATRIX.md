# Feature Reality Map (Pass 1)

**Date:** 2026-06-10  
**Commit:** `9aeb4bf089f719c23ddd6d351b9541b345fd7685`

## Summary

| Status | Count | Features |
|--------|-------|----------|
| **FULL** | 13 | Internal cross-VM routing, Supply ledger invariants, Atomic kernel, IXL MVP, Packet standard, Internal domains (X3Native/X3Evm/X3Svm), Kernel invariants/rollback, EVM gateway contracts, Native transfers, Proof taxonomy/receipts, Runtime wiring, Compile-time feature gates |
| **PARTIAL** | 4 | Settlement engine (dead code paths), LiquidityCore swap (no_std dead end), SVM token adapter (bincode non-determinism), Bitcoin SPV verifier (dead variable) |
| **STUB** | 3 | Verification router (all 5 verifiers return `accepted: true` unconditionally), External chain adapters (not runtime-wired), Relayer infrastructure (marked TESTNET_ONLY/DISABLED, not feature-gated) |
| **ABSENT** | 1 | External bridges (correctly disabled by compile_error + genesis flag) |

## Critical Gaps

### Verification Router — STUB
**Claimed:** "Production verification router with EvmReceiptVerifier, SolanaFinalizedVerifier, BitcoinSpvVerifier"
**Reality:**
```rust
// crates/x3-verification-router/src/strategies/evm.rs
fn verify(...) -> Result<VerificationResult, VerificationError> {
    Ok(VerificationResult { accepted: true, verified_at: 0 })
}
```
ALL five verifiers return `accepted: true` with no real proof checking. The `EvmReceiptVerifier` does not verify RLP receipts. `BitcoinSpvVerifier` does not verify SPV header chains. Only `X3InternalVerifier` is real.

**These are NOT feature-gated behind `external-gateway`** — they compile and could be called in a mainnet-rc1 build if `ExternalBridgesEnabled` is toggled. This is a security risk: governance could enable external bridges and these stub verifiers would accept any proof.

### External Chain Adapters — STUB
`MockChainAdapter` compiles in ALL builds (not feature-gated). Its `verify_message_proof` returns `Ok(true)` unconditionally. The real chain adapters (EVM, Solana, Bitcoin) exist as files but are NOT wired into the runtime.

### Settlement Engine — PARTIAL
The `on_initialize` auto-finalization path has dead code branches for finalized/refunded states that log warnings but do not clean up state. The `on_idle` path references `x3-sidecar` which is marked TESTNET_ONLY.

### 13 Fully Implemented Features
These are production-quality with real logic, tests, and CI gates:
- `pallet-x3-cross-vm-router` — 6-route matrix, replay protection, expiry, limits
- `pallet-x3-supply-ledger` — canonical_supply >= represented_total on all mutations
- `pallet-x3-atomic-kernel` — bundle lifecycle, PoAE proof, IXL integration
- `x3-packet-standard` — commitment, timeout, replay protection
- `x3-ixl` — planner, interpreter, rollback
- `x3_liquidity_core` — spot swap (no_std path dead, std path works)
- EVM gateway contracts — `X3ExternalGateway.sol`, `X3VmERC20.sol`, `X3KernelBridge.sol`