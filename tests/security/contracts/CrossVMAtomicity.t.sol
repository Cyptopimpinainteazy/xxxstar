// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "./InvariantProperties.sol";

/**
 * CrossVMAtomicity - Foundry Stateful Invariant Tests
 *
 * Uses Foundry's `invariant_*` harness (depth=500, runs=10000 per foundry.toml).
 * A separate Handler contract drives randomised call sequences so the fuzzer
 * exercises realistic interaction patterns, not just single-function calls.
 *
 * Run:
 *   forge test --match-contract CrossVMAtomicityTest -vvv
 *   forge test --match-contract CrossVMAtomicityTest --gas-report
 *
 * Invariants covered (registry.toml):
 *   CHAIN-STATE-001  — supply conserved
 *   ATOMIC-CROSS-001 — commit-or-revert
 *   ATOMIC-CROSS-002 — no partial write
 *   GAS-ACCT-001     — gas bounded
 */
contract CrossVMAtomicityHandler is Test {
    InvariantProperties public props;

    constructor(InvariantProperties _props) {
        props = _props;
    }

    // ── Bounded random calls that Foundry will sequence ───────────────────────

    function transfer(uint256 seed) external {
        uint256 bal = props.vmABalance();
        if (bal == 0) return;
        uint256 amount = bound(seed, 1, bal);
        props.atomicTransfer(amount);
    }

    function transferRevert(uint256 seed) external {
        uint256 bal = props.vmABalance();
        if (bal == 0) return;
        uint256 amount = bound(seed, 1, bal);
        props.atomicTransferRevert(amount);
    }

    function lockEscrow(uint256 seed) external {
        uint256 bal = props.vmABalance();
        if (bal == 0) return;
        uint256 amount = bound(seed, 1, bal);
        props.lockEscrow(amount);
    }

    function releaseEscrow(uint256 seed) external {
        uint256 locked = props.reservedEscrow();
        if (locked == 0) return;
        uint256 amount = bound(seed, 1, locked);
        props.releaseEscrow(amount);
    }
}

contract CrossVMAtomicityTest is Test {
    InvariantProperties public props;
    CrossVMAtomicityHandler public handler;

    function setUp() public {
        props   = new InvariantProperties();
        handler = new CrossVMAtomicityHandler(props);

        // Tell Foundry: only call `handler`, not `props` directly
        targetContract(address(handler));

        // Exclude view-only selectors from the fuzzer's call set
        bytes4[] memory selectors = new bytes4[](4);
        selectors[0] = handler.transfer.selector;
        selectors[1] = handler.transferRevert.selector;
        selectors[2] = handler.lockEscrow.selector;
        selectors[3] = handler.releaseEscrow.selector;
        targetSelector(FuzzSelector({ addr: address(handler), selectors: selectors }));
    }

    // ── Invariants ────────────────────────────────────────────────────────────

    /// @dev CHAIN-STATE-001: sum of all balances == totalSupply at all times
    function invariant_supplyConserved() public view {
        assertEq(
            props.vmABalance() + props.vmBBalance() + props.reservedEscrow(),
            props.totalSupply(),
            "supply not conserved"
        );
    }

    /// @dev ATOMIC-CROSS-001/002: no partial write survives a transaction boundary
    function invariant_noPartialCommit() public view {
        assertFalse(
            props.partialStateWritten(),
            "partial cross-VM state write detected - atomicity broken"
        );
    }

    /// @dev no cross-VM reentrancy window survives
    function invariant_noReentrancyWindow() public view {
        assertFalse(
            props.crossVMCallActive(),
            "cross-VM call still active after tx boundary - reentrancy leak"
        );
    }

    /// @dev GAS-ACCT-001: total gas never exceeds sanity cap
    function invariant_gasAccountingBounded() public view {
        assertLe(
            props.gasUsed(),
            props.GAS_CAP() * 1_000,
            "gas accounting unbounded - possible griefing vector"
        );
    }

    /// @dev balances stay non-negative and within supply cap
    function invariant_balancesNonNegative() public view {
        assertLe(props.vmABalance(), props.totalSupply(), "vmA balance overflow");
        assertLe(props.vmBBalance(), props.totalSupply(), "vmB balance overflow");
        assertLe(props.reservedEscrow(), props.totalSupply(), "escrow overflow");
    }

    // ── Unit smoke tests (sanity before fuzz) ─────────────────────────────────

    function test_atomicTransferConservesSupply() public {
        uint256 before = props.vmABalance() + props.vmBBalance() + props.reservedEscrow();
        props.atomicTransfer(1 ether);
        uint256 after_ = props.vmABalance() + props.vmBBalance() + props.reservedEscrow();
        assertEq(before, after_);
    }

    function test_revertedTransferConservesSupply() public {
        uint256 before = props.vmABalance() + props.vmBBalance() + props.reservedEscrow();
        props.atomicTransferRevert(1 ether);
        uint256 after_ = props.vmABalance() + props.vmBBalance() + props.reservedEscrow();
        assertEq(before, after_);
        // After revert, vmA unchanged
        assertEq(props.vmABalance(), 500_000_000 ether);
    }

    function test_revertLeavesNoPartialState() public {
        props.atomicTransferRevert(100 ether);
        assertFalse(props.partialStateWritten());
        assertFalse(props.crossVMCallActive());
    }

    function test_escrowRoundTrip() public {
        uint256 initial = props.vmABalance();
        props.lockEscrow(42 ether);
        assertEq(props.reservedEscrow(), 42 ether);
        assertEq(props.vmABalance(), initial - 42 ether);

        props.releaseEscrow(42 ether);
        assertEq(props.reservedEscrow(), 0);
        // Total supply still conserved (vmB received)
        assertEq(
            props.vmABalance() + props.vmBBalance() + props.reservedEscrow(),
            props.totalSupply()
        );
    }

    function test_fuzzTransferBounded(uint256 amount) public {
        uint256 vmA = props.vmABalance();
        amount = bound(amount, 1, vmA);
        uint256 totalBefore = props.vmABalance() + props.vmBBalance() + props.reservedEscrow();
        props.atomicTransfer(amount);
        assertEq(
            props.vmABalance() + props.vmBBalance() + props.reservedEscrow(),
            totalBefore
        );
    }
}
