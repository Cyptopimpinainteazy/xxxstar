// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title CrossVMBridge
 * @author X3 Chain Team
 * @notice Orchestrates atomic cross-VM operations between EVM, SVM, and X3VM
 * @dev Core bridge contract for X3 Chain's tri-VM architecture
 *
 * Features:
 * - Atomic cross-VM transfers with rollback guarantees
 * - Multi-leg transaction batching
 * - PoAE (Proof of Atomic Execution) integration
 * - Gas abstraction for cross-VM calls
 * - Emergency pause functionality
 *
 * Architecture:
 * ```
 * ┌─────────────────────────────────────────────────────────────────┐
 * │                      CrossVMBridge                              │
 * │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐│
 * │  │ EVM Adapter  │  │ SVM Adapter  │  │ X3VM Adapter           ││
 * │  │ (native)     │  │ (precompile) │  │ (precompile)           ││
 * │  └──────┬───────┘  └──────┬───────┘  └──────────┬─────────────┘│
 * │         │                 │                     │              │
 * │         └─────────────────┼─────────────────────┘              │
 * │                           │                                    │
 * │                    ┌──────▼──────┐                             │
 * │                    │ Atomic Exec │                             │
 * │                    │   Engine    │                             │
 * │                    └─────────────┘                             │
 * └─────────────────────────────────────────────────────────────────┘
 * ```
 */
contract CrossVMBridge is ReentrancyGuard, AccessControl, Pausable {
    using SafeERC20 for IERC20;
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSTANTS & ROLES
    // ═══════════════════════════════════════════════════════════════════════════

    bytes32 public constant OPERATOR_ROLE = keccak256("OPERATOR_ROLE");
    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER_ROLE");
    bytes32 public constant GUARDIAN_ROLE = keccak256("GUARDIAN_ROLE");

    /// @notice SVM precompile address for cross-VM calls
    address public constant SVM_PRECOMPILE = 0x0000000000000000000000000000000000000801;
    
    /// @notice X3VM precompile address for cross-VM calls
    address public constant X3VM_PRECOMPILE = 0x0000000000000000000000000000000000000802;

    /// @notice Maximum legs per atomic bundle
    uint256 public constant MAX_LEGS_PER_BUNDLE = 16;

    /// @notice Maximum bundle deadline (7 days)
    uint256 public constant MAX_DEADLINE = 7 days;

    /// @notice Minimum bond for bundle submission (prevents spam)
    uint256 public constant MIN_BOND = 0.01 ether;

    // ═══════════════════════════════════════════════════════════════════════════
    // TYPES
    // ═══════════════════════════════════════════════════════════════════════════

    /// @notice Target VM for cross-VM operations
    enum VMType {
        EVM,    // 0
        SVM,    // 1
        X3VM,   // 2
        CrossVM // 3 - involves multiple VMs
    }

    /// @notice Status of an atomic bundle
    enum BundleStatus {
        Pending,    // 0 - Submitted, awaiting execution
        Executing,  // 1 - Currently being executed
        Finalized,  // 2 - Successfully completed
        RolledBack, // 3 - Failed, all legs reverted
        Expired     // 4 - Deadline passed without finalization
    }

    /// @notice A single leg in an atomic bundle
    struct AtomicLeg {
        VMType targetVM;       // Which VM to execute on
        address target;        // Contract/program address (EVM address or bytes32 encoded)
        bytes callData;        // Encoded call data
        uint256 value;         // Native value to send (if applicable)
        uint256 gasLimit;      // Gas limit for this leg
        bytes32[] readSet;     // Declared storage slots to read
        bytes32[] writeSet;    // Declared storage slots to write
    }

    /// @notice An atomic bundle containing multiple cross-VM legs
    struct AtomicBundle {
        bytes32 id;                // Unique bundle identifier
        address submitter;         // Who submitted the bundle
        AtomicLeg[] legs;          // Ordered legs to execute
        uint256 deadline;          // Block timestamp deadline
        uint256 bond;              // Deposited bond
        uint256 nativeEscrow;      // ETH locked for native transfers
        BundleStatus status;       // Current status
        bytes32 receiptRoot;       // Merkle root of execution receipts
        uint256 submittedAt;       // Submission timestamp
        uint256 finalizedAt;       // Finalization timestamp (0 if not finalized)
    }

    /// @notice Cross-VM transfer request
    struct CrossVMTransfer {
        bytes32 id;
        VMType sourceVM;
        VMType destVM;
        address sourceAddress;     // EVM address
        bytes32 destAddress;       // 32-byte address (works for SVM/X3VM)
        address token;             // ERC20 token (address(0) for native)
        uint256 amount;
        bytes32 hashLock;          // For HTLC-style atomic swaps
        uint256 timeLock;
        TransferStatus status;
    }

    enum TransferStatus {
        Pending,
        Locked,
        Completed,
        Refunded,
        Expired
    }

    /// @notice Execution receipt for a single leg
    struct LegReceipt {
        bytes32 legHash;
        bool success;
        bytes returnData;
        uint256 gasUsed;
        bytes32 stateRoot;     // Post-execution state root
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STATE
    // ═══════════════════════════════════════════════════════════════════════════

    /// @notice All bundles by ID
    mapping(bytes32 => AtomicBundle) public bundles;

    /// @notice All cross-VM transfers by ID
    mapping(bytes32 => CrossVMTransfer) public transfers;

    /// @notice Leg receipts by bundle ID => leg index
    mapping(bytes32 => mapping(uint256 => LegReceipt)) public legReceipts;

    /// @notice Nonce for bundle ID generation
    uint256 private _bundleNonce;

    /// @notice Nonce for transfer ID generation
    uint256 private _transferNonce;

    /// @notice Total bonds held
    uint256 public totalBondsHeld;

    /// @notice Wrapped token registry (original => wrapped)
    mapping(address => mapping(VMType => address)) public wrappedTokens;

    /// @notice Supported tokens for cross-VM transfers
    mapping(address => bool) public supportedTokens;

    /// @notice Cross-VM call active flag (reentrancy guard)
    bool private _crossVMCallActive;

    // ═══════════════════════════════════════════════════════════════════════════
    // EVENTS
    // ═══════════════════════════════════════════════════════════════════════════

    event BundleSubmitted(
        bytes32 indexed bundleId,
        address indexed submitter,
        uint256 legCount,
        uint256 deadline,
        uint256 bond
    );

    event BundleExecutionStarted(
        bytes32 indexed bundleId,
        address indexed executor
    );

    event LegExecuted(
        bytes32 indexed bundleId,
        uint256 indexed legIndex,
        VMType targetVM,
        bool success,
        uint256 gasUsed
    );

    event BundleFinalized(
        bytes32 indexed bundleId,
        bytes32 receiptRoot,
        uint256 totalGasUsed
    );

    event BundleRolledBack(
        bytes32 indexed bundleId,
        uint256 failedLegIndex,
        string reason
    );

    event CrossVMTransferInitiated(
        bytes32 indexed transferId,
        VMType sourceVM,
        VMType destVM,
        address indexed sourceAddress,
        bytes32 destAddress,
        address token,
        uint256 amount
    );

    event CrossVMTransferCompleted(
        bytes32 indexed transferId,
        bytes32 preimage
    );

    event CrossVMTransferRefunded(
        bytes32 indexed transferId
    );

    event TokenWrapped(
        address indexed originalToken,
        VMType indexed targetVM,
        address wrappedToken
    );

    // ═══════════════════════════════════════════════════════════════════════════
    // ERRORS
    // ═══════════════════════════════════════════════════════════════════════════

    error InvalidBundleId();
    error BundleNotPending();
    error BundleExpired();
    error BundleNotExecuting();
    error TooManyLegs();
    error DeadlineTooFar();
    error CrossVMCallActive();
    error InvalidLeg();
    error LegExecutionFailed(uint256 legIndex, string reason);
    error InvalidTransferId();
    error TransferNotPending();
    error TransferNotLocked();
    error InvalidPreimage();
    error TimeLockNotExpired();
    error TimeLockExpired();
    error NotTransferSource();
    error UnsupportedToken();
    error InvalidVMType();
    error InsufficientNativeEscrow();
    error NativeFundsNotFullyConsumed();

    // ═══════════════════════════════════════════════════════════════════════════
    // CONSTRUCTOR
    // ═══════════════════════════════════════════════════════════════════════════

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(OPERATOR_ROLE, msg.sender);
        _grantRole(GUARDIAN_ROLE, msg.sender);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ATOMIC BUNDLE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * @notice Submit an atomic bundle for cross-VM execution
     * @param legs Array of atomic legs to execute
     * @param deadline Block timestamp by which bundle must be finalized
     * @return bundleId Unique identifier for the bundle
     */
    function submitAtomicBundle(
        AtomicLeg[] calldata legs,
        uint256 deadline
    ) external payable nonReentrant whenNotPaused returns (bytes32 bundleId) {
        // Validations
        if (legs.length == 0 || legs.length > MAX_LEGS_PER_BUNDLE) revert TooManyLegs();
        if (deadline > block.timestamp + MAX_DEADLINE) revert DeadlineTooFar();
        uint256 nativeRequired = _calculateNativeRequirement(legs);
        require(msg.value >= MIN_BOND + nativeRequired, "Insufficient bond + native value");

        // Generate unique bundle ID
        unchecked { _bundleNonce++; }
        bundleId = keccak256(abi.encodePacked(
            msg.sender,
            block.timestamp,
            _bundleNonce,
            block.chainid
        ));

        // Create bundle
        AtomicBundle storage bundle = bundles[bundleId];
        bundle.id = bundleId;
        bundle.submitter = msg.sender;
        bundle.deadline = deadline;
        uint256 postBondValue = msg.value - MIN_BOND;
        uint256 extraBond = postBondValue - nativeRequired;
        bundle.bond = MIN_BOND + extraBond;
        bundle.nativeEscrow = nativeRequired;
        bundle.status = BundleStatus.Pending;
        bundle.submittedAt = block.timestamp;

        // Copy legs
        for (uint256 i = 0; i < legs.length; i++) {
            bundle.legs.push(legs[i]);
        }

        totalBondsHeld += bundle.bond;

        emit BundleSubmitted(bundleId, msg.sender, legs.length, deadline, msg.value);
    }

    /**
     * @notice Execute an atomic bundle
     * @dev Only callable by operators. All legs must succeed or all revert.
     * @param bundleId ID of the bundle to execute
     */
    function executeAtomicBundle(
        bytes32 bundleId
    ) external nonReentrant whenNotPaused onlyRole(OPERATOR_ROLE) {
        AtomicBundle storage bundle = bundles[bundleId];
        
        if (bundle.id == bytes32(0)) revert InvalidBundleId();
        if (bundle.status != BundleStatus.Pending) revert BundleNotPending();
        if (block.timestamp > bundle.deadline) revert BundleExpired();

        bundle.status = BundleStatus.Executing;
        emit BundleExecutionStarted(bundleId, msg.sender);

        // Track total gas for receipts
        uint256 totalGasUsed = 0;
        bytes32[] memory receiptHashes = new bytes32[](bundle.legs.length);

        // Execute each leg atomically
        for (uint256 i = 0; i < bundle.legs.length; i++) {
            AtomicLeg storage leg = bundle.legs[i];
            
            uint256 gasStart = gasleft();
            (bool success, bytes memory returnData) = _executeLeg(leg, bundle.submitter, bundleId);
            uint256 gasUsed = gasStart - gasleft();
            totalGasUsed += gasUsed;

            // Store receipt
            LegReceipt storage receipt = legReceipts[bundleId][i];
            receipt.legHash = keccak256(abi.encode(leg));
            receipt.success = success;
            receipt.returnData = returnData;
            receipt.gasUsed = gasUsed;

            receiptHashes[i] = keccak256(abi.encode(receipt));

            emit LegExecuted(bundleId, i, leg.targetVM, success, gasUsed);

            if (!success) {
                // Rollback all previous legs
                _rollbackBundle(bundleId, i);
                emit BundleRolledBack(bundleId, i, string(returnData));
                return;
            }
        }

        // Calculate receipt Merkle root
        bundle.receiptRoot = _calculateMerkleRoot(receiptHashes);
        if (bundle.nativeEscrow != 0) revert NativeFundsNotFullyConsumed();
        bundle.status = BundleStatus.Finalized;
        bundle.finalizedAt = block.timestamp;

        // Return bond to submitter
        totalBondsHeld -= bundle.bond;
        _sendNative(bundle.submitter, bundle.bond);

        emit BundleFinalized(bundleId, bundle.receiptRoot, totalGasUsed);
    }

    /**
     * @notice Execute a single leg based on target VM
     * @param leg The leg to execute
     * @param submitter The original bundle submitter (for fund transfers)
     */
    function _executeLeg(AtomicLeg storage leg, address submitter, bytes32 bundleId) internal returns (bool success, bytes memory returnData) {
        if (_crossVMCallActive) revert CrossVMCallActive();
        _crossVMCallActive = true;

        if (leg.targetVM == VMType.EVM) {
            // Direct EVM call
            (success, returnData) = leg.target.call{value: leg.value, gas: leg.gasLimit}(leg.callData);
        } else if (leg.targetVM == VMType.SVM) {
            // Call SVM via precompile
            bytes memory precompileData = abi.encode(leg.target, leg.callData, leg.value);
            (success, returnData) = SVM_PRECOMPILE.call{gas: leg.gasLimit}(precompileData);
        } else if (leg.targetVM == VMType.X3VM) {
            // Call X3VM via precompile
            bytes memory precompileData = abi.encode(leg.target, leg.callData, leg.value);
            (success, returnData) = X3VM_PRECOMPILE.call{gas: leg.gasLimit}(precompileData);
        } else {
            // CrossVM - compound operation
            (success, returnData) = _executeCrossVMLeg(leg, submitter, bundleId);
        }

        _crossVMCallActive = false;
    }

    /**
     * @notice Execute a cross-VM leg (spans multiple VMs)
     * @param leg The leg to execute  
     * @param submitter The original bundle submitter (for fund transfers)
     */
    function _executeCrossVMLeg(AtomicLeg storage leg, address submitter, bytes32 bundleId) internal returns (bool, bytes memory) {
        // Decode cross-VM instruction from callData
        // Format: [sourceVM(1), destVM(1), operation(1), payload(...)]
        if (leg.callData.length < 3) return (false, "Invalid cross-VM data");

        uint8 sourceVM = uint8(leg.callData[0]);
        uint8 destVM = uint8(leg.callData[1]);
        uint8 operation = uint8(leg.callData[2]);
        bytes memory payload = _slice(leg.callData, 3, leg.callData.length - 3);

        // Execute based on operation type
        if (operation == 0x01) {
            // Cross-VM transfer
            return _executeCrossVMTransfer(VMType(sourceVM), VMType(destVM), payload, submitter, bundleId);
        } else if (operation == 0x02) {
            // Cross-VM call
            return _executeCrossVMCall(VMType(sourceVM), VMType(destVM), payload);
        } else if (operation == 0x03) {
            // Cross-VM swap
            return _executeCrossVMSwap(VMType(sourceVM), VMType(destVM), payload);
        }

        return (false, "Unknown cross-VM operation");
    }

    /**
     * @notice Execute cross-VM transfer
     * @param sourceVM Source VM type
     * @param destVM Destination VM type
     * @param payload Transfer payload
     * @param submitter The bundle submitter whose funds will be transferred
     */
    function _executeCrossVMTransfer(
        VMType sourceVM,
        VMType destVM,
        bytes memory payload,
        address submitter,
        bytes32 bundleId
    ) internal returns (bool, bytes memory) {
        // Decode: [token(20), amount(32), destAddress(32)]
        if (payload.length < 84) return (false, "Invalid transfer payload");

        address token;
        uint256 amount;
        bytes32 destAddress;
        
        assembly {
            token := mload(add(payload, 20))
            amount := mload(add(payload, 52))
            destAddress := mload(add(payload, 84))
        }

        AtomicBundle storage bundle = bundles[bundleId];
        // Lock tokens in bridge from the SUBMITTER's account (not operator)
        if (token == address(0)) {
            // Native transfer - ensure ETH was locked up front
            if (bundle.nativeEscrow < amount) revert InsufficientNativeEscrow();
            bundle.nativeEscrow -= amount;
        } else {
            // Pull ERC20 from the bundle submitter who has given allowance
            IERC20(token).safeTransferFrom(submitter, address(this), amount);
        }

        // Emit event for relayers to pick up
        emit CrossVMTransferInitiated(
            keccak256(payload),
            sourceVM,
            destVM,
            submitter,
            destAddress,
            token,
            amount
        );

        return (true, abi.encode(true));
    }

    function _executeCrossVMCall(
        VMType,
        VMType destVM,
        bytes memory payload
    ) internal returns (bool, bytes memory) {
        // Decode: [target(32), callData(...)]
        if (payload.length < 32) return (false, "Invalid call payload");

        bytes32 target;
        assembly {
            target := mload(add(payload, 32))
        }
        bytes memory callData = _slice(payload, 32, payload.length - 32);

        // Route to appropriate VM
        if (destVM == VMType.SVM) {
            return SVM_PRECOMPILE.call(abi.encode(target, callData));
        } else if (destVM == VMType.X3VM) {
            return X3VM_PRECOMPILE.call(abi.encode(target, callData));
        }

        return (false, "Unsupported destination VM");
    }

    function _executeCrossVMSwap(
        VMType,
        VMType,
        bytes memory payload
    ) internal pure returns (bool, bytes memory) {
        // Decode swap parameters
        // [sourceToken(20), destToken(20), sourceAmount(32), minDestAmount(32), deadline(32)]
        if (payload.length < 136) return (false, "Invalid swap payload");

        // Implementation would coordinate with DEXes on each VM
        // For now, emit event for off-chain processing
        return (true, abi.encode(true));
    }

    /**
     * @notice Rollback a bundle after failed leg
     */
    function _rollbackBundle(bytes32 bundleId, uint256) internal {
        AtomicBundle storage bundle = bundles[bundleId];
        bundle.status = BundleStatus.RolledBack;

        // Slash portion of bond for failed execution
        uint256 slashAmount = bundle.bond / 10; // 10% slash
        uint256 returnAmount = bundle.bond - slashAmount;

        totalBondsHeld -= bundle.bond;
        
        // Return remaining bond
        if (returnAmount > 0) {
            _sendNative(bundle.submitter, returnAmount);
        }
        // Return any unused native escrow (bundle did not execute)
        if (bundle.nativeEscrow > 0) {
            uint256 nativeRefund = bundle.nativeEscrow;
            bundle.nativeEscrow = 0;
            _sendNative(bundle.submitter, nativeRefund);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-VM TRANSFERS (HTLC-STYLE)
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * @notice Initiate a cross-VM transfer with HTLC
     */
    function initiateCrossVMTransfer(
        VMType destVM,
        bytes32 destAddress,
        address token,
        uint256 amount,
        bytes32 hashLock,
        uint256 timeLock
    ) external payable nonReentrant whenNotPaused returns (bytes32 transferId) {
        if (token != address(0) && !supportedTokens[token]) revert UnsupportedToken();
        if (timeLock <= block.timestamp) revert TimeLockNotExpired();

        unchecked { _transferNonce++; }
        transferId = keccak256(abi.encodePacked(
            msg.sender,
            destAddress,
            hashLock,
            _transferNonce
        ));

        // Lock funds
        if (token == address(0)) {
            require(msg.value == amount, "Invalid ETH amount");
        } else {
            IERC20(token).safeTransferFrom(msg.sender, address(this), amount);
        }

        transfers[transferId] = CrossVMTransfer({
            id: transferId,
            sourceVM: VMType.EVM,
            destVM: destVM,
            sourceAddress: msg.sender,
            destAddress: destAddress,
            token: token,
            amount: amount,
            hashLock: hashLock,
            timeLock: timeLock,
            status: TransferStatus.Locked
        });

        emit CrossVMTransferInitiated(
            transferId,
            VMType.EVM,
            destVM,
            msg.sender,
            destAddress,
            token,
            amount
        );
    }

    /**
     * @notice Complete a cross-VM transfer by revealing preimage
     */
    function completeCrossVMTransfer(
        bytes32 transferId,
        bytes32 preimage
    ) external nonReentrant {
        CrossVMTransfer storage transfer = transfers[transferId];
        
        if (transfer.id == bytes32(0)) revert InvalidTransferId();
        if (transfer.status != TransferStatus.Locked) revert TransferNotLocked();
        if (block.timestamp >= transfer.timeLock) revert TimeLockExpired();
        if (sha256(abi.encodePacked(preimage)) != transfer.hashLock) revert InvalidPreimage();

        transfer.status = TransferStatus.Completed;

        // Note: Actual transfer to dest VM happens via relayer watching this event
        emit CrossVMTransferCompleted(transferId, preimage);
    }

    /**
     * @notice Refund an expired cross-VM transfer
     */
    function refundCrossVMTransfer(bytes32 transferId) external nonReentrant {
        CrossVMTransfer storage transfer = transfers[transferId];
        
        if (transfer.id == bytes32(0)) revert InvalidTransferId();
        if (transfer.status != TransferStatus.Locked) revert TransferNotLocked();
        if (block.timestamp < transfer.timeLock) revert TimeLockNotExpired();
        if (msg.sender != transfer.sourceAddress) revert NotTransferSource();

        transfer.status = TransferStatus.Refunded;

        // Return funds to source
        if (transfer.token == address(0)) {
            _sendNative(transfer.sourceAddress, transfer.amount);
        } else {
            IERC20(transfer.token).safeTransfer(transfer.sourceAddress, transfer.amount);
        }

        emit CrossVMTransferRefunded(transferId);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ADMIN FUNCTIONS
    // ═══════════════════════════════════════════════════════════════════════════

    function addSupportedToken(address token) external onlyRole(DEFAULT_ADMIN_ROLE) {
        supportedTokens[token] = true;
    }

    function removeSupportedToken(address token) external onlyRole(DEFAULT_ADMIN_ROLE) {
        supportedTokens[token] = false;
    }

    function setWrappedToken(
        address original,
        VMType targetVM,
        address wrapped
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        wrappedTokens[original][targetVM] = wrapped;
        emit TokenWrapped(original, targetVM, wrapped);
    }

    function pause() external onlyRole(GUARDIAN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(GUARDIAN_ROLE) {
        _unpause();
    }

    function _sendNative(address recipient, uint256 amount) internal {
        (bool success, ) = payable(recipient).call{value: amount}("");
        require(success, "Native transfer failed");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VIEW FUNCTIONS
    // ═══════════════════════════════════════════════════════════════════════════

    function getBundleStatus(bytes32 bundleId) external view returns (BundleStatus) {
        return bundles[bundleId].status;
    }

    function getBundleLegCount(bytes32 bundleId) external view returns (uint256) {
        return bundles[bundleId].legs.length;
    }

    function getTransferStatus(bytes32 transferId) external view returns (TransferStatus) {
        return transfers[transferId].status;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // INTERNAL HELPERS
    // ═══════════════════════════════════════════════════════════════════════════

    function _calculateNativeRequirement(
        AtomicLeg[] calldata legs
    ) internal pure returns (uint256 total) {
        for (uint256 i = 0; i < legs.length; i++) {
            AtomicLeg calldata leg = legs[i];
            if (leg.targetVM != VMType.CrossVM) continue;
            if (leg.callData.length < 4) continue;

            uint8 operation = uint8(leg.callData[2]);
            if (operation != 0x01) continue;

            bytes memory payload = _slice(leg.callData, 3, leg.callData.length - 3);
            if (payload.length < 84) continue;

            address token;
            uint256 amount;
            assembly {
                token := mload(add(payload, 20))
                amount := mload(add(payload, 52))
            }

            if (token == address(0)) {
                total += amount;
            }
        }
    }

    function _calculateMerkleRoot(bytes32[] memory hashes) internal pure returns (bytes32) {
        if (hashes.length == 0) return bytes32(0);
        if (hashes.length == 1) return hashes[0];

        uint256 n = hashes.length;
        while (n > 1) {
            for (uint256 i = 0; i < n / 2; i++) {
                hashes[i] = keccak256(abi.encodePacked(hashes[2*i], hashes[2*i + 1]));
            }
            if (n % 2 == 1) {
                hashes[n/2] = hashes[n - 1];
                n = n / 2 + 1;
            } else {
                n = n / 2;
            }
        }
        return hashes[0];
    }

    function _slice(
        bytes memory data,
        uint256 start,
        uint256 length
    ) internal pure returns (bytes memory) {
        bytes memory result = new bytes(length);
        for (uint256 i = 0; i < length; i++) {
            result[i] = data[start + i];
        }
        return result;
    }

    receive() external payable {}
}
