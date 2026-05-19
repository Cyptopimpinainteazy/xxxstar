// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title MarriageLicense
 * @author Botchain Team
 * @notice Smart contract for AI agent reproduction and lifecycle management
 * @dev Requires compiler-signed manifest and checker signature for child creation
 *
 * The MarriageLicense contract manages:
 * - Child agent creation (minting)
 * - Training data certification
 * - Agent lineage tracking
 * - Quarantine and revocation
 *
 * Security model:
 * - Compiler signs manifests with ECDSA
 * - Checker validates artifacts and signs approval
 * - Multisig can revoke malicious agents
 */
contract MarriageLicense is ERC721, ERC721URIStorage, ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    // ============ Structs ============

    struct Child {
        address owner; // Creator/owner address
        bytes32 artifactCID; // IPFS CID of compiled artifact
        bytes32 datasetCID; // IPFS CID of training dataset
        bytes32 modelCID; // IPFS CID of trained model
        uint256 parentA; // First parent ID (0 for genesis)
        uint256 parentB; // Second parent ID (0 for genesis)
        uint256 createdAt; // Creation timestamp
        uint256 trainedAt; // Training completion timestamp
        bool isQuarantined; // Quarantine flag
        bool isActive; // Active status
    }

    struct PendingVerifierUpdate {
        address compilerVerifier;
        address checkerVerifier;
        uint256 executeAfter;
    }

    struct PendingMultisigUpdate {
        address multisig;
        uint256 executeAfter;
    }

    struct ScheduledChildAction {
        uint256 executeAfter;
        string reason;
    }

    // ============ State Variables ============

    /// @notice BOT token used for fees
    IERC20 public immutable botToken;

    /// @notice Compiler's public key address for manifest verification
    address public compilerVerifier;

    /// @notice Checker's public key address for artifact verification
    address public checkerVerifier;

    /// @notice Fee required to create a child (in BOT tokens)
    uint256 public marriageFee;

    /// @notice Next child ID to be minted
    uint256 public nextChildId;

    /// @notice Multisig address for admin operations
    address public multisig;

    /// @notice Mapping from child ID to Child data
    mapping(uint256 => Child) public children;

    /// @notice Mapping to track used manifest CIDs (prevent replay)
    mapping(bytes32 => bool) public usedManifests;

    /// @notice Delay for sensitive lifecycle and admin actions
    uint256 public constant ADMIN_ACTION_DELAY = 3 days;

    /// @notice Pending verifier update
    PendingVerifierUpdate public pendingVerifierUpdate;

    /// @notice Pending multisig update
    PendingMultisigUpdate public pendingMultisigUpdate;

    /// @notice Scheduled quarantines by child id
    mapping(uint256 => ScheduledChildAction) public scheduledQuarantines;

    /// @notice Scheduled revocations by child id
    mapping(uint256 => ScheduledChildAction) public scheduledRevocations;

    // ============ Events ============

    /// @notice Emitted when a new child agent is created
    event ChildCreated(
        uint256 indexed childId,
        address indexed owner,
        bytes32 artifactCID,
        uint256 parentA,
        uint256 parentB
    );

    /// @notice Emitted when a child's training is recorded
    event ChildTrained(
        uint256 indexed childId,
        bytes32 datasetCID,
        bytes32 modelCID
    );

    /// @notice Emitted when a child is quarantined
    event ChildQuarantined(uint256 indexed childId, string reason);

    /// @notice Emitted when a child is revoked
    event ChildRevoked(uint256 indexed childId);

    /// @notice Emitted when verifier addresses are updated
    event VerifiersUpdated(address compiler, address checker);

    /// @notice Emitted when a verifier update is scheduled
    event VerifierUpdateScheduled(
        address compiler,
        address checker,
        uint256 executeAfter
    );

    /// @notice Emitted when a pending verifier update is cancelled
    event VerifierUpdateCancelled();

    /// @notice Emitted when a multisig update is scheduled
    event MultisigUpdateScheduled(address multisig, uint256 executeAfter);

    /// @notice Emitted when a pending multisig update is cancelled
    event MultisigUpdateCancelled();

    /// @notice Emitted when a scheduled quarantine is created
    event QuarantineScheduled(
        uint256 indexed childId,
        string reason,
        uint256 executeAfter
    );

    /// @notice Emitted when a scheduled quarantine is cancelled
    event QuarantineCancelled(uint256 indexed childId);

    /// @notice Emitted when a scheduled revocation is created
    event RevocationScheduled(
        uint256 indexed childId,
        string reason,
        uint256 executeAfter
    );

    /// @notice Emitted when a scheduled revocation is cancelled
    event RevocationCancelled(uint256 indexed childId);

    /// @notice Emitted when marriage fee is updated
    event FeeUpdated(uint256 newFee);

    // ============ Modifiers ============

    modifier onlyMultisig() {
        require(
            msg.sender == multisig || msg.sender == owner(),
            "Not authorized"
        );
        _;
    }

    // ============ Constructor ============

    /**
     * @notice Initializes the MarriageLicense contract
     * @param _botToken Address of the BOT token
     * @param _compilerVerifier Address of compiler's public key
     * @param _checkerVerifier Address of checker's public key
     * @param _marriageFee Initial marriage fee in BOT tokens
     */
    constructor(
        address _botToken,
        address _compilerVerifier,
        address _checkerVerifier,
        uint256 _marriageFee
    ) ERC721("Botchain Agent", "AGENT") Ownable(msg.sender) {
        require(_botToken != address(0), "Invalid token address");
        require(_compilerVerifier != address(0), "Invalid compiler address");

        botToken = IERC20(_botToken);
        compilerVerifier = _compilerVerifier;
        checkerVerifier = _checkerVerifier;
        marriageFee = _marriageFee;
        multisig = msg.sender;

        // Start IDs at 1 (0 reserved for genesis)
        nextChildId = 1;
    }

    // ============ Core Functions ============

    /**
     * @notice Create a new child agent
     * @param artifactCID IPFS CID of the compiled artifact
     * @param compilerSig Compiler's signature over the manifest
     * @param checkerSig Checker's signature approving the artifact
     * @param parentA First parent ID (0 for genesis)
     * @param parentB Second parent ID (0 for genesis)
     * @return childId The ID of the newly created child
     */
    function createChild(
        bytes32 artifactCID,
        bytes memory compilerSig,
        bytes memory checkerSig,
        uint256 parentA,
        uint256 parentB
    ) external nonReentrant returns (uint256 childId) {
        // Prevent replay attacks
        require(!usedManifests[artifactCID], "Manifest already used");

        // Verify compiler signature
        require(
            _verifyCompilerSignature(
                artifactCID,
                parentA,
                parentB,
                msg.sender,
                compilerSig
            ),
            "Invalid compiler signature"
        );

        // Verify checker signature (if checker is set)
        if (checkerVerifier != address(0)) {
            require(
                _verifyCheckerSignature(
                    artifactCID,
                    parentA,
                    parentB,
                    msg.sender,
                    checkerSig
                ),
                "Invalid checker signature"
            );
        }

        // Verify parents exist (if not genesis)
        if (parentA != 0) {
            require(children[parentA].isActive, "Parent A not active");
            require(!children[parentA].isQuarantined, "Parent A quarantined");
        }
        if (parentB != 0) {
            require(children[parentB].isActive, "Parent B not active");
            require(!children[parentB].isQuarantined, "Parent B quarantined");
        }

        // Transfer BOT fee
        if (marriageFee > 0) {
            botToken.safeTransferFrom(msg.sender, address(this), marriageFee);
        }

        // Create child
        childId = nextChildId++;
        usedManifests[artifactCID] = true;

        children[childId] = Child({
            owner: msg.sender,
            artifactCID: artifactCID,
            datasetCID: bytes32(0),
            modelCID: bytes32(0),
            parentA: parentA,
            parentB: parentB,
            createdAt: block.timestamp,
            trainedAt: 0,
            isQuarantined: false,
            isActive: true
        });

        // Mint NFT
        _safeMint(msg.sender, childId);

        emit ChildCreated(childId, msg.sender, artifactCID, parentA, parentB);
    }

    /**
     * @notice Record training completion for a child
     * @param childId ID of the child to update
     * @param datasetCID IPFS CID of the training dataset
     * @param modelCID IPFS CID of the trained model
     * @param checkerSig Checker's signature approving the training
     */
    function trainChild(
        uint256 childId,
        bytes32 datasetCID,
        bytes32 modelCID,
        bytes memory checkerSig
    ) external nonReentrant {
        Child storage child = children[childId];

        require(child.owner != address(0), "Child does not exist");
        require(child.isActive, "Child not active");
        require(!child.isQuarantined, "Child quarantined");
        require(ownerOf(childId) == msg.sender, "Not child owner");
        require(child.trainedAt == 0, "Already trained");

        // Verify checker signature for training
        if (checkerVerifier != address(0)) {
            bytes32 trainingHash = keccak256(
                abi.encodePacked(childId, datasetCID, modelCID)
            );
            require(
                _verifySignature(trainingHash, checkerSig, checkerVerifier),
                "Invalid checker signature"
            );
        }

        child.datasetCID = datasetCID;
        child.modelCID = modelCID;
        child.trainedAt = block.timestamp;

        emit ChildTrained(childId, datasetCID, modelCID);
    }

    /**
     * @notice Schedule a child quarantine after the admin delay
     * @param childId ID of the child to quarantine
     * @param reason Reason for quarantine
     */
    function scheduleQuarantineChild(
        uint256 childId,
        string calldata reason
    ) external onlyMultisig {
        Child storage child = children[childId];
        require(child.owner != address(0), "Child does not exist");
        require(!child.isQuarantined, "Already quarantined");

        uint256 executeAfter = block.timestamp + ADMIN_ACTION_DELAY;
        scheduledQuarantines[childId] = ScheduledChildAction({
            executeAfter: executeAfter,
            reason: reason
        });

        emit QuarantineScheduled(childId, reason, executeAfter);
    }

    /**
     * @notice Execute a scheduled quarantine
     * @param childId ID of the child to quarantine
     */
    function executeQuarantineChild(uint256 childId) external onlyMultisig {
        ScheduledChildAction storage action = scheduledQuarantines[childId];
        Child storage child = children[childId];

        require(child.owner != address(0), "Child does not exist");
        require(!child.isQuarantined, "Already quarantined");
        require(action.executeAfter != 0, "Quarantine not scheduled");
        require(block.timestamp >= action.executeAfter, "Action not ready");

        string memory reason = action.reason;
        delete scheduledQuarantines[childId];
        child.isQuarantined = true;

        emit ChildQuarantined(childId, reason);
    }

    /**
     * @notice Cancel a scheduled quarantine
     * @param childId ID of the child
     */
    function cancelQuarantineChild(uint256 childId) external onlyMultisig {
        require(
            scheduledQuarantines[childId].executeAfter != 0,
            "Quarantine not scheduled"
        );
        delete scheduledQuarantines[childId];

        emit QuarantineCancelled(childId);
    }

    /**
     * @notice Schedule a child revocation after the admin delay
     * @param childId ID of the child to revoke
     * @param reason Reason for revocation
     */
    function scheduleRevokeChild(
        uint256 childId,
        string calldata reason
    ) external onlyMultisig {
        Child storage child = children[childId];
        require(child.owner != address(0), "Child does not exist");
        require(child.isActive, "Already revoked");

        uint256 executeAfter = block.timestamp + ADMIN_ACTION_DELAY;
        scheduledRevocations[childId] = ScheduledChildAction({
            executeAfter: executeAfter,
            reason: reason
        });

        emit RevocationScheduled(childId, reason, executeAfter);
    }

    /**
     * @notice Execute a scheduled revocation
     * @param childId ID of the child to revoke
     */
    function executeRevokeChild(uint256 childId) external onlyMultisig {
        ScheduledChildAction storage action = scheduledRevocations[childId];
        Child storage child = children[childId];

        require(child.owner != address(0), "Child does not exist");
        require(child.isActive, "Already revoked");
        require(action.executeAfter != 0, "Revocation not scheduled");
        require(block.timestamp >= action.executeAfter, "Action not ready");

        delete scheduledRevocations[childId];
        child.isActive = false;

        emit ChildRevoked(childId);
    }

    /**
     * @notice Cancel a scheduled revocation
     * @param childId ID of the child
     */
    function cancelRevokeChild(uint256 childId) external onlyMultisig {
        require(
            scheduledRevocations[childId].executeAfter != 0,
            "Revocation not scheduled"
        );
        delete scheduledRevocations[childId];

        emit RevocationCancelled(childId);
    }

    // ============ Signature Verification ============

    /**
     * @notice Verify compiler signature over artifact CID
     * @param artifactCID The artifact CID that was signed
     * @param signature The ECDSA signature
     * @return True if signature is valid
     */
    function _verifyCompilerSignature(
        bytes32 artifactCID,
        uint256 parentA,
        uint256 parentB,
        address creator,
        bytes memory signature
    ) internal view returns (bool) {
        return
            _verifySignature(
                _getCreateChildMessageHash(
                    artifactCID,
                    parentA,
                    parentB,
                    creator
                ),
                signature,
                compilerVerifier
            );
    }

    /**
     * @notice Verify checker signature over artifact CID
     * @param artifactCID The artifact CID that was signed
     * @param signature The ECDSA signature
     * @return True if signature is valid
     */
    function _verifyCheckerSignature(
        bytes32 artifactCID,
        uint256 parentA,
        uint256 parentB,
        address creator,
        bytes memory signature
    ) internal view returns (bool) {
        return
            _verifySignature(
                _getCreateChildMessageHash(
                    artifactCID,
                    parentA,
                    parentB,
                    creator
                ),
                signature,
                checkerVerifier
            );
    }

    /**
     * @notice Build the signed message hash for child creation
     * @dev Binds signatures to the parents, creator, contract, and chain
     */
    function _getCreateChildMessageHash(
        bytes32 artifactCID,
        uint256 parentA,
        uint256 parentB,
        address creator
    ) internal view returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    artifactCID,
                    parentA,
                    parentB,
                    creator,
                    block.chainid,
                    address(this)
                )
            );
    }

    /**
     * @notice Generic signature verification
     * @param messageHash The message hash that was signed
     * @param signature The ECDSA signature
     * @param signer Expected signer address
     * @return True if signature is valid
     */
    function _verifySignature(
        bytes32 messageHash,
        bytes memory signature,
        address signer
    ) internal pure returns (bool) {
        bytes32 ethSignedHash = messageHash.toEthSignedMessageHash();
        address recovered = ethSignedHash.recover(signature);
        return recovered == signer;
    }

    // ============ Admin Functions ============

    /**
     * @notice Schedule verifier updates after the admin delay
     * @param _compilerVerifier New compiler verifier address
     * @param _checkerVerifier New checker verifier address
     */
    function scheduleVerifierUpdate(
        address _compilerVerifier,
        address _checkerVerifier
    ) external onlyOwner {
        require(_compilerVerifier != address(0), "Invalid compiler address");

        uint256 executeAfter = block.timestamp + ADMIN_ACTION_DELAY;
        pendingVerifierUpdate = PendingVerifierUpdate({
            compilerVerifier: _compilerVerifier,
            checkerVerifier: _checkerVerifier,
            executeAfter: executeAfter
        });

        emit VerifierUpdateScheduled(
            _compilerVerifier,
            _checkerVerifier,
            executeAfter
        );
    }

    /**
     * @notice Execute a scheduled verifier update
     */
    function executeVerifierUpdate() external onlyOwner {
        PendingVerifierUpdate memory pending = pendingVerifierUpdate;
        require(pending.executeAfter != 0, "Verifier update not scheduled");
        require(block.timestamp >= pending.executeAfter, "Action not ready");

        compilerVerifier = pending.compilerVerifier;
        checkerVerifier = pending.checkerVerifier;
        delete pendingVerifierUpdate;

        emit VerifiersUpdated(compilerVerifier, checkerVerifier);
    }

    /**
     * @notice Cancel a pending verifier update
     */
    function cancelVerifierUpdate() external onlyOwner {
        require(
            pendingVerifierUpdate.executeAfter != 0,
            "Verifier update not scheduled"
        );
        delete pendingVerifierUpdate;

        emit VerifierUpdateCancelled();
    }

    /**
     * @notice Update marriage fee
     * @param _newFee New fee amount in BOT tokens
     */
    function setMarriageFee(uint256 _newFee) external onlyOwner {
        marriageFee = _newFee;
        emit FeeUpdated(_newFee);
    }

    /**
     * @notice Schedule a multisig address update after the admin delay
     * @param _multisig New multisig address
     */
    function scheduleMultisigUpdate(address _multisig) external onlyOwner {
        require(_multisig != address(0), "Invalid multisig address");

        uint256 executeAfter = block.timestamp + ADMIN_ACTION_DELAY;
        pendingMultisigUpdate = PendingMultisigUpdate({
            multisig: _multisig,
            executeAfter: executeAfter
        });

        emit MultisigUpdateScheduled(_multisig, executeAfter);
    }

    /**
     * @notice Execute a scheduled multisig update
     */
    function executeMultisigUpdate() external onlyOwner {
        PendingMultisigUpdate memory pending = pendingMultisigUpdate;
        require(pending.executeAfter != 0, "Multisig update not scheduled");
        require(block.timestamp >= pending.executeAfter, "Action not ready");

        multisig = pending.multisig;
        delete pendingMultisigUpdate;
    }

    /**
     * @notice Cancel a pending multisig update
     */
    function cancelMultisigUpdate() external onlyOwner {
        require(
            pendingMultisigUpdate.executeAfter != 0,
            "Multisig update not scheduled"
        );
        delete pendingMultisigUpdate;

        emit MultisigUpdateCancelled();
    }

    /**
     * @notice Withdraw accumulated fees
     * @param to Recipient address
     * @param amount Amount to withdraw
     */
    function withdrawFees(address to, uint256 amount) external onlyOwner {
        require(to != address(0), "Invalid recipient");
        botToken.safeTransfer(to, amount);
    }

    // ============ View Functions ============

    /**
     * @notice Get child details
     * @param childId ID of the child
     * @return Child struct with all data
     */
    function getChild(uint256 childId) external view returns (Child memory) {
        return children[childId];
    }

    /**
     * @notice Get child lineage (parents up to depth)
     * @param childId ID of the child
     * @param depth How many generations to trace
     * @return Array of parent IDs
     */
    function getLineage(
        uint256 childId,
        uint256 depth
    ) external view returns (uint256[] memory) {
        uint256[] memory lineage = new uint256[](depth * 2);
        uint256 index = 0;

        uint256 currentA = children[childId].parentA;
        uint256 currentB = children[childId].parentB;

        for (
            uint256 i = 0;
            i < depth && (currentA != 0 || currentB != 0);
            i++
        ) {
            if (currentA != 0 && index < lineage.length) {
                lineage[index++] = currentA;
                currentA = children[currentA].parentA;
            }
            if (currentB != 0 && index < lineage.length) {
                lineage[index++] = currentB;
                currentB = children[currentB].parentB;
            }
        }

        // Trim array to actual size
        uint256[] memory result = new uint256[](index);
        for (uint256 i = 0; i < index; i++) {
            result[i] = lineage[i];
        }
        return result;
    }

    // ============ ERC721 Overrides ============

    function tokenURI(
        uint256 tokenId
    ) public view override(ERC721, ERC721URIStorage) returns (string memory) {
        return super.tokenURI(tokenId);
    }

    function supportsInterface(
        bytes4 interfaceId
    ) public view override(ERC721, ERC721URIStorage) returns (bool) {
        return super.supportsInterface(interfaceId);
    }
}
