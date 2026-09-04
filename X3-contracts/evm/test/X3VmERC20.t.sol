// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/X3VmERC20.sol";

contract X3VmERC20Test is Test {
    X3VmERC20 public token;
    address public kernel = address(0xAAA);
    address public bridge = address(0xBBB);
    address public user = address(0x123);
    address public attacker = address(0xBAD);

    bytes32 public constant ASSET_ID = keccak256("X3.test.asset");
    uint8 public constant ORIGIN_DOMAIN = 1;
    uint256 public constant ORIGIN_CHAIN_ID = 999;
    address public constant ORIGIN_TOKEN = address(0);

    function setUp() public {
        token = new X3VmERC20(
            "X3 Test Token",
            "X3TT",
            18,
            ASSET_ID,
            ORIGIN_DOMAIN,
            ORIGIN_CHAIN_ID,
            ORIGIN_TOKEN,
            kernel
        );
    }

    function testConstructor() public {
        assertEq(token.name(), "X3 Test Token");
        assertEq(token.symbol(), "X3TT");
        assertEq(token.decimals(), 18);
        assertEq(token.assetId(), ASSET_ID);
        assertEq(token.originDomain(), ORIGIN_DOMAIN);
        assertEq(token.originChainId(), ORIGIN_CHAIN_ID);
        assertEq(token.originToken(), ORIGIN_TOKEN);
        assertTrue(token.hasRole(token.KERNEL_ROLE(), kernel));
        assertTrue(token.hasRole(token.DEFAULT_ADMIN_ROLE(), kernel));
    }

    function testConstructorZeroKernel() public {
        vm.expectRevert("ZERO_KERNEL");
        new X3VmERC20("X", "X", 18, ASSET_ID, 1, 1, address(0), address(0));
    }

    function testKernelMint() public {
        vm.prank(kernel);
        token.kernelMint(user, 100 ether);
        assertEq(token.balanceOf(user), 100 ether);
    }

    function testKernelMintZeroAddress() public {
        vm.prank(kernel);
        vm.expectRevert("ZERO_TO");
        token.kernelMint(address(0), 100 ether);
    }

    function testKernelMintZeroAmount() public {
        vm.prank(kernel);
        vm.expectRevert("ZERO_AMOUNT");
        token.kernelMint(user, 0);
    }

    function testKernelMintUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.kernelMint(user, 100 ether);
    }

    function testKernelBurn() public {
        vm.prank(kernel);
        token.kernelMint(user, 100 ether);

        vm.prank(kernel);
        token.kernelBurn(user, 50 ether);
        assertEq(token.balanceOf(user), 50 ether);
    }

    function testKernelBurnZeroAddress() public {
        vm.prank(kernel);
        vm.expectRevert("ZERO_FROM");
        token.kernelBurn(address(0), 100 ether);
    }

    function testKernelBurnZeroAmount() public {
        vm.prank(kernel);
        vm.expectRevert("ZERO_AMOUNT");
        token.kernelBurn(user, 0);
    }

    function testKernelBurnUnauthorized() public {
        vm.prank(kernel);
        token.kernelMint(user, 100 ether);

        vm.prank(attacker);
        vm.expectRevert();
        token.kernelBurn(user, 50 ether);
    }

    function testBridgeMint() public {
        vm.prank(kernel);
        token.grantBridgeRole(bridge);

        vm.prank(bridge);
        token.bridgeMint(user, 100 ether);
        assertEq(token.balanceOf(user), 100 ether);
    }

    function testBridgeMintUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.bridgeMint(user, 100 ether);
    }

    function testBridgeBurn() public {
        vm.prank(kernel);
        token.grantBridgeRole(bridge);
        vm.prank(bridge);
        token.bridgeMint(user, 100 ether);

        vm.prank(bridge);
        token.bridgeBurn(user, 50 ether);
        assertEq(token.balanceOf(user), 50 ether);
    }

    function testSendToVm() public {
        vm.prank(kernel);
        token.kernelMint(user, 100 ether);

        vm.prank(user);
        token.sendToVm(2, abi.encodePacked(address(0x456)), 50 ether);
        assertEq(token.balanceOf(user), 50 ether);
    }

    function testSendToVmInsufficientBalance() public {
        vm.prank(user);
        vm.expectRevert("INSUFFICIENT_BALANCE");
        token.sendToVm(2, abi.encodePacked(address(0x456)), 100 ether);
    }

    function testSendToVmInvalidRecipient() public {
        vm.prank(kernel);
        token.kernelMint(user, 100 ether);

        vm.prank(user);
        vm.expectRevert("INVALID_RECIPIENT");
        token.sendToVm(2, bytes(""), 50 ether);
    }

    function testGrantBridgeRole() public {
        vm.prank(kernel);
        token.grantBridgeRole(bridge);
        assertTrue(token.hasRole(token.BRIDGE_ROLE(), bridge));
    }

    function testGrantBridgeRoleUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.grantBridgeRole(bridge);
    }

    function testRevokeBridgeRole() public {
        vm.prank(kernel);
        token.grantBridgeRole(bridge);
        assertTrue(token.hasRole(token.BRIDGE_ROLE(), bridge));

        vm.prank(kernel);
        token.revokeBridgeRole(bridge);
        assertFalse(token.hasRole(token.BRIDGE_ROLE(), bridge));
    }
}
