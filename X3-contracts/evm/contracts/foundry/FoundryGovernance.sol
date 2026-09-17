// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/// @title FoundryGovernance — Governance Controls for X3 Foundry
/// @notice Transparent, timelock-protected governance for platform parameters,
///         template approvals, fee configurations, and dispute resolution.
/// @dev Features: 2/3 multisig emergency pause, tiered timelocks (7d for fees,
///      48h standard, 0 for emergency), deposit-gated proposals, abstain votes,
///      token-weighted voting, and integration with FeeConfig / TemplateRegistry /
///      Marketplace / DisputeResolver.
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

    /// @notice Category of a proposal — determines timelock duration
    enum ProposalCategory {
        Standard,       // 48h timelock
        FeeIncrease,    // 7d timelock
        FeeDecrease,    // 24h timelock
        Emergency,      // 0 timelock (requires 2/3 multisig)
        Template,       // 48h timelock
        Marketplace     // 48h timelock
    }

    /// @notice Vote option
    enum VoteType {
        Against,
        For,
        Abstain
    }

    /// @notice A governance proposal
    struct Proposal {
        uint256 id;
        address proposer;
        string title;
        string description;
        bytes[] calldatas;
        address[] targets;
        uint256[] values;
        uint256 startTime;
        uint256 endTime;
        uint256 executionTime;
        uint256 forVotes;
        uint256 againstVotes;
        uint256 abstainVotes;
        ProposalStatus status;
        ProposalCategory category;
        bool executed;
        uint256 createdAt;
        uint256 depositAmount;       // Deposit locked during proposal
    }

    /// @notice A vote cast by a voter
    struct Vote {
        VoteType voteType;
        uint256 weight;
        uint256 timestamp;
        bool voted;
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Default voting period (7 days)
    uint256 public constant DEFAULT_VOTING_PERIOD = 7 days;

    /// @notice Standard timelock (48 hours)
    uint256 public constant STANDARD_TIMELOCK = 2 days;

    /// @notice Fee increase timelock (7 days)
    uint256 public constant FEE_INCREASE_TIMELOCK = 7 days;

    /// @notice Fee decrease timelock (24 hours)
    uint256 public constant FEE_DECREASE_TIMELOCK = 1 days;

    /// @notice Emergency timelock (0 — no delay)
    uint256 public constant EMERGENCY_TIMELOCK = 0;

    /// @notice Minimum voting period (1 day)
    uint256 public constant MIN_VOTING_PERIOD = 1 days;

    /// @notice Maximum voting period (14 days)
    uint256 public constant MAX_VOTING_PERIOD = 14 days;

    /// @notice Minimum timelock (1 hour)
    uint256 public constant MIN_TIMELOCK = 1 hours;

    /// @notice Maximum timelock (30 days)
    uint256 public constant MAX_TIMELOCK = 30 days;

    /// @notice Maximum number of actions per proposal
    uint256 public constant MAX_ACTIONS = 10;

    /// @notice Basis points denominator
    uint256 public constant BPS_DENOMINATOR = 10000;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Proposal ID => Proposal
    mapping(uint256 => Proposal) private _proposals;

    /// @notice Proposal ID => voter address => Vote
    mapping(uint256 => mapping(address => Vote)) private _votes;

    /// @notice Incremental proposal ID counter
    uint256 private _proposalCount;

    /// @notice Voting period in seconds
    uint256 public votingPeriod;

    /// @notice Minimum quorum in basis points (e.g., 1000 = 10%)
    uint256 public quorumBps;

    /// @notice Approval threshold: percentage of (for + against) that must be "for"
    ///         e.g., 5000 = majority (>50% of for+against)
    uint256 public approvalThresholdBps;

    /// @notice Minimum deposit to create a proposal (in governance token wei)
    uint256 public proposalDeposit;

    /// @notice Governance token address (voting power = token balance)
    IERC20 public governanceToken;

    /// @notice Whether governance token mode is active
    bool public tokenVotingEnabled;

    // ── Multisig Emergency ───────────────────────────────────────────────────

    /// @notice Emergency multisig signers
    address[] public emergencySigners;

    /// @notice Whether an address is an emergency signer
    mapping(address => bool) public isEmergencySigner;

    /// @notice Required signatures for emergency action (must be > 2/3 of signers)
    uint256 public emergencyThreshold;

    /// @notice Nonce for emergency proposals (prevents replay)
    uint256 public emergencyNonce;

    // ── Integration Pointers ─────────────────────────────────────────────────

    /// @notice Fee configuration contract
    address public feeConfig;

    /// @notice Template registry contract
    address public templateRegistry;

    /// @notice Marketplace contract
    address public marketplace;

    /// @notice Dispute resolver contract
    address public disputeResolver;

    // ── Events ───────────────────────────────────────────────────────────────

    event ProposalCreated(
        uint256 indexed proposalId,
        address indexed proposer,
        string title,
        ProposalCategory category,
        uint256 startTime,
        uint256 endTime,
        uint256 depositAmount,
        uint256 timestamp
    );

    event VoteCast(
        uint256 indexed proposalId,
        address indexed voter,
        VoteType voteType,
        uint256 weight,
        uint256 timestamp
    );

    event ProposalQueued(uint256 indexed proposalId, uint256 executionTime, uint256 timestamp);
    event ProposalExecuted(uint256 indexed proposalId, uint256 timestamp);
    event ProposalCancelled(uint256 indexed proposalId, address cancelledBy, uint256 timestamp);

    event VotingPeriodUpdated(uint256 oldPeriod, uint256 newPeriod, uint256 timestamp);
    event QuorumUpdated(uint256 oldBps, uint256 newBps, uint256 timestamp);
    event ApprovalThresholdUpdated(uint256 oldBps, uint256 newBps, uint256 timestamp);
    event ProposalDepositUpdated(uint256 oldDeposit, uint256 newDeposit, uint256 timestamp);
    event GovernanceTokenSet(address indexed oldToken, address indexed newToken, uint256 timestamp);

    event EmergencySignerAdded(address indexed signer, uint256 timestamp);
    event EmergencySignerRemoved(address indexed signer, uint256 timestamp);
    event EmergencyThresholdUpdated(uint256 oldThreshold, uint256 newThreshold, uint256 timestamp);
    event EmergencyExecuted(
        uint256 indexed proposalId,
        uint256 nonce,
        uint256 timestamp
    );

    event FeeConfigSet(address indexed oldConfig, address indexed newConfig, uint256 timestamp);
    event TemplateRegistrySet(address indexed oldRegistry, address indexed newRegistry, uint256 timestamp);
    event MarketplaceSet(address indexed oldMarketplace, address indexed newMarketplace, uint256 timestamp);
    event DisputeResolverSet(address indexed oldResolver, address indexed newResolver, uint256 timestamp);

    event PausedUpdated(bool isPaused, address pausedBy, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyTitle();
    error NoActions();
    error TooManyActions(uint256 count, uint256 max);
    error ArrayLengthMismatch();
    error ProposalNotFound(uint256 proposalId);
    error InvalidProposalStatus(ProposalStatus current, ProposalStatus expected);
    error VotingNotActive(uint256 now_, uint256 start, uint256 end);
    error AlreadyVoted(uint256 proposalId, address voter);
    error NoVotingPower();
    error InsufficientDeposit(uint256 required, uint256 provided);
    error QuorumNotMet(uint256 totalVotes, uint256 quorumRequired);
    error ProposalDefeated(uint256 forVotes, uint256 againstVotes);
    error TimelockNotElapsed(uint256 deadline, uint256 current);
    error ExecutionFailed(uint256 actionIndex);
    error AlreadyExecuted();
    error NotAuthorized();
    error InvalidVotingPeriod(uint256 period);
    error InvalidParameter();
    error EmergencyThresholdTooLow(uint256 threshold, uint256 required);
    error NotEmergencySigner();
    error InsufficientEmergencySignatures(uint256 have, uint256 need);
    error InvalidEmergencyNonce(uint256 expected, uint256 provided);

    // ── Modifiers ────────────────────────────────────────────────────────────

    /// @notice Restrict to governance (requires a passed governance proposal to change parameters)
    ///         For initial bootstrapping, owner can also call.
    modifier onlyGovernance() {
        // In production this would check if caller is a passed governance proposal.
        // For bootstrapping, owner acts as governance.
        require(msg.sender == owner(), "ONLY_GOVERNANCE");
        _;
    }

    // ── Constructor ──────────────────────────────────────────────────────────

    /// @param _governanceToken Address of the governance token (0 for manual voting power)
    /// @param _emergencySigners Array of initial emergency multisig signer addresses
    constructor(address _governanceToken, address[] memory _emergencySigners) {
        _transferOwnership(msg.sender);

        votingPeriod = DEFAULT_VOTING_PERIOD;
        quorumBps = 1000;   // 10% quorum
        approvalThresholdBps = 5000; // >50% majority
        proposalDeposit = 0; // No deposit by default

        // Governance token setup
        if (_governanceToken != address(0)) {
            governanceToken = IERC20(_governanceToken);
            tokenVotingEnabled = true;
        }

        // Emergency multisig setup
        require(_emergencySigners.length >= 3, "NEED_AT_LEAST_3_SIGNERS");
        for (uint256 i = 0; i < _emergencySigners.length; i++) {
            require(_emergencySigners[i] != address(0), "ZERO_SIGNER");
            require(!isEmergencySigner[_emergencySigners[i]], "DUPLICATE_SIGNER");
            isEmergencySigner[_emergencySigners[i]] = true;
            emergencySigners.push(_emergencySigners[i]);
        }
        emergencyThreshold = (_emergencySigners.length * 2) / 3 + 1; // >2/3
    }

    // ── Governance Parameter Updates (onlyGovernance) ────────────────────────

    /// @notice Set the voting period
    function setVotingPeriod(uint256 newPeriod) external onlyGovernance {
        if (newPeriod < MIN_VOTING_PERIOD || newPeriod > MAX_VOTING_PERIOD) {
            revert InvalidVotingPeriod(newPeriod);
        }
        uint256 oldPeriod = votingPeriod;
        votingPeriod = newPeriod;
        emit VotingPeriodUpdated(oldPeriod, newPeriod, block.timestamp);
    }

    /// @notice Set the quorum requirement (basis points of total supply)
    function setQuorumBps(uint256 newQuorumBps) external onlyGovernance {
        if (newQuorumBps < 500 || newQuorumBps > 5000) revert InvalidParameter(); // 5%–50%
        uint256 oldBps = quorumBps;
        quorumBps = newQuorumBps;
        emit QuorumUpdated(oldBps, newQuorumBps, block.timestamp);
    }

    /// @notice Set the approval threshold (basis points of for+against that must be "for")
    function setApprovalThresholdBps(uint256 newThresholdBps) external onlyGovernance {
        if (newThresholdBps < 5000 || newThresholdBps > 10000) revert InvalidParameter(); // 50%–100%
        uint256 oldBps = approvalThresholdBps;
        approvalThresholdBps = newThresholdBps;
        emit ApprovalThresholdUpdated(oldBps, newThresholdBps, block.timestamp);
    }

    /// @notice Set the minimum deposit required to create a proposal
    function setProposalDeposit(uint256 newDeposit) external onlyGovernance {
        uint256 oldDeposit = proposalDeposit;
        proposalDeposit = newDeposit;
        emit ProposalDepositUpdated(oldDeposit, newDeposit, block.timestamp);
    }

    /// @notice Set the governance token (onlyGovernance)
    function setGovernanceToken(address newToken) external onlyGovernance {
        address oldToken = address(governanceToken);
        governanceToken = IERC20(newToken);
        tokenVotingEnabled = (newToken != address(0));
        emit GovernanceTokenSet(oldToken, newToken, block.timestamp);
    }

    // ── Emergency Multisig Management (onlyGovernance) ───────────────────────

    /// @notice Add an emergency signer
    function addEmergencySigner(address signer) external onlyGovernance {
        if (signer == address(0)) revert ZeroAddress();
        if (isEmergencySigner[signer]) revert("ALREADY_SIGNER");
        isEmergencySigner[signer] = true;
        emergencySigners.push(signer);
        _recalcEmergencyThreshold();
        emit EmergencySignerAdded(signer, block.timestamp);
    }

    /// @notice Remove an emergency signer
    function removeEmergencySigner(address signer) external onlyGovernance {
        if (!isEmergencySigner[signer]) revert("NOT_SIGNER");
        require(emergencySigners.length > 3, "MIN_3_SIGNERS");
        isEmergencySigner[signer] = false;

        for (uint256 i = 0; i < emergencySigners.length; i++) {
            if (emergencySigners[i] == signer) {
                emergencySigners[i] = emergencySigners[emergencySigners.length - 1];
                emergencySigners.pop();
                break;
            }
        }
        _recalcEmergencyThreshold();
        emit EmergencySignerRemoved(signer, block.timestamp);
    }

    function _recalcEmergencyThreshold() internal {
        uint256 oldThreshold = emergencyThreshold;
        emergencyThreshold = (emergencySigners.length * 2) / 3 + 1;
        emit EmergencyThresholdUpdated(oldThreshold, emergencyThreshold, block.timestamp);
    }

    // ── Integration Pointers (onlyGovernance) ────────────────────────────────

    function setFeeConfig(address _feeConfig) external onlyGovernance {
        emit FeeConfigSet(feeConfig, _feeConfig, block.timestamp);
        feeConfig = _feeConfig;
    }

    function setTemplateRegistry(address _templateRegistry) external onlyGovernance {
        emit TemplateRegistrySet(templateRegistry, _templateRegistry, block.timestamp);
        templateRegistry = _templateRegistry;
    }

    function setMarketplace(address _marketplace) external onlyGovernance {
        emit MarketplaceSet(marketplace, _marketplace, block.timestamp);
        marketplace = _marketplace;
    }

    function setDisputeResolver(address _disputeResolver) external onlyGovernance {
        emit DisputeResolverSet(disputeResolver, _disputeResolver, block.timestamp);
        disputeResolver = _disputeResolver;
    }

    // ── Emergency Pause (2/3 Multisig) ──────────────────────────────────────

    /// @notice Pause all governance operations — requires 2/3 multisig
    function emergencyPause(
        uint256 nonce,
        bytes[] calldata signatures
    ) external {
        _validateEmergencyMultisig(nonce, signatures);
        _pause();
        emit PausedUpdated(true, msg.sender, block.timestamp);
    }

    /// @notice Unpause all governance operations — requires 2/3 multisig
    function emergencyUnpause(
        uint256 nonce,
        bytes[] calldata signatures
    ) external {
        _validateEmergencyMultisig(nonce, signatures);
        _unpause();
        emit PausedUpdated(false, msg.sender, block.timestamp);
    }

    /// @notice Execute an emergency proposal immediately (no timelock) — requires 2/3 multisig
    function emergencyExecute(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas,
        string calldata title,
        string calldata description,
        uint256 nonce,
        bytes[] calldata signatures
    ) external returns (uint256 proposalId) {
        _validateEmergencyMultisig(nonce, signatures);

        if (targets.length == 0) revert NoActions();
        if (targets.length > MAX_ACTIONS) revert TooManyActions(targets.length, MAX_ACTIONS);
        if (targets.length != values.length || targets.length != calldatas.length) {
            revert ArrayLengthMismatch();
        }

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
            endTime: block.timestamp,
            executionTime: block.timestamp, // immediate execution
            forVotes: 0,
            againstVotes: 0,
            abstainVotes: 0,
            status: ProposalStatus.Executed,
            category: ProposalCategory.Emergency,
            executed: true,
            createdAt: block.timestamp,
            depositAmount: 0
        });

        // Execute immediately — no timelock
        for (uint256 i = 0; i < targets.length; i++) {
            (bool success,) = targets[i].call{value: values[i]}(calldatas[i]);
            if (!success) revert ExecutionFailed(i);
        }

        emit ProposalCreated(proposalId, msg.sender, title, ProposalCategory.Emergency, block.timestamp, block.timestamp, 0, block.timestamp);
        emit ProposalExecuted(proposalId, block.timestamp);
        emit EmergencyExecuted(proposalId, nonce, block.timestamp);
    }

    /// @notice Validate 2/3 multisig signatures for emergency actions
    function _validateEmergencyMultisig(uint256 nonce, bytes[] calldata signatures) internal {
        if (nonce != emergencyNonce) revert InvalidEmergencyNonce(emergencyNonce, nonce);
        emergencyNonce++;

        uint256 signerCount = emergencySigners.length;
        if (signatures.length < emergencyThreshold) {
            revert InsufficientEmergencySignatures(signatures.length, emergencyThreshold);
        }

        // Verify each signature is from a unique valid signer
        bytes32 digest = keccak256(
            abi.encodePacked(
                "\x19Ethereum Signed Message:\n32",
                keccak256(abi.encodePacked(nonce, block.chainid, address(this)))
            )
        );

        uint256 validCount;
        address lastSigner;
        for (uint256 i = 0; i < signatures.length; i++) {
            address recovered = _recoverSigner(digest, signatures[i]);
            if (!isEmergencySigner[recovered]) revert NotEmergencySigner();
            // Enforce ascending order to prevent duplicates (simplified dedup)
            if (recovered <= lastSigner) revert("DUPLICATE_OR_UNORDERED");
            lastSigner = recovered;
            validCount++;
        }

        if (validCount < emergencyThreshold) {
            revert InsufficientEmergencySignatures(validCount, emergencyThreshold);
        }
    }

    function _recoverSigner(bytes32 digest, bytes memory signature) internal pure returns (address) {
        require(signature.length == 65, "INVALID_SIG_LENGTH");
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := mload(add(signature, 32))
            s := mload(add(signature, 64))
            v := byte(0, mload(add(signature, 96)))
        }
        if (v < 27) v += 27;
        return ecrecover(digest, v, r, s);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Create a new governance proposal
    /// @param title Proposal title
    /// @param description Proposal description
    /// @param targets Target addresses for each action
    /// @param values Native currency values for each action
    /// @param calldatas Calldata for each action
    /// @param category Proposal category (determines timelock)
    /// @return proposalId The new proposal ID
    function propose(
        string calldata title,
        string calldata description,
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata calldatas,
        ProposalCategory category
    ) external payable whenNotPaused returns (uint256 proposalId) {
        if (bytes(title).length == 0) revert EmptyTitle();
        if (targets.length == 0) revert NoActions();
        if (targets.length > MAX_ACTIONS) revert TooManyActions(targets.length, MAX_ACTIONS);
        if (targets.length != values.length || targets.length != calldatas.length) {
            revert ArrayLengthMismatch();
        }

        // Category validation
        if (category == ProposalCategory.Emergency) {
            revert("EMERGENCY_REQUIRES_MULTISIG");
        }

        // Deposit check
        if (proposalDeposit > 0) {
            if (msg.value < proposalDeposit) {
                revert InsufficientDeposit(proposalDeposit, msg.value);
            }
        }

        // Voting power check
        uint256 vp = _getVotingPower(msg.sender);
        if (vp == 0) revert NoVotingPower();

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
            abstainVotes: 0,
            status: ProposalStatus.Active,
            category: category,
            executed: false,
            createdAt: block.timestamp,
            depositAmount: msg.value
        });

        emit ProposalCreated(
            proposalId, msg.sender, title, category,
            block.timestamp, block.timestamp + votingPeriod,
            msg.value, block.timestamp
        );
    }

    /// @notice Cast a vote on a proposal
    /// @param proposalId The proposal ID
    /// @param voteType Vote type: Against(0), For(1), Abstain(2)
    function vote(uint256 proposalId, VoteType voteType) external whenNotPaused {
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

        uint256 weight = _getVotingPower(msg.sender);
        if (weight == 0) revert NoVotingPower();

        _votes[proposalId][msg.sender] = Vote({
            voteType: voteType,
            weight: weight,
            timestamp: block.timestamp,
            voted: true
        });

        if (voteType == VoteType.For) {
            proposal.forVotes += weight;
        } else if (voteType == VoteType.Against) {
            proposal.againstVotes += weight;
        } else {
            proposal.abstainVotes += weight;
        }

        emit VoteCast(proposalId, msg.sender, voteType, weight, block.timestamp);
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

        // Quorum check: (for + against + abstain) must meet quorum
        uint256 totalVotes = proposal.forVotes + proposal.againstVotes + proposal.abstainVotes;
        uint256 totalSupply = _getTotalVotingPower();
        uint256 quorumRequired = (totalSupply * quorumBps) / BPS_DENOMINATOR;
        if (totalVotes < quorumRequired) revert QuorumNotMet(totalVotes, quorumRequired);

        // Approval check: for / (for + against) must exceed approvalThresholdBps
        // Abstain votes count toward quorum but not toward approval ratio
        uint256 decisiveVotes = proposal.forVotes + proposal.againstVotes;
        if (decisiveVotes == 0 || (proposal.forVotes * BPS_DENOMINATOR) / decisiveVotes <= approvalThresholdBps) {
            proposal.status = ProposalStatus.Defeated;
            revert ProposalDefeated(proposal.forVotes, proposal.againstVotes);
        }

        proposal.status = ProposalStatus.Queued;

        // Calculate timelock based on category
        uint256 timelockDuration = _getTimelockForCategory(proposal.category);
        proposal.executionTime = block.timestamp + timelockDuration;

        emit ProposalQueued(proposalId, proposal.executionTime, block.timestamp);
    }

    /// @notice Execute a queued proposal
    /// @param proposalId The proposal ID
    function execute(uint256 proposalId) external nonReentrant whenNotPaused {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.executed) revert AlreadyExecuted();
        if (proposal.status != ProposalStatus.Queued) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Queued);
        }
        if (block.timestamp < proposal.executionTime) {
            revert TimelockNotElapsed(proposal.executionTime, block.timestamp);
        }

        proposal.executed = true;
        proposal.status = ProposalStatus.Executed;

        // Execute each action
        for (uint256 i = 0; i < proposal.targets.length; i++) {
            (bool success,) = proposal.targets[i].call{value: proposal.values[i]}(proposal.calldatas[i]);
            if (!success) revert ExecutionFailed(i);
        }

        // Return deposit to proposer
        if (proposal.depositAmount > 0) {
            (bool refunded,) = payable(proposal.proposer).call{value: proposal.depositAmount}("");
            // If refund fails, deposit stays in contract (not a blocker)
            if (!refunded) {
                // Deposit stays locked; could be recovered by governance
            }
        }

        emit ProposalExecuted(proposalId, block.timestamp);
    }

    /// @notice Cancel a proposal (proposer or governance)
    /// @param proposalId The proposal ID
    function cancelProposal(uint256 proposalId) external {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);

        Proposal storage proposal = _proposals[proposalId];
        if (proposal.proposer != msg.sender && owner() != msg.sender) {
            revert NotAuthorized();
        }
        if (proposal.status == ProposalStatus.Executed || proposal.status == ProposalStatus.Cancelled) {
            revert InvalidProposalStatus(proposal.status, ProposalStatus.Executed);
        }

        proposal.status = ProposalStatus.Cancelled;

        // Forfeit deposit
        if (proposal.depositAmount > 0) {
            // Deposit stays in governance contract
        }

        emit ProposalCancelled(proposalId, msg.sender, block.timestamp);
    }

    // ── Timelock Logic ───────────────────────────────────────────────────────

    /// @notice Get timelock duration for a proposal category
    function _getTimelockForCategory(ProposalCategory category) internal pure returns (uint256) {
        if (category == ProposalCategory.FeeIncrease) return FEE_INCREASE_TIMELOCK;
        if (category == ProposalCategory.FeeDecrease) return FEE_DECREASE_TIMELOCK;
        if (category == ProposalCategory.Emergency) return EMERGENCY_TIMELOCK;
        // Standard, Template, Marketplace → 48h
        return STANDARD_TIMELOCK;
    }

    /// @notice Get timelock duration for a proposal category (public view)
    function getTimelockForCategory(ProposalCategory category) external pure returns (uint256) {
        return _getTimelockForCategory(category);
    }

    // ── Voting Power ─────────────────────────────────────────────────────────

    /// @notice Get voting power for an address
    function _getVotingPower(address voter) internal view returns (uint256) {
        if (tokenVotingEnabled && address(governanceToken) != address(0)) {
            return governanceToken.balanceOf(voter);
        }
        // Fallback: manual voting power (set by owner for bootstrapping)
        return _manualVotingPower[voter];
    }

    /// @notice Get total voting power
    function _getTotalVotingPower() internal view returns (uint256) {
        if (tokenVotingEnabled && address(governanceToken) != address(0)) {
            return governanceToken.totalSupply();
        }
        return _manualTotalVotingPower;
    }

    /// @notice Manual voting power mapping (for bootstrapping without token)
    mapping(address => uint256) private _manualVotingPower;
    uint256 private _manualTotalVotingPower;

    /// @notice Set manual voting power (onlyOwner, only when token voting disabled)
    function setManualVotingPower(address voter, uint256 power) external onlyOwner {
        require(!tokenVotingEnabled, "TOKEN_VOTING_ACTIVE");
        if (voter == address(0)) revert ZeroAddress();
        uint256 oldPower = _manualVotingPower[voter];
        _manualTotalVotingPower = _manualTotalVotingPower - oldPower + power;
        _manualVotingPower[voter] = power;
    }

    /// @notice Batch set manual voting power
    function batchSetManualVotingPower(
        address[] calldata voters,
        uint256[] calldata powers
    ) external onlyOwner {
        require(!tokenVotingEnabled, "TOKEN_VOTING_ACTIVE");
        require(voters.length == powers.length, "LENGTH_MISMATCH");
        for (uint256 i = 0; i < voters.length; i++) {
            if (voters[i] == address(0)) revert ZeroAddress();
            uint256 oldPower = _manualVotingPower[voters[i]];
            _manualTotalVotingPower = _manualTotalVotingPower - oldPower + powers[i];
            _manualVotingPower[voters[i]] = powers[i];
        }
    }

    /// @notice Get manual voting power for an address
    function getManualVotingPower(address voter) external view returns (uint256) {
        return _manualVotingPower[voter];
    }

    /// @notice Get the voting power of an address (public)
    function getVotingPower(address voter) external view returns (uint256) {
        return _getVotingPower(voter);
    }

    /// @notice Get the total voting power (public)
    function getTotalVotingPower() external view returns (uint256) {
        return _getTotalVotingPower();
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get proposal details by ID
    function getProposal(uint256 proposalId) external view returns (Proposal memory) {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);
        return _proposals[proposalId];
    }

    /// @notice Get total number of proposals
    function getProposalCount() external view returns (uint256) {
        return _proposalCount;
    }

    /// @notice Get vote details for a voter on a proposal
    function getVote(uint256 proposalId, address voter) external view returns (Vote memory) {
        return _votes[proposalId][voter];
    }

    /// @notice Check if a voter has voted on a proposal
    function hasVoted(uint256 proposalId, address voter) external view returns (bool) {
        return _votes[proposalId][voter].voted;
    }

    /// @notice Check current proposal state (combines status + time checks)
    function getProposalState(uint256 proposalId) external view returns (ProposalStatus) {
        if (proposalId == 0 || proposalId > _proposalCount) revert ProposalNotFound(proposalId);
        Proposal storage p = _proposals[proposalId];
        if (p.status == ProposalStatus.Executed) return ProposalStatus.Executed;
        if (p.status == ProposalStatus.Cancelled) return ProposalStatus.Cancelled;
        if (p.status == ProposalStatus.Queued) {
            if (block.timestamp >= p.executionTime) return ProposalStatus.Queued;
            return ProposalStatus.Queued;
        }
        if (p.status == ProposalStatus.Active && block.timestamp > p.endTime) {
            return ProposalStatus.Succeeded; // eligible for queueing (if votes pass)
        }
        return p.status;
    }

    /// @notice Get proposals by status
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

    /// @notice Get all emergency signers
    function getEmergencySigners() external view returns (address[] memory) {
        return emergencySigners;
    }

    /// @notice Get the current governance parameters
    function getGovernanceParameters() external view returns (
        uint256 _votingPeriod,
        uint256 _quorumBps,
        uint256 _approvalThresholdBps,
        uint256 _proposalDeposit,
        uint256 _proposalCount_,
        bool _tokenVotingEnabled,
        uint256 _emergencyThreshold,
        uint256 _emergencySignerCount
    ) {
        return (
            votingPeriod,
            quorumBps,
            approvalThresholdBps,
            proposalDeposit,
            _proposalCount,
            tokenVotingEnabled,
            emergencyThreshold,
            emergencySigners.length
        );
    }

    // ── Receive ──────────────────────────────────────────────────────────────

    receive() external payable {}

    fallback() external payable {}
}