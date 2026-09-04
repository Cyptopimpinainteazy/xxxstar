// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/WrappedX3.sol";

contract WrappedX3Test is Test {
    WrappedX3 public wX3;
    address public treasury = address(0xAAA);
    address public adapter = address(0xBBB);
    address public user = address(0x123);
    address public user2 = address(0x456);
    address public attacker = address(0xBAD);
    uint256 public constant CHAIN_ID = 8453;

    function setUp() public {
        wX3 = new WrappedX3(treasury, adapter, CHAIN_ID);
    }

    function testConstructor() public {
        assertEq(wX3.name(), "Wrapped X3");
        assertEq(wX3.symbol(), "wX3");
        assertEq(wX3.treasury(), treasury);
        assertEq(wX3.adapter(), adapter);
        assertEq(wX3.chainId(), CHAIN_ID);
        assertEq(wX3.treasuryFeeBps(), 50);
        assertEq(wX3.owner(), address(this));
    }

    function testConstructorZeroTreasury() public {
        vm.expectRevert("ZERO_TREASURY");
        new WrappedX3(address(0), adapter, CHAIN_ID);
    }

    function testConstructorZeroAdapter() public {
        vm.expectRevert("ZERO_ADAPTER");
        new WrappedX3(treasury, address(0), CHAIN_ID);
    }

    function testSetTreasury() public {
        address newTreasury = address(0xCCC);
        wX3.setTreasury(newTreasury);
        assertEq(wX3.treasury(), newTreasury);
    }

    function testSetTreasuryUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        wX3.setTreasury(address(0xCCC));
    }

    function testSetAdapter() public {
        address newAdapter = address(0xDDD);
        wX3.setAdapter(newAdapter);
        assertEq(wX3.adapter(), newAdapter);
    }

    function testSetAdapterUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        wX3.setAdapter(address(0xDDD));
    }

    function testSetFee() public {
        wX3.setFee(100);
        assertEq(wX3.treasuryFeeBps(), 100);
    }

    function testSetFeeUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        wX3.setFee(100);
    }

    function testSetFeeExempt() public {
        wX3.setFeeExempt(user, true);
        assertTrue(wX3.feeExempt(user));

        wX3.setFeeExempt(user, false);
        assertFalse(wX3.feeExempt(user));
    }

    function testSetFeeExemptUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        wX3.setFeeExempt(user, true);
    }

    function testMint() public {
        uint256 amount = 100 ether;
        uint256 fee = (amount * 50) / 10000;
        wX3.mint(user, amount);
        assertEq(wX3.balanceOf(user), amount - fee);
    }

    function testMintWithFee() public {
        uint256 amount = 100 ether;
        uint256 fee = (amount * 50) / 10000;
        wX3.mint(user, amount);
        assertEq(wX3.balanceOf(user), amount - fee);
        assertEq(wX3.balanceOf(treasury), fee);
    }

    function testMintFeeExempt() public {
        wX3.setFeeExempt(user, true);
        wX3.mint(user, 100 ether);
        assertEq(wX3.balanceOf(user), 100 ether);
        assertEq(wX3.balanceOf(treasury), 0);
    }

    function testMintUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        wX3.mint(user, 100 ether);
    }

    function testMintZeroAddress() public {
        vm.expectRevert("ZERO_TO");
        wX3.mint(address(0), 100 ether);
    }

    function testMintZeroAmount() public {
        vm.expectRevert("ZERO_AMOUNT");
        wX3.mint(user, 0);
    }

    function testBurn() public {
        uint256 mintAmount = 100 ether;
        uint256 fee = (mintAmount * 50) / 10000;
        wX3.mint(user, mintAmount);
        uint256 balanceAfterMint = mintAmount - fee;
        wX3.burn(user, 50 ether);
        assertEq(wX3.balanceOf(user), balanceAfterMint - 50 ether);
    }

    function testBurnUnauthorized() public {
        wX3.mint(user, 100 ether);
        vm.prank(attacker);
        vm.expectRevert();
        wX3.burn(user, 50 ether);
    }

    function testBurnZeroAddress() public {
        vm.expectRevert("ZERO_FROM");
        wX3.burn(address(0), 100 ether);
    }

    function testBurnZeroAmount() public {
        vm.expectRevert("ZERO_AMOUNT");
        wX3.burn(user, 0);
    }

    function testBurnInsufficientBalance() public {
        vm.expectRevert();
        wX3.burn(user, 100 ether);
    }
}
