// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contracts/WrappedX3.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract StakingNFT is ERC721, Ownable {
    uint256 public nextId;
    constructor() ERC721("Staking Position", "sX3NFT") {}
    function mint(address to) external onlyOwner returns (uint256) {
        uint256 id = nextId++;
        _mint(to, id);
        return id;
    }
    function burn(uint256 id) external onlyOwner {
        _burn(id);
    }
}

contract StakingPool is Ownable {
    IERC20 public stakingToken;
    StakingNFT public stakingNFT;
    address public treasury;
    uint256 public rewardRate; // per block
    uint256 public totalStaked;
    uint256 public lastUpdateBlock;
    uint256 public accRewardPerShare;
    mapping(uint256 => uint256) public stakeAmount;
    mapping(uint256 => uint256) public rewardDebt;
    mapping(address => uint256[]) public userNFTs;

    event Staked(address indexed user, uint256 nftId, uint256 amount);
    event Unstaked(address indexed user, uint256 nftId, uint256 amount);
    event Claimed(address indexed user, uint256 nftId, uint256 reward);

    constructor(address _stakingToken, address _treasury) {
        stakingToken = IERC20(_stakingToken);
        treasury = _treasury;
        stakingNFT = new StakingNFT();
        rewardRate = 1e18; // placeholder
        lastUpdateBlock = block.number;
    }

    function setRewardRate(uint256 rate) external onlyOwner {
        updatePool();
        rewardRate = rate;
    }

    function updatePool() public {
        if (totalStaked == 0) {
            lastUpdateBlock = block.number;
            return;
        }
        uint256 blocks = block.number - lastUpdateBlock;
        accRewardPerShare += (blocks * rewardRate * 1e12) / totalStaked;
        lastUpdateBlock = block.number;
    }

    function stake(uint256 amount) external {
        updatePool();
        stakingToken.transferFrom(msg.sender, address(this), amount);
        uint256 nftId = stakingNFT.mint(msg.sender);
        stakeAmount[nftId] = amount;
        rewardDebt[nftId] = (amount * accRewardPerShare) / 1e12;
        userNFTs[msg.sender].push(nftId);
        totalStaked += amount;
        emit Staked(msg.sender, nftId, amount);
    }

    function unstake(uint256 nftId) external {
        require(stakingNFT.ownerOf(nftId) == msg.sender, "Not owner");
        updatePool();
        uint256 amount = stakeAmount[nftId];
        uint256 pending = ((amount * accRewardPerShare) / 1e12) - rewardDebt[nftId];
        stakingToken.transfer(msg.sender, amount);
        if (pending > 0) stakingToken.transfer(msg.sender, pending);
        stakingNFT.burn(nftId);
        totalStaked -= amount;
        emit Unstaked(msg.sender, nftId, amount);
        emit Claimed(msg.sender, nftId, pending);
    }

    function claim(uint256 nftId) external {
        require(stakingNFT.ownerOf(nftId) == msg.sender, "Not owner");
        updatePool();
        uint256 amount = stakeAmount[nftId];
        uint256 pending = ((amount * accRewardPerShare) / 1e12) - rewardDebt[nftId];
        if (pending > 0) stakingToken.transfer(msg.sender, pending);
        rewardDebt[nftId] = (amount * accRewardPerShare) / 1e12;
        emit Claimed(msg.sender, nftId, pending);
    }
}
