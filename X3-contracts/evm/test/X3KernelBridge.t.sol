// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/X3KernelBridge.sol";
import "../contracts/X3VmERC20.sol";

contract X3KernelBridgeTest is Test {
    X3KernelBridge public bridge;
    X3VmERC20 public token;
    address public kernel = address(this);
    address public attacker = address(0xBAD);
    address public user = address(0x123);
    bytes32 public constant ASSET_ID = keccak256("X3.test.asset");

    function setUp() public {
        bridge = new X3KernelBridge();
        token = new X3VmERC20(
            "X3 Test Token",
            "X3TT",
            18,
            ASSET_ID,
            1,
            999,
            address(0),
            address(bridge)
        );
        bridge.registerTokenAdapter(ASSET_ID, address(token));
    }

    function testConstructor() public {
        assertTrue(bridge.hasRole(bridge.KERNEL_ROLE(), address(this)));
        assertTrue(bridge.hasRole(bridge.DEFAULT_ADMIN_ROLE(), address(this)));
    }

    function testRegisterTokenAdapter() public {
        address newAdapter = address(0x777);
        bytes32 newAsset = keccak256("X3.new.asset");
        bridge.registerTokenAdapter(newAsset, newAdapter);
        assertEq(bridge.tokenAdapters(newAsset), newAdapter);
    }

    function testRegisterTokenAdapterZeroAddress() public {
        vm.expectRevert("ZERO_ADAPTER");
        bridge.registerTokenAdapter(keccak256("X3.zero"), address(0));
    }

    function testRegisterTokenAdapterAlreadyRegistered() public {
        vm.expectRevert("ALREADY_REGISTERED");
        bridge.registerTokenAdapter(ASSET_ID, address(token));
    }

    function testRegisterTokenAdapterUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        bridge.registerTokenAdapter(keccak256("X3.bad"), address(0x777));
    }

    function testRegisterExternalGateway() public {
        address gateway = address(0x888);
        bridge.registerExternalGateway(8453, gateway);
        assertEq(bridge.externalGateways(8453), gateway);
    }

    function testRegisterExternalGatewayZeroAddress() public {
        vm.expectRevert("ZERO_GATEWAY");
        bridge.registerExternalGateway(8453, address(0));
    }

    function testRegisterExternalGatewayUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        bridge.registerExternalGateway(8453, address(0x888));
    }

    function testCreditUser() public {
        bytes32 msgId = keccak256("credit_1");
        vm.expectEmit(true, true, true, true);
        emit X3KernelBridge.CrossVmTransferCompleted(msgId, ASSET_ID, user, 100 ether);
        bool result = bridge.creditUser(msgId, ASSET_ID, user, 100 ether);
        assertTrue(result);
        assertEq(token.balanceOf(user), 100 ether);
    }

    function testCreditUserNoAdapter() public {
        bytes32 badAsset = keccak256("X3.unknown");
        vm.expectRevert("NO_ADAPTER");
        bridge.creditUser(keccak256("credit_2"), badAsset, user, 100 ether);
    }

    function testCreditUserZeroRecipient() public {
        vm.expectRevert("ZERO_RECIPIENT");
        bridge.creditUser(keccak256("credit_3"), ASSET_ID, address(0), 100 ether);
    }

    function testCreditUserZeroAmount() public {
        vm.expectRevert("ZERO_AMOUNT");
        bridge.creditUser(keccak256("credit_4"), ASSET_ID, user, 0);
    }

    function testCreditUserUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        bridge.creditUser(keccak256("credit_5"), ASSET_ID, user, 100 ether);
    }

    function testDebitUser() public {
        bytes32 msgId = keccak256("debit_1");
        bridge.creditUser(keccak256("credit_before"), ASSET_ID, user, 100 ether);

        bool result = bridge.debitUser(msgId, ASSET_ID, user, 50 ether);
        assertTrue(result);
        assertEq(token.balanceOf(user), 50 ether);
    }

    function testDebitUserNoAdapter() public {
        bytes32 badAsset = keccak256("X3.unknown");
        vm.expectRevert("NO_ADAPTER");
        bridge.debitUser(keccak256("debit_2"), badAsset, user, 100 ether);
    }

    function testDebitUserZeroUser() public {
        vm.expectRevert("ZERO_USER");
        bridge.debitUser(keccak256("debit_3"), ASSET_ID, address(0), 100 ether);
    }

    function testDebitUserZeroAmount() public {
        vm.expectRevert("ZERO_AMOUNT");
        bridge.debitUser(keccak256("debit_4"), ASSET_ID, user, 0);
    }

    function testDebitUserUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        bridge.debitUser(keccak256("debit_5"), ASSET_ID, user, 100 ether);
    }

    function testGetAdapter() public {
        assertEq(bridge.getAdapter(ASSET_ID), address(token));
    }

    function testGetGateway() public {
        address gateway = address(0x888);
        bridge.registerExternalGateway(8453, gateway);
        assertEq(bridge.getGateway(8453), gateway);
    }
}
