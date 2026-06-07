# EVM Rules — Knowledge Core

## Overview

These are the mandatory security rules for all Solidity/EVM code in the X3 ecosystem. Every smart contract — whether a bridge, DEX, token, governance module, or utility — must comply with these rules. No exceptions.

## Access Control

### Rule EVM-1: Explicit Access Control

Every function must have a clearly defined access pattern.

- Default to the most restrictive visibility (`private` > `internal` > `external` > `public`).
- Use `onlyRole`, `onlyOwner`, or custom modifiers for restricted functions.
- No function is public by default. If a function is `public` or `external`, there must be an explicit reason.
- Admin functions must use a role-based system (e.g., `AccessControl`). Single-owner patterns must have a timelock or multisig.
- `initialize` functions in upgradeable contracts must have an `initializer` modifier and be callable exactly once.

### Rule EVM-2: Role Separation

- The `DEFAULT_ADMIN_ROLE` must not be held by a single EOA.
- Guardian, pauser, and operator roles must be separate.
- No role may grant privileges that bypass other access controls.

## Events

### Rule EVM-3: Events for State Changes

Every state-changing function must emit an event documenting:

- **Who** changed the state (`msg.sender` or the relevant actor).
- **What** changed (the state variable or asset).
- **From what** value (the previous state).
- **To what** value (the new state).

No silent state changes. If a function modifies storage and does not emit an event, that is a bug.

### Rule EVM-4: Event Indexing

- Key identifiers (token addresses, user addresses, nonce values) must be indexed.
- Value fields (amounts, balances) must not be indexed.
- Events must be consumable by off-chain indexers without requiring additional on-chain queries.

## Reentrancy Protection

### Rule EVM-5: ReentrancyGuard

- All external-facing functions that interact with other contracts or transfer tokens must use `ReentrancyGuard` or an equivalent.
- The checks-effects-interactions pattern is mandatory but not sufficient alone. A malicious contract can re-enter through a different function.
- Use `nonReentrant` modifier on all functions that make external calls or transfer assets.
- Assume ERC-777 and similar callback-capable tokens are adversarial. Apply `ReentrancyGuard` even for "simple" token transfers.

### Rule EVM-6: No Unchecked Callbacks

- Never trust callback functions from external contracts.
- Validate all return data from external calls.
- Use `try/catch` for external calls that may fail, and handle both success and failure cases explicitly.

## Slippage and Deadline

### Rule EVM-7: Slippage Checks

Every swap, trade, or transfer must include a minimum output amount (`amountOutMinimum` or equivalent).

- No unchecked return values from swaps.
- The caller must specify the minimum acceptable output.
- If the actual output is less than the minimum, the transaction must revert.
- For cross-chain or cross-VM swaps, slippage must account for latency and finality delays.

### Rule EVM-8: Deadline Checks

Every user-facing transaction must include a `deadline` parameter.

- Expired transactions must revert.
- The deadline must be compared against `block.timestamp`.
- No transaction may be valid indefinitely.

## Pause and Guardian

### Rule EVM-9: Pause Mechanism

All contracts handling user funds must implement a pause mechanism.

- The `pause()` function must be callable by a guardian role.
- Pause must stop all deposits, swaps, and transfers, but must NOT block withdrawals.
- `unpause()` must require a separate role or a timelock.
- Pause state must be emitted as an event.

### Rule EVM-10: Guardian Role

- The guardian can pause but cannot steal.
- The guardian cannot bypass access controls.
- The guardian cannot modify critical state (e.g., token supply, bridge parameters) without a timelock.

## Security Boundaries

### Rule EVM-11: Never Use tx.origin

Always use `msg.sender`. No exceptions. Using `tx.origin` for access control is a well-known attack vector.

### Rule EVM-12: No Hidden Taxes or Fees

- Fee-on-transfer tokens must be handled with explicit accounting.
- No hidden fee mechanisms that the user cannot predict.
- No admin functions that can drain user funds.
- No hidden mint functions.
- No timelocked rug pulls.

### Rule EVM-13: Integer Overflow/Underflow

- Use Solidity 0.8+ for built-in overflow checks, or use SafeMath for older versions.
- No unchecked arithmetic on user-facing functions unless the overflow/underflow is provably impossible.
- Use `unchecked` blocks only for gas optimization where safety is mathematically proven, and document the proof.

### Rule EVM-14: External Call Safety

- Use `call` instead of `transfer` or `send` for ETH transfers (to avoid gas limit issues).
- Check the return value of `call`.
- Use `ReentrancyGuard` when making external calls.
- Never assume external calls succeed.

## Bridge and Cross-Chain Contracts

### Rule EVM-15: Bridge Contract Rules

- Every bridge must call the UAK before minting or releasing assets.
- Every bridge must verify proofs on-chain (Merkle proof, SPV, ZK, or multi-sig).
- Every bridge must have a timeout and refund path.
- Every bridge must have replay protection (nonce + chain ID).
- Every bridge must emit events for lock, release, mint, burn, and refund.
- Bridge parameters (thresholds, signers, timeouts) must be upgradable only via governance with a timelock.

### Rule EVM-16: No Wrapped-Token Shortcuts

Refer to **UNIVERSAL_ASSET_KERNEL.md** Rule 1. Every wrapped token must have:

- A defined custodian.
- A defined mint path.
- A defined burn path.
- A defined lock path.
- A defined release path.
- A defined refund path.
- Verifiable on-chain custody proof.

## Testing Requirements

### Rule EVM-17: Foundry/Hardhat Tests Required

Every contract must have:

- **Unit tests** for each function, including access control, edge cases, and failure modes.
- **Integration tests** for cross-contract interactions (especially bridge and DEX contracts).
- **Invariant tests** (Foundry `InvariantTest` or equivalent) for critical invariants (supply, balance, access control).
- **Fuzz tests** for functions that accept user input (amounts, addresses, calldata).
- **Fork tests** for contracts that interact with external protocols (Uniswap, Aave, etc.).

### Rule EVM-18: Test Coverage

- Minimum 90% line coverage for all contracts.
- 100% branch coverage for critical paths (bridge, DEX, token, governance).
- Every `require` statement must have a test that triggers it.
- Every `revert` path must have a test that exercises it.

## Deployment Checklist

For every contract deployment:

1. Constructor arguments are correct and verified.
2. Initial state matches specification (roles, thresholds, parameters).
3. Access control is set up correctly (no single EOA has admin).
4. Proxy (if upgradeable) is initialized exactly once.
5. Contract is verified on the block explorer.
6. Events are indexed and monitored.
7. Pause mechanism is tested.
8. UAK integration is tested (for bridge contracts).

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the canonical supply invariant that EVM contracts must preserve.
- **UNIVERSAL_ASSET_KERNEL.md** — EVM contracts must call the UAK for every asset movement.
- **CROSS_VM_ROUTING.md** — EVM bridge and cross-VM contracts must follow route specifications.
- **TRADING_SAFETY_KERNEL.md** — DEX and arb contracts must comply with trading safety rules.
- **FORBIDDEN_PATTERNS.md** — Explicit list of forbidden contract patterns.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*