// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Pausable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract AtlasSphereX3 is ERC20, ERC20Burnable, ERC20Pausable, Ownable {
    address public treasury;
    uint256 public transferFeeBps; // basis points (e.g. 50 = 0.5%)
    uint256 public stakingFeeBps;
    uint256 public swapFeeBps;
    mapping(address => bool) public feeExempt;

    event TreasuryChanged(address indexed newTreasury);
    event FeeUpdated(string feeType, uint256 newBps);
    event FeeExempt(address indexed user, bool exempt);

    constructor(address _treasury) ERC20("Atlas Sphere X3", "X3") {
        treasury = _treasury;
        transferFeeBps = 50;
        stakingFeeBps = 100;
        swapFeeBps = 25;
        _mint(_treasury, 1_000_000_000 ether);
    }

    function setTreasury(address _treasury) external onlyOwner {
        treasury = _treasury;
        emit TreasuryChanged(_treasury);
    }

    function setFee(string memory feeType, uint256 bps) external onlyOwner {
        if (keccak256(bytes(feeType)) == keccak256("transfer")) transferFeeBps = bps;
        else if (keccak256(bytes(feeType)) == keccak256("staking")) stakingFeeBps = bps;
        else if (keccak256(bytes(feeType)) == keccak256("swap")) swapFeeBps = bps;
        else revert("Invalid fee type");
        emit FeeUpdated(feeType, bps);
    }

    function setFeeExempt(address user, bool exempt) external onlyOwner {
        feeExempt[user] = exempt;
        emit FeeExempt(user, exempt);
    }

    function _transfer(address from, address to, uint256 amount) internal override(ERC20) {
        if (feeExempt[from] || feeExempt[to] || transferFeeBps == 0) {
            super._transfer(from, to, amount);
        } else {
            uint256 fee = (amount * transferFeeBps) / 10000;
            super._transfer(from, treasury, fee);
            super._transfer(from, to, amount - fee);
        }
    }

    function pause() public onlyOwner { _pause(); }
    function unpause() public onlyOwner { _unpause(); }

    function mint(address to, uint256 amount) public onlyOwner {
        _mint(to, amount);
    }

    function burnFrom(address account, uint256 amount) public override onlyOwner {
        _burn(account, amount);
    }
}
