// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/governance/CrossChainGovernance.sol";
import "../contracts/AtlasSphereX3.sol";
import "../contracts/WrappedX3.sol";

contract CrossChainGovernanceTest is Test {
    CrossChainGovernance public governance;
    AtlasSphereX3 public x3Token;
    WrappedX3 public wX3;
    address public treasury = address(0xAAA);
    address public voter = address(0x123);
    address public voter2 = address(0x456);
    address public attacker = address(0xBAD);

    function setUp() public {
        x3Token = new AtlasSphereX3(treasury);
        x3Token.setFeeExempt(treasury, true);
        governance = new CrossChainGovernance(address(x3Token), treasury);

        vm.prank(treasury);
        x3Token.transfer(voter, 100 ether);
        vm.prank(treasury);
        x3Token.transfer(voter2, 200 ether);
    }

    function testConstructor() public {
        assertEq(address(governance.x3()), address(x3Token));
        assertEq(governance.treasury(), treasury);
        assertEq(governance.proposalCount(), 0);
        assertEq(governance.owner(), address(this));
    }

    function testAddWrapped() public {
        wX3 = new WrappedX3(treasury, address(this), 8453);
        governance.addWrapped(8453, address(wX3));
        assertEq(address(governance.wrappedTokens(8453)), address(wX3));
    }

    function testAddWrappedUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        governance.addWrapped(8453, address(0x888));
    }

    function testCreateProposal() public {
        governance.createProposal("Test proposal", 100);
        assertEq(governance.proposalCount(), 1);

        (string memory desc, uint256 voteStart, uint256 voteEnd, uint256 forVotes, uint256 againstVotes, bool executed) =
            governance.proposals(1);
        assertEq(desc, "Test proposal");
        assertEq(voteEnd, block.number + 100);
        assertEq(forVotes, 0);
        assertEq(againstVotes, 0);
        assertFalse(executed);
    }

    function testCreateProposalUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        governance.createProposal("Bad proposal", 100);
    }

    function testVoteFor() public {
        governance.createProposal("Test", 100);

        vm.prank(voter);
        governance.vote(1, true);

        (, , , uint256 forVotes, uint256 againstVotes, ) = governance.proposals(1);
        assertEq(forVotes, 100 ether);
        assertEq(againstVotes, 0);
    }

    function testVoteAgainst() public {
        governance.createProposal("Test", 100);

        vm.prank(voter);
        governance.vote(1, false);

        (, , , uint256 forVotes, uint256 againstVotes, ) = governance.proposals(1);
        assertEq(forVotes, 0);
        assertEq(againstVotes, 100 ether);
    }

    function testVoteWithWrappedTokens() public {
        uint256 wX3Amount = 50 ether;
        wX3 = new WrappedX3(treasury, address(this), 1);
        wX3.mint(voter, wX3Amount);
        uint256 wX3Balance = wX3.balanceOf(voter);
        governance.addWrapped(1, address(wX3));

        governance.createProposal("Test", 100);

        vm.prank(voter);
        governance.vote(1, true);

        (, , , uint256 forVotes, , ) = governance.proposals(1);
        assertEq(forVotes, 100 ether + wX3Balance);
    }

    function testVoteDoubleVote() public {
        governance.createProposal("Test", 100);

        vm.prank(voter);
        governance.vote(1, true);

        vm.prank(voter);
        vm.expectRevert("Already voted");
        governance.vote(1, false);
    }

    function testVoteNoVotingPower() public {
        governance.createProposal("Test", 100);

        vm.prank(attacker);
        vm.expectRevert("No voting power");
        governance.vote(1, true);
    }

    function testVoteOutsideWindow() public {
        governance.createProposal("Test", 100);

        vm.roll(block.number + 101);

        vm.prank(voter);
        vm.expectRevert("Voting closed");
        governance.vote(1, true);
    }

    function testVoteBeforeStart() public {
        governance.createProposalWithStart("Test", block.number + 10, 100);

        vm.prank(voter);
        vm.expectRevert("Voting not started");
        governance.vote(1, true);
    }

    function testExecute() public {
        governance.createProposal("Test", 100);
        vm.roll(block.number + 101);
        governance.execute(1);

        (, , , , , bool executed) = governance.proposals(1);
        assertTrue(executed);
    }

    function testExecuteUnauthorized() public {
        governance.createProposal("Test", 100);
        vm.roll(block.number + 101);

        vm.prank(attacker);
        vm.expectRevert();
        governance.execute(1);
    }

    function testExecuteAlreadyExecuted() public {
        governance.createProposal("Test", 100);
        vm.roll(block.number + 101);
        governance.execute(1);

        vm.expectRevert("Already executed");
        governance.execute(1);
    }

    function testExecuteVotingNotEnded() public {
        governance.createProposal("Test", 100);
        vm.expectRevert("Voting not ended");
        governance.execute(1);
    }

    function testMultipleVoters() public {
        governance.createProposal("Test", 100);

        vm.prank(voter);
        governance.vote(1, true);

        vm.prank(voter2);
        governance.vote(1, false);

        (, , , uint256 forVotes, uint256 againstVotes, ) = governance.proposals(1);
        assertEq(forVotes, 100 ether);
        assertEq(againstVotes, 200 ether);
    }

    /// @notice Fuzz: voting weight equals voter's X3 balance
    function testFuzz_VoteWeightEqualsBalance(uint64 _amt) public {
        uint256 amt = uint256(_amt) % 900_000_000 ether;
        vm.assume(amt > 0);

        address fuzzVoter = address(0xFACE);
        vm.prank(treasury);
        x3Token.transfer(fuzzVoter, amt);

        uint256 expectedWeight = x3Token.balanceOf(fuzzVoter);

        governance.createProposal("Test", 100);

        vm.prank(fuzzVoter);
        governance.vote(1, true);

        (, , , uint256 forVotes, , ) = governance.proposals(1);
        assertEq(forVotes, expectedWeight);
    }

    /// @notice Fuzz: create proposals with arbitrary durations
    function testFuzz_CreateProposalDuration(uint256 duration) public {
        duration = bound(duration, 1, 100_000);
        governance.createProposal("Fuzz", duration);

        (, , uint256 voteEnd, , , ) = governance.proposals(1);
        assertEq(voteEnd, block.number + duration);
    }

    /// @notice Fuzz: vote count accumulates across multiple voters
    function testFuzz_MultipleVotes(uint256 votes) public {
        votes = bound(votes, 1, 50);
        governance.createProposal("Fuzz", 1000);

        for (uint256 i = 0; i < votes; i++) {
            address newVoter = address(uint160(0x1000 + i));
            vm.prank(treasury);
            x3Token.transfer(newVoter, 1 ether);
            vm.prank(newVoter);
            governance.vote(1, i % 2 == 0);
        }

        (, , , uint256 forVotes, uint256 againstVotes, ) = governance.proposals(1);
        assertEq(forVotes + againstVotes, votes * 1 ether);
    }
}
