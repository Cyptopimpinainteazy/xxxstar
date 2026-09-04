# PRIV-ENCLAVE-003 — Embedded Private Execution Environments

| Field       | Value |
|-------------|-------|
| **ID**      | PRIV-ENCLAVE-003 |
| **Status**  | DRAFT |
| **Authors** | X3 Core Team |
| **Created** | 2026-02-13 |
| **Priority** | P1 — Narrative dominance + MEV protection |

---

## Summary

Add a native "private execution mode" where users route transactions through
encrypted mempools that are decrypted and executed only inside trusted GPU
enclaves (NVIDIA Confidential Computing / AMD SEV-SNP). Execution results
are committed as encrypted state diffs with optional ZK proofs to the public
chain. This creates a built-in L1-speed dark pool for DeFi — combining
Solana-level throughput with privacy that rollups promise but deliver slowly.

---

## Motivation

### Problem

- Public mempools at 2.7M TPS create an **industrial-scale MEV extraction
  opportunity**. Front-running bots can observe, simulate, and sandwich
  transactions in sub-millisecond timeframes.
- Users lose value on every trade — estimated 1–5 bps per swap to MEV.
- Existing solutions (Flashbots Protect, MEV-Share, encrypted mempools on
  rollups) add latency, centralization, or both.
- No L1 currently offers private execution at native speed.

### Opportunity

- Modern NVIDIA GPUs (H100, B200) support **Confidential Computing** —
  hardware-enforced memory encryption with attestable enclaves.
- X3 validators already run these GPUs; CC mode is a firmware toggle.
- Regional finality keeps latency low even with enclave overhead (~1ms).
- Premium fees for private execution create a new revenue stream.

### Market Position

| Feature | X3 (with this proposal) | Solana | Monad | Rollups |
|---------|---------------------------|--------|-------|---------|
| Speed | 2.7M TPS | 65K TPS | Unknown | ~1K TPS |
| Parallel execution | Yes (GPU) | Yes (Sealevel) | Yes (optimistic) | Limited |
| MEV protection | Native private mode | None | None | Partial (sequencer) |
| Private execution | L1, ~1ms overhead | No | No | Via ZK rollup (~minutes) |

---

## Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         X3 Node                                   │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Public Pipeline                           │   │
│  │  Mempool → SigVerify → PoH → GPU Execute → State Commit    │   │
│  └────────────────────────┬────────────────────────────────────┘   │
│                           │                                         │
│  ┌────────────────────────▼────────────────────────────────────┐   │
│  │                  Private Pipeline                            │   │
│  │                                                              │   │
│  │  ┌───────────┐  ┌───────────────┐  ┌────────────────────┐  │   │
│  │  │ Encrypted │  │ Decrypt in    │  │ Execute in         │  │   │
│  │  │ Mempool   │──│ Enclave only  │──│ Confidential GPU   │  │   │
│  │  │ (E2E enc) │  │ (attestation) │  │ (NVIDIA CC / SEV)  │  │   │
│  │  └───────────┘  └───────────────┘  └─────────┬──────────┘  │   │
│  │                                               │              │   │
│  │                                    ┌──────────▼──────────┐  │   │
│  │                                    │ Encrypted State Diff│  │   │
│  │                                    │ + Optional ZK Proof │  │   │
│  │                                    └──────────┬──────────┘  │   │
│  │                                               │              │   │
│  └───────────────────────────────────────────────┘              │   │
│                                                  │               │   │
│  ┌───────────────────────────────────────────────▼──────────┐   │
│  │  pallet-private-execution (Substrate)                     │   │
│  │  - Private TX registration + fee collection               │   │
│  │  - Enclave attestation registry                           │   │
│  │  - Encrypted state diff commitment                        │   │
│  │  - ZK proof verification (optional)                       │   │
│  │  - Confidential validator set management                  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. `pallet-private-execution` (New Substrate Pallet)

```rust
// pallets/private-execution/src/lib.rs (sketch)

#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type Currency: ReservableCurrency<Self::AccountId>;

    /// Premium fee multiplier for private execution (e.g., 1.5x = 150%)
    type PrivateFeePremiumBps: Get<u16>;  // 150 = 1.5%

    /// Minimum number of confidential validators to accept a private TX
    type MinConfidentialQuorum: Get<u32>; // 3

    /// ZK verifier for proof validation
    type ZkVerifier: ZkProofVerifier;

    /// Attestation verifier for GPU enclave reports
    type AttestationVerifier: EnclaveAttestationVerifier;
}

#[pallet::storage]
pub type ConfidentialValidators<T> = StorageMap<
    _, Blake2_128Concat, T::AccountId, EnclaveAttestation<T>
>;

#[pallet::storage]
pub type PrivateTransactions<T> = StorageMap<
    _, Blake2_128Concat, TxHash, PrivateTxRecord<T>
>;

#[pallet::storage]
pub type EncryptedStateDiffs<T> = StorageMap<
    _, Blake2_128Concat, BlockNumberFor<T>, BoundedVec<EncryptedDiff, MaxDiffsPerBlock>
>;
```

**Key types:**

```rust
pub struct EnclaveAttestation<T: Config> {
    pub validator: T::AccountId,
    pub gpu_model: BoundedVec<u8, MaxGpuModelLen>,
    pub attestation_report: BoundedVec<u8, MaxAttestationLen>,  // NVIDIA CC attestation blob
    pub public_key: [u8; 32],          // enclave's ephemeral encryption key
    pub last_refreshed: BlockNumberFor<T>,
    pub status: EnclaveStatus,
}

pub enum EnclaveStatus {
    Verified,       // attestation valid, accepting private TXs
    Expired,        // attestation needs refresh
    Revoked,        // failed attestation check
}

pub struct PrivateTxRecord<T: Config> {
    pub tx_hash: H256,
    pub sender: T::AccountId,              // can be pseudonymous
    pub encrypted_payload: BoundedVec<u8, MaxPayloadLen>,
    pub assigned_enclaves: BoundedVec<T::AccountId, MaxEnclaves>,
    pub fee_paid: BalanceOf<T>,
    pub status: PrivateTxStatus,
    pub submitted_at: BlockNumberFor<T>,
}

pub enum PrivateTxStatus {
    Pending,        // in encrypted mempool
    Executing,      // inside enclave
    Committed,      // state diff on chain
    Verified,       // ZK proof verified (if applicable)
    Failed,         // execution failed inside enclave
}

pub struct EncryptedDiff {
    pub tx_hash: H256,
    pub encrypted_state_changes: Vec<u8>,   // encrypted to chain key
    pub commitment: H256,                   // Pedersen commitment to plaintext diff
    pub zk_proof: Option<Vec<u8>>,          // optional validity proof
    pub enclave_signature: [u8; 64],        // signed by enclave attestation key
}
```

#### 2. Encrypted Mempool (`crates/private-mempool/`)

New crate managing the encrypted transaction queue:

```rust
pub struct EncryptedMempool {
    /// Transactions encrypted to the enclave committee's threshold key
    queue: BTreeMap<TxHash, EncryptedTransaction>,
    /// Threshold encryption public key (DKG among confidential validators)
    committee_pubkey: ThresholdPublicKey,
    /// Maximum queue depth
    max_depth: usize,
}

pub struct EncryptedTransaction {
    pub ciphertext: Vec<u8>,           // AES-256-GCM encrypted TX
    pub ephemeral_pubkey: [u8; 32],    // sender's ephemeral key for ECDH
    pub nonce: [u8; 12],
    pub fee_commitment: H256,          // committed fee (visible for ordering)
    pub priority_fee: u64,             // visible for block inclusion ordering
    pub submitted_at: Instant,
}
```

**Properties:**
- TX content is opaque to validators until decrypted inside enclave.
- Fee commitment is visible so block builders can order by fee.
- Priority fee is in the clear — prevents free-loading.
- Threshold encryption ensures no single validator can decrypt alone.

#### 3. GPU Confidential Execution Runtime (`crates/confidential-gpu/`)

Interfaces with NVIDIA Confidential Computing:

```rust
pub struct ConfidentialGpuRuntime {
    /// Handle to the trusted execution environment
    enclave: NvTrustHandle,
    /// Attestation keypair generated inside enclave
    attestation_key: AttestationKeypair,
    /// Decryption share for threshold decryption
    dkg_share: DkgShare,
}

impl ConfidentialGpuRuntime {
    /// Decrypt and execute a private transaction inside the enclave.
    /// Returns encrypted state diff + optional ZK proof.
    pub fn execute_private(
        &self,
        encrypted_tx: &EncryptedTransaction,
        current_state: &StateSnapshot,
    ) -> Result<EncryptedDiff, PrivateExecutionError> {
        // 1. Threshold decrypt TX inside enclave
        let plaintext_tx = self.threshold_decrypt(encrypted_tx)?;

        // 2. Execute against state snapshot (inside CC memory)
        let state_diff = self.execute_in_enclave(&plaintext_tx, current_state)?;

        // 3. Generate commitment to state diff
        let commitment = pedersen_commit(&state_diff);

        // 4. Optionally generate ZK proof of valid execution
        let zk_proof = if self.config.generate_zk_proofs {
            Some(self.generate_validity_proof(&plaintext_tx, &state_diff)?)
        } else {
            None
        };

        // 5. Encrypt state diff to chain key
        let encrypted_diff = self.encrypt_diff(&state_diff)?;

        // 6. Sign with attestation key
        let signature = self.attestation_key.sign(&encrypted_diff);

        Ok(EncryptedDiff {
            tx_hash: plaintext_tx.hash(),
            encrypted_state_changes: encrypted_diff,
            commitment,
            zk_proof,
            enclave_signature: signature,
        })
    }
}
```

#### 4. Distributed Key Generation (DKG)

Confidential validators perform DKG to establish a threshold encryption key:

- **Threshold**: t-of-n where t = `MinConfidentialQuorum` (default 3) and
  n = total confidential validators.
- **Protocol**: Pedersen DKG (well-studied, post-quantum upgradeable).
- **Key rotation**: every epoch (configurable, default 1 hour).
- **Implementation**: extend existing `crates/quantum-crypto/` or use
  `threshold-crypto` crate.

#### 5. Fee Model

```
Private TX fee = base_fee × (1 + PrivateFeePremiumBps / 10_000)
```

Default premium: 1.5% (150 bps). Split:
- 60% to confidential validators (higher than normal — they run CC hardware)
- 25% burned
- 15% to stakers

Premium is market-driven — governance can adjust based on demand.

---

## Integration Points

| Existing Component | Change Required |
|---|---|
| `pallets/x3-kernel/` | Route private TXs to confidential pipeline |
| `crates/x3-vm/` | Add `execute_confidential` variant |
| `crates/gpu-swarm/src/gpu_backends/` | Add `ConfidentialCuda` backend |
| `crates/gpu-swarm/src/warden/` | Add `ComputeLane::Confidential` |
| `crates/quantum-crypto/` | Threshold encryption + DKG |
| `node/src/rpc.rs` | `submit_private_transaction` RPC method |
| `runtime/` | Add `pallet-private-execution` to runtime |
| `packages/ts-sdk/` | Client-side encryption helpers |
| `contracts/` | Solidity interface for private execution requests |

---

## Invariants

```toml
[[invariant]]
id = "PRIV-EXEC-001"
description = "Private TX content is never exposed in plaintext outside the enclave"
severity = "CRITICAL"
layer = "CONSENSUS"
tested_by = ["tests/private_execution/test_confidentiality.rs::no_plaintext_leak"]
property = "forall private_tx: plaintext only inside attested enclave memory"

[[invariant]]
id = "PRIV-EXEC-002"
description = "Encrypted state diff when applied produces identical state to public execution of same TX"
severity = "CRITICAL"
layer = "VM"
tested_by = ["tests/private_execution/test_correctness.rs::private_matches_public"]
property = "apply(decrypt(encrypted_diff)) == apply(public_execute(tx))"

[[invariant]]
id = "PRIV-EXEC-003"
description = "No single validator can decrypt a private TX alone (threshold t-of-n required)"
severity = "CRITICAL"
layer = "CONSENSUS"
tested_by = ["crates/confidential-gpu/tests/threshold.rs::single_share_insufficient"]
property = "decrypt(ciphertext, [share]) fails when |[share]| < t"

[[invariant]]
id = "PRIV-EXEC-004"
description = "Enclave attestation is verified before validator joins confidential set"
severity = "CRITICAL"
layer = "CONSENSUS"
tested_by = ["pallets/private-execution/src/tests.rs::reject_unattested"]
property = "register_confidential(validator) requires valid_attestation(validator)"

[[invariant]]
id = "PRIV-EXEC-005"
description = "Private execution premium fee is correctly collected and split"
severity = "HIGH"
layer = "PALLET"
tested_by = ["pallets/private-execution/src/tests.rs::fee_premium_accounting"]
property = "fee_collected == base_fee * (1 + premium_bps / 10000)"

[[invariant]]
id = "PRIV-EXEC-006"
description = "Regional finality latency increase from private execution is ≤1ms"
severity = "HIGH"
layer = "CONSENSUS"
tested_by = ["tests/private_execution/benches/latency.rs::finality_overhead"]
property = "finality_latency(private) - finality_latency(public) <= 1ms"
```

---

## Testing Strategy

| Phase | Scope | Method |
|-------|-------|--------|
| Unit | Pallet logic (attestation, fees, state diffs) | `cargo test -p pallet-private-execution` |
| Unit | Threshold encryption/decryption | Property tests with various t/n configs |
| Unit | Encrypted mempool ordering | Ensure fee-based ordering with opaque content |
| Integration | DKG key generation + rotation | Multi-node test with 5+ validators |
| Integration | Enclave execution → state diff → commit | NVIDIA CC dev environment |
| E2E | Full private TX lifecycle | Testnet with 3+ confidential validators |
| Security | Enclave escape, side-channel, timing | External security audit |
| Benchmark | Latency overhead vs public execution | A/B comparison on testnet |
| Adversarial | Colluding validators, malformed attestations | Formal verification of threshold properties |

---

## Rollout Plan

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **Phase 1** | 3 weeks | `pallet-private-execution` with attestation registry; `crates/private-mempool` |
| **Phase 2** | 3 weeks | `crates/confidential-gpu` with NVIDIA CC integration; DKG protocol |
| **Phase 3** | 2 weeks | Threshold encryption end-to-end; encrypted state diffs |
| **Phase 4** | 2 weeks | ZK proof generation (optional path); fee model |
| **Phase 5** | 2 weeks | E2E integration; TS SDK client encryption; RPC methods |
| **Phase 6** | 2 weeks | Security audit; testnet deployment; CC hardware provisioning guide |

Total: **~14 weeks** to testnet-ready. (Can be parallelized with DEPIN-GPU-001.)

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| NVIDIA CC side-channel vulnerabilities | Low | Critical | Defense in depth: encrypt outputs even inside enclave; security audit |
| Limited CC-capable GPU availability | Medium | High | Support AMD SEV-SNP as fallback; CC is standard on H100+/B200+ |
| DKG liveness failure (not enough participants) | Medium | High | Fallback to optimistic execution with post-hoc privacy (commit-reveal) |
| Regulatory opposition to private execution | Medium | High | Optional per-jurisdiction; compliance mode with auditor keys |
| Higher hardware requirements exclude small validators | Medium | Medium | Tiered: CC validators earn premium; non-CC validators run public pipeline |
| Performance overhead exceeds 1ms | Low | Medium | Profile on target hardware; optimize DKG with precomputation |

---

## Open Questions

- [ ] Should ZK proofs be mandatory or optional? (Propose: optional initially,
      mandatory after proving system is production-ready.)
- [ ] Which ZK proving system? (Propose: Groth16 for speed, upgradeable to Plonk/Halo2.)
- [ ] How to handle private TX that interacts with public state?
      (Propose: read public state freely; write to private state partition;
      public effects committed as encrypted diffs.)
- [ ] Should there be a "reveal after N blocks" policy for transparency?
      (Propose: configurable per-TX; default reveal after 100 blocks.)
- [ ] How does this interact with the cross-chain GPU validator?
      (Propose: cross-chain private atomic swaps in Phase 2.)
- [ ] What's the minimum number of confidential validators for mainnet?
      (Propose: at least 7 for t=3, n=7 threshold.)
