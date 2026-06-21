// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "../contracts/staking/StakingPool.sol";

contract MockERC20 is ERC20("Mock", "MCK") {
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract StakingPoolTest is Test {
    using stdStorage for StdStorage;

    StakingPool public pool;
    MockERC20 public stakingToken;
    StakingNFT public nft;
    address public treasury = address(0xAAA);
    address public user = address(0x123);
    address public attacker = address(0xBAD);

    function setUp() public {
        stakingToken = new MockERC20();
        pool = new StakingPool(address(stakingToken), treasury);
        nft = StakingNFT(pool.stakingNFT());

        stakingToken.mint(user, 100_000 ether);
        stakingToken.mint(address(pool), 1_000_000 ether);
        vm.prank(user);
        stakingToken.approve(address(pool), type(uint256).max);
    }

    function _getUserNFTs(address userAddr, uint256 index) internal view returns (uint256) {
        return pool.userNFTs(userAddr, index);
    }

    function _getUserNFTCount(address userAddr) internal view returns (uint256) {
        bytes32 slot = keccak256(abi.encode(userAddr, uint256(8)));
        return uint256(vm.load(address(pool), slot));
    }

    function _getUserNFTId(address userAddr, uint256 index) internal view returns (uint256) {
        bytes32 baseSlot = keccak256(abi.encode(userAddr, uint256(8)));
        bytes32 arraySlot = keccak256(abi.encode(baseSlot));
        return uint256(vm.load(address(pool), bytes32(uint256(arraySlot) + index)));
    }

    function testConstructor() public {
        assertEq(address(pool.stakingToken()), address(stakingToken));
        assertEq(pool.treasury(), treasury);
        assertEq(pool.rewardRate(), 1e18);
        assertEq(pool.totalStaked(), 0);
        assertEq(pool.owner(), address(this));
        assertTrue(address(pool.stakingNFT()) != address(0));
    }

    function testSetRewardRate() public {
        pool.setRewardRate(2e18);
        assertEq(pool.rewardRate(), 2e18);
    }

    function testSetRewardRateUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        pool.setRewardRate(2e18);
    }

    function testStake() public {
        vm.prank(user);
        pool.stake(100 ether);

        assertEq(pool.totalStaked(), 100 ether);
        uint256 nftCount = _getUserNFTCount(user);
        assertEq(nftCount, 1);
        uint256 nftId = _getUserNFTId(user, 0);
        assertEq(pool.stakeAmount(nftId), 100 ether);
        assertEq(nft.ownerOf(nftId), user);

        assertEq(stakingToken.balanceOf(address(pool)), 1_000_000 ether + 100 ether);
    }

    function testStakeMultiple() public {
        vm.startPrank(user);
        pool.stake(100 ether);
        pool.stake(200 ether);
        vm.stopPrank();

        assertEq(pool.totalStaked(), 300 ether);
        uint256 nftCount = _getUserNFTCount(user);
        assertEq(nftCount, 2);
        assertEq(pool.stakeAmount(_getUserNFTId(user, 0)), 100 ether);
        assertEq(pool.stakeAmount(_getUserNFTId(user, 1)), 200 ether);
    }

    function testUnstake() public {
        vm.prank(user);
        pool.stake(100 ether);

        assertEq(pool.totalStaked(), 100 ether);

        uint256 nftId = _getUserNFTId(user, 0);

        vm.prank(user);
        pool.unstake(nftId);

        assertEq(pool.totalStaked(), 0);
        vm.expectRevert();
        nft.ownerOf(nftId);
        assertEq(stakingToken.balanceOf(user), 100_000 ether);
    }

    function testUnstakeNotOwner() public {
        vm.prank(user);
        pool.stake(100 ether);

        uint256 nftId = _getUserNFTId(user, 0);

        vm.prank(attacker);
        vm.expectRevert("Not owner");
        pool.unstake(nftId);
    }

    function testClaim() public {
        vm.prank(user);
        pool.stake(100 ether);

        vm.roll(block.number + 100);

        uint256 nftId = _getUserNFTId(user, 0);
        uint256 balanceBefore = stakingToken.balanceOf(user);

        vm.prank(user);
        pool.claim(nftId);

        uint256 balanceAfter = stakingToken.balanceOf(user);
        assertGt(balanceAfter, balanceBefore);
    }

    function testClaimNotOwner() public {
        vm.prank(user);
        pool.stake(100 ether);

        uint256 nftId = _getUserNFTId(user, 0);

        vm.prank(attacker);
        vm.expectRevert("Not owner");
        pool.claim(nftId);
    }

    function testUnstakeWithRewards() public {
        vm.prank(user);
        pool.stake(100 ether);

        vm.roll(block.number + 100);

        uint256 nftId = _getUserNFTId(user, 0);
        uint256 balanceBefore = stakingToken.balanceOf(user);

        vm.prank(user);
        pool.unstake(nftId);

        uint256 balanceAfter = stakingToken.balanceOf(user);
        assertGt(balanceAfter, balanceBefore);
        assertEq(pool.totalStaked(), 0);
    }
}
