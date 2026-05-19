// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contracts/WrappedX3.sol";

contract UniversalAdapter {
    address public bridge;
    mapping(uint256 => address) public chainRegistry; // chainId => bridge
    mapping(address => bool) public isOperator;

    event BridgeSet(address indexed bridge);
    event ChainRegistered(uint256 indexed chainId, address bridge);
    event OperatorSet(address indexed operator, bool enabled);

    modifier onlyOperator() {
        require(isOperator[msg.sender], "Not operator");
        _;
    }

    function setBridge(address _bridge) external {
        bridge = _bridge;
        emit BridgeSet(_bridge);
    }

    function registerChain(uint256 chainId, address bridgeAddr) external {
        chainRegistry[chainId] = bridgeAddr;
        emit ChainRegistered(chainId, bridgeAddr);
    }

    function setOperator(address operator, bool enabled) external {
        isOperator[operator] = enabled;
        emit OperatorSet(operator, enabled);
    }

    function deposit(address wrapped, address user, uint256 amount, uint256 chainId) external onlyOperator {
        WrappedX3(wrapped).mint(user, amount);
    }

    function withdraw(address wrapped, address user, uint256 amount, uint256 chainId) external onlyOperator {
        WrappedX3(wrapped).burn(user, amount);
    }
}
