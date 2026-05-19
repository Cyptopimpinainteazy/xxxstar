// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title AtlasHTLC
 * @notice Hash Time-Locked Contract for cross-chain atomic swaps on EVM chains.
 *
 * Supports native ETH and ERC-20 tokens.
 *
 * Function selectors (must match the SDK's evm.ts adapter):
 *   createHTLC:  0x4b2f336d
 *   claimHTLC:   0x84cc315c
 *   refundHTLC:  0x7249fbb6
 *   getHTLC:     0x905d22a5
 */

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract AtlasHTLC is ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ─── Types ──────────────────────────────────────────────

    enum HTLCStatus {
        Pending,   // 0 — created, not yet funded (for ERC-20)
        Funded,    // 1 — funds locked
        Claimed,   // 2 — recipient claimed with secret
        Refunded,  // 3 — sender refunded after expiry
        Expired    // 4 — timelock passed, awaiting refund
    }

    struct HTLC {
        address sender;
        address recipient;
        address token;          // address(0) = native ETH
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;       // Unix timestamp
        HTLCStatus status;
        bytes32 secret;         // Stored on claim
    }

    // ─── State ──────────────────────────────────────────────

    mapping(bytes32 => HTLC) public htlcs;
    uint256 public htlcCount;

    // ─── Events ─────────────────────────────────────────────

    event HTLCCreated(
        bytes32 indexed id,
        address indexed sender,
        address indexed recipient,
        address token,
        uint256 amount,
        bytes32 hashLock,
        uint256 timeLock
    );

    event HTLCClaimed(
        bytes32 indexed id,
        address indexed claimant,
        bytes32 secret
    );

    event HTLCRefunded(
        bytes32 indexed id,
        address indexed sender
    );

    // ─── Modifiers ──────────────────────────────────────────

    modifier htlcExists(bytes32 _id) {
        require(htlcs[_id].sender != address(0), "HTLC does not exist");
        _;
    }

    // ─── Create ─────────────────────────────────────────────

    /**
     * @notice Create a new HTLC.
     * @param _recipient   Address that can claim with the secret
     * @param _hashLock    SHA-256 hash of the secret
     * @param _timeLock    Unix timestamp after which sender can refund
     * @param _token       ERC-20 token address (address(0) for native ETH)
     * @param _amount      Amount to lock (ignored for native ETH — uses msg.value)
     * @return id          Unique HTLC identifier
     *
     * selector: 0x4b2f336d
     */
    function createHTLC(
        address _recipient,
        bytes32 _hashLock,
        uint256 _timeLock,
        address _token,
        uint256 _amount
    ) external payable nonReentrant returns (bytes32 id) {
        require(_recipient != address(0), "Invalid recipient");
        require(_hashLock != bytes32(0), "Invalid hashLock");
        require(_timeLock > block.timestamp, "TimeLock must be in the future");

        htlcCount++;
        id = keccak256(
            abi.encodePacked(msg.sender, _recipient, _hashLock, htlcCount)
        );

        require(htlcs[id].sender == address(0), "HTLC already exists");

        uint256 lockAmount;

        if (_token == address(0)) {
            // Native ETH
            require(msg.value > 0, "Must send ETH");
            lockAmount = msg.value;
        } else {
            // ERC-20
            require(_amount > 0, "Amount must be > 0");
            lockAmount = _amount;
            IERC20(_token).safeTransferFrom(msg.sender, address(this), _amount);
        }

        htlcs[id] = HTLC({
            sender: msg.sender,
            recipient: _recipient,
            token: _token,
            amount: lockAmount,
            hashLock: _hashLock,
            timeLock: _timeLock,
            status: HTLCStatus.Funded,
            secret: bytes32(0)
        });

        emit HTLCCreated(id, msg.sender, _recipient, _token, lockAmount, _hashLock, _timeLock);
    }

    // ─── Claim ──────────────────────────────────────────────

    /**
     * @notice Claim an HTLC by revealing the secret.
     * @param _id      HTLC identifier
     * @param _secret  Pre-image of the hashLock
     *
     * selector: 0x84cc315c
     */
    function claimHTLC(
        bytes32 _id,
        bytes32 _secret
    ) external nonReentrant htlcExists(_id) {
        HTLC storage h = htlcs[_id];

        require(h.status == HTLCStatus.Funded, "HTLC not claimable");
        require(h.recipient == msg.sender, "Not the recipient");
        require(
            sha256(abi.encodePacked(_secret)) == h.hashLock,
            "Invalid secret"
        );

        h.status = HTLCStatus.Claimed;
        h.secret = _secret;

        if (h.token == address(0)) {
            // Native ETH
            (bool sent, ) = payable(h.recipient).call{value: h.amount}("");
            require(sent, "ETH transfer failed");
        } else {
            // ERC-20
            IERC20(h.token).safeTransfer(h.recipient, h.amount);
        }

        emit HTLCClaimed(_id, msg.sender, _secret);
    }

    // ─── Refund ─────────────────────────────────────────────

    /**
     * @notice Refund an expired HTLC to the sender.
     * @param _id HTLC identifier
     *
     * selector: 0x7249fbb6
     */
    function refundHTLC(
        bytes32 _id
    ) external nonReentrant htlcExists(_id) {
        HTLC storage h = htlcs[_id];

        require(h.status == HTLCStatus.Funded, "HTLC not refundable");
        require(block.timestamp >= h.timeLock, "TimeLock not expired");
        require(h.sender == msg.sender, "Not the sender");

        h.status = HTLCStatus.Refunded;

        if (h.token == address(0)) {
            (bool sent, ) = payable(h.sender).call{value: h.amount}("");
            require(sent, "ETH transfer failed");
        } else {
            IERC20(h.token).safeTransfer(h.sender, h.amount);
        }

        emit HTLCRefunded(_id, msg.sender);
    }

    // ─── View ───────────────────────────────────────────────

    /**
     * @notice Get HTLC details.
     * @param _id HTLC identifier
     *
     * selector: 0x905d22a5
     */
    function getHTLC(bytes32 _id)
        external
        view
        htlcExists(_id)
        returns (
            address sender,
            address recipient,
            address token,
            uint256 amount,
            bytes32 hashLock,
            uint256 timeLock,
            HTLCStatus status,
            bytes32 secret
        )
    {
        HTLC storage h = htlcs[_id];
        return (
            h.sender,
            h.recipient,
            h.token,
            h.amount,
            h.hashLock,
            h.timeLock,
            h.status,
            h.secret
        );
    }

    /**
     * @notice Check if an HTLC is funded.
     */
    function isHTLCFunded(bytes32 _id) external view returns (bool) {
        return htlcs[_id].status == HTLCStatus.Funded;
    }

    /**
     * @notice Check if an HTLC has been claimed, and return the secret if so.
     */
    function isHTLCClaimed(bytes32 _id) external view returns (bool claimed, bytes32 secret) {
        HTLC storage h = htlcs[_id];
        if (h.status == HTLCStatus.Claimed) {
            return (true, h.secret);
        }
        return (false, bytes32(0));
    }

    /**
     * @notice Check if an HTLC timelock has expired.
     */
    function isHTLCExpired(bytes32 _id) external view returns (bool) {
        return block.timestamp >= htlcs[_id].timeLock && htlcs[_id].status == HTLCStatus.Funded;
    }
}
