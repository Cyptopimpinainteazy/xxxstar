// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "../adapters/UniversalAdapter.sol";

contract AtomicBridge is Ownable, ReentrancyGuard {
    address public adapter;
    address public treasury;
    uint256 public bridgeFeeBps;
    mapping(bytes32 => bool) public completedSwaps;
    mapping(uint256 => bool) public chainDown;

    event BridgeFeeUpdated(uint256 newBps);
    event ChainDown(uint256 indexed chainId, bool down);
    event SwapInitiated(bytes32 indexed swapId, address indexed user, uint256 amount, uint256 fromChain, uint256 toChain);
    event SwapCompleted(bytes32 indexed swapId);
    event FallbackTriggered(bytes32 indexed swapId, uint256 chainId);

    constructor(address _adapter, address _treasury) Ownable() {
        require(_adapter != address(0), "Adapter is zero address");
        require(_treasury != address(0), "Treasury is zero address");
        adapter = _adapter;
        treasury = _treasury;
        bridgeFeeBps = 50;
    }

    /// @notice Owner-only: update the bridge fee in basis points (1 bps = 0.01%).
    /// @param bps New fee.  Must be <= 1000 (10%) to prevent extreme drains.
    function setBridgeFee(uint256 bps) external onlyOwner {
        require(bps <= 1000, "Fee too high");
        bridgeFeeBps = bps;
        emit BridgeFeeUpdated(bps);
    }

    /// @notice Owner-only: mark a chain as down (halt bridging to it).
    function setChainDown(uint256 chainId, bool down) external onlyOwner {
        chainDown[chainId] = down;
        emit ChainDown(chainId, down);
    }

    /// @notice Execute an atomic bridge swap between two chains.
    /// @dev Protected against reentrancy, zero amounts, and same-chain bridging.
    function bridgeSwap(
        address wrapped,
        address user,
        uint256 amount,
        uint256 fromChain,
        uint256 toChain,
        bytes32 swapId
    ) external nonReentrant {
        require(amount > 0, "Zero amount");
        require(fromChain != toChain, "Same chain");
        require(!completedSwaps[swapId], "Already completed");
        require(!chainDown[toChain], "Target chain down");

        uint256 fee = (amount * bridgeFeeBps) / 10000;
        uint256 net = amount - fee;

        UniversalAdapter(adapter).withdraw(wrapped, user, amount, fromChain);
        UniversalAdapter(adapter).deposit(wrapped, user, net, toChain);

        completedSwaps[swapId] = true;
        emit SwapInitiated(swapId, user, amount, fromChain, toChain);
        emit SwapCompleted(swapId);
    }

    /// @notice Owner-only: trigger fallback path when a chain is confirmed down.
    function fallbackSwap(bytes32 swapId, uint256 chainId) external onlyOwner {
        require(chainDown[chainId], "Chain not down");
        emit FallbackTriggered(swapId, chainId);
    }
}