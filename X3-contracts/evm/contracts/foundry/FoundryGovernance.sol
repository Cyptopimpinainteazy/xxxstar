// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/security/Pausable.sol";

/// @title FoundryGovernance — Governance Controls for X3 Foundry
/// @notice Governance controls with proposal, voting, execution, timelock, and pause
/// @dev Timelock for fee increases, emergency pause for security, transparent proposal records
contract FoundryGovernance is Ownable, ReentrancyGuard, Pausable {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Status of a proposal
    enum ProposalStatus {
        Pending,
        Active,
        Succeeded,
        Defeated,
        Queued,
        Executed,
        Cancelled
    }

    /// @notice A governance proposal
    struct Proposal {
        uint256 id;
        address proposer;
        string title;
        string description;
        bytes[] calldatas;              // Calldata for each action
        address[] targets;              // Target addresses for each action
        uint256[] values;               // Native currency values for each action
        uint256 startTime;              // When voting starts
        uint256 endTime;                // When voting ends
        uint256 executionTime;          // When queued for execution (after timelock)
        uint256 forVotes;
        uint256 againstVotes;
        ProposalStatus status;
        bool executed;
        uint256 createdAt;
    }

    /// @notice A vote cast by a voter
    struct Vote {
        bool support;       // true = for, false = against
        uint256 weight;
        uint256 timestamp;
        bool voted;
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Default voting period (7 days)
    uint256 public constant DEFAULT_VOTING_PERIOD = 7 days;

    /// @notice Default timelock period (2 days)
    uint256 public constant DEFAULT_TIMELOCK = 2 days;

    /// @notice Minimum voting period (1 day)
    uint256 public constant MIN_VOTING_PERIOD = 1 days;

    /// @notice Maximum voting period (30 days)
    uint256 public constant MAX_VOTING_PERIOD = 30 days;

    /// @notice Minimum timelock (1 hour)
    uint256 public constant MIN_TIMELOCK = 1 hours;

    /// @notice Maximum timelock (30 days)
    uint256 public constant MAX_TIMELOCK = 30 days;

    /// @notice Maximum number of actions per proposal
    uint256 public constant MAX_ACTIONS = 10;

    /// @notice Quorum basis points (e.g., 400 = 4% of total voting power)
    uint256 public constant QUORUM_BPS = 400;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Proposal ID => Proposal
    mapping(uint256 => Proposal) private _proposals;

    /// @notice Proposal ID => voter address => Vote
    mapping(uint256 => mapping(address => Vote)) private _votes;

    /// @notice Incremental proposal ID counter
    uint256 private _proposalCount;

    /// @notice Voting period in seconds
    uint256 public votingPeriod;

    /// @notice Timelock period in seconds
    uint256 public governanceTimelock;

    /// @notice Minimum quorum (in basis points of total voting power)
    uint256 public quorumBps;

    /// @notice Total voting power (simplified - in production this would be token-based)
    uint256 public totalVotingPower;

    /// @notice Mapping of addresses with voting power
    mapping(address => uint256) public votingPower;

    /// @notice Whether fee increases are timelocked
    bool public feeIncreaseTimelocked;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a proposal is created
    event ProposalCreated(
        uint256 indexed proposalId,
        address indexed proposer,
        string title,
        uint256 startTime,
        uint256 endTime,
        uint256 timestamp
    );

    /// @notice Emitted when a vote is cast
    event VoteCast(
        uint256 indexed proposalId,
        address indexed voter,
        bool support,
        uint256 weight,
        uint256 timestamp
    );

    /// @notice Emitted when a proposal is executed
    event ProposalExecuted(uint256 indexed proposalId, uint256 timestamp);

    /// @notice Emitted when a proposal is cancelled
    event ProposalCancelled(uint256 indexed proposalId, uint256 timestamp);

    /// @notice Emitted when voting period is updated
    event VotingPeriodUpdated(uint256 oldPeriod, uint256 newPeriod, uint256 timestamp);

    /// @notice Emitted when timelock is updated
    event TimelockUpdated(uint256 oldTimelock, uint256 newTimelock, uint256 timestamp);

    /// @notice Emitted when quorum is updated
    event QuorumBpsUpdated(uint256 oldBps, uint256 newBps, uint256 timestamp);

    /// @notice Emitted when voting power is set
    event VotingPowerSet(address indexed voter, uint256 oldPower, uint256 newPower, uint256 timestamp);

    /// @notice Emitted when contract is paused/unpaused
    event PausedUpdated(bool isPaused, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyTitle();
    error NoActions();
    error TooManyActions(uint256 count, uint256 max);
    error ProposalNotFound(uint256 proposalId);
    error InvalidProposalStatus(ProposalStatus current, ProposalStatus expected);
    error VotingNotActive(uint256 now, uint256 start, uint256 end);
    error AlreadyVoted(uint256 proposalId, address voter);
    error NoVotingPower();
    error QuorumNotMet(uint256 forVotes, uint256 againstVotes, uint256 quorum);
    error TimelockNotElapsed(uint256 deadline, uint256 current);
    error ExecutionFailed(uint256 actionIndex);
    error InvalidVotingPeriod(uint256 period);
    error InvalidTimelock(uint256 timelock);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
        votingPeriod = DEFAULT_VOTING_PERIOD;
        governanceTimelock = DEFAULT_TIMELOCK;
        quorumBps = QUORUM_BPS;
        feeIncreaseTimelocked = true;
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the voting period
    /// @param newPeriod The new voting period in seconds
    function setVotingPeriod(uint256 newPeriod) external onlyOwner {
        if (newPeriod < MIN_VOTING_PERIOD || newPeriod > MAX_VOTING_PERIOD) {
            revert InvalidVotingPeriod(newPeriod);
        }
        uint256 oldPeriod = votingPeriod;
        votingPeriod = newPeriod;
        emit VotingPeriodUpdated(oldPeriod, newPeriod, block.timestamp);
    }

    /// @notice Set the timelock period
    /// @param newTimelock The new timelock in seconds
    function setTimelock(uint256 newTimelock) external onlyOwner {
        if (newTimelock < MIN_TIMELOCK || newTimelock > MAX_TIMELOCK) {
            revert InvalidTimelock(newTimelock);
        }
        uint256 oldTimelock = governanceTimelock;
        governanceTimelock = newTimelock;
        emit TimelockUpdated(oldTimelock, newTimelock, block.timestamp);
    }

    /// @notice Set quorum basis points
    /// @param newQuorumBps The new quorum in basis points
    function setQuorumBps(uint256 newQuorumBps) external onlyOwner {
        if (newQuorumBps > 5000) revert("QUORUM_TOO_HIGH"); // Max 50%
        uint256 oldBps = quorumBps;
        quorumBps = newQuorumBps;
        emit QuorumBpsUpdated(oldBps, newQuorumBps, block.timestamp);
    }

    /// @notice Set voting power for an address
    /// @param voter The voter address
    /// @param power The voting power
    function setVotingPower(address voter, uint256 power) external onlyOwner {
        if (voter == address(0)) revert ZeroAddress();
        uint256 oldPower = votingPower[voter];
        totalVotingPower = totalVotingPower - oldPower + power;
        votingPower[voter] = power;
        emit VotingPowerSet(voter, oldPower, power, block.timestamp);
    }

    /// @notice Enable/disable fee increase timelock
    /// @param locked Whether fee increases require timelock
    function setFeeIncreaseTimelock(bool locked) external onlyOwner {
        feeIncreaseTimelocked = locked;
    }

    /// @notice Pause the contract (emergency stop)
    function pause() external onlyOwner {
        _pause();
        emit PausedUpdated(true, block.timestamp);
    }

    /// @notice Unpause the contract
    function unpause() external onlyOwner {
        _unpause();
        emit PausedUpdated(false, block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Create a new governance proposal
    /// @param title Proposal title
    /// @param description Proposal description
    /// @param targets Target addresses for each action
    /// @param values Native currency values for each action
    /// @param calldatas Calldata for each action
    /// @return proposalId The new proposal ID
    function propose(
        string calldata title,
        string calldata description,
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas
    ) external whenNotPaused returns (uint256 proposalId) {
        if (bytes(title).length == 0) revert EmptyTitle();
        if (targets.length == 0) revert NoActions();
        if (targets.length > MAX_ACTIONS) revert TooManyActions(targets.length, MAX_ACTIONS);
        if (targets.length != values.length || targets.length != calldatas.length) {
            revert("ARRAY_LENGTH_MISMATCH");
        }

        // Check proposer has voting power
        if (votingPower[msg.sender] == 0) revert NoVotingPower();

        _proposalCount++;
        proposalId = _proposalCount;

        _proposals[proposalId] = Proposal({
            id: proposalId,
            proposer: msg.sender,
            title: title,
            description: description,
            calldatas: calldatas,
            targets: targets,
            values: values,
            startTime: block.timestamp,
            endTime: block.timestamp + votingPeriod,
            executionTime: 0,
            forVotes: 0,
            againstVotes: 0,
            status: ProposalStatus.Active,
            executed: false,
            createdAt: block.timestamp
        });

        emit ProposalCreated(proposalId, msg.sender, title, block.timestamp, block.timestamp + votingPeriod, block.timestamp);
    }

    /// @notice Cast a vote on a proposal
    /// @param proposalId The proposal ID
    /// @param support True for "for", false for "against"
    function vote(uint256 proposalId, bool support) external whenNotPaused {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.status != ProposalStatus.Active) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Active);
        }
        if (block.timestamp < proposal.startTime || block.timestamp > proposal.endTime) {
            revert VotingNotActive(block.timestamp, proposal.startTime, proposal.endTime);
        }
        if (_votes[proposalId][msg.sender].voted) {
            revert AlreadyVoted(proposalId, msg.sender);
        }

        uint256 weight = votingPower[msg.sender];
        if (weight == 0) revert NoVotingPower();

        _votes[proposalId][msg.sender] = Vote({
            support: support,
            weight: weight,
            timestamp: block.timestamp,
            voted: true
        });

        if (support) {
            proposal.forVotes += weight;
        } else {
            proposal.againstVotes += weight;
        }

        emit VoteCast(proposalId, msg.sender, support, weight, block.timestamp);
    }

    /// @notice Queue a successful proposal for execution (after timelock)
    /// @param proposalId The proposal ID
    function queueProposal(uint256 proposalId) external {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.status != ProposalStatus.Active) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Active);
        }
        if (block.timestamp < proposal.endTime) revert("VOTING_NOT_ENDED");

        // Check quorum
        uint256 totalVotes = proposal.forVotes + proposal.againstVotes;
        uint256 quorum = (totalVotingPower * quorumBps) / 10000;
        if (totalVotes < quorum) revert QuorumNotMet(proposal.forVotes, proposal.againstVotes, quorum);

        if (proposal.forVotes <= proposal.againstVotes) {
            proposal.status = ProposalStatus.Defeated;
            revert("PROPOSAL_DEFEATED");
        }

        proposal.status = ProposalStatus.Succeeded;
        proposal.executionTime = block.timestamp + governanceTimelock;
        proposal.status = ProposalStatus.Queued;
    }

    /// @notice Execute a queued proposal
    /// @param proposalId The proposal ID
    function execute(uint256 proposalId) external nonReentrant whenNotPaused {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.status != ProposalStatus.Queued) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Queued);
        }
        if (block.timestamp < proposal.executionTime) {
            revert TimelockNotElapsed(proposal.executionTime, block.timestamp);
        }
        if (proposal.executed) revert("ALREADY_EXECUTED");

        proposal.executed = true;
        proposal.status = ProposalStatus.Executed;

        // Execute each action
        for (uint256 i = 0; i < proposal.targets.length; i++) {
            (bool success,) = proposal.targets[i].call{value: proposal.values[i]}(proposal.calldatas[i]);
            if (!success) revert ExecutionFailed(i);
        }

        emit ProposalExecuted(proposalId, block.timestamp);
    }

    /// @notice Cancel a proposal (only proposer or owner)
    /// @param proposalId The proposal ID
    function cancelProposal(uint256 proposalId) external {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.proposer != msg.sender && owner() != msg.sender) {
            revert("NOT_AUTHORIZED");
        }
        if (proposal.status == ProposalStatus.Executed) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Executed);
        }

        proposal.status = ProposalStatus.Cancelled;

        emit ProposalCancelled(proposalId, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get proposal details by ID
    /// @param proposalId The proposal ID
    /// @return Proposal struct
    function getProposal(uint256 proposalId) external view returns (Proposal memory) {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);
        return _proposals[proposalId];
    }

    /// @notice Get total number of proposals
    /// @return count The proposal count
    function getProposalCount() external view returns (uint256) {
        return _proposalCount;
    }

    /// @notice Get vote details for a voter on a proposal
    /// @param proposalId The proposal ID
    /// @param voter The voter address
    /// @return Vote struct
    function getVote(uint256 proposalId, address voter) external view returns (Vote memory) {
        return _votes[proposalId][voter];
    }

    /// @notice Check if a voter has voted on a proposal
    /// @param proposalId The proposal ID
    /// @param voter The voter address
    /// @return True if voted
    function hasVoted(uint256 proposalId, address voter) external view returns (bool) {
        return _votes[proposalId][voter].voted;
    }

    /// @notice Get proposals by status
    /// @param status The proposal status
    /// @return proposalIds Array of proposal IDs
    function getProposalsByStatus(ProposalStatus status) external view returns (uint256[] memory proposalIds) {
        uint256 count = 0;
        for (uint256 i = 1; i <= _proposalCount; i++) {
            // slither-disable-next-line incorrect-equality
            if (_proposals[i].status == status) count++;
        }
        proposalIds = new uint256[](count);
        uint256 idx = 0;
        for (uint256 i = 1; i <= _proposalCount; i++) {
            // slither-disable-next-line incorrect-equality
            if (_proposals[i].status == status) {
                proposalIds[idx] = i;
                idx++;
            }
        }
    }

    /// @notice Get proposals by proposer
    /// @param proposer The proposer address
    /// @return proposalIds Array of proposal IDs
    function getProposalsByProposer(address proposer) external view returns (uint256[] memory proposalIds) {
        uint256 count = 0;
        for (uint256 i = 1; i <= _proposalCount; i++) {
            if (_proposals[i].proposer == proposer) count++;
        }
        proposalIds = new uint256[](count);
        uint256 idx = 0;
        for (uint256 i = 1; i <= _proposalCount; i++) {
            if (_proposals[i].proposer == proposer) {
                proposalIds[idx] = i;
                idx++;
            }
        }
    }

    /// @notice Get the current governance parameters
    /// @return _votingPeriod The voting period
    /// @return _timelock The timelock period
    /// @return _quorumBps The quorum basis points
    /// @return _totalVotingPower The total voting power
    /// @return _feeIncreaseTimelocked Whether fee increases are timelocked
    function getGovernanceParameters() external view returns (
        uint256 _votingPeriod,
        uint256 _timelock,
        uint256 _quorumBps,
        uint256 _totalVotingPower,
        bool _feeIncreaseTimelocked
    ) {
        return (votingPeriod, governanceTimelock, quorumBps, totalVotingPower, feeIncreaseTimelocked);
    }
}
