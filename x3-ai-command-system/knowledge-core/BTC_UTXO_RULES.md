# BTC UTXO Rules — Knowledge Core

## Overview

These are the mandatory security rules for all Bitcoin/UTXO integration code in the X3 ecosystem. Bitcoin's UTXO model, Script language, and confirmation-based finality differ fundamentally from account-based VMs. Every BTC integration — bridges, custody, verification, PSBT handling — must comply with these rules. Bitcoin transactions are irreversible after sufficient confirmations; errors are far more costly than on other chains.

## UTXO Model Correctness

### Rule BTC-1: UTXO Model Correctness

The UTXO model requires that:

- **Inputs reference specific UTXOs** by transaction hash and output index (txid:vout). No partial spending.
- **Outputs consume the entire input value.** Change must be explicitly sent back to the sender. Any value not allocated to an output is paid as a fee.
- **Double-spending is prevented by consuming UTXOs.** Once a UTXO is spent, it cannot be spent again.
- **Transaction ordering matters.** A transaction that spends a UTXO created by another transaction must appear after the creating transaction in the block.
- **UTXO set is the authoritative state.** The balance of an address is the sum of all unspent UTXOs controlled by that address.

### Rule BTC-2: No Partial UTXO Spending

- Every transaction input must fully consume the referenced UTXO.
- There is no way to spend "part" of a UTXO. The entire value must be allocated to outputs (including change and fees).
- If a bridge needs to spend a portion of a locked UTXO, it must create a change output back to the custody address.

### Rule BTC-3: UTXO Locking and Unlocking

- Every UTXO has a locking script (scriptPubKey) that defines who can spend it.
- Every transaction input must provide an unlocking script (scriptSig or witness) that satisfies the locking script.
- Never assume a locking script is a simple pay-to-pubkey-hash (P2PKH). It could be P2SH, P2WSH, P2TR, or a custom script.
- Always verify the locking script type before constructing the unlocking script.

## Taproot

### Rule BTC-4: Taproot Key Path and Script Path

Taproot (P2TR) outputs have two spending paths:

1. **Key path**: Spend with a single Schnorr signature. This is the default, most efficient path.
2. **Script path**: Spend with a script that was committed to in the output. The script is revealed when spending.

Rules for Taproot:

- Both paths must be validated. A bridge must know which path is being used and verify it correctly.
- Key path spending must verify the Schnorr signature against the internal public key (or the output public key if no script path exists).
- Script path spending must verify the control block (internal public key + script tree path) and the script execution.
- Taproot outputs must not be created with an insecure script tree. All script path conditions must be reviewed for security.
- The internal public key must be known and documented. Do not accept Taproot outputs with unknown internal keys.

### Rule BTC-5: Taproot Script Tree

- The script tree must be balanced or have a documented structure.
- Every leaf in the script tree must be a valid script that has been reviewed.
- The Merkle path from the leaf to the root must be verifiable.
- Do not create Taproot outputs with more than 128 script path levels (the Bitcoin consensus limit).

## PSBT Validation

### Rule BTC-6: PSBT (Partially Signed Bitcoin Transaction) Validation

PSBTs are the standard format for multi-party transaction construction. Rules:

- **Version**: The PSBT version must be specified. Version 0 and Version 2 have different fields and validation rules.
- **Inputs**: Every input must reference a valid UTXO. The UTXO must be provided in the PSBT (non-witness UTXO for segwit, witness UTXO for legacy).
- **Outputs**: Every output must have a valid scriptPubKey and amount.
- **Signatures**: Signatures must be verified against the correct public key and sighash type.
- **Sighash types**: Must be explicitly specified. Do not default to SIGHASH_ALL without understanding the implications. SIGHASH_NONE, SIGHASH_SINGLE, and SIGHASH_ANYONECANPAY have specific use cases but also specific risks.
- **Fee**: The fee must be calculated as `sum(inputs) - sum(outputs)`. The fee must be within an acceptable range.
- **Locktime**: If the transaction has a locktime, it must be validated against the current block height or time.
- **Change output**: The PSBT must include a change output if the fee is not exactly the desired amount. The change address must be verified.
- **Completeness**: A PSBT must be finalized before broadcasting. All inputs must have valid signatures and witness data.

## Fee Rate Estimation

### Rule BTC-7: Fee Rate Estimation

- Fee rate must be calculated as `fee / vsize` (satoshis per virtual byte).
- Use the current mempool conditions to estimate the required fee rate for timely confirmation.
- Do not use hardcoded fee rates. Fee rates change based on network congestion.
- For time-sensitive transactions (e.g., bridge refunds), use a higher fee rate to prioritize inclusion.
- For low-priority transactions, use a lower fee rate and accept delayed confirmation.
- Fee rate must be validated before signing. A transaction with an excessively high fee rate (>10x the expected rate) may be a sign of a fee attack.

## RBF (Replace-By-Fee)

### Rule BTC-8: RBF Handling

- RBF (BIP 125) allows a transaction to be replaced by a higher-fee version.
- RBF must be explicitly signaled by setting the sequence number to less than 0xffffffff - 1.
- If RBF is signaled, the replacement transaction must:
  - Have a higher fee rate than the original.
  - Pay for its own bandwidth (the replacement fee must cover the additional transactions removed from the mempool).
  - Not introduce new inputs that were not in the original transaction (unless the original signaled opt-in RBF).
- Bridge transactions should signal RBF only if there is a mechanism to handle replacements (e.g., updating the bridge state on X3).
- If a bridge transaction is replaced, the bridge must verify that the replacement transaction spends the same UTXOs and sends to the same destination (or a known change address).

## Dust Limits

### Rule BTC-9: Dust Limits

- The dust limit is the minimum output value that is economically spendable. Below the dust limit, the output value is less than the fee required to spend it.
- Do not create outputs below the dust limit. They will be rejected by most nodes.
- The dust limit depends on the output type (P2PKH, P2SH, P2WSH, P2TR) and the fee rate.
- If a transaction would create a dust output, either increase the output value or remove the output and add its value to the fee.

## Locktime and Sequence

### Rule BTC-10: Locktime and Sequence

- **Locktime (nLockTime)**: The earliest time or block height at which the transaction can be included in a block.
  - If locktime < 500,000,000, it is a block height.
  - If locktime >= 500,000,000, it is a Unix timestamp.
  - If all inputs have sequence == 0xffffffff, the locktime is disabled.
- **Sequence (nSequence)**: Used for relative timelocks (BIP 68) and RBF signaling (BIP 125).
  - If sequence < 0x80000000, relative timelock is enabled.
  - If sequence < 0xffffffff - 1, RBF is signaled.
- Bridge transactions using timelocks must validate locktime and sequence correctly.
- Do not assume locktime is in blocks or seconds without checking the value.

## Finality and Confirmation

### Rule BTC-11: Finality Confirmation

- Bitcoin does not have instant finality. Confirmations represent probabilistic finality.
- **1 confirmation**: The transaction is in a block. ~8% chance of reorg.
- **3 confirmations**: The transaction is 3 blocks deep. Very low chance of reorg for normal transactions.
- **6 confirmations**: The transaction is 6 blocks deep. Standard for high-value transactions. Extremely low chance of reorg.
- **30+ confirmations**: Recommended for very high-value bridge transactions. Near-zero chance of reorg.
- Bridge transactions must not be considered final until they have reached the required confirmation depth:
  - Low value (< 10 BTC): 6 confirmations.
  - Medium value (10-100 BTC): 12 confirmations.
  - High value (> 100 BTC): 30+ confirmations.
- These confirmation depths are minimums. Higher values may be appropriate for risk-averse applications.

## Bridge Custody Proofs

### Rule BTC-12: Bridge Custody Proofs

- Every BTC locked in a bridge custody address must have a verifiable proof:
  - **SPV proof**: A Merkle proof that the transaction is included in a Bitcoin block. The block header must be known to X3.
  - **UTXO proof**: A proof that the UTXO is unspent. This requires monitoring the Bitcoin chain for spends.
- Custody addresses must be multi-sig (minimum 3-of-5 or equivalent threshold scheme).
- Custody key rotations must be documented and verified on-chain.
- Custody address scripts must be reviewed for security (no backdoors, no timelocked drains, no hidden conditions).
- UTXO set must be monitored for spends. If a custody UTXO is spent without a corresponding bridge release, that is a critical security event.

## Testing Requirements

### Rule BTC-13: Testing Requirements

Every BTC integration must have:

- **Unit tests** for all UTXO handling, script validation, and fee calculation.
- **Integration tests** for PSBT creation, signing, and broadcasting.
- **Fuzz tests** for script interpretation and transaction parsing.
- **Scenario tests** for edge cases: reorgs, RBF replacements, dust outputs, high-fee mempools.
- **End-to-end tests** for bridge lock, release, and refund flows on testnet.
- **Finality tests** that simulate varying confirmation depths.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the canonical supply invariant. BTC locked in custody is the `btc_locked` term.
- **UNIVERSAL_ASSET_KERNEL.md** — BTC locks must be accounted for in the UAK. No unbacked mints.
- **CROSS_VM_ROUTING.md** — BTC bridges are cross-chain routes with delayed finality.
- **TRADING_SAFETY_KERNEL.md** — BTC bridge operations must comply with trading safety rules.
- **MAINNET_READINESS.md** — BTC custody must be audited and verified before mainnet deployment.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*