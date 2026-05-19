// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../contracts/AtlasSphereX3.sol";
import "../contracts/WrappedX3.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract CrossChainGovernance is Ownable {
    struct Proposal {
        string description;
        uint256 voteStart;
        uint256 voteEnd;
        uint256 forVotes;
        uint256 againstVotes;
        bool executed;
        mapping(address => bool) voted;
    }

    AtlasSphereX3 public x3;
    mapping(uint256 => WrappedX3) public wrappedTokens; // chainId => wrapped
    uint256 public proposalCount;
    mapping(uint256 => Proposal) public proposals;
    address public treasury;

    event ProposalCreated(uint256 indexed id, string description);
    event Voted(uint256 indexed id, address indexed voter, bool support, uint256 weight);
    event Executed(uint256 indexed id);

    constructor(address _x3, address _treasury) {
        x3 = AtlasSphereX3(_x3);
        treasury = _treasury;
    }

    function addWrapped(uint256 chainId, address wrapped) external onlyOwner {
        wrappedTokens[chainId] = WrappedX3(wrapped);
    }

    function createProposal(string memory description, uint256 duration) external onlyOwner {
        Proposal storage p = proposals[++proposalCount];
        p.description = description;
        p.voteStart = block.number;
        p.voteEnd = block.number + duration;
        emit ProposalCreated(proposalCount, description);
    }

    function vote(uint256 id, bool support) external {
        Proposal storage p = proposals[id];
        require(block.number >= p.voteStart && block.number <= p.voteEnd, "Voting closed");
        require(!p.voted[msg.sender], "Already voted");
        uint256 weight = x3.balanceOf(msg.sender);
        for (uint256 i = 0; i < 103; i++) {
            if (address(wrappedTokens[i]) != address(0)) {
                weight += wrappedTokens[i].balanceOf(msg.sender);
            }
        }
        require(weight > 0, "No voting power");
        if (support) p.forVotes += weight;
        else p.againstVotes += weight;
        p.voted[msg.sender] = true;
        emit Voted(id, msg.sender, support, weight);
    }

    function execute(uint256 id) external onlyOwner {
        Proposal storage p = proposals[id];
        require(!p.executed, "Already executed");
        require(block.number > p.voteEnd, "Voting not ended");
        p.executed = true;
        emit Executed(id);
    }
}
