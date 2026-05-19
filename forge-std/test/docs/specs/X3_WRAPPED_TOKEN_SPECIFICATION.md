# X3 Wrapped Token Specification v1

## Placement in go-mode sequence

This spec defines the omnichain asset portability layer that sits after [Phase 4.5 liquidity backbone](X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md). It is Phase 5 of [GO_MODE_EXECUTION_ORDER.md](../../GO_MODE_EXECUTION_ORDER.md).

This phase assumes:
- ✅ Constitutional controls and canonical truth doctrine (Phase 0)
- ✅ Accounting spine and fee routing (Phase 3)
- ✅ Signer architecture and custody boundaries (Phase 3.5)
- ✅ RPC strategy and health routing (Phase 4)
- ✅ Liquidity, inventory, solvency machinery (Phase 4.5)

## Purpose

Make X3 a single governed asset with many chain representations and zero accounting ambiguity. Users should see one X3 token with one governance power mapping, one supply total, and one reconciliation ledger—regardless of which chain they are on or which wrapped representation they hold.

The platform becomes portable across chains only after wrapped tokens, bridge mechanics, governance power distribution, supply reconciliation, and failure recovery are all defined and tested together.

## Operating doctrine

**One token, many chains.**
- A single canonical X3 supply exists (aggregate across all chains)
- Each chain carries a wrapped representation pegged to that canonical supply
- Governance power is registered per wrapped token holder irrespective of chain
- Supply is reconciled deterministically across all participating chains
- If reconciliation diverges, the platform cannot proceed

**Bridge discipline.**
- Only authorized signers may mint or burn wrapped representations
- Every mint event reduces available bridge capacity on the source chain
- Every burn event unlocks bridge capacity and releases canonical tokens
- Bridge fees are collected and routed to treasury via canonical accounting
- Rollback paths exist and are rehearsed for every supported bridge method

**Failure recovery.**
- If a bridge lock fails, canonical tokens remain in custody and wrapped tokens are not minted
- If settlement confirmation is delayed, wrapped tokens remain in restricted state until settlement completes
- If supply reconciliation diverges, all minting halts automatically until the discrepancy is resolved
- Every bridge failure mode has a named recovery path and an operator runbook

## System actors

### Governance Council
Holds the ability to update governance power mappings, approve new chains, update bridge fees, approve signers, and activate emergency pause. Acts through signed proposals that are recorded on-chain.

### Treasury
Manages bridge reserves, governance token staking pools, and reserve balances used to cover bridge settlement failures.

### Mint/Burn Signers
Execute mint and burn operations on each supported chain. These signers are constrained by per-chain daily caps, require multi-sig for large operations, and are rotated according to custody lifecycle.

### Bridge Validators
Run the cross-chain settlement process for each bridge type. They observe source chain lock proof, validate the canonical state, and authorize settlement on the destination chain.

### Canonical State Registry
Records the current wrapped supply on each chain, the canonical X3 total, governance power mappings, and active bridge transactions. This registry is the source of truth for reconciliation.

### Failure Recovery Authority
May execute alternative settlement, unlock frozen bridge capacity, or perform emergency supply reconciliation under named conditions.

## Wrapped token rules

### Mint Rules

Mint is only allowed when:
- The signer is authorized for the destination chain
- Daily mint cap for the chain-pair is not exceeded
- A canonical lock proof exists in the settlement registry
- The source chain inventory has provisioned bridge capacity

Mint increases wrapped-token supply on the destination chain and decreases available bridge capacity. Every mint is recorded with a unique proof ID so burn can later be matched to mint.

### Burn Rules

Burn is only allowed when:
- The caller holds wrapped tokens on the source chain
- The settlement registry has reserved bridge capacity on the destination chain
- The canonical registry is not in partial-reconciliation state

Burn decreases wrapped-token supply on the source chain and unlocks bridge capacity. Every burn creates a settlement obligation that must complete within a bounded window.

If burn settlement fails, the wrapped tokens are restored and the burn operation is marked failed in the settlement registry.

### Bridge Fee Collection

Every mint and burn operation collects a fee proportional to the amount and the corridor distance (further corridors cost more). Fees are:
- Deducted from the amount being moved
- Routed to the treasury reserve vault immediately
- Reconciled per-corridor and cumulative for compliance reporting

Bridge fee schedule is updated by governance and recorded in the registry.

## Governance power mapping

### Canonical Power

Each X3 token holder has voting power equal to their balance. The canonical voting power is registered against their canonical account (which may be a multisig, a governance delegator role, or a personal account).

### Chain-Specific Representation

When a token holder wraps tokens on a specific chain, they receive wrapped tokens on that chain. Those wrapped tokens automatically grant them the same canonical voting power on governance proposals, regardless of whether the wrapped tokens are staked.

Governance proposals are voted on a global basis. A proposal is valid if it passes majority on the canonical vote count, calculated by aggregating votes from all chains weighted by their registered wrapped supply.

### Delegation

Token holders may delegate voting power to another account, even across chains. Delegation is recorded in the canonical governance registry and applies globally until revoked.

### Power Divergence Protection

If the sum of chain-specific voting power diverges from canonical power by more than 1%, an automatic alert is raised and governance is suspended until reconciliation is performed. This prevents the governance set from fragmenting.

## Supply reconciliation

### Canonical Supply Total

The canonical X3 supply is the sum of:
- (Non-wrapped X3 held in canonical treasury accounts)
- (Wrapped X3 on Chain A + Wrapped X3 on Chain B + ... + Wrapped X3 on Chain N)

### Reconciliation Cycle

Every 24 hours (or on-demand), the canonical registry runs a supply reconciliation:
1. Query wrapped supply on each connected chain
2. Query canonical treasury holding
3. Calculate divergence: `abs(total_supply - canonical_total)`
4. If divergence > tolerance (`0.01%` for standard, `0.1%` for degraded), raise alert
5. If divergence cannot be resolved within 1 hour, halt all minting

### Divergence Recovery

If reconciliation diverges:
1. Audit all recent mint/burn operations for errors
2. Check settlement confirmation timestamps
3. Identify which chain(s) are diverged
4. Determine if divergence is a settlement IN-FLIGHT or a permanent loss
5. If permanent loss: execute emergency bridge settlement or seek treasury backstop
6. If IN-FLIGHT: extend settlement window and retry
7. Resume minting only after divergence is resolved and governance approves

## Staking and governance integration

### Staking Mechanics

X3 token holders may stake wrapped tokens on any chain to receive staking rewards. Staked tokens continue to grant governance power, but they cannot be moved until unstaking completes.

### Staking Derivatives

Some chains may support liquid staking (e.g., stX3 on Ethereum representing staked X3). Liquid staking tokens are not themselves governance-bearing. Governance power is retained by the underlying X3 only. Liquid staking providers must reconcile daily with the canonical registry.

### Vault Staking Strategy

The treasury vault may participate in staking for strategic chains to provide depth and baseline yield. Vault staking is subject to exposure caps and daily rebalance constraints, just like settlement inventory.

## Bridge architecture options

### Type 1: Inter-chain Lock and Mint

Source: User sends canonical X3 to a time-locked custody vault on the canonical chain. After a settlement window, wrapped tokens are minted on the destination chain. Reverse: Burn wrapped tokens, and after settlement, canonical X3 is released from the custody vault.

Safety: Uses escrow to hold canonical tokens. Safe for rare, large movements. Slower (typically 1+ hours per direction).

### Type 2: Sidecar Bridge with Validator Consensus

Source: Sidecar validators observe a user burn wrapped tokens on the source chain. They reach consensus on the burn event. Canonical minter is authorized to mint on the destination chain.

Safety: Consensus is required, providing security against single-signer compromise. Faster than escrow (typically 5-30 min per direction). Requires validator quorum.

### Type 3: Fast Proof Bridge

Source and destination chains support fast proof relay (e.g., light client). A user burns wrapped tokens on the source chain. A proof of the burn is relayed to the destination chain and validated by its light client. Upon proof validation, minting is authorized.

Safety: Depends on light-client correctness. Fastest bridge type (typically 10-60 sec per direction). Requires both chains to support light-client messaging.

**Recommended Phase 5 approach**: Implement Type 1 (safest, well-tested) and Type 2 (good balance). Type 3 can be added in Phase 6 for strategic chains.

## Failure modes and recovery

### Settlement Timeout
**Scenario**: User burns wrapped tokens. After 30 min, no mint proof arrives on destination chain.
**Recovery**: Wrapped tokens are restored to user's account. User can retry with different corridor or route. Failure is logged.

### Mint Capacity Exceeded
**Scenario**: Mint daily cap for a chain is reached during business hours.
**Recovery**: User is queued. Mint is retried when the next daily window opens (UTC midnight). User is notified with exact retry time.

### Governance Power Divergence
**Scenario**: Wrapped supply on Chain A increases, but canonical registry is not updated.
**Recovery**: Automatic 1-hour reconciliation check. If divergence persists, halt minting. Treasury can approve manual reconciliation if divergence is due to IN-FLIGHT settlements.

### Bridge Validator Outage
**Scenario**: Sidecar validators (Type 2) fail to reach consensus.
**Recovery**: Fall back to Type 1 escrow bridge if available. Notify governance. Increase validator redundancy. Restart failed validators if transient.

### Supply Loss (Severe)
**Scenario**: A bridge lock is compromised and canonical X3 is stolen.
**Recovery**: Automatically halt all minting on affected chains. Raise governance emergency motion. Treasury may absorb loss using insurance reserves (within cap) or governance may vote to accept dilution.

## Compliance and regulatory

Every mint, burn, bridge fee, and governance power change is recorded with timestamp, signer, amount, parties, and proof links. This immutable record is the basis for compliance audit and regulatory reporting.

Bridge corridors to specific jurisdictions may be rate-limited or require additional verification depending on local requirements. Governance council may disable bridges to certain jurisdictions via administrative proposal.

## Exit gates for Phase 5

Phase 5 is complete when:
- ✅ Wrapped X3 is live on at least 2 major chains (e.g., Ethereum + Cosmos)
- ✅ Supply reconciliation runs daily and divergence stays within 0.01%
- ✅ Bridge validators have run for 30 days without failed settlements
- ✅ Governance power aggregation is correct across all chains
- ✅ All failure recovery modes have been tested and latencies are within bounds
- ✅ At least one bridge (Type 1 or Type 2) has processed 500+ transactions without loss
- ✅ Compliance audit and regulatory filings are complete for initial jurisdictions

After these gates pass, Phase 6 (auctions) may proceed.
