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
    uint256 public chainId;
    mapping(address => bool) public feeExempt;

    event TreasuryChanged(address indexed newTreasury);
    event AdapterChanged(address indexed newAdapter);
    event FeeUpdated(uint256 newBps);
    event FeeExempt(address indexed user, bool exempt);

    constructor(address _treasury, address _adapter, uint256 _chainId) ERC20("Wrapped X3", "wX3") {
        treasury = _treasury;
        adapter = _adapter;
        chainId = _chainId;
        treasuryFeeBps = 50;
    }

    function setTreasury(address _treasury) external onlyOwner {
        treasury = _treasury;
        emit TreasuryChanged(_treasury);
    }

    function setAdapter(address _adapter) external onlyOwner {
        adapter = _adapter;
        emit AdapterChanged(_adapter);
    }

    function setFee(uint256 bps) external onlyOwner {
        treasuryFeeBps = bps;
        emit FeeUpdated(bps);
    }

    function setFeeExempt(address user, bool exempt) external onlyOwner {
        feeExempt[user] = exempt;
        emit FeeExempt(user, exempt);
    }

    function mint(address to, uint256 amount) external {
        require(msg.sender == adapter, "Only adapter");
        uint256 fee = feeExempt[to] ? 0 : (amount * treasuryFeeBps) / 10000;
        if (fee > 0) _mint(treasury, fee);
        _mint(to, amount - fee);
    }

    function burn(address from, uint256 amount) external {
        require(msg.sender == adapter, "Only adapter");
        _burn(from, amount);
    }
}
