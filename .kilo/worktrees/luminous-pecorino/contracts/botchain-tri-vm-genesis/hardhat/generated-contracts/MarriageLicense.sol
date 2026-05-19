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
            _verifyCompilerSignature(artifactCID, compilerSig),
            "Invalid compiler signature"
        );

        // Verify checker signature (if checker is set)
        if (checkerVerifier != address(0)) {
            require(
                _verifyCheckerSignature(artifactCID, checkerSig),
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
     * @notice Quarantine a child agent (multisig only)
     * @param childId ID of the child to quarantine
     * @param reason Reason for quarantine
     */
    function quarantineChild(
        uint256 childId,
        string calldata reason
    ) external onlyMultisig {
        Child storage child = children[childId];
        require(child.owner != address(0), "Child does not exist");
        require(!child.isQuarantined, "Already quarantined");

        child.isQuarantined = true;

        emit ChildQuarantined(childId, reason);
    }

    /**
     * @notice Revoke a child agent permanently (multisig only)
     * @param childId ID of the child to revoke
     */
    function revokeChild(uint256 childId) external onlyMultisig {
        Child storage child = children[childId];
        require(child.owner != address(0), "Child does not exist");
        require(child.isActive, "Already revoked");

        child.isActive = false;

        emit ChildRevoked(childId);
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
        bytes memory signature
    ) internal view returns (bool) {
        return _verifySignature(artifactCID, signature, compilerVerifier);
    }

    /**
     * @notice Verify checker signature over artifact CID
     * @param artifactCID The artifact CID that was signed
     * @param signature The ECDSA signature
     * @return True if signature is valid
     */
    function _verifyCheckerSignature(
        bytes32 artifactCID,
        bytes memory signature
    ) internal view returns (bool) {
        return _verifySignature(artifactCID, signature, checkerVerifier);
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
     * @notice Update verifier addresses
     * @param _compilerVerifier New compiler verifier address
     * @param _checkerVerifier New checker verifier address
     */
    function setVerifiers(
        address _compilerVerifier,
        address _checkerVerifier
    ) external onlyOwner {
        require(_compilerVerifier != address(0), "Invalid compiler address");
        compilerVerifier = _compilerVerifier;
        checkerVerifier = _checkerVerifier;
        emit VerifiersUpdated(_compilerVerifier, _checkerVerifier);
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
     * @notice Update multisig address
     * @param _multisig New multisig address
     */
    function setMultisig(address _multisig) external onlyOwner {
        require(_multisig != address(0), "Invalid multisig address");
        multisig = _multisig;
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
