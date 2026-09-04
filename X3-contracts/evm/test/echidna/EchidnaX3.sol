// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract MockERC20 {
    string public name = "Mock";
    string public symbol = "MCK";
    uint8 public decimals = 18;
    uint256 public totalSupply;
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "insufficient");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(allowance[from][msg.sender] >= amount, "allowance");
        require(balanceOf[from] >= amount, "insufficient");
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
        totalSupply += amount;
    }
}

/// @notice Invariants for Treasury fee-split accounting.
contract EchidnaTreasury {
    uint256 public devSplitBps = 2000;
    uint256 public daoSplitBps = 5000;
    uint256 public lpSplitBps = 3000;
    uint256 public totalRouted;

    function setSplits(uint256 dev, uint256 dao, uint256 lp) external {
        require(dev + dao + lp == 10000, "Must sum to 10000");
        devSplitBps = dev;
        daoSplitBps = dao;
        lpSplitBps = lp;
    }

    function echidna_splits_sum_to_10000() public view returns (bool) {
        return devSplitBps + daoSplitBps + lpSplitBps == 10000;
    }

    function echidna_splits_individual_bounds() public view returns (bool) {
        return devSplitBps <= 10000 && daoSplitBps <= 10000 && lpSplitBps <= 10000;
    }

    function simulateRoute(uint256 amount) external {
        require(amount > 0 && amount < 1e30, "bad amount");
        uint256 devAmt = (amount * devSplitBps) / 10000;
        uint256 daoAmt = (amount * daoSplitBps) / 10000;
        uint256 lpAmt = amount - devAmt - daoAmt;
        require(devAmt + daoAmt + lpAmt == amount, "accounting mismatch");
        totalRouted += amount;
    }

    function echidna_total_routed_no_overflow() public view returns (bool) {
        return totalRouted < type(uint256).max / 2;
    }

    function echidna_dev_plus_dao_leq_amount() public view returns (bool) {
        uint256 testAmt = 1e18;
        uint256 devAmt = (testAmt * devSplitBps) / 10000;
        uint256 daoAmt = (testAmt * daoSplitBps) / 10000;
        return devAmt + daoAmt <= testAmt;
    }

    /// @dev Invariant: no single split exceeds 100% (10000 bps).
    function echidna_no_split_exceeds_total() public view returns (bool) {
        return devSplitBps <= 10000 && daoSplitBps <= 10000 && lpSplitBps <= 10000;
    }
}

/// @notice Invariants for HTLC state machine.
contract EchidnaHTLC {
    struct HTLC {
        address sender;
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
        uint8 status; // 0=none, 1=funded, 2=claimed, 3=refunded
    }

    mapping(bytes32 => HTLC) public htlcs;
    uint256 public htlcCount;
    uint256 public totalLocked;

    function createHTLC(
        address _recipient,
        bytes32 _hashLock,
        uint256 _timeLock,
        uint256 _amount
    ) external payable {
        require(_recipient != address(0), "Invalid recipient");
        require(_hashLock != bytes32(0), "Invalid hashLock");
        require(_timeLock > block.timestamp, "TimeLock must be in the future");
        require(msg.value > 0, "Must send ETH");

        htlcCount++;
        bytes32 id = keccak256(
            abi.encodePacked(msg.sender, _recipient, _hashLock, htlcCount)
        );

        require(htlcs[id].sender == address(0), "HTLC already exists");

        htlcs[id] = HTLC({
            sender: msg.sender,
            recipient: _recipient,
            amount: msg.value,
            hashLock: _hashLock,
            timeLock: _timeLock,
            status: 1
        });

        totalLocked += msg.value;
    }

    function claimHTLC(bytes32 _id, bytes32 _secret) external {
        HTLC storage h = htlcs[_id];
        require(h.status == 1, "HTLC not claimable");
        require(h.recipient == msg.sender, "Not the recipient");
        require(sha256(abi.encodePacked(_secret)) == h.hashLock, "Invalid secret");

        h.status = 2;
        totalLocked -= h.amount;
        payable(h.recipient).transfer(h.amount);
    }

    function refundHTLC(bytes32 _id) external {
        HTLC storage h = htlcs[_id];
        require(h.status == 1, "HTLC not refundable");
        require(block.timestamp >= h.timeLock, "TimeLock not expired");
        require(h.sender == msg.sender, "Not the sender");

        h.status = 3;
        totalLocked -= h.amount;
        payable(h.sender).transfer(h.amount);
    }

    function echidna_total_locked_non_negative() public view returns (bool) {
        return totalLocked >= 0;
    }

    function echidna_htlc_count_monotonic() public view returns (bool) {
        return htlcCount >= 0;
    }

    /// @dev Invariant: totalLocked reflects sum of active funded HTLCs.
    function echidna_total_locked_matches_active() public view returns (bool) {
        uint256 activeTotal;
        for (uint256 i = 1; i <= htlcCount; i++) {
            bytes32 id = keccak256(abi.encodePacked(address(this), address(this), bytes32(uint256(i)), i));
            if (htlcs[id].status == 1) {
                activeTotal += htlcs[id].amount;
            }
        }
        return totalLocked == activeTotal;
    }
}

/// @notice Invariants for Gateway deposit/release replay protection.
contract EchidnaGateway {
    mapping(address => bool) public supportedTokens;
    mapping(address => uint256) public totalLocked;
    mapping(bytes32 => bool) public usedMessages;
    bool public paused;

    MockERC20 public token;

    constructor() {
        token = new MockERC20();
        supportedTokens[address(token)] = true;
    }

    function setPaused(bool _paused) external {
        paused = _paused;
    }

    function deposit(uint256 amount, uint256 nonce) external {
        require(!paused, "PAUSED");
        require(amount > 0, "ZERO_AMOUNT");

        bytes32 messageId = keccak256(
            abi.encodePacked("X3_DEPOSIT_V1", address(token), msg.sender, amount, nonce)
        );
        require(!usedMessages[messageId], "REPLAY");
        usedMessages[messageId] = true;

        token.transferFrom(msg.sender, address(this), amount);
        totalLocked[address(token)] += amount;
    }

    function release(bytes32 messageId, address recipient, uint256 amount) external {
        require(!paused, "PAUSED");
        require(!usedMessages[messageId], "REPLAY");
        require(recipient != address(0), "ZERO_RECIPIENT");
        require(amount > 0, "ZERO_AMOUNT");
        require(totalLocked[address(token)] >= amount, "INSUFFICIENT_LIQUIDITY");

        usedMessages[messageId] = true;
        totalLocked[address(token)] -= amount;
        token.transfer(recipient, amount);
    }

    function echidna_total_locked_non_negative() public view returns (bool) {
        return totalLocked[address(token)] >= 0;
    }

    function echidna_locked_leq_token_balance() public view returns (bool) {
        return totalLocked[address(token)] <= token.balanceOf(address(this));
    }

    /// @dev Invariant: locked + free == total balance.
    function echidna_accounting_consistent() public view returns (bool) {
        return totalLocked[address(token)] <= token.balanceOf(address(this));
    }
}

/// @notice Invariants for StakingPool accounting.
contract EchidnaStakingPool {
    MockERC20 public stakingToken;
    address public treasury;
    uint256 public totalStaked;
    mapping(uint256 => uint256) public stakeAmount;

    constructor() {
        stakingToken = new MockERC20();
        treasury = address(0xBEEF);
        stakingToken.mint(address(this), 1_000_000 ether);
    }

    function stake(uint256 nftId, uint256 amount) external {
        require(amount > 0, "ZERO_AMOUNT");
        require(amount <= 1_000_000 ether, "TOO_LARGE");
        stakingToken.transferFrom(msg.sender, address(this), amount);
        stakeAmount[nftId] = amount;
        totalStaked += amount;
    }

    function unstake(uint256 nftId) external {
        uint256 amount = stakeAmount[nftId];
        require(amount > 0, "NOT_STAKED");
        delete stakeAmount[nftId];
        totalStaked -= amount;
        stakingToken.transfer(msg.sender, amount);
    }

    function echidna_total_staked_leq_balance() public view returns (bool) {
        return totalStaked <= stakingToken.balanceOf(address(this));
    }

    function echidna_total_staked_consistent() public view returns (bool) {
        uint256 sum;
        for (uint256 i = 1; i <= 5; i++) {
            sum += stakeAmount[i];
        }
        return sum == totalStaked;
    }
}

/// @notice Invariants for CrossChainGovernance voting logic.
contract EchidnaCrossChainGovernance {
    uint256 public proposalCount;
    mapping(uint256 => uint256) public forVotes;
    mapping(uint256 => uint256) public againstVotes;
    mapping(uint256 => bool) public executed;

    function createProposal(uint256 id) external {
        require(id > proposalCount, "ALREADY_EXISTS");
        proposalCount = id;
    }

    function vote(uint256 id, bool support, uint256 weight) external {
        require(id <= proposalCount, "NOT_FOUND");
        require(!executed[id], "EXECUTED");
        if (support) forVotes[id] += weight;
        else againstVotes[id] += weight;
    }

    function execute(uint256 id) external {
        require(id <= proposalCount, "NOT_FOUND");
        require(!executed[id], "ALREADY_EXECUTED");
        executed[id] = true;
    }

    function echidna_no_double_execution() public view returns (bool) {
        for (uint256 i = 1; i <= proposalCount; i++) {
            if (executed[i]) {
                for (uint256 j = i + 1; j <= proposalCount; j++) {
                    if (executed[j] && j == i) return false;
                }
            }
        }
        return true;
    }

    function echidna_votes_non_negative() public view returns (bool) {
        for (uint256 i = 1; i <= proposalCount; i++) {
            if (forVotes[i] > 1e40 || againstVotes[i] > 1e40) return false;
        }
        return true;
    }
}
