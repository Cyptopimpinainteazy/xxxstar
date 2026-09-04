// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../AtlasSphereX3.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

contract Treasury is Ownable, ReentrancyGuard {
    address public immutable devWallet;
    address public immutable daoWallet;
    address public immutable lpWallet;
    uint256 public devSplitBps;
    uint256 public daoSplitBps;
    uint256 public lpSplitBps;
    mapping(address => uint256) public accounting;

    event SplitUpdated(uint256 dev, uint256 dao, uint256 lp);
    event FeeRouted(address indexed from, uint256 amount, string category);

    constructor(address devAddr, address daoAddr, address lpAddr) {
        require(devAddr != address(0), "ZERO_DEV");
        require(daoAddr != address(0), "ZERO_DAO");
        require(lpAddr != address(0), "ZERO_LP");
        devWallet = devAddr;
        daoWallet = daoAddr;
        lpWallet = lpAddr;
        devSplitBps = 2000; // 20%
        daoSplitBps = 5000; // 50%
        lpSplitBps = 3000; // 30%
    }

    function setSplits(uint256 dev, uint256 dao, uint256 lp) external onlyOwner {
        require(dev + dao + lp == 10000, "Must sum to 10000");
        devSplitBps = dev;
        daoSplitBps = dao;
        lpSplitBps = lp;
        emit SplitUpdated(dev, dao, lp);
    }

    function routeFee(address from, uint256 amount, string memory category) external onlyOwner nonReentrant {
        require(from != address(0), "ZERO_FROM");
        require(amount > 0, "ZERO_AMOUNT");
        uint256 devAmt = (amount * devSplitBps) / 10000;
        uint256 daoAmt = (amount * daoSplitBps) / 10000;
        uint256 lpAmt = amount - devAmt - daoAmt;
        accounting[from] += amount;
        (bool devOk, ) = payable(devWallet).call{value: devAmt}("");
        require(devOk, "DEV_TRANSFER_FAILED");
        (bool daoOk, ) = payable(daoWallet).call{value: daoAmt}("");
        require(daoOk, "DAO_TRANSFER_FAILED");
        (bool lpOk, ) = payable(lpWallet).call{value: lpAmt}("");
        require(lpOk, "LP_TRANSFER_FAILED");
        emit FeeRouted(from, amount, category);
    }

    receive() external payable {}
}
