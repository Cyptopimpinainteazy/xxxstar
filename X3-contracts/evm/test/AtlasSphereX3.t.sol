// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/AtlasSphereX3.sol";

contract AtlasSphereX3Test is Test {
    AtlasSphereX3 public token;
    address public treasury = address(0xAAA);
    address public user = address(0x123);
    address public user2 = address(0x456);
    address public attacker = address(0xBAD);

    function setUp() public {
        token = new AtlasSphereX3(treasury);
        token.setFeeExempt(treasury, true);
    }

    function testConstructor() public {
        assertEq(token.name(), "Atlas Sphere X3");
        assertEq(token.symbol(), "X3");
        assertEq(token.treasury(), treasury);
        assertEq(token.transferFeeBps(), 50);
        assertEq(token.stakingFeeBps(), 100);
        assertEq(token.swapFeeBps(), 25);
        assertEq(token.balanceOf(treasury), 1_000_000_000 ether);
        assertEq(token.owner(), address(this));
    }

    function testSetTreasury() public {
        address newTreasury = address(0xBBB);
        token.setTreasury(newTreasury);
        assertEq(token.treasury(), newTreasury);
    }

    function testSetTreasuryUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.setTreasury(address(0xBBB));
    }

    function testSetFee() public {
        token.setFee("transfer", 100);
        assertEq(token.transferFeeBps(), 100);

        token.setFee("staking", 200);
        assertEq(token.stakingFeeBps(), 200);

        token.setFee("swap", 50);
        assertEq(token.swapFeeBps(), 50);
    }

    function testSetFeeInvalid() public {
        vm.expectRevert("Invalid fee type");
        token.setFee("invalid", 100);
    }

    function testSetFeeUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.setFee("transfer", 100);
    }

    function testSetFeeExempt() public {
        token.setFeeExempt(user, true);
        assertTrue(token.feeExempt(user));

        token.setFeeExempt(user, false);
        assertFalse(token.feeExempt(user));
    }

    function testSetFeeExemptUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.setFeeExempt(user, true);
    }

    function testTransferWithFee() public {
        vm.prank(treasury);
        token.transfer(user, 1000 ether);

        assertEq(token.balanceOf(user), 1000 ether);

        vm.prank(user);
        token.transfer(user2, 100 ether);

        uint256 fee = (100 ether * 50) / 10000;
        assertEq(token.balanceOf(user2), 100 ether - fee);
        assertEq(token.balanceOf(treasury), 1_000_000_000 ether - 1000 ether + fee);
    }

    function testTransferFeeExempt() public {
        vm.prank(treasury);
        token.transfer(user, 1000 ether);

        token.setFeeExempt(user, true);

        vm.prank(user);
        token.transfer(user2, 100 ether);

        assertEq(token.balanceOf(user2), 100 ether);
    }

    function testTransferZeroFee() public {
        vm.prank(treasury);
        token.transfer(user, 1000 ether);

        token.setFee("transfer", 0);

        vm.prank(user);
        token.transfer(user2, 100 ether);

        assertEq(token.balanceOf(user2), 100 ether);
    }

    function testPause() public {
        token.pause();
        vm.prank(treasury);
        vm.expectRevert();
        token.transfer(user, 100 ether);
    }

    function testUnpause() public {
        token.pause();
        token.unpause();

        vm.prank(treasury);
        token.transfer(user, 100 ether);
        assertEq(token.balanceOf(user), 100 ether);
    }

    function testPauseUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.pause();
    }

    function testMint() public {
        token.mint(user, 100 ether);
        assertEq(token.balanceOf(user), 100 ether);
    }

    function testMintUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        token.mint(user, 100 ether);
    }

    function testBurnFrom() public {
        token.mint(user, 100 ether);
        token.burnFrom(user, 50 ether);
        assertEq(token.balanceOf(user), 50 ether);
    }

    function testBurnFromUnauthorized() public {
        token.mint(user, 100 ether);
        vm.prank(attacker);
        vm.expectRevert();
        token.burnFrom(user, 50 ether);
    }

    function testBurn() public {
        token.mint(user, 100 ether);
        vm.prank(user);
        token.burn(50 ether);
        assertEq(token.balanceOf(user), 50 ether);
    }
}
