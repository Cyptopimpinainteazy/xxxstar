// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "../contracts/X3ExternalGateway.sol";
import "../contracts/interfaces/IX3Verification.sol";

/// @title TestOnlyVerifier — Mock verifier for testnet use only
/// @dev NOT for production. Fails when `production` feature is enabled.
contract TestOnlyVerifier is IX3Verification {
    bool public shouldVerify;
    uint256 public verifierSet = 1;

    constructor(bool _shouldVerify) {
        shouldVerify = _shouldVerify;
    }

    function setShouldVerify(bool _v) external { shouldVerify = _v; }

    function verifyX3WithdrawalProof(
        bytes32, uint256, bytes calldata, address, uint256, bytes calldata
    ) external view override returns (bool) { return shouldVerify; }

    function verifyDepositProof(
        bytes32, address, address, bytes calldata, uint256, bytes calldata
    ) external view override returns (bool) { return shouldVerify; }

    function verifierSetId() external view override returns (uint256) { return verifierSet; }
}

contract MockERC20 is ERC20("Mock", "MCK") {
    function mint(address to, uint256 amount) external { _mint(to, amount); }
}

contract X3ExternalGatewayTest is Test {
    X3ExternalGateway public gateway;
    TestOnlyVerifier public verifier;
    MockERC20 public token;
    address public user = address(0x123);
    address public recipient = address(0x456);

    function setUp() public {
        verifier = new TestOnlyVerifier(true);
        gateway = new X3ExternalGateway(address(verifier), 8453, 42, 1);
        token = new MockERC20();
        token.mint(user, 200_000 ether);

        gateway.setSupportedToken(address(token), true, 100_000 ether, 50_000 ether);
    }

    function testDeposit() public {
        vm.startPrank(user);
        token.approve(address(gateway), 100 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);

        vm.expectEmit(true, true, true, true);
        emit X3ExternalGateway.DepositLocked(
            keccak256(abi.encodePacked("X3_DEPOSIT_V1", uint256(8453), address(token), user, x3Recipient, uint256(100 ether), uint256(1))),
            address(token), user, x3Recipient, 100 ether, 1, 8453
        );

        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();
    }

    function testDepositReplay() public {
        vm.startPrank(user);
        token.approve(address(gateway), 200 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.expectRevert("REPLAY");
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();
    }

    function testDepositExceedsDailyLimit() public {
        vm.startPrank(user);
        token.approve(address(gateway), 200_000 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 99_999 ether, 1);
        vm.expectRevert("DAILY_DEPOSIT_LIMIT");
        gateway.depositToX3(address(token), x3Recipient, 2 ether, 2);
        vm.stopPrank();
    }

    function testReleaseFromX3() public {
        // First deposit tokens so the gateway has liquidity
        vm.startPrank(user);
        token.approve(address(gateway), 200 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();

        bytes32 messageId = keccak256("withdrawal_1");
        bytes memory sender = abi.encodePacked(user);

        vm.expectEmit(true, true, true, true);
        emit X3ExternalGateway.WithdrawalReleased(messageId, address(token), recipient, 50 ether);

        gateway.releaseFromX3(messageId, address(token), recipient, 50 ether, sender, hex"deadbeef");
    }

    function testReleaseReplay() public {
        vm.startPrank(user);
        token.approve(address(gateway), 200 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();

        bytes32 messageId = keccak256("withdrawal_1");
        bytes memory sender = abi.encodePacked(user);
        gateway.releaseFromX3(messageId, address(token), recipient, 50 ether, sender, hex"deadbeef");

        vm.expectRevert("REPLAY");
        gateway.releaseFromX3(messageId, address(token), recipient, 50 ether, sender, hex"deadbeef");
    }

    function testPauseBlocksDeposit() public {
        gateway.setPaused(true);
        vm.startPrank(user);
        token.approve(address(gateway), 100 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        vm.expectRevert("GATEWAY_PAUSED");
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();
    }

    function testUnsupportedTokenDeposit() public {
        MockERC20 unsupported = new MockERC20();
        unsupported.mint(user, 100 ether);
        vm.startPrank(user);
        unsupported.approve(address(gateway), 100 ether);
        vm.expectRevert("TOKEN_NOT_SUPPORTED");
        gateway.depositToX3(address(unsupported), abi.encodePacked(recipient), 100 ether, 1);
        vm.stopPrank();
    }

    function testZeroAmountDeposit() public {
        vm.startPrank(user);
        vm.expectRevert("ZERO_AMOUNT");
        gateway.depositToX3(address(token), abi.encodePacked(recipient), 0, 1);
        vm.stopPrank();
    }

    function testRemainingCapacity() public {
        vm.startPrank(user);
        token.approve(address(gateway), 50_000 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 30_000 ether, 1);
        vm.stopPrank();

        uint256 remaining = gateway.getRemainingDailyDeposit(address(token));
        assertEq(remaining, 70_000 ether, "incorrect remaining capacity");
    }

    function testTotalLockedAccounting() public {
        vm.startPrank(user);
        token.approve(address(gateway), 100 ether);
        bytes memory x3Recipient = abi.encodePacked(recipient);
        gateway.depositToX3(address(token), x3Recipient, 100 ether, 1);
        vm.stopPrank();

        assertEq(gateway.totalLocked(address(token)), 100 ether);

        bytes32 messageId = keccak256("withdrawal_1");
        gateway.releaseFromX3(messageId, address(token), recipient, 40 ether, abi.encodePacked(user), hex"deadbeef");
        assertEq(gateway.totalLocked(address(token)), 60 ether);
    }
}