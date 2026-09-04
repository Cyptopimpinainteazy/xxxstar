# Comit Specification v0.1

> **Status:** Draft – MVP implementation  
> **Audience:** Wallet providers, integrators, runtime engineers

---

## 1. Introduction

The **Comit** (Cross-VM Operation Transaction) is the fundamental transaction primitive for the X3 Chain protocol. Each Comit expresses an atomic intent spanning the Ethereum Virtual Machine (EVM) and the Solana Virtual Machine (SVM). This document formally specifies the Comit structure, serialization formats, signing and verification rules, and the two-phase commit protocol that guarantees atomic dual-VM execution.

Comits are submitted to the X3 Kernel pallet within the X3 Chain runtime. They drive state changes in the canonical ledger, coordinate external execution environments, and provide a uniform interface for cross-VM applications.

---

## 2. Terminology

| Term | Definition |
|------|------------|
| **X3 Kernel** | The runtime pallet responsible for Comit verification, canonical ledger maintenance, and dual-VM coordination. |
| **Canonical Ledger** | Ledger tracking balances for all registered assets across X3 accounts. |
| **Comit Origin** | Substrate account that signs and submits the Comit. |
| **Prepare Root** | Deterministic commitment to the Comit contents used in the prepare phase of the two-phase commit protocol. |
| **Dual-VM** | Combined execution of EVM and SVM payloads that must succeed or fail atomically. |
| **H256** | 256-bit hash encoded in big-endian byte order. |
| **BoundedVec** | SCALE-encoded vector with compile-time length bound. |

---

## 3. Comit Object Overview

A Comit is defined in the runtime as:

```rust
pub struct Comit<AccountId, Balance> {
    pub comit_id: H256,
    pub origin: AccountId,
    pub evm_payload: Vec<u8>,
    pub svm_payload: Vec<u8>,
    pub nonce: u64,
    pub fee: Balance,
    pub prepare_root: H256,
}
```

### 3.1 Field Definitions

| Field | Type | Description | Constraints |
|-------|------|-------------|-------------|
| `comit_id` | `H256` | Globally unique identifier for the Comit. | MUST be unique per Comit; wallets SHOULD derive via `blake2b-256` over signed payload. |
| `origin` | `AccountId` | Substrate account submitting the Comit. | MUST be implicitly derived from signature. |
| `evm_payload` | `Vec<u8>` | ABI-encoded EVM execution payload. | Length ≤ `MaxPayloadLength`; MAY be empty if only SVM execution is needed. |
| `svm_payload` | `Vec<u8>` | Borsh-encoded SVM payload. | Length ≤ `MaxPayloadLength`; MAY be empty if only EVM execution is needed. |
| `nonce` | `u64` | Sequential Comit nonce scoped to the origin account. | MUST equal on-chain `Nonces[origin]`. |
| `fee` | `Balance` | Fee to be debited from canonical ledger. | Denominated in X3 unless configured otherwise. |
| `prepare_root` | `H256` | Commitment to Comit data for the prepare phase. | MUST follow §8.2 computation. |

### 3.2 Payload Schemas (Reference)

While runtime stores payloads as opaque byte vectors, off-chain tooling SHOULD use the structured representations defined in `pallets/x3-kernel/src/types.rs`:

```rust
pub struct EvmPayload {
    pub target: H160,
    pub input: Vec<u8>,
    pub value: u128,
}

pub struct SvmPayload {
    pub program_id: [u8; 32],
    pub accounts: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}
```

Wallets MAY embed these structures prior to encoding (see §5).

---

## 4. Serialization Formats

### 4.1 Canonical Encoding for Hashing & Submission

- **Primary encoding:** SCALE codec.
- **Ordering:** Fields encoded in struct order.
- **Origin:** Not serialized; implied by signature.
- **Fee:** Encoded using runtime `Balance` SCALE representation (default `u128`).

Wallets MUST encode Comits via SCALE when deriving the canonical hash or transmitting to Substrate RPC endpoints (e.g., `author_submitExtrinsic`).

### 4.2 JSON Representation (Human-Friendly)

JSON is RECOMMENDED for application-layer signing requests:

```json
{
  "comitId": "0x6f2c...b3",
  "nonce": 7,
  "fee": "1000000000000",
  "evmPayload": {
    "target": "0x7e57...",
    "input": "0xa9059cbb000000000000...",
    "value": "0"
  },
  "svmPayload": {
    "programId": "7Yb9kZrK1g9Y7eP8cGk3Y4f7VcZsJtCwZq5sYdV1UEq",
    "accounts": [
      "9sdf8FJpX3n48E3TJnH6TqbwXo5oXJ7xjDkYvZax1p7",
      "F79dqP1n9dSc1h4uS4b9rL4zWZq7u5w6m9q6QMk1a9u"
    ],
    "data": "0x03000000ff..."
  }
}
```

- Hex strings MUST be prefixed with `0x`.
- Base58 (Solana-style) addresses SHOULD be accepted for SVM fields but MUST be canonicalized prior to encoding.

### 4.3 CBOR Representation (Compact)

CBOR is RECOMMENDED for bandwidth-constrained clients. The normative map keys are integers:

| Key | Field |
|-----|-------|
| 0 | `comit_id` (byte string, 32 bytes) |
| 1 | `nonce` (unsigned) |
| 2 | `fee` (unsigned) |
| 3 | `evm_payload` (byte string) |
| 4 | `svm_payload` (byte string) |
| 5 | `prepare_root` (byte string, 32 bytes) |

Example diagnostic notation:

```
{
  0: h'6F2C...B3',
  1: 7,
  2: 1000000000000,
  3: h'F8A8...',
  4: h'0300...',
  5: h'9C12...EF'
}
```

Tooling MUST convert JSON → structured payload → SCALE/CBOR for signing and submission.

---

## 5. Signing Process

1. **Canonical Preimage Construction**
   - Construct `ComitPreimage` by SCALE-encoding the tuple  
     `(comit_id, evm_payload, svm_payload, nonce, fee, prepare_root)`.  
     *Note:* `origin` is excluded.
2. **Hashing**
   - Compute `comit_hash = blake2_256(ComitPreimage)`.
3. **Signature**
   - Sign `comit_hash` using the origin's private key.
   - Supported signature schemes mirror Substrate accounts (e.g., Sr25519, Ed25519, ECDSA).
4. **Extrinsic Assembly**
   - Wrap the `submit_comit` call with the signature payload.
   - Include mortality, era, and tip metadata as per standard Substrate signed extrinsics.

Wallets MUST display the decoded payloads before signing. They SHOULD warn users if both payloads are empty.

---

## 6. Verification Requirements

Upon receiving `submit_comit`, the X3 Kernel MUST perform the following checks:

1. **Signature Validation**
   - Verify extrinsic signature to authenticate `origin`.
2. **Nonce Verification**
   - Ensure `nonce == Nonces[origin]`. Reject with `InvalidNonce` otherwise.
3. **Payload Sanity**
   - Reject if both payloads empty (`EmptyPayloads`).
   - Reject if either payload length exceeds `MaxPayloadLength` (`PayloadTooLarge`).
4. **Prepare Root Validation**
   - Invoke §8.2 computation. If non-zero prepare root mismatch occurs, emit `ComitFailed` with `Verification`.
5. **State Transition**
   - Increment stored nonce (`Nonces[origin] += 1`).
   - Emit `ComitSubmitted`.

Runtime implementations MAY extend verification (e.g., static analysis of payloads) but MUST NOT skip the above steps.

---

## 7. Two-Phase Commit Protocol

X3 Chain adopts a two-phase commit (2PC) adapted for dual-VM execution.

### 7.1 Phase 1 – Prepare

- **Initiation:** Upon `ComitSubmitted`, off-chain executors (EVM and SVM coordinators) fetch the Comit and validate payloads.
- **Prepare Root:** Executors recompute `prepare_root` using §8.2. A mismatch MUST halt execution.
- **Resource Locking:** Executors reserve required balances in the canonical ledger via temporary locks (implementation-specific). No ledger mutation occurs yet.
- **Prepare Receipt:** Each VM produces a prepare receipt containing:
  - Comit ID
  - VM-specific state digest
  - Execution intent summary
  - Signature from the executor operator

### 7.2 Phase 2 – Commit / Finalize

- **Condition:** Both VMs report successful prepare receipts referencing identical Comit IDs and prepare root.
- **Ledger Update:** Authorized off-chain worker (or root governance during MVP) calls `update_canonical_balance` with the finalized balances and optional `comit_id`.
- **Event Emission:** `ComitFinalized { comit_id }` signals completion.
- **Failure Handling:** If any VM fails, operators invoke a recovery procedure:
  - Emit `ComitFailed { comit_id, reason }`.
  - Release reserved resources.
  - No ledger mutation occurs.

### 7.3 Atomicity Guarantees

- Comits MUST only be finalized if both VMs succeed.
- Partial executions MUST be rolled back during Prepare.
- Prepare receipts SHOULD be persisted off-chain for auditability.

---

## 8. Cryptographic Commitments

### 8.1 Canonical Comit Hash (For Signatures)

```
ComitPreimage = SCALE(
    comit_id: [u8; 32],
    evm_payload: Vec<u8>,
    svm_payload: Vec<u8>,
    nonce: u64,
    fee: Balance,
    prepare_root: [u8; 32]
)

comit_hash = blake2_256(ComitPreimage)
```

Wallets MUST sign `comit_hash`. Runtimes SHOULD expose this hash for transparency.

### 8.2 Prepare Root Calculation

The prepare root commits to the intent executed across both VMs:

```
PreparePreimage = comit_id || evm_payload || svm_payload || LE(nonce) || SCALE(fee)

prepare_root = blake2_256(PreparePreimage)
```

Notes:

- `||` denotes byte concatenation.
- `LE(nonce)` is the 8-byte little-endian encoding.
- `SCALE(fee)` MUST match the runtime `Balance` SCALE encoding.
- If a wallet cannot compute `prepare_root`, it MUST set it to zero (treated as “no commitment”). In this case the runtime skips verification, but wallets SHOULD warn users.

---

## 9. Asset Registry & Fees (Contextual)

- Assets MUST be registered via `register_asset` before appearing in Comits.
- Fees are settled against the canonical ledger during finalization.
- Executors MAY deduct additional execution costs on external VMs but MUST report them in final ledger updates.

---

## 10. Example Comits

### 10.1 Cross-VM Arbitrage

**Scenario:** Swap X3 → ETH on EVM DEX, swap ETH → X3 on SVM liquidity pool.

```jsonc
{
  "comitId": "0x0fd4...9c2a",
  "nonce": 42,
  "fee": "250000000000",
  "evmPayload": {
    "target": "0x1111111254EEB25477B68fb85Ed929f73A960582",
    "input": "0x38ed1739000000000000000000000000000000000000000000000000000000000000003c...",
    "value": "0"
  },
  "svmPayload": {
    "programId": "Arb1tr4ge111111111111111111111111111111111",
    "accounts": [
      "UserAtlas111111111111111111111111111111111",
      "DexPool1111111111111111111111111111111111"
    ],
    "data": "0x02000000a0860100"
  },
  "prepareRoot": "0xb3c4...ff09"
}
```

- Prepare root calculated per §8.2.
- Executors ensure both swaps succeed before finalizing ledger adjustments.

### 10.2 Canonical Asset Swap (X3 ↔ USDC)

- EVM payload calls X3 DEX router.
- SVM payload empty (pure EVM execution).

```json
{
  "comitId": "0xa1b2...c3d4",
  "nonce": 17,
  "fee": "50000000000",
  "evmPayload": { "...": "..." },
  "svmPayload": "",
  "prepareRoot": "0x0000...0000"
}
```

Runtime accepts zero prepare root. Wallet SHOULD caution user that dual-VM guarantees are bypassed.

### 10.3 Governance Operation

- SVM payload votes on Solana governance program.
- EVM payload triggers mirror governance registry.

```json
{
  "comitId": "0x7fe9...d012",
  "nonce": 5,
  "fee": "1000000000",
  "evmPayload": {
    "target": "0xGovernanceRegistry",
    "input": "0xdeadbeefcafebabe",
    "value": "0"
  },
  "svmPayload": {
    "programId": "Gov111111111111111111111111111111111111111",
    "accounts": ["VoteRecord11111111111111111111111111111111"],
    "data": "0x0100000002"
  },
  "prepareRoot": "0x8c10...44aa"
}
```

---

## 11. Error Handling

X3 Kernel emits `ComitFailed` with `ComitFailureReason`. Wallets SHOULD map reasons to user-friendly messages:

| Reason | User Message |
|--------|--------------|
| `PayloadTooLarge` | “Payload exceeds maximum size.” |
| `EmptyPayloads` | “EVM and SVM payloads cannot both be empty.” |
| `InvalidNonce` | “Nonce mismatch; refresh account state.” |
| `Verification` | “Prepare root mismatch; verify payload integrity.” |

---

## 12. Implementation Guidance

- **Wallets**
  - Track nonces via RPC (`atlasKernel_nonces` future RPC) or events.
  - Surface prepare root verification to users.
  - Support fallback path where prepare root is zero, but mark as *unsafe*.

- **Indexers**
  - Store `ComitSubmitted`, `ComitFinalized`, `ComitFailed` events.
  - Correlate with off-chain prepare receipts.

- **Executors**
  - Maintain deterministic encoding pipelines to ensure prepare root consistency.
  - Provide signed receipts for audit trails.

---

## 13. Future Extensions

- Merkle-based prepare root including witness data.
- Multi-leg Comits spanning >2 VMs.
- Native support for batched Comits with aggregated signatures.
- On-chain automation of finalize phase via trustless verification proofs.

---

## 14. References

- X3 Kernel pallet implementation (`pallets/x3-kernel/src/lib.rs`)
- Payload types (`pallets/x3-kernel/src/types.rs`)
- Runtime integration (`runtime/src/lib.rs`)
- SCALE codec specification
- CBOR RFC 7049