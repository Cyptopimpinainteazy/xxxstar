# SVM Rules — Knowledge Core

## Overview

These are the mandatory security rules for all Solana/SVM programs in the X3 ecosystem. Every program — whether a token, DEX, bridge, or utility — must comply with these rules. The SVM's parallel execution model, account-based architecture, and compute constraints require specific security patterns that differ significantly from EVM.

## Account Validation

### Rule SVM-1: Account Validation

Every account passed to a Solana program must be validated.

- **Owner check**: Every account must have its `owner` field checked. Only the expected program should own writable accounts. Accounts owned by the system program are acceptable only for rent-exempt transfers.
- **Data check**: Account data must be deserialized and validated against expected types and states. Never trust raw account data without checking the discriminator.
- **Writable check**: If an account is expected to be writable, verify `is_writable`. If an account should be read-only, verify it is NOT writable.
- **Signer check**: If an account is expected to have signed the transaction, verify `is_signer`. Never assume an account is a signer without explicit verification.
- **Executable check**: If an account is a program, verify `executable`. If an account should not be a program, verify it is NOT executable.
- **Rent-exempt check**: All accounts must be rent-exempt unless they are being closed. Never create accounts that will fall below the rent-exempt minimum.

### Rule SVM-2: PDA Derivation

Program Derived Addresses (PDAs) must be derived deterministically.

- Seeds must be documented and must not include user-supplied values that could cause collisions unless explicitly designed for that purpose.
- Use `Pubkey::find_program_address()` for derivation, and verify the bump seed matches.
- Never use `create_program_address()` without also verifying the bump seed.
- PDA collisions between different seed patterns must be documented and mitigated.
- Seeds must not allow an attacker to derive a PDA that grants unauthorized access.

### Rule SVM-3: Signer Checks

- Every instruction that modifies state must verify the authority/signer.
- Use `assert!(ctx.accounts.authority.is_signer)` for all authority checks.
- Never trust `ctx.accounts.authority.key()` without checking `is_signer`.
- CPI calls must propagate signer privileges correctly. A PDA signer from one program does not automatically have signer privileges in another program's CPI.

## CPI Safety

### Rule SVM-4: CPI Safety

Cross-Program Invocations (CPIs) must be safe.

- Validate all accounts passed to CPIs, including the program ID.
- Never CPI to an arbitrary program. The target program must be a known, expected program.
- Use `invoke_signed()` for CPIs from PDAs, and verify the seeds match the derivation.
- CPIs must not exceed the compute budget. Account for the compute cost of CPIs in the instruction's budget.
- CPIs must not create reentrancy. If program A calls program B which calls program A, that is a critical bug.
- CPI depth must not exceed 4 levels (Solana limit).
- CPI return data must be validated. Never trust return data from an external program without checking.

## Rent Exemption

### Rule SVM-5: Rent Exemption

- All accounts must be rent-exempt. Accounts that are not rent-exempt will be garbage-collected.
- When creating accounts, ensure the lamport balance is at least the rent-exempt minimum.
- When closing accounts, transfer all lamports to the owner or a designated recipient. Do not leave accounts with zero balance.
- Use `Rent::get()` to get the current rent-exempt minimum. Do not hardcode it.
- Account size must be correctly calculated. An account that is too small will not hold the data, and the extra space will be wasted.

## Compute Budget

### Rule SVM-6: Compute Budget

- Every instruction must be designed to complete within the compute budget (default 200,000 CU, max 1,400,000 CU with `ComputeBudgetInstruction`).
- Use `ComputeBudgetInstruction::set_compute_unit_limit()` for instructions that need more than the default budget.
- Use `ComputeBudgetInstruction::set_compute_unit_price()` for priority fees.
- Avoid loops that iterate over unbounded data. Use pagination or bounded arrays.
- Avoid expensive cryptographic operations (e.g., secp256k1 recovery) in hot paths.
- Profile instructions with `solana-program-runtime` to measure compute usage.
- If an instruction requires multiple signatures, account for the signature verification cost.

### Rule SVM-7: Parallel Execution Constraints

- The SVM executes transactions in parallel. Programs must not assume serial execution.
- No global locks. Programs must use fine-grained account locking.
- Transactions that touch the same account are serialized. Minimize write contention on popular accounts.
- Design state structures to minimize account conflicts. Use separate accounts for separate concerns.
- Do not assume transaction ordering. Use commitment levels and slot checks for ordering.

## Lamport Accounting

### Rule SVM-8: Lamport Accounting

- Every lamport transfer must be balanced. Total lamports in must equal total lamports out (plus rent and fees).
- Use checked math for all lamport operations. No silent overflows.
- Account for transaction fees in lamport calculations.
- When closing accounts, transfer all remaining lamports to the designated recipient.
- Bridge programs must reconcile lamport movements with the UAK's canonical supply invariant.

## Slot and Epoch Awareness

### Rule SVM-9: Slot and Epoch Awareness

- Programs must be aware of the current slot and epoch for time-based operations.
- Use `Clock::get()` to get the current slot, epoch, and timestamp.
- Do not assume `Clock::get().unix_timestamp` is precise. It is an estimate.
- Staking-related operations must use epoch boundaries, not slots.
- Time-based operations (timeouts, deadlines) must use slot-based or epoch-based thresholds, not just timestamps.
- Programs must handle the case where a slot is skipped (no leader produced a block).

## Token Operations

### Rule SVM-10: Token Program Rules

- Use the SPL Token program (or Token-2022) for all token operations. Do not implement custom token logic.
- Verify the token mint authority matches the expected program.
- Verify token account owners match the expected signer.
- Use `TokenInstruction::SetAuthority` carefully — authority changes must be gated by access control.
- Close token accounts only when the balance is zero and the owner has authorized closure.
- For Token-2022, be aware of extension-specific behaviors (transfer fees, confidential transfers, etc.).

## Bridge and Cross-VM Programs

### Rule SVM-11: SVM Bridge Rules

- Every SVM bridge program must verify incoming messages against the UAK.
- Every SVM bridge program must emit events (via log instructions) for lock, release, mint, burn, and refund.
- Every SVM bridge program must handle timeouts and refunds on the SVM side.
- Replay protection must use a combination of nonce, source chain ID, and destination chain ID.
- PDA seeds for bridge accounts must include the nonce and chain ID to prevent collision.

## Testing Requirements

### Rule SVM-12: Testing Requirements

Every program must have:

- **Unit tests** for each instruction handler, including access control, edge cases, and failure modes.
- **Integration tests** for cross-program interactions (especially CPIs).
- **Fuzz tests** for instructions that accept user-supplied account data.
- **Stress tests** for compute budget and account size constraints.
- **Security tests** for known SVM attack vectors (account substitution, PDA collision, CPI reentrancy, signer spoofing).

### Rule SVM-13: Test Frameworks

- Use `solana-program-test` for local integration testing.
- Use `anchor test` for Anchor programs.
- Use `proptest` or `honggfuzz` for fuzz testing.
- Use `solana-bench-tps` for performance testing.

## Deployment Checklist

1. Program binary is compiled and verified (no unexpected instructions).
2. Upgrade authority is set to a multisig or governance.
3. Initial accounts are created with correct data and rent-exempt balances.
4. PDA seeds are documented and tested.
5. CPI targets are verified (no CPI to unknown programs).
6. Compute budget is profiled and within limits.
7. Bridge programs are tested against the UAK.
8. Events are indexed and monitored.

## Relationship to Other Knowledge Core Documents

- **X3_ARCHITECTURE.md** — Defines the canonical supply invariant that SVM programs must preserve.
- **UNIVERSAL_ASSET_KERNEL.md** — SVM bridge programs must call the UAK for every asset movement.
- **CROSS_VM_ROUTING.md** — SVM cross-VM programs must follow route specifications.
- **TRADING_SAFETY_KERNEL.md** — DEX and arb programs on SVM must comply with trading safety rules.
- **FORBIDDEN_PATTERNS.md** — Explicit list of forbidden program patterns.

---

*This document is part of the X3 Knowledge Core. All X3 models must apply these principles before making recommendations.*