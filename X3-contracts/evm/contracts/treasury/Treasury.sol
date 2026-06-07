// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contracts/AtlasSphereX3.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract Treasury is Ownable {
    address public devWallet;
    address public daoWallet;
    address public lpWallet;
    uint256 public devSplitBps;
    uint256 public daoSplitBps;
    uint256 public lpSplitBps;
    mapping(address => uint256) public accounting;

    event SplitUpdated(uint256 dev, uint256 dao, uint256 lp);
    event FeeRouted(address indexed from, uint256 amount, string category);

    constructor(address _dev, address _dao, address _lp) {
        devWallet = _dev;
        daoWallet = _dao;
        lpWallet = _lp;
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

    function routeFee(address from, uint256 amount, string memory category) external {
        uint256 devAmt = (amount * devSplitBps) / 10000;
        uint256 daoAmt = (amount * daoSplitBps) / 10000;
        uint256 lpAmt = amount - devAmt - daoAmt;
        payable(devWallet).transfer(devAmt);
        payable(daoWallet).transfer(daoAmt);
        payable(lpWallet).transfer(lpAmt);
        accounting[from] += amount;
        emit FeeRouted(from, amount, category);
    }

    receive() external payable {}
}
