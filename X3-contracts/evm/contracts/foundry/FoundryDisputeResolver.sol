// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title FoundryDisputeResolver — Dispute Resolution for X3 Foundry
/// @notice Dispute resolution for dApp transactions with timelock-based resolution and escrow hold
/// @dev Uses timelock for resolution and ReentrancyGuard for safety
contract FoundryDisputeResolver is Ownable, ReentrancyGuard {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Status of a dispute
    enum DisputeStatus {
        None,
        Raised,
        UnderReview,
        Resolved,
        Appealed,
        Escalated
    }

    /// @notice Resolution type
    enum ResolutionType {
        None,
        BuyerWins,
        SellerWins,
        Split,
        Cancelled
    }

    /// @notice Full dispute data
    struct Dispute {
        uint256 id;
        address disputant;          // The party raising the dispute
        address respondent;         // The other party
        address dapp;               // The dApp involved
        bytes32 transactionId;      // The transaction being disputed
        string reason;              // Reason for the dispute
        uint256 escrowAmount;       // Amount held in escrow (wei)
        DisputeStatus status;
        ResolutionType resolution;
        address resolver;           // Who resolved it
        uint256 raisedAt;
        uint256 resolvedAt;
        uint256 appealDeadline;     // Deadline for appeal
        string evidenceURI;         // URI to off-chain evidence
        string resolutionNote;      // Note on resolution
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Default timelock period for resolution (3 days)
    uint256 public constant DEFAULT_TIMELOCK = 3 days;

    /// @notice Appeal period after resolution (7 days)
    uint256 public constant APPEAL_PERIOD = 7 days;

    /// @notice Maximum dispute reason length
    uint256 public constant MAX_REASON_LENGTH = 1024;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Dispute ID => Dispute
    mapping(uint256 => Dispute) private _disputes;

    /// @notice Transaction ID => dispute ID
    mapping(bytes32 => uint256) private _disputeByTransaction;

    /// @notice Incremental dispute ID counter
    uint256 private _disputeCount;

    /// @notice Timelock period for resolution
    uint256 public timelockPeriod;

    /// @notice Address of the dispute resolver role
    address public disputeResolver;

    /// @notice Total escrow held
    uint256 public totalEscrowHeld;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a dispute is raised
    event DisputeRaised(
        uint256 indexed disputeId,
        address indexed disputant,
        address indexed respondent,
        bytes32 transactionId,
        uint256 escrowAmount,
        string reason,
        uint256 timestamp
    );

    /// @notice Emitted when a dispute is resolved
    event DisputeResolved(
        uint256 indexed disputeId,
        ResolutionType resolution,
        address indexed resolver,
        string note,
        uint256 timestamp
    );

    /// @notice Emitted when a dispute is appealed
    event DisputeAppealed(
        uint256 indexed disputeId,
        address indexed appellant,
        uint256 timestamp
    );

    /// @notice Emitted when timelock period is updated
    event TimelockUpdated(uint256 oldPeriod, uint256 newPeriod, uint256 timestamp);

    /// @notice Emitted when dispute resolver address is updated
    event DisputeResolverUpdated(address indexed oldResolver, address indexed newResolver, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error ZeroAmount();
    error DisputeAlreadyExists(bytes32 transactionId);
    error DisputeNotFound(uint256 disputeId);
    error InvalidDisputeStatus(DisputeStatus current, DisputeStatus expected);
    error NotDisputant(uint256 disputeId);
    error NotResolver();
    error TimelockNotElapsed(uint256 deadline, uint256 current);
    error AppealPeriodExpired(uint256 deadline, uint256 current);
    error ReasonTooLong(uint256 length, uint256 max);
    error TransferFailed();

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor(address _disputeResolver) {
        _transferOwnership(msg.sender);
        if (_disputeResolver == address(0)) revert ZeroAddress();
        disputeResolver = _disputeResolver;
        timelockPeriod = DEFAULT_TIMELOCK;
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the timelock period for dispute resolution
    /// @param newPeriod The new timelock period in seconds
    function setTimelock(uint256 newPeriod) external onlyOwner {
        if (newPeriod < 1 hours) revert("TIMELOCK_TOO_SHORT");
        uint256 oldPeriod = timelockPeriod;
        timelockPeriod = newPeriod;
        emit TimelockUpdated(oldPeriod, newPeriod, block.timestamp);
    }

    /// @notice Set the dispute resolver address
    /// @param newResolver The new resolver address
    function setDisputeResolver(address newResolver) external onlyOwner {
        if (newResolver == address(0)) revert ZeroAddress();
        address oldResolver = disputeResolver;
        disputeResolver = newResolver;
        emit DisputeResolverUpdated(oldResolver, newResolver, block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Raise a new dispute with escrow hold
    /// @param respondent The other party in the dispute
    /// @param dapp The dApp involved
    /// @param transactionId The transaction ID being disputed
    /// @param reason The reason for the dispute
    /// @param evidenceURI URI to off-chain evidence
    /// @return disputeId The new dispute ID
    function raiseDispute(
        address respondent,
        address dapp,
        bytes32 transactionId,
        string calldata reason,
        string calldata evidenceURI
    ) external payable nonReentrant returns (uint256 disputeId) {
        if (respondent == address(0)) revert ZeroAddress();
        if (dapp == address(0)) revert ZeroAddress();
        if (msg.value == 0) revert ZeroAmount();
        if (bytes(reason).length > MAX_REASON_LENGTH) {
            revert ReasonTooLong(bytes(reason).length, MAX_REASON_LENGTH);
        }
        if (_disputeByTransaction[transactionId] != 0) {
            revert DisputeAlreadyExists(transactionId);
        }

        _disputeCount++;
        disputeId = _disputeCount;

        _disputes[disputeId] = Dispute({
            id: disputeId,
            disputant: msg.sender,
            respondent: respondent,
            dapp: dapp,
            transactionId: transactionId,
            reason: reason,
            escrowAmount: msg.value,
            status: DisputeStatus.Raised,
            resolution: ResolutionType.None,
            resolver: address(0),
            raisedAt: block.timestamp,
            resolvedAt: 0,
            appealDeadline: 0,
            evidenceURI: evidenceURI,
            resolutionNote: ""
        });

        _disputeByTransaction[transactionId] = disputeId;
        totalEscrowHeld += msg.value;

        emit DisputeRaised(disputeId, msg.sender, respondent, transactionId, msg.value, reason, block.timestamp);
    }

    /// @notice Resolve a dispute (only callable by resolver after timelock)
    /// @param disputeId The dispute ID
    /// @param resolution The resolution type
    /// @param note Resolution note
    function resolveDispute(
        uint256 disputeId,
        ResolutionType resolution,
        string calldata note
    ) external nonReentrant {
        if (msg.sender != disputeResolver && msg.sender != owner()) revert NotResolver();

        Dispute storage dispute = _disputes[disputeId];
        if (dispute.id == 0) revert DisputeNotFound(disputeId);
        if (dispute.status != DisputeStatus.Raised && dispute.status != DisputeStatus.UnderReview) {
            revert InvalidDisputeStatus(dispute.status, DisputeStatus.Raised);
        }

        // Enforce timelock
        uint256 deadline = dispute.raisedAt + timelockPeriod;
        if (block.timestamp < deadline) revert TimelockNotElapsed(deadline, block.timestamp);

        dispute.status = DisputeStatus.Resolved;
        dispute.resolution = resolution;
        dispute.resolver = msg.sender;
        dispute.resolvedAt = block.timestamp;
        dispute.appealDeadline = block.timestamp + APPEAL_PERIOD;
        dispute.resolutionNote = note;

        // Handle escrow distribution based on resolution
        _distributeEscrow(dispute);

        emit DisputeResolved(disputeId, resolution, msg.sender, note, block.timestamp);
    }

    /// @notice Appeal a resolved dispute
    /// @param disputeId The dispute ID to appeal
    function appealDispute(uint256 disputeId) external nonReentrant {
        Dispute storage dispute = _disputes[disputeId];
        if (dispute.id == 0) revert DisputeNotFound(disputeId);
        if (dispute.status != DisputeStatus.Resolved) {
            revert InvalidDisputeStatus(dispute.status, DisputeStatus.Resolved);
        }
        if (block.timestamp > dispute.appealDeadline) {
            revert AppealPeriodExpired(dispute.appealDeadline, block.timestamp);
        }
        if (msg.sender != dispute.disputant && msg.sender != dispute.respondent) {
            revert NotDisputant(disputeId);
        }

        dispute.status = DisputeStatus.Appealed;

        emit DisputeAppealed(disputeId, msg.sender, block.timestamp);
    }

    /// @notice Escalate an appealed dispute (admin function for final resolution)
    /// @param disputeId The dispute ID
    /// @param resolution The final resolution
    /// @param note Final resolution note
    function escalateDispute(
        uint256 disputeId,
        ResolutionType resolution,
        string calldata note
    ) external onlyOwner nonReentrant {
        Dispute storage dispute = _disputes[disputeId];
        if (dispute.id == 0) revert DisputeNotFound(disputeId);
        if (dispute.status != DisputeStatus.Appealed) {
            revert InvalidDisputeStatus(dispute.status, DisputeStatus.Appealed);
        }

        dispute.status = DisputeStatus.Escalated;
        dispute.resolution = resolution;
        dispute.resolver = msg.sender;
        dispute.resolvedAt = block.timestamp;
        dispute.resolutionNote = note;

        // Re-distribute escrow based on final resolution
        _distributeEscrow(dispute);

        emit DisputeResolved(disputeId, resolution, msg.sender, note, block.timestamp);
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Distribute escrow based on resolution
    function _distributeEscrow(Dispute storage dispute) internal {
        uint256 amount = dispute.escrowAmount;
        if (amount == 0) return;

        totalEscrowHeld -= amount;
        dispute.escrowAmount = 0;

        if (dispute.resolution == ResolutionType.BuyerWins) {
            // Return escrow to disputant
            (bool success,) = payable(dispute.disputant).call{value: amount}("");
            if (!success) revert TransferFailed();
        } else if (dispute.resolution == ResolutionType.SellerWins) {
            // Release escrow to respondent
            (bool success,) = payable(dispute.respondent).call{value: amount}("");
            if (!success) revert TransferFailed();
        } else if (dispute.resolution == ResolutionType.Split) {
            // Split 50/50
            uint256 half = amount / 2;
            (bool success1,) = payable(dispute.disputant).call{value: half}("");
            (bool success2,) = payable(dispute.respondent).call{value: amount - half}("");
            if (!success1 || !success2) revert TransferFailed();
        } else if (dispute.resolution == ResolutionType.Cancelled) {
            // Return to disputant
            (bool success,) = payable(dispute.disputant).call{value: amount}("");
            if (!success) revert TransferFailed();
        }
        // ResolutionType.None: escrow stays (should not happen)
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get dispute details by ID
    /// @param disputeId The dispute ID
    /// @return Dispute struct
    function getDispute(uint256 disputeId) external view returns (Dispute memory) {
        if (disputeId == 0 || disputeId > _disputeCount) revert DisputeNotFound(disputeId);
        return _disputes[disputeId];
    }

    /// @notice Get dispute ID by transaction ID
    /// @param transactionId The transaction ID
    /// @return disputeId The dispute ID (0 if none)
    function getDisputeByTransaction(bytes32 transactionId) external view returns (uint256) {
        return _disputeByTransaction[transactionId];
    }

    /// @notice Get total number of disputes
    /// @return count The dispute count
    function getDisputeCount() external view returns (uint256) {
        return _disputeCount;
    }

    /// @notice Get disputes for a specific address (as disputant or respondent)
    /// @param participant The address to query
    /// @return disputeIds Array of dispute IDs
    function getDisputesByParticipant(address participant) external view returns (uint256[] memory disputeIds) {
        uint256 count = 0;
        for (uint256 i = 1; i <= _disputeCount; i++) {
            if (_disputes[i].disputant == participant || _disputes[i].respondent == participant) {
                count++;
            }
        }
        disputeIds = new uint256[](count);
        uint256 idx = 0;
        for (uint256 i = 1; i <= _disputeCount; i++) {
            if (_disputes[i].disputant == participant || _disputes[i].respondent == participant) {
                disputeIds[idx] = i;
                idx++;
            }
        }
    }

    /// @notice Get disputes by status
    /// @param status The dispute status to filter
    /// @return disputeIds Array of dispute IDs
    function getDisputesByStatus(DisputeStatus status) external view returns (uint256[] memory disputeIds) {
        uint256 count = 0;
        for (uint256 i = 1; i <= _disputeCount; i++) {
            // slither-disable-next-line incorrect-equality
            if (_disputes[i].status == status) count++;
        }
        disputeIds = new uint256[](count);
        uint256 idx = 0;
        for (uint256 i = 1; i <= _disputeCount; i++) {
            // slither-disable-next-line incorrect-equality
            if (_disputes[i].status == status) {
                disputeIds[idx] = i;
                idx++;
            }
        }
    }
}
