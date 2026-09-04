// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "../contracts/AtlasHTLC.sol";

/// @notice Minimal ERC-20 for testing AtlasHTLC token swaps
contract MockERC20 is ERC20("Test Token", "TST") {
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

/// @title AtlasHTLCTest
/// @notice Comprehensive test suite for AtlasHTLC covering ETH and ERC-20 flows
contract AtlasHTLCTest is Test {
    AtlasHTLC public htlc;
    MockERC20 public token;

    address public alice = address(0xA11CE);
    address public bob   = address(0xB0B);

    bytes32 public secret    = keccak256("secret");
    bytes32 public hashLock;

    uint256 public constant AMOUNT = 1 ether;
    uint256 public constant TIMELOCK = 1 hours;

    event HTLCCreated(
        bytes32 indexed id,
        address indexed sender,
        address indexed recipient,
        address token,
        uint256 amount,
        bytes32 hashLock,
        uint256 timeLock
    );
    event HTLCClaimed(bytes32 indexed id, address indexed claimant, bytes32 secret);
    event HTLCRefunded(bytes32 indexed id, address indexed sender);

    function setUp() public {
        htlc = new AtlasHTLC();
        token = new MockERC20();

        // Compute the SHA-256 hashLock (matches contract's sha256 check)
        hashLock = sha256(abi.encodePacked(secret));

        // Fund Alice with ETH and tokens
        vm.deal(alice, 100 ether);
        token.mint(alice, 100 ether);
    }

    // ─── Native ETH Tests ────────────────────────────────────

    function testCreateHTLC_ETH() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        assertTrue(id != bytes32(0));

        (address sender, address recipient, address tokenAddr, uint256 amount, , , AtlasHTLC.HTLCStatus status, ) =
            htlc.getHTLC(id);

        assertEq(sender, alice);
        assertEq(recipient, bob);
        assertEq(tokenAddr, address(0));
        assertEq(amount, AMOUNT);
        assertEq(uint8(status), uint8(AtlasHTLC.HTLCStatus.Funded));
    }

    function testCreateHTLC_ETH_EmitsEvent() public {
        vm.prank(alice);
        vm.expectEmit(true, true, true, true);
        emit HTLCCreated(
            keccak256(abi.encodePacked(alice, bob, hashLock, uint256(1))),
            alice, bob, address(0), AMOUNT, hashLock, block.timestamp + TIMELOCK
        );
        htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);
    }

    function testClaimHTLC_ETH() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        uint256 bobBalanceBefore = bob.balance;

        vm.prank(bob);
        htlc.claimHTLC(id, secret);

        ( , , , , , , AtlasHTLC.HTLCStatus status, bytes32 storedSecret) = htlc.getHTLC(id);
        assertEq(uint8(status), uint8(AtlasHTLC.HTLCStatus.Claimed));
        assertEq(storedSecret, secret);
        assertEq(bob.balance - bobBalanceBefore, AMOUNT);
    }

    function testRefundHTLC_ETH() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        uint256 aliceBalanceBefore = alice.balance;

        vm.warp(block.timestamp + TIMELOCK + 1);
        vm.prank(alice);
        htlc.refundHTLC(id);

        ( , , , , , , AtlasHTLC.HTLCStatus status, ) = htlc.getHTLC(id);
        assertEq(uint8(status), uint8(AtlasHTLC.HTLCStatus.Refunded));
        assertEq(alice.balance - aliceBalanceBefore, AMOUNT);
    }

    function testWrongPreimageRejected() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        vm.prank(bob);
        vm.expectRevert("Invalid secret");
        htlc.claimHTLC(id, keccak256("wrongsecret"));
    }

    function testOnlyRecipientCanClaim() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        address charlie = address(0xCAFE);
        vm.prank(charlie);
        vm.expectRevert("Not the recipient");
        htlc.claimHTLC(id, secret);
    }

    function testOnlySenderCanRefund() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        vm.warp(block.timestamp + TIMELOCK + 1);
        vm.prank(bob);
        vm.expectRevert("Not the sender");
        htlc.refundHTLC(id);
    }

    function testCannotRefundBeforeExpiry() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        vm.prank(alice);
        vm.expectRevert("TimeLock not expired");
        htlc.refundHTLC(id);
    }

    function testCannotClaimAfterRefund() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        vm.warp(block.timestamp + TIMELOCK + 1);
        vm.prank(alice);
        htlc.refundHTLC(id);

        vm.prank(bob);
        vm.expectRevert("HTLC not claimable");
        htlc.claimHTLC(id, secret);
    }

    function testCannotCreateHTLCWithZeroHashLock() public {
        vm.prank(alice);
        vm.expectRevert("Invalid hashLock");
        htlc.createHTLC{value: AMOUNT}(bob, bytes32(0), block.timestamp + TIMELOCK, address(0), 0);
    }

    function testCannotCreateHTLCWithPastTimelock() public {
        vm.prank(alice);
        vm.expectRevert("TimeLock must be in the future");
        htlc.createHTLC(bob, hashLock, block.timestamp - 1, address(0), 0);
    }

    function testCannotCreateHTLCWithoutValue() public {
        vm.prank(alice);
        vm.expectRevert("Must send ETH");
        htlc.createHTLC(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);
    }

    // ─── View helpers (ETH) ──────────────────────────────────

    function testIsHTLCFunded() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        assertTrue(htlc.isHTLCFunded(id));

        vm.prank(bob);
        htlc.claimHTLC(id, secret);

        assertFalse(htlc.isHTLCFunded(id));
    }

    function testIsHTLCClaimed() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        (bool claimedBefore, bytes32 secretBefore) = htlc.isHTLCClaimed(id);
        assertFalse(claimedBefore);
        assertEq(secretBefore, bytes32(0));

        vm.prank(bob);
        htlc.claimHTLC(id, secret);

        (bool claimedAfter, bytes32 secretAfter) = htlc.isHTLCClaimed(id);
        assertTrue(claimedAfter);
        assertEq(secretAfter, secret);
    }

    function testIsHTLCExpired() public {
        vm.prank(alice);
        bytes32 id = htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);

        assertFalse(htlc.isHTLCExpired(id));

        vm.warp(block.timestamp + TIMELOCK + 1);
        assertTrue(htlc.isHTLCExpired(id));
    }

    // ─── ERC-20 Tests ───────────────────────────────────────

    function testCreateHTLC_ERC20() public {
        vm.startPrank(alice);
        token.approve(address(htlc), AMOUNT);
        bytes32 id = htlc.createHTLC(bob, hashLock, block.timestamp + TIMELOCK, address(token), AMOUNT);
        vm.stopPrank();

        (address sender, address recipient, address tokenAddr, uint256 amount, , , AtlasHTLC.HTLCStatus status, ) =
            htlc.getHTLC(id);

        assertEq(sender, alice);
        assertEq(recipient, bob);
        assertEq(tokenAddr, address(token));
        assertEq(amount, AMOUNT);
        assertEq(uint8(status), uint8(AtlasHTLC.HTLCStatus.Funded));
        assertEq(token.balanceOf(address(htlc)), AMOUNT);
    }

    function testClaimHTLC_ERC20() public {
        vm.startPrank(alice);
        token.approve(address(htlc), AMOUNT);
        bytes32 id = htlc.createHTLC(bob, hashLock, block.timestamp + TIMELOCK, address(token), AMOUNT);
        vm.stopPrank();

        uint256 bobBalanceBefore = token.balanceOf(bob);

        vm.prank(bob);
        htlc.claimHTLC(id, secret);

        assertEq(token.balanceOf(bob) - bobBalanceBefore, AMOUNT);
        assertEq(token.balanceOf(address(htlc)), 0);
    }

    function testRefundHTLC_ERC20() public {
        vm.startPrank(alice);
        token.approve(address(htlc), AMOUNT);
        bytes32 id = htlc.createHTLC(bob, hashLock, block.timestamp + TIMELOCK, address(token), AMOUNT);
        vm.stopPrank();

        uint256 aliceBalanceBefore = token.balanceOf(alice);

        vm.warp(block.timestamp + TIMELOCK + 1);
        vm.prank(alice);
        htlc.refundHTLC(id);

        assertEq(token.balanceOf(alice) - aliceBalanceBefore, AMOUNT);
        assertEq(token.balanceOf(address(htlc)), 0);
    }

    function testCreateHTLC_ERC20_ZeroAmount() public {
        vm.prank(alice);
        vm.expectRevert("Amount must be > 0");
        htlc.createHTLC(bob, hashLock, block.timestamp + TIMELOCK, address(token), 0);
    }

    // ─── HTLC count ──────────────────────────────────────────

    function testHTLCCount() public {
        assertEq(htlc.htlcCount(), 0);

        vm.prank(alice);
        htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);
        assertEq(htlc.htlcCount(), 1);

        vm.prank(alice);
        htlc.createHTLC{value: AMOUNT}(bob, hashLock, block.timestamp + TIMELOCK, address(0), 0);
        assertEq(htlc.htlcCount(), 2);
    }
}
