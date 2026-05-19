# X3 Coin Layer + Multi-Chain Mirror Proofs

## Purpose
Define the canonical X3 coin, the mirror proof system across EVM/SVM/BTC, and the X3‑Lang primitives used for atomic swaps and batch execution.

## Canonical Parameters
- Asset ID: **1000**
- Symbol: **X3**
- Decimals: **18**
- Total Supply: **8,888,888,888 X3**

## Genesis Allocations
- Treasury: **25%** (2,222,222,222 X3)
- Validators / Staking: **20%** (1,777,777,778 X3)
- Ecosystem / Grants: **20%** (1,777,777,778 X3)
- Presale / Early Investors: **15%** (1,333,333,333 X3)
- Bonus Pool: **10%** (888,888,889 X3)
  - GPU / Hardware Contributors: **40%**
  - Validators / Staking Spot Buyers: **20%**
  - Presale / Traders: **10%**
  - Auditors / Bug Hunters: **30%**
- Team / Core Contributors: **10%** (888,888,888 X3)

## Recipient Accounts (Testnet)
The following SS58 addresses are **testnet-only**. Do not reuse these seeds for mainnet.

- Treasury multisig (4-of-7): `5Gm3cbEyVgrzTJghqvePTFkfP5Gyw8JxpQPv73r2ycWjc7DL`
- Treasury signers (7): `5EaCLaiQm83XjbtfC9X75HFvBdiGJqBwLG8L696gwAvyWKph`, `5ERSgzasRe7oDHHCFbJf6fYH3bk7dD3SjzEqfJgPNzZbPhKh`, `5CamMYFngjHg3Dw3u57aTnAP2wBSSWnhLKSdXYAYTzMApMXU`, `5FRtvbownp9gQ3x2UPsEvS7kmxiEk8eGtfJ7BcfxMrKbtfv1`, `5HEx3QZCkdQVfsrUasg36ZvcoXsvs2tcsCN9PrHHBraipXRw`, `5HHLEct2KxZnYnr8svLZPwGpnkgk5LytKuAWwfxyQ2DvPWm5`, `5CtNJ4eqvM9m84eZCFKTxktJTTCfU1kVkTPXr4oHyASJ2gge`
- Validators / Staking: `5DCNYZ4PPFZyCKZZkqFn5oiWd78zRAyG41L6SzkugAVE5uHV`
- Ecosystem / Grants: `5G472MxRo1XGtWudyXtq2oh9uKGACRqFqv2QoH2ygTnNoDBR`
- Presale / Early Investors: `5DPXVzc36fT9sEtHLb4Xo5e28xCVAx5exytFLxPAwrKbLBfL`
- Bonus Pool: `5CqSUtcWaxen8F5Q4XPhDPzFmPqVRyuPKMzrvnw932k2bdcT`
- Team / Core Contributors: `5CzThhqqAD52z4qzdZE1Gk9DgfpnzoZfYZBNLSZcukjbJQep`

## Mirror Proof System
- Threshold scheme: **BLS aggregation**
- Quorum: **2/3 of active signer set**
- Signer set: **Dedicated pallet** with activation + rotation controls
- Proof domain separation: `{x3_chain_id, mirror_chain_id, nonce, intent_hash}`
- Replay protection: **proof hash registry + per‑domain nonce**

### Target EVM Chains
- Ethereum Mainnet: **1**
- BSC: **56**
- Polygon: **137**
- Base: **8453**

### Optional (future)
- SVM-Test
- BTC-L1

## Stress-Test Parameters
- Target TPS: **2,000,000,000**
- Max batch per validator: **50,000**
- Atomic swap parallelism: **100,000**
- Proof emission frequency: **10ms**
- GPU nodes required: **128** (linear scaling with batch size)

## X3-Lang Primitives (Pseudo)

### deposit_and_mint
Lock canonical X3, generate BLS proof, mint mirror tokens on target chains.

```x3
primitive X3CoinLayer {
    function deposit_and_mint(user_addr: Address, amount: UInt, chain: ChainID) {
        lock_canonical_x3(user_addr, amount)
        let proof = emit_bls_proof(user_addr, amount, chain)
        mirror_mint(chain, user_addr, amount, proof)
    }
}
```

### execute_atomic_swap
Create atomic swap with hashlock unlock across chains.

```x3
primitive AtomicSwap {
    function execute_atomic_swap(src_chain: ChainID, dst_chain: ChainID, token_amount: UInt, recipient: Address) {
        let secret = new_secret()
        let hashlock = sha256(secret)
        lock_mirror(src_chain, token_amount, hashlock)
        lock_mirror(dst_chain, token_amount, hashlock)
        reveal(secret)
        unlock_mirror(src_chain, recipient, token_amount)
        unlock_mirror(dst_chain, recipient, token_amount)
    }
}
```

### batch_execute
Batch multiple actions in one tick and aggregate proofs.

```x3
primitive BatchEngine {
    function batch_execute(batch_list: [Action]) {
        for action in batch_list {
            action.execute()
        }
        emit_batch_proof(batch_list)
    }
}
```

### burn_and_exit
Burn mirror tokens, verify proofs, release canonical X3.

```x3
primitive ExitFlow {
    function burn_and_exit(user_addr: Address, mirror_amount: UInt, chain: ChainID) {
        let burn_proof = mirror_burn(chain, user_addr, mirror_amount)
        verify_proof(burn_proof)
        release_canonical_x3(user_addr, mirror_amount)
    }
}
```

### stress_test_loop
Simulate maximum TPS throughput.

```x3
primitive StressTest {
    function stress_test_loop(batch_list: [Action]) {
        while true {
            batch_execute(batch_list)
            emit_proofs_every(10ms)
        }
    }
}
```

## Integration Notes
- Wire X3‑Lang primitives into Substrate runtime entry points.
- Plug GPU swarm / validator nodes into BLS signer set for proof aggregation.
- Mirror proof aggregation uses BLS; enforce 2/3 quorum.
- Batch execution uses max batch + parallelism settings for stress tests.
- Dashboard metrics should include TPS, proof counts, active swaps, and node health.

## Developer Example
A single program that deposits, swaps, batches, and emits proofs:

```x3
program multi_chain_swap(sender: Address, recipient: Address, amount: UInt) {
    X3CoinLayer.deposit_and_mint(sender, amount, EVM)
    AtomicSwap.execute_atomic_swap(EVM, SVM, amount, recipient)
    BatchEngine.batch_execute([/* actions */])
    ExitFlow.burn_and_exit(recipient, amount, EVM)
}
```
