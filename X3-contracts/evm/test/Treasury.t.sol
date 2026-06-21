// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/treasury/Treasury.sol";

contract TreasuryTest is Test {
    Treasury public treasury;
    address public dev = payable(address(0xDDD));
    address public dao = payable(address(0xAAA));
    address public lp = payable(address(0x3));
    address public user = address(0x123);
    address public attacker = address(0xBAD);

    function setUp() public {
        treasury = new Treasury(dev, dao, lp);
        vm.deal(address(treasury), 100 ether);
    }

    function testConstructor() public {
        assertEq(treasury.devWallet(), dev);
        assertEq(treasury.daoWallet(), dao);
        assertEq(treasury.lpWallet(), lp);
        assertEq(treasury.devSplitBps(), 2000);
        assertEq(treasury.daoSplitBps(), 5000);
        assertEq(treasury.lpSplitBps(), 3000);
        assertEq(treasury.owner(), address(this));
    }

    function testConstructorZeroDev() public {
        vm.expectRevert("ZERO_DEV");
        new Treasury(address(0), dao, lp);
    }

    function testConstructorZeroDao() public {
        vm.expectRevert("ZERO_DAO");
        new Treasury(dev, address(0), lp);
    }

    function testConstructorZeroLp() public {
        vm.expectRevert("ZERO_LP");
        new Treasury(dev, dao, address(0));
    }

    function testSetSplits() public {
        treasury.setSplits(3000, 4000, 3000);
        assertEq(treasury.devSplitBps(), 3000);
        assertEq(treasury.daoSplitBps(), 4000);
        assertEq(treasury.lpSplitBps(), 3000);
    }

    function testSetSplitsInvalidSum() public {
        vm.expectRevert("Must sum to 10000");
        treasury.setSplits(3000, 3000, 3000);
    }

    function testSetSplitsUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        treasury.setSplits(3000, 4000, 3000);
    }

    function testRouteFeeOnlyOwner() public {
        vm.prank(attacker);
        vm.expectRevert();
        treasury.routeFee(user, 1 ether, "test");
    }

    function testRouteFee() public {
        uint256 devBefore = dev.balance;
        uint256 daoBefore = dao.balance;
        uint256 lpBefore = lp.balance;

        treasury.routeFee(user, 100 ether, "fees");

        uint256 expectedDev = (100 ether * 2000) / 10000;
        uint256 expectedDao = (100 ether * 5000) / 10000;
        uint256 expectedLp = 100 ether - expectedDev - expectedDao;

        assertEq(dev.balance - devBefore, expectedDev);
        assertEq(dao.balance - daoBefore, expectedDao);
        assertEq(lp.balance - lpBefore, expectedLp);
        assertEq(treasury.accounting(user), 100 ether);
    }

    function testRouteFeeZeroFrom() public {
        vm.expectRevert("ZERO_FROM");
        treasury.routeFee(address(0), 1 ether, "test");
    }

    function testRouteFeeZeroAmount() public {
        vm.expectRevert("ZERO_AMOUNT");
        treasury.routeFee(user, 0, "test");
    }

    function testReceiveEther() public {
        uint256 contractBalanceBefore = address(treasury).balance;
        (bool ok, ) = address(treasury).call{value: 1 ether}("");
        assertTrue(ok);
        assertEq(address(treasury).balance, contractBalanceBefore + 1 ether);
    }
}
