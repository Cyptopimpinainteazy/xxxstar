# X3 Token System Implementation Roadmap
## From Cross-VM to Universal Asset Kernel

**Version:** 1.0  
**Status:** Build Plan — Ready for Execution  
**Audience:** Core Engineers, Runtime Devs, Bridge Team, Auditors  
**Companion Docs:** `X3_UNIVERSAL_ASSET_KERNEL.md`, `EXCHANGE_INTEGRATION_PROCEDURES.md`, `VALIDATOR_TOKEN_OPERATIONS.md`

---

## 0. GUIDING PRINCIPLES (NON-NEGOTIABLE)

These are the rules everything else defers to. If a PR violates any of these, it does not land.

### The King Invariant

> **No sequence of calls can create more token representation than locked collateral or canonical supply.**

```
For every asset_id:
  native_supply + evm_vm_supply + svm_vm_supply + pending_supply
    <= canonical_supply (or locked_collateral for external assets)
```

This invariant is checked:
- On every mint / burn / lock / release
- In unit tests, integration tests, and fuzz tests
- As a runtime assertion (panic = consensus failure) on debug builds
- As a `ensure!()` guard on production builds

### Four Other Non-Negotiables

1. **One AssetId per asset.** Every representation reports to one supply ledger. No parallel bookkeeping.
2. **Cross-VM is atomic. Cross-chain is async-with-proofs.** Never blur the line.
3. **Relayers submit proofs. Runtime verifies proofs. Relayers cannot mint.**
4. **Every transfer has an expiry and a refund path.** No stuck-forever user funds.

---

## 1. TOKEN TAXONOMY (DECIDE PER ASSET)

Before any asset ships, the registry entry must declare its type:

```rust
pub enum AssetType {
    /// Minted on X3 first; lives across X3 Native / EVM / SVM.
    /// Easiest case. Start here.
    X3Native,

    /// Locked on an external chain; wrapped representation on X3.
    /// Requires real proof verification.
    ExternalWrapped {
        origin_domain: DomainId,
        origin_contract: Vec<u8>,
    },

    /// Backed by liquidity / reserves, not direct 1:1 lockup.
    /// Requires solvency proof + liquidation rules. Ship last.
    SyntheticBacked {
        collateral_assets: Vec<AssetId>,
        collateralization_ratio_bps: u32,
    },
}
```

| Type | Launch Phase | Examples |
|---|---|---|
| `X3Native` | Phase 1 | X3, governance token, X3-DEX LP tokens |
| `ExternalWrapped` | Phase 2–3 | USDC, ETH, SOL, ARB |
| `SyntheticBacked` | Phase 5+ | x3BTC (pre-custody), x3ETH synthetic |

---

## 2. PHASED BUILD PLAN

Five levels. Do not skip. Do not parallelize across levels without a written exception.

### Level 1 — Internal Dev (Week 1–2 after mainnet launch)

**Goal:** Prove the kernel shape on a dev chain with mocks.

**Scope:**
- [ ] `x3-asset-registry` pallet: register/pause/update assets
- [ ] `x3-token-vault` pallet: SupplyLedger with invariant enforcement
- [ ] `x3-cross-vm-router` pallet: X3 Native ↔ X3 EVM ↔ X3 SVM
- [ ] Mock EVM + mock SVM precompiles (no real cross-VM wiring yet)
- [ ] Mock finality oracle (always returns `Finalized`)
- [ ] Unit tests for every pallet extrinsic
- [ ] Property tests for supply invariant

**Exit Criteria:**
- [ ] Cross-VM round trip works in integration tests
- [ ] Supply invariant holds under 1M fuzzed operation sequences
- [ ] All admin controls require Root or multisig origin

---

### Level 2 — Closed Testnet (Week 3–4)

**Goal:** Real cross-VM on a small validator set with real EVM/SVM execution.

**Scope:**
- [ ] Wire `x3-cross-vm-router` to real X3 EVM precompile
- [ ] Wire `x3-cross-vm-router` to real X3 SVM syscall
- [ ] Deploy ERC20 kernel adapter contract on X3 EVM
- [ ] Deploy SPL-style kernel program on X3 SVM
- [ ] `AccountBytes` canonical address mapping (see §5)
- [ ] Decimal conversion module with integer-only math (see §6)
- [ ] Transfer expiry + refund flow (see §7)
- [ ] Per-asset and per-route pause controls (see §8)
- [ ] Route limits enforcement (see §9)

**Exit Criteria:**
- [ ] 10,000 successful cross-VM round trips with zero supply drift
- [ ] Replay attack tests all fail correctly
- [ ] Expired transfers refund correctly
- [ ] Paused routes reject transfers correctly

---

### Level 3 — Public Testnet (Week 5–8)

**Goal:** Real external chains (Ethereum Sepolia, Base Sepolia, Solana devnet). Fake value only.

**Scope:**
- [ ] `x3-crosschain-gateway` pallet
- [ ] `x3-finality-oracle` pallet with per-chain finality rules (§12)
- [ ] Ethereum light-client verifier OR validator attestation quorum (Tier 1)
- [ ] Solana finality verifier (`finalized` commitment)
- [ ] Solidity `X3ExternalGateway.sol` deployed on Sepolia + Base Sepolia
- [ ] Solana lock program deployed on devnet
- [ ] Relayer network (minimum 3 relayers, threshold attestation)
- [ ] Reorg-handling in oracle (`Observed` → `Confirmed` → `Finalized` → `Invalidated`)
- [ ] Indexer emits events for explorer (§13)
- [ ] Public block explorer shows transfer status

**Exit Criteria:**
- [ ] 1,000 successful external deposits (Sepolia USDC → X3 wrapped USDC)
- [ ] 1,000 successful external withdrawals (X3 wrapped USDC → Sepolia USDC)
- [ ] Reorg test: injected reorg invalidates pending transfer correctly
- [ ] Relayer compromise test: single malicious relayer cannot mint
- [ ] User can track any transfer end-to-end via explorer

---

### Level 4 — Guarded Mainnet (Week 9–12)

**Goal:** Small caps, few assets, heavy monitoring, fast pause.

**Scope:**
- [ ] Audited kernel pallets (external audit)
- [ ] Audited Solidity gateway contracts
- [ ] Route caps: start at $10K/day/route, $1K/tx
- [ ] First production assets: X3 native only (cross-VM), then USDC (cross-chain)
- [ ] 24/7 monitoring with PagerDuty escalation
- [ ] Emergency pause tested under load
- [ ] Insurance/reserve fund seeded
- [ ] Multisig for all admin functions

**Exit Criteria:**
- [ ] 30 days clean operation at guarded caps
- [ ] Zero supply invariant violations in production logs
- [ ] All emergency pause drills executed successfully
- [ ] Bug bounty program live

---

### Level 5 — Full Mainnet (Month 4+)

**Goal:** Raise caps, add chains, enable governance.

**Scope:**
- [ ] Raise route limits 10x after 30-day stability window
- [ ] Add Arbitrum, BSC, Polygon gateways
- [ ] Add Solana SPL route
- [ ] Enable on-chain governance for route/limit changes (with timelocks, §16)
- [ ] BTC vault design (federated multisig or threshold sig) — **separate project**
- [ ] SyntheticBacked assets (x3BTC, x3ETH) — **requires solvency proof system**

---

## 3. MODULE DEPENDENCY GRAPH

```
x3-asset-registry  (pure data, no dependencies)
        │
        ▼
x3-token-vault  (supply ledger; depends on registry)
        │
        ├────────────────────────┐
        ▼                        ▼
x3-cross-vm-router        x3-crosschain-gateway
        │                        │
        │                        ▼
        │                x3-finality-oracle
        │                        │
        └───────┬────────────────┘
                ▼
         x3-message-bus  (domain-separated hashing)
                │
                ▼
         x3-transfer-explorer  (read-only indexer API)
```

**Build Order:** registry → vault → message-bus → cross-vm-router → finality-oracle → crosschain-gateway → explorer.

---

## 4. CANONICAL ADDRESS MAPPING

Do not use raw `Vec<u8>` for recipients. Ever.

```rust
pub type DomainId = u32;

pub const DOMAIN_X3_NATIVE: DomainId = 1;
pub const DOMAIN_X3_EVM:    DomainId = 2;
pub const DOMAIN_X3_SVM:    DomainId = 3;
pub const DOMAIN_ETHEREUM:  DomainId = 10;
pub const DOMAIN_BASE:      DomainId = 11;
pub const DOMAIN_ARBITRUM:  DomainId = 12;
pub const DOMAIN_BSC:       DomainId = 13;
pub const DOMAIN_SOLANA:    DomainId = 20;
pub const DOMAIN_BITCOIN:   DomainId = 30;

pub enum AccountBytes {
    X3Native([u8; 32]),
    Evm([u8; 20]),
    Svm([u8; 32]),
    Bitcoin(BoundedVec<u8, ConstU32<64>>),
}

pub struct Recipient {
    pub domain: DomainId,
    pub account: AccountBytes,
}
```

**Validation rules:**
- `domain` must match `AccountBytes` variant
- `Recipient { domain: DOMAIN_EVM, account: AccountBytes::Svm(..) }` → reject
- All decoding goes through `Recipient::try_from_raw(domain, bytes)` — one entrypoint

---

## 5. DECIMAL HANDLING

Integer math only. No floats. No approximations.

```rust
pub struct AssetMetadata {
    pub asset_id: AssetId,
    pub canonical_decimals: u8,
    pub representation_decimals: BoundedBTreeMap<DomainId, u8, ConstU32<64>>,
    // ...
}

pub fn convert(
    amount: u128,
    from_decimals: u8,
    to_decimals: u8,
) -> Result<u128, Error> {
    if from_decimals == to_decimals {
        return Ok(amount);
    }
    if from_decimals < to_decimals {
        let scale = 10u128.checked_pow((to_decimals - from_decimals) as u32)
            .ok_or(Error::Overflow)?;
        amount.checked_mul(scale).ok_or(Error::Overflow)
    } else {
        let scale = 10u128.checked_pow((from_decimals - to_decimals) as u32)
            .ok_or(Error::Overflow)?;
        // Reject if lossy (non-zero remainder)
        if amount % scale != 0 {
            return Err(Error::LossyConversion);
        }
        Ok(amount / scale)
    }
}
```

**Policy:** Reject lossy conversions by default. Only allow explicit `convert_with_rounding` in user-initiated flows where the user has signed off on the rounded amount.

---

## 6. TRANSFER LIFECYCLE & EXPIRY

```rust
pub enum TransferStatus {
    Created,
    SourceDebited,
    ProofSubmitted,
    DestinationCredited,
    Finalized,
    Expired,
    Refunded,
    Invalidated, // reorg killed it
    Failed(FailureReason),
}

pub struct TransferRecord {
    pub message_id: H256,
    pub asset_id: AssetId,
    pub source_domain: DomainId,
    pub destination_domain: DomainId,
    pub sender: AccountBytes,
    pub recipient: Recipient,
    pub amount: u128,
    pub nonce: u64,
    pub status: TransferStatus,
    pub created_at_block: BlockNumberFor<T>,
    pub expiry_block: BlockNumberFor<T>,
    pub refund_allowed_after: BlockNumberFor<T>,
}
```

**Expiry defaults:**
- Cross-VM transfer: 100 blocks (~10 min)
- Cross-chain EVM deposit: 256 blocks (~26 min) — covers Ethereum finality window + relayer retry
- Cross-chain Solana deposit: 600 blocks (~60 min) — covers slot finalization + relayer retry

**Refund flow:**
- After `expiry_block`, anyone can call `refund(message_id)`
- Refund credits sender on the source domain
- Refund is idempotent (status transition `SourceDebited` → `Refunded`)

---

## 7. PAUSE CONTROLS (SURGICAL, NOT NUCLEAR)

```rust
pub enum PauseScope {
    Global,                                    // last resort
    Asset(AssetId),
    Route { asset: AssetId, from: DomainId, to: DomainId },
    ExternalChain(DomainId),
    Relayer(AccountId),
    Deposits(DomainId),                        // one direction
    Withdrawals(DomainId),
}
```

**Origins:**
- `Global` pause: requires security council multisig (3-of-5), instant
- `Asset` / `Route` pause: requires ops multisig (2-of-3), instant
- Unpause: requires full governance vote + 48h timelock

**Rule:** Pauses are instant. Unpauses are slow. Reversed from normal governance.

---

## 8. ROUTE LIMITS

```rust
pub struct RouteLimits {
    pub min_amount: u128,
    pub max_amount: u128,
    pub daily_limit: u128,
    pub per_wallet_daily_limit: u128,
    pub pending_limit: u32,
}
```

**Initial mainnet caps (§2 Level 4):**

| Route | Max tx | Daily limit | Per-wallet daily |
|---|---|---|---|
| X3 Native ↔ X3 EVM | 1,000,000 X3 | unlimited | unlimited |
| X3 Native ↔ X3 SVM | 1,000,000 X3 | unlimited | unlimited |
| X3 EVM ↔ X3 SVM | 1,000,000 X3 | unlimited | unlimited |
| Ethereum USDC → X3 | 10,000 USDC | 100,000 USDC | 10,000 USDC |
| X3 → Ethereum USDC | 10,000 USDC | 100,000 USDC | 10,000 USDC |
| Solana SPL → X3 | 10,000 USDC-eq | 100,000 | 10,000 |

Raise 10x only after 30 days clean at current caps.

---

## 9. FEE MODEL

Separate fee accounting for cross-VM and cross-chain.

```rust
pub struct FeeConfig {
    pub cross_vm_base_fee: u128,        // flat, in X3
    pub cross_vm_bps: u32,              // basis points of amount
    pub cross_chain_base_fee: u128,     // flat, in X3
    pub cross_chain_bps: u32,
    pub relayer_fee_bps: u32,           // to relayer network
    pub protocol_fee_bps: u32,          // to treasury
    pub insurance_fee_bps: u32,         // to insurance fund
}
```

**Example (USDC deposit from Ethereum):**
```
amount           = 10,000 USDC
base_fee         = 0.5 USDC (covers X3 gas)
relayer_fee      = 10 bps = 10 USDC (covers ETH gas)
protocol_fee     = 5 bps = 5 USDC
insurance_fee    = 2 bps = 2 USDC
total_fee        = 17.5 USDC
user_receives    = 9,982.5 USDC
```

Cross-VM fees are an order of magnitude lower (no external gas costs).

---

## 10. PROOF TIERS (§11 FROM USER GUIDANCE)

| Tier | Model | Use Case | X3 Phase |
|---|---|---|---|
| 0 | Mock verifier | Dev only | Level 1 |
| 1 | Validator attestation quorum | Testnet / early mainnet | Level 2–4 |
| 2 | Light-client verification | Serious production | Level 5 |
| 3 | Full zk/validity proof | High-value settlement | Level 6+ |

**Phase 2 mainnet target:** Tier 2 for Ethereum / Base / Arbitrum (light-client). Tier 1 acceptable for Solana until a production-grade Solana light-client library exists in Substrate.

---

## 11. FINALITY ORACLE RULES

```rust
pub enum FinalityRequirement {
    EvmConfirmations(u32),       // Ethereum: 64, Base: 128 (L1 finality)
    SolanaFinalized,             // commitment == "finalized"
    BitcoinConfirmations(u32),   // 6+ for small, 100+ for large
    X3Finalized,                 // built-in grandpa finality
}

pub enum ProofStatus {
    Observed,      // event seen, not yet confirmed
    Confirmed,     // enough confirmations
    Finalized,     // past reorg horizon
    Invalidated,   // reorg wiped the source event
}
```

**Rule:** Never mint from `Observed` or `Confirmed`. Only `Finalized`.

---

## 12. REPLAY PROTECTION

Use `StorageMap<H256, bool>`, never `Vec` scanning.

```rust
#[pallet::storage]
pub type ProcessedMessages<T> =
    StorageMap<_, Blake2_128Concat, H256, (), OptionQuery>;

fn mark_processed(message_id: H256) -> DispatchResult {
    ensure!(
        !ProcessedMessages::<T>::contains_key(&message_id),
        Error::<T>::ReplayedMessage
    );
    ProcessedMessages::<T>::insert(message_id, ());
    Ok(())
}
```

**Message ID derivation (domain-separated):**

```rust
let message_id = blake2_256(&[
    b"X3_CROSS_DOMAIN_TRANSFER_V1",
    &source_domain.encode(),
    &destination_domain.encode(),
    &asset_id.encode(),
    &sender.encode(),
    &recipient.encode(),
    &amount.to_be_bytes(),
    &nonce.to_be_bytes(),
].concat());
```

Any field change → different ID. No collisions possible across versions.

---

## 13. GOVERNANCE & UPGRADE DISCIPLINE

| Action | Origin | Timelock |
|---|---|---|
| Pause route / asset | Ops multisig (2-of-3) | Instant |
| Pause global | Security council (3-of-5) | Instant |
| Unpause | Governance vote | 48h |
| Register new asset | Governance vote | 24h |
| Change route limits (raise) | Governance vote | 24h |
| Change route limits (lower) | Ops multisig | Instant |
| Upgrade verifier contract | Governance vote | 48h |
| Upgrade kernel pallet | Root (governance-gated) | Runtime upgrade flow |
| Release stuck funds | Security council + governance | 72h |

**Contract upgradeability:**
- Solidity gateways: upgradeable during Level 4, frozen at Level 5 for audited assets
- Substrate pallets: upgradeable via standard runtime upgrade (inherently timelocked by governance)

---

## 14. AUDIT TARGETS (BEFORE LEVEL 4)

External audit scope:

1. `x3-asset-registry` pallet (asset identity, metadata integrity)
2. `x3-token-vault` pallet (supply ledger, invariant enforcement)
3. `x3-cross-vm-router` pallet (state machine, replay, atomicity)
4. `x3-crosschain-gateway` pallet (proof handling, mint authority)
5. `x3-finality-oracle` pallet (reorg handling, finality rules)
6. `x3-message-bus` (hash domain separation)
7. `X3ExternalGateway.sol` (deposit/withdrawal, admin controls, upgradeability)
8. Solana lock program (equivalent gateway logic)
9. Decimal conversion module (overflow, lossy conversion)
10. Relayer code paths (proof submission, retry logic)

**Required audit deliverables:**
- Fuzz harness for supply invariant
- Property test suite for every state transition
- Formal verification of replay protection (Kani/Creusot)
- Manual review of admin control surface

---

## 15. "DO NOT MAINNET YET" CHECKLIST (§20)

Hard gate before Level 4. Every box must be checked and signed off.

- [ ] Cross-VM round trip works (10,000 successful round trips)
- [ ] External deposit proof works (1,000 successful on testnet)
- [ ] External withdrawal proof works (1,000 successful on testnet)
- [ ] Same-proof replay fails (automated test)
- [ ] Same-nonce replay fails (automated test)
- [ ] Wrong chain ID fails (automated test)
- [ ] Wrong asset mapping fails (automated test)
- [ ] Wrong recipient fails (automated test)
- [ ] Wrong amount fails (automated test)
- [ ] Paused route rejects transfers (automated test)
- [ ] Paused asset rejects transfers (automated test)
- [ ] Expired transfer refunds correctly (automated test)
- [ ] Supply invariant holds under 1M-op fuzz run
- [ ] Gateway contract audit report: no unresolved High/Critical
- [ ] Kernel pallet audit report: no unresolved High/Critical
- [ ] Relayer cannot mint without proof (penetration test)
- [ ] Admin keys are multisig (verified on-chain)
- [ ] Route limits enforced (automated test)
- [ ] Events indexed and queryable via explorer
- [ ] Users can track transfer status end-to-end
- [ ] Emergency pause drill executed in staging
- [ ] Insurance fund seeded (≥ 10% of expected TVL)
- [ ] Bug bounty program live with published scope

If any box is unchecked → it is a demo, not a launch.

---

## 16. FIRST-90-DAYS ROUTE PRIORITY

Ship this order. Nothing else.

1. **X3 Native ↔ X3 EVM** (Phase 1 Level 1-2)
2. **X3 Native ↔ X3 SVM** (Phase 1 Level 1-2)
3. **X3 EVM ↔ X3 SVM** (Phase 1 Level 1-2)
4. **Ethereum USDC ↔ X3** (Phase 2 Level 3-4)
5. **Base USDC ↔ X3** (Phase 2 Level 4)
6. **Solana SPL ↔ X3** (Phase 3 Level 4-5)
7. **Arbitrum USDC ↔ X3** (Phase 2 Level 5)
8. **BSC USDC ↔ X3** (Phase 2 Level 5)

**BTC is deferred.** It needs its own track: federated multisig or threshold-sig vault, DLC/HTLC analysis, BTC-specific relayer. Not part of Q3 2026.

---

## 17. THE PITCH (USE THIS VERBATIM)

> X3 lets one token exist across native runtime, EVM, SVM, and external chains without fragmented supply. Every representation maps to one canonical AssetId, one supply ledger, and one finality-aware settlement kernel.

Short form:

> One token. Every VM. Every chain. One supply ledger.

The differentiator:

> Other chains bridge. X3 does atomic cross-VM token movement inside one chain, then extends it to external chains with the same kernel.

---

## 18. RISK REGISTER (TOP 10)

| # | Risk | Mitigation | Owner |
|---|---|---|---|
| 1 | Supply invariant violation | Invariant check on every mutation + fuzz suite | Runtime Lead |
| 2 | Proof replay | Domain-separated message IDs + StorageMap dedup | Bridge Lead |
| 3 | Relayer compromise mints tokens | Relayer submits proofs only; runtime verifies | Bridge Lead |
| 4 | Reorg mints invalid tokens | Oracle requires `Finalized` before mint | Bridge Lead |
| 5 | Decimal conversion bug | Integer-only math + reject lossy + unit tests | Runtime Lead |
| 6 | Admin key compromise | Multisig + timelocks on dangerous actions | Security Lead |
| 7 | Stuck user funds | Expiry + refund path + explorer visibility | UX Lead |
| 8 | Gateway contract bug post-deploy | Audits + upgradeability (Level 4) + freeze (Level 5) | Bridge Lead |
| 9 | Limit-breaking edge case | Conservative initial caps + 30-day stability window before raising | Ops Lead |
| 10 | BTC pressure forces early launch | BTC explicitly deferred; written policy | Product Lead |

---

## 19. OWNERSHIP MATRIX

| Module | Primary Owner | Reviewer | Auditor |
|---|---|---|---|
| `x3-asset-registry` | Runtime Lead | Bridge Lead | External |
| `x3-token-vault` | Runtime Lead | Security Lead | External + formal verification |
| `x3-cross-vm-router` | Runtime Lead | Bridge Lead | External |
| `x3-crosschain-gateway` | Bridge Lead | Security Lead | External |
| `x3-finality-oracle` | Bridge Lead | Runtime Lead | External |
| `x3-message-bus` | Runtime Lead | Bridge Lead | External |
| `X3ExternalGateway.sol` | Bridge Lead | External Solidity reviewer | External |
| Solana lock program | Bridge Lead | External Solana reviewer | External |
| Relayer binary | Infra Lead | Bridge Lead | External |
| Explorer / indexer | Frontend Lead | Bridge Lead | Internal |

---

## 20. IMMEDIATE NEXT ACTIONS (POST-MAINNET, WEEK 1)

Do these in order. No skipping.

1. [ ] Scaffold `crates/pallets/x3-asset-registry` with empty extrinsics
2. [ ] Scaffold `crates/pallets/x3-token-vault` with `SupplyLedger` struct + invariant test
3. [ ] Scaffold `crates/pallets/x3-message-bus` with domain-separated hash helper
4. [ ] Write property test: "for all op sequences, supply invariant holds"
5. [ ] Write property test: "for all messages, replay fails"
6. [ ] Scaffold `crates/pallets/x3-cross-vm-router` with state machine
7. [ ] Wire router to mock EVM/SVM precompiles
8. [ ] Publish RFC: AssetId derivation + AccountBytes canonical encoding
9. [ ] Kick off Solidity gateway design review (security team)
10. [ ] Schedule audit vendor (target: Level 4 entry date)

---

## STATUS

✅ **Roadmap Approved**  
📅 **Effective Date:** Post-mainnet launch (May 20, 2026)  
🔗 **Companion Specs:** `X3_UNIVERSAL_ASSET_KERNEL.md`, `EXCHANGE_INTEGRATION_PROCEDURES.md`, `VALIDATOR_TOKEN_OPERATIONS.md`  
📝 **Last Updated:** April 21, 2026

---

**Remember:**
> Cross-VM is where X3 looks like magic. Cross-chain is where bridges get robbed.  
> Ship cross-VM first. Nail it. Then extend.
