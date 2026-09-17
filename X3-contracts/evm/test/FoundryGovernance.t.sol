// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/foundry/FoundryGovernance.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @notice Mock governance token for testing
contract MockGovToken is ERC20 {
    constructor() ERC20("Governance Token", "GOV") {}
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract FoundryGovernanceTest is Test {
    FoundryGovernance public governance;
    MockGovToken public govToken;

    // Emergency signers (3 default keys)
    uint256 public signer1Key;
    uint256 public signer2Key;
    uint256 public signer3Key;
    address public signer1;
    address public signer2;
    address public signer3;

    // Governance members
    address public voter1 = address(0x101);
    address public voter2 = address(0x102);
    address public voter3 = address(0x103);
    address public voter4 = address(0x104);
    address public attacker = address(0xBAD);
    address public treasury = address(0xAAA);

    // Test proposal parameters
    address[] public targets;
    uint256[] public values;
    bytes[] public calldatas;

    function setUp() public {
        signer1Key = 0x1001;
        signer2Key = 0x1002;
        signer3Key = 0x1003;
        signer1 = vm.addr(signer1Key);
        signer2 = vm.addr(signer2Key);
        signer3 = vm.addr(signer3Key);

        govToken = new MockGovToken();

        address[] memory signers = new address[](3);
        signers[0] = signer1;
        signers[1] = signer2;
        signers[2] = signer3;
        governance = new FoundryGovernance(address(govToken), signers);

        govToken.mint(voter1, 100 ether);
        govToken.mint(voter2, 200 ether);
        govToken.mint(voter3, 50 ether);
        govToken.mint(voter4, 150 ether);

        targets.push(treasury);
        values.push(1 ether);
        calldatas.push(hex"");
    }

    // ── Constructor Tests ──────────────────────────────────────────────

    function testConstructor() public {
        assertEq(governance.votingPeriod(), 7 days);
        assertEq(governance.quorumBps(), 1000);
        assertEq(governance.approvalThresholdBps(), 5000);
        assertEq(address(governance.governanceToken()), address(govToken));
        assertTrue(governance.tokenVotingEnabled());
        assertEq(governance.getEmergencySigners().length, 3);
        assertTrue(governance.isEmergencySigner(signer1));
        assertTrue(governance.isEmergencySigner(signer2));
        assertTrue(governance.isEmergencySigner(signer3));
        assertEq(governance.emergencyThreshold(), 3);
        assertEq(governance.owner(), address(this));
    }

    function testConstructorRequiresMin3Signers() public {
        address[] memory tooFew = new address[](2);
        tooFew[0] = signer1;
        tooFew[1] = signer2;
        vm.expectRevert("NEED_AT_LEAST_3_SIGNERS");
        new FoundryGovernance(address(0), tooFew);
    }

    function testConstructorRejectsZeroSigner() public {
        address[] memory signers = new address[](3);
        signers[0] = signer1;
        signers[1] = address(0);
        signers[2] = signer3;
        vm.expectRevert("ZERO_SIGNER");
        new FoundryGovernance(address(0), signers);
    }

    function testConstructorRejectsDuplicateSigner() public {
        address[] memory signers = new address[](3);
        signers[0] = signer1;
        signers[1] = signer1;
        signers[2] = signer3;
        vm.expectRevert("DUPLICATE_SIGNER");
        new FoundryGovernance(address(0), signers);
    }

    // ── Proposal Creation Tests ─────────────────────────────────────────

    function testCreateProposal() public {
        vm.prank(voter1);
        uint256 id = governance.propose(
            "Update Fee Config",
            "Set platform min fee to 250 bps",
            targets, values, calldatas,
            FoundryGovernance.ProposalCategory.FeeIncrease
        );

        assertEq(id, 1);
        FoundryGovernance.Proposal memory prop = governance.getProposal(1);

        assertEq(prop.proposer, voter1);
        assertEq(prop.title, "Update Fee Config");
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Active));
        assertEq(uint256(prop.category), uint256(FoundryGovernance.ProposalCategory.FeeIncrease));
        assertEq(prop.endTime, block.timestamp + 7 days);
        assertEq(prop.forVotes, 0);
        assertEq(prop.againstVotes, 0);
        assertEq(prop.abstainVotes, 0);
        assertFalse(prop.executed);
        assertEq(prop.depositAmount, 0);
    }

    function testCreateProposalFailsNoVotingPower() public {
        vm.prank(attacker);
        vm.expectRevert(FoundryGovernance.NoVotingPower.selector);
        governance.propose(
            "Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    function testCreateProposalFailsEmptyTitle() public {
        vm.prank(voter1);
        vm.expectRevert(FoundryGovernance.EmptyTitle.selector);
        governance.propose(
            "", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    function testCreateProposalFailsNoActions() public {
        address[] memory emptyTargets = new address[](0);
        uint256[] memory emptyValues = new uint256[](0);
        bytes[] memory emptyCalldatas = new bytes[](0);

        vm.prank(voter1);
        vm.expectRevert(FoundryGovernance.NoActions.selector);
        governance.propose(
            "Test", "", emptyTargets, emptyValues, emptyCalldatas,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    function testCreateProposalFailsTooManyActions() public {
        address[] memory manyTargets = new address[](11);
        uint256[] memory manyValues = new uint256[](11);
        bytes[] memory manyCalldatas = new bytes[](11);
        for (uint256 i = 0; i < 11; i++) {
            manyTargets[i] = treasury;
            manyCalldatas[i] = hex"";
        }

        vm.prank(voter1);
        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.TooManyActions.selector, 11, 10
        ));
        governance.propose(
            "Test", "", manyTargets, manyValues, manyCalldatas,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    function testCreateProposalFailsMismatchedArrays() public {
        address[] memory badTargets = new address[](2);
        badTargets[0] = treasury;
        badTargets[1] = treasury;
        uint256[] memory oneValue = new uint256[](1);
        oneValue[0] = 0;
        bytes[] memory oneCalldata = new bytes[](1);
        oneCalldata[0] = hex"";

        vm.prank(voter1);
        vm.expectRevert(FoundryGovernance.ArrayLengthMismatch.selector);
        governance.propose(
            "Test", "", badTargets, oneValue, oneCalldata,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    function testCreateProposalWithDeposit() public {
        governance.setProposalDeposit(10 ether);

        vm.prank(voter1);
        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.InsufficientDeposit.selector, 10 ether, 0
        ));
        governance.propose(
            "Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );

        vm.deal(voter1, 20 ether);
        vm.prank(voter1);
        uint256 id = governance.propose{value: 10 ether}(
            "Test with deposit", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );

        FoundryGovernance.Proposal memory depositProp = governance.getProposal(id);
        assertEq(depositProp.depositAmount, 10 ether);
    }

    function testCreateProposalCannotBeEmergency() public {
        vm.prank(voter1);
        vm.expectRevert("EMERGENCY_REQUIRES_MULTISIG");
        governance.propose(
            "Emergency", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Emergency
        );
    }

    function testCreateProposalWhenPaused() public {
        uint256 nonce = governance.emergencyNonce();
        bytes[] memory sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        governance.emergencyPause(nonce, sigs);
        assertTrue(governance.paused());

        vm.prank(voter1);
        vm.expectRevert("Pausable: paused");
        governance.propose(
            "Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );
    }

    // ── Voting Tests ────────────────────────────────────────────────────

    function testVoteFor() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.forVotes, 100 ether);
        assertEq(prop.againstVotes, 0);
        assertEq(prop.abstainVotes, 0);
        assertTrue(governance.hasVoted(1, voter1));
    }

    function testVoteAgainst() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.Against);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.forVotes, 0);
        assertEq(prop.againstVotes, 100 ether);
        assertEq(prop.abstainVotes, 0);
    }

    function testVoteAbstain() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.Abstain);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.forVotes, 0);
        assertEq(prop.againstVotes, 0);
        assertEq(prop.abstainVotes, 100 ether);
    }

    function testVoteDoubleVote() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.prank(voter1);
        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.AlreadyVoted.selector, 1, voter1
        ));
        governance.vote(1, FoundryGovernance.VoteType.Against);
    }

    function testVoteOutsideWindow() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.warp(block.timestamp + 8 days);

        vm.prank(voter1);
        vm.expectRevert();
        governance.vote(1, FoundryGovernance.VoteType.For);
    }

    function testVoteInvalidProposalId() public {
        vm.prank(voter1);
        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.ProposalNotFound.selector, 0
        ));
        governance.vote(0, FoundryGovernance.VoteType.For);
    }

    function testVoteNonexistentProposal() public {
        vm.prank(voter1);
        vm.expectRevert();
        governance.vote(999, FoundryGovernance.VoteType.For);
    }

    function testVoteMultipleVoters() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.Against);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.Abstain);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.forVotes, 100 ether);
        assertEq(prop.againstVotes, 200 ether);
        assertEq(prop.abstainVotes, 50 ether);
    }

    // ── Queue Tests ─────────────────────────────────────────────────────

    function testQueueProposalSucceeds() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Queued));
        assertEq(prop.executionTime, block.timestamp + 2 days);
    }

    function testQueueProposalFeeIncreaseTimelock() public {
        vm.prank(voter1);
        governance.propose("Fee increase", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.FeeIncrease);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.executionTime, block.timestamp + 7 days);
    }

    function testQueueProposalFeeDecreaseTimelock() public {
        vm.prank(voter1);
        governance.propose("Fee decrease", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.FeeDecrease);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.executionTime, block.timestamp + 1 days);
    }

    function testQueueProposalFailsQuorumNotMet() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        governance.setQuorumBps(2000);

        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);

        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.QuorumNotMet.selector, 50 ether, 100 ether
        ));
        governance.queueProposal(1);
    }

    function testQueueProposalFailsDefeated() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.Against);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.Against);

        vm.warp(block.timestamp + 8 days);

        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.ProposalDefeated.selector, 150 ether, 350 ether
        ));
        governance.queueProposal(1);
    }

    function testQueueProposalVotingNotEnded() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.expectRevert("VOTING_NOT_ENDED");
        governance.queueProposal(1);
    }

    function testQueueProposalWithAbstainVotes() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.Against);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.Abstain);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.Abstain);

        vm.warp(block.timestamp + 8 days);

        // Decisive: 100 for vs 200 against -> defeated
        vm.expectRevert();
        governance.queueProposal(1);
    }

    // ── Execute Tests ───────────────────────────────────────────────────

    function testExecuteProposal() public {
        vm.deal(address(governance), 10 ether);

        address[] memory sendTargets = new address[](1);
        uint256[] memory sendValues = new uint256[](1);
        bytes[] memory sendCalldatas = new bytes[](1);
        sendTargets[0] = treasury;
        sendValues[0] = 1 ether;
        sendCalldatas[0] = hex"";

        vm.prank(voter1);
        governance.propose("Send funds", "", sendTargets, sendValues, sendCalldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);
        vm.warp(block.timestamp + 3 days);

        uint256 treasuryBalanceBefore = treasury.balance;
        governance.execute(1);
        uint256 treasuryBalanceAfter = treasury.balance;

        assertEq(treasuryBalanceAfter - treasuryBalanceBefore, 1 ether);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertTrue(prop.executed);
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Executed));
    }

    function testExecuteFailsTimelockNotElapsed() public {
        vm.deal(address(governance), 10 ether);

        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.FeeIncrease);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);
        vm.warp(block.timestamp + 1 days);

        vm.expectRevert();
        governance.execute(1);
    }

    function testExecuteFailsAlreadyExecuted() public {
        vm.deal(address(governance), 10 ether);

        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter2);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter3);
        governance.vote(1, FoundryGovernance.VoteType.For);
        vm.prank(voter4);
        governance.vote(1, FoundryGovernance.VoteType.For);

        vm.warp(block.timestamp + 8 days);
        governance.queueProposal(1);
        vm.warp(block.timestamp + 3 days);
        governance.execute(1);

        vm.expectRevert(FoundryGovernance.AlreadyExecuted.selector);
        governance.execute(1);
    }

    function testExecuteFailsWrongStatus() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.expectRevert();
        governance.execute(1);
    }

    // ── Cancel Tests ────────────────────────────────────────────────────

    function testCancelProposalByProposer() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter1);
        governance.cancelProposal(1);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Cancelled));
    }

    function testCancelProposalByOwner() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        governance.cancelProposal(1);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Cancelled));
    }

    function testCancelProposalUnauthorized() public {
        vm.prank(voter1);
        governance.propose("Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(voter2);
        vm.expectRevert(FoundryGovernance.NotAuthorized.selector);
        governance.cancelProposal(1);
    }

    function testCancelProposalForfeitsDeposit() public {
        governance.setProposalDeposit(5 ether);

        vm.deal(voter1, 10 ether);
        vm.prank(voter1);
        governance.propose{value: 5 ether}(
            "Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard
        );

        uint256 contractBalanceBefore = address(governance).balance;

        vm.prank(voter1);
        governance.cancelProposal(1);

        assertEq(address(governance).balance, contractBalanceBefore);
    }

    // ── Emergency Multisig Helpers ──────────────────────────────────────

    function _signEmergency(
        uint256 key1, uint256 key2, uint256 key3, uint256 nonce
    ) internal view returns (bytes[] memory sigs) {
        bytes32 digest = keccak256(abi.encodePacked(
            "\x19Ethereum Signed Message:\n32",
            keccak256(abi.encodePacked(nonce, block.chainid, address(governance)))
        ));

        sigs = new bytes[](3);
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(key1, digest);
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(key2, digest);
        (uint8 v3, bytes32 r3, bytes32 s3) = vm.sign(key3, digest);

        bytes memory sig1 = abi.encodePacked(r1, s1, v1);
        bytes memory sig2 = abi.encodePacked(r2, s2, v2);
        bytes memory sig3 = abi.encodePacked(r3, s3, v3);

        address a1 = vm.addr(key1);
        address a2 = vm.addr(key2);
        address a3 = vm.addr(key3);

        // Sort sigs by ascending recovered address (required by contract dedup logic)
        (address[3] memory addrs, bytes[3] memory sigBytes) =
            _sortThree(a1, sig1, a2, sig2, a3, sig3);

        sigs[0] = sigBytes[0];
        sigs[1] = sigBytes[1];
        sigs[2] = sigBytes[2];
    }

    function _sortThree(
        address a1, bytes memory s1, address a2, bytes memory s2, address a3, bytes memory s3
    ) internal pure returns (address[3] memory addrs, bytes[3] memory sigBytes) {
        // Simple bubble sort for 3 elements by address ascending
        if (a1 > a2) { (a1, a2) = (a2, a1); (s1, s2) = (s2, s1); }
        if (a2 > a3) { (a2, a3) = (a3, a2); (s2, s3) = (s3, s2); }
        if (a1 > a2) { (a1, a2) = (a2, a1); (s1, s2) = (s2, s1); }
        addrs[0] = a1; addrs[1] = a2; addrs[2] = a3;
        sigBytes[0] = s1; sigBytes[1] = s2; sigBytes[2] = s3;
    }

    function testEmergencyPause() public {
        uint256 nonce = governance.emergencyNonce();
        bytes[] memory sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        assertFalse(governance.paused());
        governance.emergencyPause(nonce, sigs);
        assertTrue(governance.paused());
    }

    function testEmergencyUnpause() public {
        uint256 nonce = governance.emergencyNonce();
        bytes[] memory sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        governance.emergencyPause(nonce, sigs);
        assertTrue(governance.paused());

        nonce = governance.emergencyNonce();
        sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        governance.emergencyUnpause(nonce, sigs);
        assertFalse(governance.paused());
    }

    function testEmergencyExecute() public {
        vm.deal(address(governance), 10 ether);

        uint256 nonce = governance.emergencyNonce();
        bytes[] memory sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        uint256 treasuryBalanceBefore = treasury.balance;

        governance.emergencyExecute(
            targets, values, calldatas,
            "Emergency refund", "Refunding user after exploit",
            nonce, sigs
        );

        assertEq(treasury.balance - treasuryBalanceBefore, 1 ether);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.title, "Emergency refund");
        assertEq(uint256(prop.status), uint256(FoundryGovernance.ProposalStatus.Executed));
        assertEq(uint256(prop.category), uint256(FoundryGovernance.ProposalCategory.Emergency));
        assertTrue(prop.executed);
    }

    function testEmergencyPauseFailsMissingSignatures() public {
        uint256 nonce = governance.emergencyNonce();
        bytes32 digest = keccak256(abi.encodePacked(
            "\x19Ethereum Signed Message:\n32",
            keccak256(abi.encodePacked(nonce, block.chainid, address(governance)))
        ));
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(signer1Key, digest);

        bytes[] memory sigs = new bytes[](1);
        sigs[0] = abi.encodePacked(r1, s1, v1);

        vm.expectRevert();
        governance.emergencyPause(nonce, sigs);
    }

    function testEmergencyPauseFailsBadNonce() public {
        uint256 nonce = governance.emergencyNonce() + 1;
        bytes[] memory sigs = _signEmergency(signer1Key, signer2Key, signer3Key, nonce);

        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.InvalidEmergencyNonce.selector, 0, 1
        ));
        governance.emergencyPause(nonce, sigs);
    }

    function testEmergencyPauseFailsNonSigner() public {
        uint256 nonce = governance.emergencyNonce();
        bytes32 digest = keccak256(abi.encodePacked(
            "\x19Ethereum Signed Message:\n32",
            keccak256(abi.encodePacked(nonce, block.chainid, address(governance)))
        ));

        uint256 badKey = 0x9999;
        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(signer1Key, digest);
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(signer2Key, digest);
        (uint8 v3, bytes32 r3, bytes32 s3) = vm.sign(badKey, digest);

        bytes[] memory sigs = new bytes[](3);
        sigs[0] = abi.encodePacked(r1, s1, v1);
        sigs[1] = abi.encodePacked(r2, s2, v2);
        sigs[2] = abi.encodePacked(r3, s3, v3);

        vm.expectRevert(FoundryGovernance.NotEmergencySigner.selector);
        governance.emergencyPause(nonce, sigs);
    }

    function testEmergencyPauseFailsDuplicateSigners() public {
        uint256 nonce = governance.emergencyNonce();
        bytes32 digest = keccak256(abi.encodePacked(
            "\x19Ethereum Signed Message:\n32",
            keccak256(abi.encodePacked(nonce, block.chainid, address(governance)))
        ));

        (uint8 v1, bytes32 r1, bytes32 s1) = vm.sign(signer1Key, digest);
        (uint8 v2, bytes32 r2, bytes32 s2) = vm.sign(signer1Key, digest);
        (uint8 v3, bytes32 r3, bytes32 s3) = vm.sign(signer2Key, digest);

        bytes[] memory sigs = new bytes[](3);
        sigs[0] = abi.encodePacked(r1, s1, v1);
        sigs[1] = abi.encodePacked(r2, s2, v2);
        sigs[2] = abi.encodePacked(r3, s3, v3);

        vm.expectRevert("DUPLICATE_OR_UNORDERED");
        governance.emergencyPause(nonce, sigs);
    }

    // ── Governance Parameter Tests ──────────────────────────────────────

    function testSetVotingPeriod() public {
        governance.setVotingPeriod(14 days);
        assertEq(governance.votingPeriod(), 14 days);
    }

    function testSetVotingPeriodFailsOutOfRange() public {
        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.InvalidVotingPeriod.selector, 12 hours
        ));
        governance.setVotingPeriod(12 hours);

        vm.expectRevert(abi.encodeWithSelector(
            FoundryGovernance.InvalidVotingPeriod.selector, 31 days
        ));
        governance.setVotingPeriod(31 days);
    }

    function testSetQuorumBps() public {
        governance.setQuorumBps(2000);
        assertEq(governance.quorumBps(), 2000);
    }

    function testSetQuorumBpsFailsOutOfRange() public {
        vm.expectRevert(FoundryGovernance.InvalidParameter.selector);
        governance.setQuorumBps(100);
    }

    function testSetApprovalThreshold() public {
        governance.setApprovalThresholdBps(6000);
        assertEq(governance.approvalThresholdBps(), 6000);
    }

    function testSetProposalDeposit() public {
        governance.setProposalDeposit(100 ether);
        assertEq(governance.proposalDeposit(), 100 ether);
    }

    function testSetGovernanceToken() public {
        governance.setGovernanceToken(address(0));
        assertFalse(governance.tokenVotingEnabled());

        governance.setGovernanceToken(address(govToken));
        assertTrue(governance.tokenVotingEnabled());
    }

    // ── Emergency Multisig Management Tests ─────────────────────────────

    function testAddEmergencySigner() public {
        address signer4 = address(0x400);
        governance.addEmergencySigner(signer4);

        assertTrue(governance.isEmergencySigner(signer4));
        assertEq(governance.getEmergencySigners().length, 4);
        assertEq(governance.emergencyThreshold(), 3); // (4*2)/3 + 1 = 2 + 1 = 3
    }

    function testRemoveEmergencySigner() public {
        governance.addEmergencySigner(address(0x400));
        governance.removeEmergencySigner(signer3);

        assertFalse(governance.isEmergencySigner(signer3));
        assertEq(governance.getEmergencySigners().length, 3);
        assertEq(governance.emergencyThreshold(), 3);
    }

    function testRemoveEmergencySignerFailsMin3() public {
        vm.expectRevert("MIN_3_SIGNERS");
        governance.removeEmergencySigner(signer1);
    }

    // ── Integration Pointer Tests ───────────────────────────────────────

    function testSetIntegrationPointers() public {
        governance.setFeeConfig(address(0x300));
        governance.setTemplateRegistry(address(0x301));
        governance.setMarketplace(address(0x302));
        governance.setDisputeResolver(address(0x303));

        assertEq(governance.feeConfig(), address(0x300));
        assertEq(governance.templateRegistry(), address(0x301));
        assertEq(governance.marketplace(), address(0x302));
        assertEq(governance.disputeResolver(), address(0x303));
    }

    // ── Manual Voting Power Tests ───────────────────────────────────────

    function testManualVotingPower() public {
        governance.setGovernanceToken(address(0));

        governance.setManualVotingPower(voter1, 100);
        governance.setManualVotingPower(voter2, 200);

        assertEq(governance.getVotingPower(voter1), 100);
        assertEq(governance.getVotingPower(voter2), 200);
        assertEq(governance.getTotalVotingPower(), 300);
    }

    function testManualVotingPowerBatch() public {
        governance.setGovernanceToken(address(0));

        address[] memory voters = new address[](2);
        voters[0] = voter1;
        voters[1] = voter2;
        uint256[] memory powers = new uint256[](2);
        powers[0] = 50;
        powers[1] = 75;

        governance.batchSetManualVotingPower(voters, powers);

        assertEq(governance.getVotingPower(voter1), 50);
        assertEq(governance.getVotingPower(voter2), 75);
        assertEq(governance.getTotalVotingPower(), 125);
    }

    function testManualVotingPowerFailsWhenTokenActive() public {
        vm.expectRevert("TOKEN_VOTING_ACTIVE");
        governance.setManualVotingPower(voter1, 100);
    }

    // ── View Function Tests ─────────────────────────────────────────────

    function testGetProposalCount() public {
        assertEq(governance.getProposalCount(), 0);

        vm.prank(voter1);
        governance.propose("P1", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);
        assertEq(governance.getProposalCount(), 1);

        vm.prank(voter2);
        governance.propose("P2", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);
        assertEq(governance.getProposalCount(), 2);
    }

    function testGetProposalState() public {
        vm.prank(voter1);
        governance.propose("P1", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        assertEq(uint256(governance.getProposalState(1)),
            uint256(FoundryGovernance.ProposalStatus.Active));

        vm.warp(block.timestamp + 8 days);
        assertEq(uint256(governance.getProposalState(1)),
            uint256(FoundryGovernance.ProposalStatus.Succeeded));
    }

    function testGetProposalsByStatus() public {
        vm.prank(voter1);
        governance.propose("P1", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);
        vm.prank(voter2);
        governance.propose("P2", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        uint256[] memory active = governance.getProposalsByStatus(
            FoundryGovernance.ProposalStatus.Active
        );
        assertEq(active.length, 2);
        assertEq(active[0], 1);
        assertEq(active[1], 2);

        uint256[] memory executed = governance.getProposalsByStatus(
            FoundryGovernance.ProposalStatus.Executed
        );
        assertEq(executed.length, 0);
    }

    function testGetProposalsByProposer() public {
        vm.prank(voter1);
        governance.propose("P1", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);
        vm.prank(voter2);
        governance.propose("P2", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);
        vm.prank(voter1);
        governance.propose("P3", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        uint256[] memory v1Props = governance.getProposalsByProposer(voter1);
        assertEq(v1Props.length, 2);
        assertEq(v1Props[0], 1);
        assertEq(v1Props[1], 3);

        uint256[] memory v2Props = governance.getProposalsByProposer(voter2);
        assertEq(v2Props.length, 1);
        assertEq(v2Props[0], 2);
    }

    function testGetGovernanceParameters() public {
        (
            uint256 _votingPeriod,
            uint256 _quorumBps,
            uint256 _approvalThresholdBps,
            uint256 _proposalDeposit,
            uint256 _proposalCount,
            bool _tokenVotingEnabled,
            uint256 _emergencyThreshold,
            uint256 _emergencySignerCount
        ) = governance.getGovernanceParameters();

        assertEq(_votingPeriod, 7 days);
        assertEq(_quorumBps, 1000);
        assertEq(_approvalThresholdBps, 5000);
        assertEq(_proposalDeposit, 0);
        assertEq(_proposalCount, 0);
        assertTrue(_tokenVotingEnabled);
        assertEq(_emergencyThreshold, 3);
        assertEq(_emergencySignerCount, 3);
    }

    function testTimelockForCategory() public {
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.Standard), 2 days);
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.FeeIncrease), 7 days);
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.FeeDecrease), 1 days);
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.Emergency), 0);
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.Template), 2 days);
        assertEq(governance.getTimelockForCategory(FoundryGovernance.ProposalCategory.Marketplace), 2 days);
    }

    // ── Fuzz Tests ──────────────────────────────────────────────────────

    function testFuzz_ProposalCount(uint8 n) public {
        vm.assume(n > 0 && n <= 50);
        for (uint8 i = 0; i < n; i++) {
            vm.prank(voter1);
            governance.propose(
                string(abi.encodePacked("Proposal ", i)), "",
                targets, values, calldatas,
                FoundryGovernance.ProposalCategory.Standard
            );
        }
        assertEq(governance.getProposalCount(), n);
    }

    function testFuzz_VoteWeightUsesTokenBalance(uint64 amount) public {
        uint256 amt = uint256(amount) % 1000 ether;
        vm.assume(amt > 0);

        address fuzzVoter = address(0xFACE);
        govToken.mint(fuzzVoter, amt);
        uint256 expectedWeight = govToken.balanceOf(fuzzVoter);

        vm.prank(voter1);
        governance.propose("Fuzz Test", "", targets, values, calldatas,
            FoundryGovernance.ProposalCategory.Standard);

        vm.prank(fuzzVoter);
        governance.vote(1, FoundryGovernance.VoteType.For);

        FoundryGovernance.Proposal memory prop = governance.getProposal(1);
        assertEq(prop.forVotes, expectedWeight);
    }

    function testFuzz_QuorumCalculation(uint64 supplyAmount, uint16 quorumBps_) public {
        supplyAmount = uint64(bound(uint256(supplyAmount), 1 ether, 1_000_000 ether));
        quorumBps_ = uint16(bound(uint256(quorumBps_), 500, 5000));

        address[] memory signers = new address[](3);
        signers[0] = signer1;
        signers[1] = signer2;
        signers[2] = signer3;
        FoundryGovernance gov = new FoundryGovernance(address(0), signers);
        gov.setGovernanceToken(address(0));
        gov.setQuorumBps(quorumBps_);
        gov.setManualVotingPower(voter1, supplyAmount);

        uint256 totalSupply = gov.getTotalVotingPower();
        assertEq(totalSupply, supplyAmount);

        uint256 expectedQuorum = (supplyAmount * uint256(quorumBps_)) / 10000;
        assertEq(expectedQuorum, (totalSupply * quorumBps_) / 10000);
    }

    function testFuzz_VotingPeriodInRange(uint64 period) public {
        period = uint64(bound(uint256(period), 1 days, 14 days));
        governance.setVotingPeriod(period);
        assertEq(governance.votingPeriod(), period);
    }
}