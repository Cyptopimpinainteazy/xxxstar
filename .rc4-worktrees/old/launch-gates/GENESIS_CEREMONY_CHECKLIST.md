# X3 Mainnet Genesis Ceremony Checklist

**Phase**: Pre-Launch  
**Owner**: Chain Launch Committee  
**Approval Required**: YES (block launch if any item fails)  

---

## Executive Summary

This is your **launch gate**. Every item must be verified before genesis block is cut.

No shortcuts. No exceptions. This is not a suggestion list—it is the law.

---

## Phase 1: Chain Configuration Finalization

### Chain ID & Network Identity

- [ ] **Chain ID finalized** and locked in stone
  - Value: `________________` (numeric, unique across all chains)
  - Source: `chain-spec.json`, `lib.rs` constants
  - Approved by: _______________________
  - Date: _______________________
  - **Note**: Once genesis is cut, chain ID cannot change. Changing it creates a new chain.

- [ ] **Network name finalized**
  - Value: `________________` (e.g., "X3 Mainnet")
  - Source: chain spec, telemetry, explorer
  - Approved by: _______________________

- [ ] **Network protocol version locked**
  - Value: `spec_version = ________________`
  - Value: `impl_version = ________________`
  - Committed to git with tag: `v________________`
  - Cannot be bumped until runtime upgrade is ready

### Canonical Supply & Token Economics

- [ ] **Total canonical supply finalized**
  - Supply: `________________` (in smallest unit, e.g., satoshis)
  - Breakdown: 
    - Native genesis allocation: `________________`
    - EVM wrapped supply: `________________`
    - SVM wrapped supply: `________________`
    - BTC bridge reserve: `________________`
    - Treasury initial: `________________`
    - Vesting contracts: YES / NO
  - Verified by: _______________________
  - Cannot be inflated post-genesis

- [ ] **No hidden inflation paths**
  - ✅ Verified: No minting functions except via governance
  - ✅ Verified: No unbounded reward paths
  - ✅ Verified: Slashing cannot reduce total supply below zero
  - Audit date: _______________________

- [ ] **Token decimal places locked**
  - Value: `________________` (e.g., 18 decimals)
  - Consistent across: runtime, RPC, explorer, wallets
  - Verified by: _______________________

### Genesis Balances & Allocations

- [ ] **Genesis account list finalized and audited**
  - Total accounts: `________________`
  - Largest account: `________________` (% of supply)
  - Top 10 concentration: `________________`%
  - Audit: All balances sum to canonical supply
  - Audit date: _______________________
  - **Note**: Re-audit after final snapshot

- [ ] **No test/dev accounts in genesis**
  - ✅ "Alice" removed
  - ✅ "Bob" removed
  - ✅ "Charlie" removed
  - ✅ No hardcoded sudo keys
  - Verified by: _______________________

- [ ] **Vesting schedule locked** (if applicable)
  - ✅ All vesting contracts reviewed
  - ✅ All vesting end dates reasonable (no 50-year cliffs)
  - ✅ No vesting contract exploit paths
  - Audit date: _______________________

- [ ] **Treasury initial balance set**
  - Amount: `________________`
  - Purpose: `________________`
  - Spending governance required: YES / NO
  - Verified by: _______________________

---

## Phase 2: Governance & Authority Configuration

### Root/Sudo Policy

- [ ] **Sudo/Root access policy decided**
  - Decision: 
    - [ ] Sudo enabled with single key (risky, set expiry)
    - [ ] Sudo disabled immediately after genesis
    - [ ] Multisig required to invoke sudo
  - Approved by: _______________________
  - If enabled, expiry block: `________________`
  - **Note**: Sudo disabling is irreversible. Plan carefully.

- [ ] **Root authorization tested**
  - ✅ Sudo can call privileged extrinsics
  - ✅ Non-sudo cannot bypass checks
  - Test results: _______________________

### Governance Bootstrap

- [ ] **Governance council/democracy parameters finalized**
  - Voting period: `________________` blocks
  - Proposal deposit: `________________`
  - Referendum threshold: `________________`%
  - Council size: `________________`
  - Approved by: _______________________

- [ ] **Initial governance authority keys configured**
  - Democracy proposer: `________________` (address)
  - Council members (initial): `________________`
  - Verified unique: YES / NO
  - All keys are cold storage: YES / NO
  - Backup keys held by: _______________________

- [ ] **No governance bypass paths**
  - ✅ Verified: All governance-gated calls require vote
  - ✅ Verified: No hardcoded approvals
  - ✅ Verified: Emergency halt requires governance
  - Audit date: _______________________

---

## Phase 3: Validator Configuration

### Session Keys & Validator Set

- [ ] **Initial validator set finalized**
  - Validator count: `________________`
  - All validators confirmed ready: YES / NO
  - List location: `________________`

- [ ] **Session keys collected and verified**
  - Session key format: `________________` (e.g., sr25519)
  - Total keys collected: `________________`
  - Keys verified unique: YES / NO
  - Keys stored securely: YES / NO (describe: ________________)
  - Expiry date (if applicable): _______________________

- [ ] **Validator consensus algorithm locked**
  - Algorithm: `________________` (e.g., BABE)
  - Finality engine: `________________` (e.g., GRANDPA)
  - Slot duration: `________________` seconds
  - Epoch length: `________________` blocks
  - Verified by: _______________________

- [ ] **Staking parameters set**
  - Minimum stake per validator: `________________`
  - Reward rate: `________________`%
  - Slashing amount: `________________`%
  - Era duration: `________________`
  - Approved by: _______________________

### Bootnodes & Network Discovery

- [ ] **Bootnodes configured**
  - Count: `________________` (recommend 3+)
  - Bootnodes:
    - `________________`
    - `________________`
    - `________________`
  - All resolvable and online: YES / NO
  - Tested by: _______________________

- [ ] **DNS seeds configured** (if applicable)
  - DNS: `________________`
  - Fallback DNS: `________________`
  - TTL: `________________`
  - Tested: YES / NO

---

## Phase 4: Chain Spec Finalization

### Chain Spec Generation & Validation

- [ ] **Chain spec generated and reviewed**
  - File: `chain-spec-mainnet.json`
  - Size: `________________` bytes
  - Format valid: YES / NO
  - Generated by: _______________________
  - Date: _______________________

- [ ] **Chain spec immutably stored**
  - Git commit: `________________`
  - Git tag: `v-mainnet-genesis`
  - Signed (if applicable): YES / NO
  - Signature: `________________`

- [ ] **Chain spec can be loaded by node**
  - ✅ `x3-chain-node build-spec --chain mainnet` succeeds
  - ✅ No parsing errors
  - ✅ All validators can load it
  - Tested by: _______________________

### Raw Chain Spec & Hash

- [ ] **Raw chain spec generated**
  - File: `chain-spec-mainnet-raw.json`
  - SHA256: `________________`
  - Stored publicly: YES / NO (location: ________________)
  - Archived: YES / NO

- [ ] **Chain spec hash announced publicly**
  - Hash: `________________`
  - Announced via: ________________ (e.g., Twitter, governance post)
  - Date announced: _______________________
  - All validators acknowledge: YES / NO

---

## Phase 5: Runtime & Wasm Verification

### Runtime Build & Hash

- [ ] **Production runtime built and verified**
  - Build command: `cargo build -p x3-chain-node --release`
  - Build successful: YES / NO
  - Build time: `________________` minutes
  - Built by: _______________________

- [ ] **Runtime WASM blob extracted**
  - Wasm file: `target/release/wbuild/x3-chain-node/x3_chain_node.wasm`
  - Size: `________________` bytes
  - SHA256: `________________`
  - Archived: YES / NO

- [ ] **Runtime WASM hash in genesis**
  - Hash in spec: `________________`
  - Matches blob: YES / NO
  - Verified by: _______________________

- [ ] **No debug symbols in production WASM**
  - ✅ Built with `--release`
  - ✅ No `--debug` flags
  - Verified by: _______________________

---

## Phase 6: Pre-Launch Testing

### Genesis Block Dry Run

- [ ] **Genesis state initializes without error**
  - ✅ `x3-chain-node build-spec --chain mainnet --raw` succeeds
  - ✅ Chain spec size reasonable (< 50MB)
  - ✅ No panics during state initialization
  - Tested by: _______________________

- [ ] **First block produces**
  - ✅ Single validator produces block 1
  - ✅ Block time correct
  - ✅ Finalization works
  - Tested by: _______________________

### Multi-Validator Dry Run

- [ ] **Multi-validator genesis tested**
  - ✅ Run: `./launch-gates/multi-node-testnet-proof.sh`
  - ✅ Result: PASS
  - ✅ All validators sync
  - ✅ Blocks finalize
  - Test date: _______________________

### RPC & Node Startup

- [ ] **RPC endpoints functional**
  - ✅ `chain_getBlock` works
  - ✅ `state_getMetadata` works
  - ✅ `author_submitExtrinsic` works
  - ✅ `system_health` responds
  - Tested by: _______________________

- [ ] **Node restart preserves state**
  - ✅ Start node, produce blocks, stop
  - ✅ Restart node, verify state intact
  - ✅ No corruption or rollback
  - Tested by: _______________________

---

## Phase 7: Security & Operational Readiness

### Key Management

- [ ] **All private keys generated offline**
  - Validator keys: air-gapped machine
  - Session keys: secure vault
  - Governance keys: multisig or custody
  - Verified by: _______________________

- [ ] **No private keys committed to git**
  - ✅ Git scanned for secrets
  - ✅ Chain spec contains only public data
  - ✅ No seed phrases in config files
  - Scanned by: _______________________

- [ ] **Key backup verified**
  - Validator keys backed up: YES / NO (location: ________________)
  - Recovery tested: YES / NO
  - Date tested: _______________________

### Monitoring & Alerts

- [ ] **Monitoring deployed**
  - [ ] Prometheus metrics scraping
  - [ ] Alert rules configured for:
    - [ ] Block production stalls (> 2 minutes)
    - [ ] Finality stops (> 5 minutes)
    - [ ] Network peer count drops
    - [ ] RPC errors spike
  - Dashboard URL: `________________`
  - Tested by: _______________________

- [ ] **Incident response playbook ready**
  - Location: `docs/incident-response.md`
  - Reviewed by: _______________________
  - All responders trained: YES / NO

### Telemetry & Analytics

- [ ] **Telemetry endpoint configured**
  - Endpoint: `________________`
  - Public dashboard: YES / NO (URL: ________________)
  - Data retention: `________________` days
  - Tested by: _______________________

---

## Phase 8: Communication & Announcement

### Public Announcement

- [ ] **Launch announcement prepared**
  - Blog post: YES / NO
  - Twitter thread: YES / NO
  - Discord/Community notification: YES / NO
  - Media outreach: YES / NO
  - Launch time: `_______________________` (UTC)

- [ ] **Chain ID & network details announced**
  - Chain ID in announcement: YES / NO
  - Bootnodes list provided: YES / NO
  - Explorer URL provided: YES / NO

### Documentation

- [ ] **Validator onboarding guide ready**
  - File: `docs/validator-onboarding.md`
  - Tested by external validator: YES / NO
  - Feedback incorporated: YES / NO

- [ ] **Network upgrade docs prepared**
  - Runtime upgrade procedure: YES / NO
  - Rollback procedure: YES / NO
  - Known issues documented: YES / NO

---

## Phase 9: Final Sign-Off

### Launch Committee Approvals

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Chain Architect | _______________ | _______________ | _______________ |
| Security Lead | _______________ | _______________ | _______________ |
| Validator Lead | _______________ | _______________ | _______________ |
| Operations Lead | _______________ | _______________ | _______________ |

### Final Checkpoint

- [ ] **All above items verified: ✅ YES / ❌ NO**
- [ ] **P0 blockers addressed: ✅ YES / ❌ NO**
- [ ] **Emergency halt procedure tested: ✅ YES / ❌ NO**
- [ ] **Rollback procedure tested: ✅ YES / ❌ NO**

**FINAL DECISION:**

```
GENESIS APPROVED FOR LAUNCH: [ ] YES  [ ] NO

Launch window: _________________ (UTC)
Genesis block timestamp: _________________ (UTC)

Approved by: _________________ 
Date: _______________________
```

---

## Post-Genesis Checklist

(After genesis block is cut)

- [ ] Genesis block produced at scheduled time
- [ ] Validators joined and produced blocks
- [ ] RPC responding to queries
- [ ] Block explorers showing data
- [ ] No emergency halt required in first 24h
- [ ] All metrics within expected ranges
- [ ] Community successfully joining network

---

**Document History:**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-04-26 | X3 Team | Initial genesis checklist |

**Next Review Date:** After each dry run or significant configuration change
