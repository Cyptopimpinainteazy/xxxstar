// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

interface IUniversalAdapter {
    function verifyDeposit(address user, uint256 amount, uint256 chainId) external returns (bool);
    function verifyWithdraw(address user, uint256 amount, uint256 chainId) external returns (bool);
}

contract WrappedX3 is ERC20, Ownable {
    address public treasury;
    address public adapter;
    uint256 public treasuryFeeBps;
    uint256 public immutable chainId;
    mapping(address => bool) public feeExempt;

    event TreasuryChanged(address indexed newTreasury);
    event AdapterChanged(address indexed newAdapter);
    event FeeUpdated(uint256 newBps);
    event FeeExempt(address indexed user, bool exempt);

    constructor(address treasuryAddr, address adapterAddr, uint256 chainId_) ERC20("Wrapped X3", "wX3") {
        require(treasuryAddr != address(0), "ZERO_TREASURY");
        require(adapterAddr != address(0), "ZERO_ADAPTER");
        treasury = treasuryAddr;
        adapter = adapterAddr;
        chainId = chainId_;
        treasuryFeeBps = 50;
    }

    function setTreasury(address newTreasury) external onlyOwner {
        require(newTreasury != address(0), "ZERO_TREASURY");
        treasury = newTreasury;
        emit TreasuryChanged(newTreasury);
    }

    function setAdapter(address newAdapter) external onlyOwner {
        require(newAdapter != address(0), "ZERO_ADAPTER");
        adapter = newAdapter;
        emit AdapterChanged(newAdapter);
    }

    function setFee(uint256 bps) external onlyOwner {
        treasuryFeeBps = bps;
        emit FeeUpdated(bps);
    }

    function setFeeExempt(address user, bool exempt) external onlyOwner {
        feeExempt[user] = exempt;
        emit FeeExempt(user, exempt);
    }

    event Mint(address indexed to, uint256 amount, uint256 fee);
    event Burn(address indexed from, uint256 amount);

    function mint(address to, uint256 amount) external onlyOwner {
        require(to != address(0), "ZERO_TO");
        require(amount > 0, "ZERO_AMOUNT");
        uint256 fee = feeExempt[to] ? 0 : (amount * treasuryFeeBps) / 10000;
        if (fee > 0) _mint(treasury, fee);
        _mint(to, amount - fee);
        emit Mint(to, amount, fee);
    }

    function burn(address from, uint256 amount) external onlyOwner {
        require(from != address(0), "ZERO_FROM");
        require(amount > 0, "ZERO_AMOUNT");
        _burn(from, amount);
        emit Burn(from, amount);
    }
}
