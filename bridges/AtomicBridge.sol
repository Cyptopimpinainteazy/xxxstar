// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../adapters/UniversalAdapter.sol";

contract AtomicBridge {
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

    constructor(address _adapter, address _treasury) {
        adapter = _adapter;
        treasury = _treasury;
        bridgeFeeBps = 50;
    }

    function setBridgeFee(uint256 bps) external {
        bridgeFeeBps = bps;
        emit BridgeFeeUpdated(bps);
    }

    function setChainDown(uint256 chainId, bool down) external {
        chainDown[chainId] = down;
        emit ChainDown(chainId, down);
    }

    function bridgeSwap(address wrapped, address user, uint256 amount, uint256 fromChain, uint256 toChain, bytes32 swapId) external {
        require(!completedSwaps[swapId], "Already completed");
        require(!chainDown[toChain], "Target chain down");
        uint256 fee = (amount * bridgeFeeBps) / 10000;
        UniversalAdapter(adapter).withdraw(wrapped, user, amount, fromChain);
        UniversalAdapter(adapter).deposit(wrapped, user, amount - fee, toChain);
        completedSwaps[swapId] = true;
        emit SwapInitiated(swapId, user, amount, fromChain, toChain);
        emit SwapCompleted(swapId);
    }

    function fallbackSwap(bytes32 swapId, uint256 chainId) external {
        require(chainDown[chainId], "Chain not down");
        emit FallbackTriggered(swapId, chainId);
    }
}
