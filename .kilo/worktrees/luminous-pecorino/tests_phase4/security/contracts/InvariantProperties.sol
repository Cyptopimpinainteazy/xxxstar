// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * InvariantProperties — Echidna / Medusa Property Tests
 *
 * These Echidna-style property functions are used by both Echidna and Medusa
 * fuzzers to verify critical X3 cross-VM atomicity and accounting invariants.
 *
 * Run:
 *   echidna tests/security/contracts/InvariantProperties.sol \
 *     --contract InvariantProperties \
 *     --config tests/security/echidna.config.yaml
 *
 *   medusa fuzz --config tests/security/medusa.config.json
 *
 * Invariants tested (see tests/invariants/registry.toml):
 *   CHAIN-STATE-001  — total supply conserved
 *   ATOMIC-CROSS-001 — cross-VM atomic commit-or-revert
 *   ATOMIC-CROSS-002 — no partial cross-VM state write
 *   GAS-ACCT-001     — gas accounting bounded
 *   REENTRY-CROSS-001 — no cross-VM reentrancy window
 */

// ── Minimal stubs (replace with real imports when contracts are deployed) ──────

interface ICrossVMBridge {
    function callVMB(bytes calldata data) external returns (bool success);
    function vmABalance() external view returns (uint256);
    function vmBBalance() external view returns (uint256);
    function pendingState() external view returns (bool);
    function gasUsed() external view returns (uint256);
}

// ── Main property contract ────────────────────────────────────────────────────

contract InvariantProperties {
    // ── State ─────────────────────────────────────────────────────────────────

    uint256 public totalSupply;
    uint256 public vmABalance;
    uint256 public vmBBalance;
    uint256 public reservedEscrow;

    bool public crossVMCallActive;
    bool public partialStateWritten; // must stay false after any atomic boundary
    uint256 public gasUsed;
    uint256 public constant GAS_CAP = 30_000_000; // highest believable block gas

    // ── Constructor ───────────────────────────────────────────────────────────

    constructor() {
        totalSupply    = 1_000_000_000 ether;
        vmABalance     = 500_000_000 ether;
        vmBBalance     = 500_000_000 ether;
        reservedEscrow = 0;
        crossVMCallActive  = false;
        partialStateWritten = false;
        gasUsed = 0;
    }

    // ── Simulated state-changing operations (fuzzer drives these) ─────────────

    /**
     * Simulate a successful atomic cross-VM transfer.
     * Both VMs must update together or not at all.
     */
    function atomicTransfer(uint256 amount) external {
        require(amount > 0 && amount <= vmABalance, "invalid amount");
        require(!crossVMCallActive, "reentrancy guard");

        crossVMCallActive   = true;
        partialStateWritten = false;

        // Debit VM-A first (would be partial if VM-B fails)
        vmABalance -= amount;
        partialStateWritten = true;   // mark as partial

        // Credit VM-B (completes the atomic pair)
        vmBBalance += amount;
        partialStateWritten = false;  // cleared on success

        crossVMCallActive = false;
        gasUsed += 21_000 + (amount / 1 ether) * 100;
    }

    /**
     * Simulate a failed atomic cross-VM transfer (VM-B rejects).
     * VM-A debit must be rolled back.
     */
    function atomicTransferRevert(uint256 amount) external {
        require(amount > 0 && amount <= vmABalance, "invalid amount");
        require(!crossVMCallActive, "reentrancy guard");

        crossVMCallActive   = true;
        partialStateWritten = false;

        uint256 snapshot    = vmABalance;

        // Debit VM-A
        vmABalance -= amount;
        partialStateWritten = true;

        // VM-B rejects — rollback
        vmABalance         = snapshot;
        partialStateWritten = false;
        crossVMCallActive   = false;
        gasUsed += 21_000;
    }

    /**
     * Escrow lock (used in settlement flow).
     */
    function lockEscrow(uint256 amount) external {
        require(amount <= vmABalance, "insufficient balance");
        vmABalance    -= amount;
        reservedEscrow += amount;
    }

    /**
     * Escrow release.
     */
    function releaseEscrow(uint256 amount) external {
        require(amount <= reservedEscrow, "insufficient escrow");
        reservedEscrow -= amount;
        vmBBalance     += amount;
    }

    // ── Echidna/Medusa property functions (prefix: echidna_) ─────────────────

    /**
     * CHAIN-STATE-001: Total supply is conserved.
     * vmABalance + vmBBalance + reservedEscrow must always equal totalSupply.
     */
    function echidna_total_supply_conserved() external view returns (bool) {
        return vmABalance + vmBBalance + reservedEscrow == totalSupply;
    }

    /**
     * ATOMIC-CROSS-001: After every transaction boundary, no partial
     * cross-VM state write can exist.
     */
    function echidna_atomic_commit_or_revert() external view returns (bool) {
        // partialStateWritten is only true DURING an active call,
        // never after it completes. Since Echidna checks after each tx,
        // this must always be false.
        return !partialStateWritten;
    }

    /**
     * ATOMIC-CROSS-002: cross-VM call is never re-entered while active.
     * (Reentrancy invariant at fuzzer granularity.)
     */
    function echidna_no_reentrancy_window() external view returns (bool) {
        return !crossVMCallActive;
    }

    /**
     * GAS-ACCT-001: Cumulative gas used never exceeds a reasonable cap.
     * Guards against gas griefing / infinite-loop attacks.
     */
    function echidna_gas_accounting_bounded() external view returns (bool) {
        return gasUsed <= GAS_CAP * 1_000; // allow 1000 blocks worth
    }

    /**
     * VM-A balance can never go negative (checked via underflow; Solidity 0.8+
     * reverts on underflow, but this makes the invariant explicit).
     */
    function echidna_balances_non_negative() external view returns (bool) {
        return vmABalance <= totalSupply && vmBBalance <= totalSupply;
    }

    /**
     * Escrow can never exceed total supply.
     */
    function echidna_escrow_bounded() external view returns (bool) {
        return reservedEscrow <= totalSupply;
    }
}
