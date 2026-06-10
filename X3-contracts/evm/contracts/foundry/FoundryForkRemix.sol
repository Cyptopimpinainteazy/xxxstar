// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title FoundryForkRemix — Fork/Remix System for X3 Foundry
/// @notice Fork/remix system that tracks fork lineage and handles remix royalties
/// @dev Supports forking dApps and remixing templates with lineage tracking
contract FoundryForkRemix is Ownable, ReentrancyGuard {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Represents a fork or remix
    struct Fork {
        uint256 id;
        address originalDapp;       // The original dApp that was forked
        address forkedDapp;         // The new forked dApp address
        address forkCreator;        // Who created the fork
        uint256 originalTemplateId; // Template ID if forked from a template
        string name;
        string description;
        uint256 remixRoyaltyBps;    // Royalty for original creator in basis points
        uint256 createdAt;
        bool isActive;
    }

    /// @notice Lineage node for tracking ancestry
    struct LineageNode {
        uint256 forkId;
        address dappAddress;
        address parentDapp;
        uint256 depth;              // How deep in the lineage (0 = original)
    }

    // ── Constants ────────────────────────────────────────────────────────────

    /// @notice Maximum basis points (100%)
    uint256 public constant MAX_BPS = 10000;

    /// @notice Maximum remix royalty basis points (25%)
    uint256 public constant MAX_REMIX_ROYALTY_BPS = 2500;

    /// @notice Default remix royalty basis points (5%)
    uint256 public constant DEFAULT_REMIX_ROYALTY_BPS = 500;

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Fork ID => Fork
    mapping(uint256 => Fork) private _forks;

    /// @notice dApp address => fork ID
    mapping(address => uint256) private _forkIdByDapp;

    /// @notice Original dApp => array of fork IDs
    mapping(address => uint256[]) private _forksOfOriginal;

    /// @notice dApp address => LineageNode
    mapping(address => LineageNode) private _lineage;

    /// @notice dApp address => remix royalty BPS (set by original creator)
    mapping(address => uint256) private _remixRoyalties;

    /// @notice dApp address => accumulated remix revenue
    mapping(address => uint256) private _remixRevenue;

    /// @notice Incremental fork ID counter
    uint256 private _forkCount;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a dApp is forked
    event AppForked(
        uint256 indexed forkId,
        address indexed originalDapp,
        address indexed forkedDapp,
        address forkCreator,
        string name,
        uint256 timestamp
    );

    /// @notice Emitted when a template is remixed
    event TemplateRemixed(
        uint256 indexed forkId,
        uint256 indexed templateId,
        address indexed forkedDapp,
        address remixCreator,
        uint256 timestamp
    );

    /// @notice Emitted when remix royalty is set
    event RemixRoyaltySet(
        address indexed dapp,
        uint256 royaltyBps,
        uint256 timestamp
    );

    /// @notice Emitted when remix revenue is claimed
    event RemixRevenueClaimed(
        address indexed dapp,
        address indexed claimer,
        uint256 amount,
        uint256 timestamp
    );

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyName();
    error AlreadyForked(address dapp);
    error ForkNotFound(uint256 forkId);
    error InvalidRoyaltyBps(uint256 bps);
    error NoRevenueToClaim();
    error TransferFailed();
    error MaxDepthExceeded(uint256 depth);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Fork an existing dApp
    /// @param originalDapp The original dApp address to fork
    /// @param forkedDapp The new forked dApp address
    /// @param name Name for the fork
    /// @param description Description for the fork
    /// @return forkId The new fork ID
    function forkApp(
        address originalDapp,
        address forkedDapp,
        string calldata name,
        string calldata description
    ) external nonReentrant returns (uint256 forkId) {
        if (originalDapp == address(0)) revert ZeroAddress();
        if (forkedDapp == address(0)) revert ZeroAddress();
        if (bytes(name).length == 0) revert EmptyName();
        if (_forkIdByDapp[forkedDapp] != 0) revert AlreadyForked(forkedDapp);

        // Check lineage depth
        uint256 parentDepth = _lineage[originalDapp].depth;
        if (parentDepth >= 10) revert MaxDepthExceeded(parentDepth);

        _forkCount++;
        forkId = _forkCount;

        uint256 royaltyBps = _remixRoyalties[originalDapp] > 0
            ? _remixRoyalties[originalDapp]
            : DEFAULT_REMIX_ROYALTY_BPS;

        _forks[forkId] = Fork({
            id: forkId,
            originalDapp: originalDapp,
            forkedDapp: forkedDapp,
            forkCreator: msg.sender,
            originalTemplateId: 0,
            name: name,
            description: description,
            remixRoyaltyBps: royaltyBps,
            createdAt: block.timestamp,
            isActive: true
        });

        _forkIdByDapp[forkedDapp] = forkId;
        _forksOfOriginal[originalDapp].push(forkId);

        // Set lineage
        _lineage[forkedDapp] = LineageNode({
            forkId: forkId,
            dappAddress: forkedDapp,
            parentDapp: originalDapp,
            depth: parentDepth + 1
        });

        emit AppForked(forkId, originalDapp, forkedDapp, msg.sender, name, block.timestamp);
    }

    /// @notice Remix a template (create a new dApp from a template)
    /// @param templateId The template ID
    /// @param remixedDapp The new dApp address
    /// @param name Name for the remix
    /// @param description Description for the remix
    /// @return forkId The new fork ID
    function remixTemplate(
        uint256 templateId,
        address remixedDapp,
        string calldata name,
        string calldata description
    ) external nonReentrant returns (uint256 forkId) {
        if (templateId == 0) revert("INVALID_TEMPLATE");
        if (remixedDapp == address(0)) revert ZeroAddress();
        if (bytes(name).length == 0) revert EmptyName();
        if (_forkIdByDapp[remixedDapp] != 0) revert AlreadyForked(remixedDapp);

        _forkCount++;
        forkId = _forkCount;

        _forks[forkId] = Fork({
            id: forkId,
            originalDapp: address(0),
            forkedDapp: remixedDapp,
            forkCreator: msg.sender,
            originalTemplateId: templateId,
            name: name,
            description: description,
            remixRoyaltyBps: 0,
            createdAt: block.timestamp,
            isActive: true
        });

        _forkIdByDapp[remixedDapp] = forkId;

        // Set lineage (template remixes start at depth 0)
        _lineage[remixedDapp] = LineageNode({
            forkId: forkId,
            dappAddress: remixedDapp,
            parentDapp: address(0),
            depth: 0
        });

        emit TemplateRemixed(forkId, templateId, remixedDapp, msg.sender, block.timestamp);
    }

    /// @notice Set the remix royalty for a dApp (by original creator)
    /// @param dapp The dApp address
    /// @param royaltyBps The royalty in basis points
    function setRemixRoyalty(address dapp, uint256 royaltyBps) external {
        if (dapp == address(0)) revert ZeroAddress();
        if (royaltyBps > MAX_REMIX_ROYALTY_BPS) revert InvalidRoyaltyBps(royaltyBps);

        // Only the original creator or owner can set royalty
        uint256 forkId = _forkIdByDapp[dapp];
        if (forkId != 0) {
            Fork storage fork = _forks[forkId];
            if (fork.forkCreator != msg.sender && owner() != msg.sender) {
                revert("NOT_AUTHORIZED");
            }
        } else if (owner() != msg.sender) {
            revert("NOT_AUTHORIZED");
        }

        _remixRoyalties[dapp] = royaltyBps;

        emit RemixRoyaltySet(dapp, royaltyBps, block.timestamp);
    }

    /// @notice Record remix revenue (called when a fork generates revenue)
    /// @param forkedDapp The forked dApp that generated revenue
    /// @param revenueAmount The revenue amount
    function recordRemixRevenue(address forkedDapp, uint256 revenueAmount) external payable nonReentrant {
        if (forkedDapp == address(0)) revert ZeroAddress();
        uint256 forkId = _forkIdByDapp[forkedDapp];
        if (forkId == 0) revert ForkNotFound(forkId);

        Fork storage fork = _forks[forkId];
        if (!fork.isActive) revert("FORK_INACTIVE");

        uint256 actualAmount = revenueAmount > 0 ? revenueAmount : msg.value;
        if (actualAmount == 0) revert("ZERO_AMOUNT");

        uint256 royaltyAmount = (actualAmount * fork.remixRoyaltyBps) / MAX_BPS;

        if (royaltyAmount > 0 && fork.originalDapp != address(0)) {
            _remixRevenue[fork.originalDapp] += royaltyAmount;
        }
    }

    /// @notice Claim accumulated remix revenue (pull-over-push)
    /// @param amount The amount to claim
    function claimRemixRevenue(uint256 amount) external nonReentrant {
        if (amount == 0) revert("ZERO_AMOUNT");
        if (_remixRevenue[msg.sender] < amount) revert NoRevenueToClaim();

        _remixRevenue[msg.sender] -= amount;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit RemixRevenueClaimed(msg.sender, msg.sender, amount, block.timestamp);
    }

    /// @notice Claim all accumulated remix revenue
    function claimAllRemixRevenue() external nonReentrant {
        uint256 amount = _remixRevenue[msg.sender];
        if (amount == 0) revert NoRevenueToClaim();

        _remixRevenue[msg.sender] = 0;

        (bool success,) = payable(msg.sender).call{value: amount}("");
        if (!success) revert TransferFailed();

        emit RemixRevenueClaimed(msg.sender, msg.sender, amount, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get fork details by ID
    /// @param forkId The fork ID
    /// @return Fork struct
    function getFork(uint256 forkId) external view returns (Fork memory) {
        if (forkId == 0 || forkId > _forkCount) revert ForkNotFound(forkId);
        return _forks[forkId];
    }

    /// @notice Get fork ID by dApp address
    /// @param dapp The dApp address
    /// @return forkId The fork ID (0 if not a fork)
    function getForkIdByDapp(address dapp) external view returns (uint256) {
        return _forkIdByDapp[dapp];
    }

    /// @notice Get the lineage of a dApp (ancestry chain)
    /// @param dapp The dApp address
    /// @return lineage Array of LineageNode
    function getLineage(address dapp) external view returns (LineageNode[] memory lineage) {
        // Walk up the lineage tree
        uint256 depth = _lineage[dapp].depth;
        lineage = new LineageNode[](depth + 1);

        address current = dapp;
        for (uint256 i = 0; i <= depth; i++) {
            lineage[i] = _lineage[current];
            current = lineage[i].parentDapp;
        }
    }

    /// @notice Get all forks of an original dApp
    /// @param originalDapp The original dApp address
    /// @return forks Array of Fork structs
    function getForksOfOriginal(address originalDapp) external view returns (Fork[] memory forks) {
        uint256[] storage forkIds = _forksOfOriginal[originalDapp];
        uint256 len = forkIds.length;
        forks = new Fork[](len);
        for (uint256 i = 0; i < len; i++) {
            forks[i] = _forks[forkIds[i]];
        }
    }

    /// @notice Get the remix royalty for a dApp
    /// @param dapp The dApp address
    /// @return royaltyBps The royalty in basis points
    function getRemixRoyalty(address dapp) external view returns (uint256) {
        return _remixRoyalties[dapp];
    }

    /// @notice Get accumulated remix revenue for a dApp
    /// @param dapp The dApp address
    /// @return revenue The accumulated revenue
    function getRemixRevenue(address dapp) external view returns (uint256) {
        return _remixRevenue[dapp];
    }

    /// @notice Get total number of forks
    /// @return count The fork count
    function getForkCount() external view returns (uint256) {
        return _forkCount;
    }

    /// @notice Get lineage depth of a dApp
    /// @param dapp The dApp address
    /// @return depth The depth
    function getLineageDepth(address dapp) external view returns (uint256) {
        return _lineage[dapp].depth;
    }
}
