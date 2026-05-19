# X3 Cross-Chain Atomic Swap Architecture (YOLO Diagram)

## Purpose
This document defines the canonical cross‑chain atomic swap flow across X3VM, EVM, SVM, and BTC. It is a deterministic, proof‑gated mirror system where X3VM is the source of truth and external chains reflect proof‑authorized state only.

## YOLO Diagram

```
+--------------------------------------------------------------------------------+
|                               X3VM Canonical Ledger                             |
|--------------------------------------------------------------------------------|
| 1. Internal Ledger (canonical X3 balances)                                     |
| 2. Atomic Swap Coordinator                                                      |
|    - Receives swap intents                                                      |
|    - Locks canonical X3                                                         |
|    - Computes hashlock H = SHA256(S)                                            |
|    - Manages timeout graph (BTC > EVM > SVM > X3VM)                             |
| 3. Batch Execution Engine                                                       |
|    - Batch multiple swaps per tick                                              |
|    - Generates VM proofs                                                        |
| 4. Native Opcodes                                                               |
|    - HASHLOCK_UNLOCK(preimage, tradeID)                                         |
|    - PROOF_EMIT(tradeID, chainID, preimage)                                     |
|    - X3MT_MINT/BURN (mirror token mint/burn tied to proofs)                     |
+--------------------------------------------------------------------------------+
           |                         |                        |
           | VM Proofs (serialized)  |                        |
           v                         v                        v
+--------------------+     +--------------------+     +--------------------+
|      EVM Chain     |     |      SVM Chain     |     |       BTC Chain    |
|--------------------|     |-------------------|     |-------------------|
| X3 Mirror Token    |     | PDA Escrow Program |     | HTLC Script       |
| Contract           |     | Lock Escrow Funds  |     | Lock native BTC   |
|--------------------|     |-------------------|     |-------------------|
| - Mint via proof   |     | - Lock via proof   |     | - Lock via hash H |
| - Burn → release   |     | - Unlock via S     |     | - Unlock via S    |
| - Atomic swap legs |     | - Atomic swap legs |     | - Atomic swap legs|
| - Timeout enforced |     | - Timeout enforced |     | - Timeout enforced|
+--------------------+     +-------------------+     +-------------------+

                               ^
                               |
                               | Optional Validator / Relayer Transport
                               | (threshold-signed attestations, slashable)
                               v

+--------------------------------------------------------------------------------+
|                         Arbitrage / Mirror Token Loops                          |
|--------------------------------------------------------------------------------|
| 1. Entry: mint X3MT from X3VM proof → trade on DEX/AMM                          |
| 2. Arbitrage: swap X3MT ↔ other assets, cross-chain if needed                   |
| 3. Exit: burn X3MT → proof back to X3VM → canonical X3 released                 |
| 4. Batch multiple trades → single proof → unlocks across EVM/SVM/BTC in parallel|
+--------------------------------------------------------------------------------+
```

## Key Properties
- X3VM is canonical: all supply changes happen in the runtime.
- Mirror tokens are proof‑gated: no wrapping, custody, or operator minting.
- Hashlocks + timeouts enforce atomicity across chains.
- Batch proofs decouple internal TPS from external finality latency.

## Deterministic Proof Format
All proofs are deterministic and domain‑separated by chain.

```
Proof {
  trade_id: bytes32
  mirror_chain_id: bytes32
  action: enum { MINT, BURN, UNLOCK, REFUND }
  amount: u128
  recipient: bytes
  preimage_hash: bytes32
  nonce: u64
  signature: bytes   // threshold signature over proof_hash
}

proof_hash = SHA256(encode_canonical(ProofWithoutSignature))
```

Rules:
- `encode_canonical` is fixed byte order, fixed field order, no optional ambiguity.
- `nonce` is monotonic per mirror domain.
- `signature` is aggregated threshold signature over `proof_hash`.

## X3-Lang Templates (Pseudo)

### X3 Mirror Token Primitive
```x3
primitive X3MT {
    function mintMirror(user: Address, amount: UInt, proof: Bytes) {
        assert verifyProof(proof, user, amount)
        ledger[user] += amount
        emitProofMint(user, amount, proof)
    }

    function burnMirror(user: Address, amount: UInt) {
        assert ledger[user] >= amount
        ledger[user] -= amount
        emitProofBurn(user, amount)
    }
}
```

### Atomic Swap Primitive
```x3
primitive AtomicSwap {
    struct Leg {
        chain: ChainID
        recipient: Address
        amount: UInt
        timeout: Time
    }

    function createSwap(legs: [Leg], hashlock: Bytes32) {
        for leg in legs {
            lockFunds(leg.chain, leg.recipient, leg.amount, hashlock, leg.timeout)
        }
    }

    function executeSwap(preimage: Bytes) {
        for leg in legs {
            assert sha256(preimage) == hashlock
            releaseFunds(leg.chain, leg.recipient, leg.amount)
        }
        emitProofSwap(legs, preimage)
    }
}
```

### Batch Execution
```x3
primitive SwapBatch {
    swaps: [AtomicSwap]

    function executeBatch(preimages: [Bytes]) {
        for i in 0..swaps.length {
            swaps[i].executeSwap(preimages[i])
        }
        emitBatchProof(swaps, preimages)
    }
}
```

## Runnable Scaffold
A deterministic, in‑memory scaffold is provided to exercise the multi‑chain flow across X3VM → EVM/SVM/BTC.

Run:
```
python3 scripts/x3_atomic_swap_scaffold.py
```

The scaffold demonstrates:
- Canonical X3 lock
- Proof‑gated mirror mint across EVM/SVM/BTC
- Hashlock‑based unlock
- Mirror burn → canonical release
- Batch processing for multiple swaps

See `scripts/x3_atomic_swap_scaffold.py` for the runnable flow.

## Related Docs
- `docs/X3_COIN_LAYER_SPEC.md`
- `x3-lang/examples/x3_coin_layer.x3`
