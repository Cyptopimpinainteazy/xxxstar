// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title FoundryRegistry — Main dApp Registry for X3 Foundry
/// @notice Stores all deployed dApps with full metadata on-chain
/// @dev Admin functions are restricted to the contract owner
contract FoundryRegistry is Ownable, ReentrancyGuard {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Full metadata for a registered dApp
    struct DappInfo {
        address dappAddress;
        address creator;
        string name;
        string description;
        string category;
        string version;
        string metadataURI;
        bool isActive;
        uint256 registeredAt;
        uint256 updatedAt;
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Incremental ID counter for dApps
    uint256 private _dappCount;

    /// @notice dApp ID => DappInfo
    mapping(uint256 => DappInfo) private _dapps;

    /// @notice dApp address => dApp ID (for reverse lookup)
    mapping(address => uint256) private _dappIdByAddress;

    /// @notice dApp address => whether it's registered
    mapping(address => bool) private _isRegistered;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a new dApp is registered
    event DappRegistered(
        uint256 indexed dappId,
        address indexed dappAddress,
        address indexed creator,
        string name,
        string category,
        uint256 timestamp
    );

    /// @notice Emitted when a dApp's metadata is updated
    event DappUpdated(
        uint256 indexed dappId,
        address indexed dappAddress,
        string name,
        string category,
        uint256 timestamp
    );

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyName();
    error DappAlreadyRegistered(address dappAddress);
    error DappNotRegistered(uint256 dappId);
    error DappNotActive(uint256 dappId);
    error NotDappCreator(uint256 dappId);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Admin / Write Functions ──────────────────────────────────────────────

    /// @notice Register a new dApp in the registry
    /// @param dappAddress The deployed contract address of the dApp
    /// @param name The name of the dApp
    /// @param description A short description of the dApp
    /// @param category The category (e.g., "DeFi", "Gaming", "Social")
    /// @param version The version string (e.g., "1.0.0")
    /// @param metadataURI URI pointing to off-chain metadata (JSON)
    /// @return dappId The assigned dApp ID
    function registerDapp(
        address dappAddress,
        string calldata name,
        string calldata description,
        string calldata category,
        string calldata version,
        string calldata metadataURI
    ) external nonReentrant returns (uint256 dappId) {
        if (dappAddress == address(0)) revert ZeroAddress();
        if (bytes(name).length == 0) revert EmptyName();
        if (_isRegistered[dappAddress]) revert DappAlreadyRegistered(dappAddress);

        _dappCount++;
        dappId = _dappCount;

        _dapps[dappId] = DappInfo({
            dappAddress: dappAddress,
            creator: msg.sender,
            name: name,
            description: description,
            category: category,
            version: version,
            metadataURI: metadataURI,
            isActive: true,
            registeredAt: block.timestamp,
            updatedAt: block.timestamp
        });

        _dappIdByAddress[dappAddress] = dappId;
        _isRegistered[dappAddress] = true;

        emit DappRegistered(dappId, dappAddress, msg.sender, name, category, block.timestamp);
    }

    /// @notice Update an existing dApp's metadata
    /// @param dappId The ID of the dApp to update
    /// @param name New name
    /// @param description New description
    /// @param category New category
    /// @param version New version
    /// @param metadataURI New metadata URI
    /// @param isActive Whether the dApp is active
    function updateDapp(
        uint256 dappId,
        string calldata name,
        string calldata description,
        string calldata category,
        string calldata version,
        string calldata metadataURI,
        bool isActive
    ) external nonReentrant {
        if (dappId == 0 || dappId > _dappCount) revert DappNotRegistered(dappId);
        DappInfo storage dapp = _dapps[dappId];
        if (dapp.creator != msg.sender && owner() != msg.sender) revert NotDappCreator(dappId);
        if (bytes(name).length == 0) revert EmptyName();

        dapp.name = name;
        dapp.description = description;
        dapp.category = category;
        dapp.version = version;
        dapp.metadataURI = metadataURI;
        dapp.isActive = isActive;
        dapp.updatedAt = block.timestamp;

        emit DappUpdated(dappId, dapp.dappAddress, name, category, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get full dApp info by ID
    /// @param dappId The dApp ID
    /// @return DappInfo struct
    function getDapp(uint256 dappId) external view returns (DappInfo memory) {
        if (dappId == 0 || dappId > _dappCount) revert DappNotRegistered(dappId);
        return _dapps[dappId];
    }

    /// @notice Get dApp ID by contract address
    /// @param dappAddress The dApp contract address
    /// @return dappId The dApp ID (0 if not registered)
    function getDappIdByAddress(address dappAddress) external view returns (uint256) {
        return _dappIdByAddress[dappAddress];
    }

    /// @notice Total number of registered dApps
    /// @return count The dApp count
    function getDappCount() external view returns (uint256) {
        return _dappCount;
    }

    /// @notice Check if a dApp address is registered
    /// @param dappAddress The dApp contract address
    /// @return True if registered
    function isDappRegistered(address dappAddress) external view returns (bool) {
        return _isRegistered[dappAddress];
    }

    /// @notice Get a paginated list of all registered dApps
    /// @param offset Starting index (0-based)
    /// @param limit Maximum number of results
    /// @return dapps Array of DappInfo
    function getDapps(uint256 offset, uint256 limit) external view returns (DappInfo[] memory dapps) {
        if (offset >= _dappCount) return new DappInfo[](0);
        uint256 end = offset + limit;
        if (end > _dappCount) end = _dappCount;
        uint256 resultCount = end - offset;
        dapps = new DappInfo[](resultCount);
        for (uint256 i = 0; i < resultCount; i++) {
            dapps[i] = _dapps[offset + i + 1];
        }
    }

    /// @notice Get all dApps by a specific creator
    /// @param creator The creator address
    /// @return dapps Array of DappInfo
    function getDappsByCreator(address creator) external view returns (DappInfo[] memory dapps) {
        uint256 count;
        for (uint256 i = 1; i <= _dappCount; i++) {
            if (_dapps[i].creator == creator) count++;
        }
        dapps = new DappInfo[](count);
        uint256 idx;
        for (uint256 i = 1; i <= _dappCount; i++) {
            if (_dapps[i].creator == creator) {
                dapps[idx] = _dapps[i];
                idx++;
            }
        }
    }
}
