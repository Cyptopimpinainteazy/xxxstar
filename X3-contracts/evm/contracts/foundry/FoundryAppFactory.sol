// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/Create2.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title FoundryAppFactory — dApp Factory for X3 Foundry
/// @notice Factory contract that deploys new dApp contracts from templates using CREATE2
/// @dev Uses deterministic addresses via CREATE2 for predictable deployments
contract FoundryAppFactory is Ownable, ReentrancyGuard {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Represents a deployed dApp instance
    struct DeployedApp {
        uint256 appId;
        address appAddress;
        address deployer;
        uint256 templateId;
        bytes initData;
        uint256 deployedAt;
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Incremental app ID counter
    uint256 private _appCount;

    /// @notice App ID => DeployedApp
    mapping(uint256 => DeployedApp) private _deployedApps;

    /// @notice Deployer address => array of deployed app IDs
    mapping(address => uint256[]) private _deployerApps;

    /// @notice App address => app ID
    mapping(address => uint256) private _appIdByAddress;

    /// @notice Template registry address (for template lookups)
    address public templateRegistry;

    /// @notice Default salt nonce for CREATE2
    uint256 private _saltNonce;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a new dApp is created
    event AppCreated(
        uint256 indexed appId,
        address indexed appAddress,
        address indexed deployer,
        uint256 templateId,
        bytes initData,
        uint256 timestamp
    );

    /// @notice Emitted when the template registry is updated
    event TemplateRegistryUpdated(address indexed newRegistry, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyInitData();
    error DeploymentFailed();
    error AppAlreadyExists(address appAddress);
    error InvalidTemplate();

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor(address registry) {
        _transferOwnership(msg.sender);
        if (registry == address(0)) revert ZeroAddress();
        templateRegistry = registry;
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Update the template registry address
    /// @param registry New template registry address
    function setTemplateRegistry(address registry) external onlyOwner {
        if (registry == address(0)) revert ZeroAddress();
        templateRegistry = registry;
        emit TemplateRegistryUpdated(registry, block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Create a new dApp by deploying contract bytecode directly
    /// @param initCode The full init code (creation code + constructor args) of the dApp
    /// @param salt Optional salt for CREATE2 (use bytes32(0) for auto-generated)
    /// @return appAddress The address of the deployed dApp
    function createApp(bytes calldata initCode, bytes32 salt) external nonReentrant returns (address appAddress) {
        if (initCode.length == 0) revert EmptyInitData();

        // Generate deterministic salt if not provided
        if (salt == bytes32(0)) {
            _saltNonce++;
            salt = keccak256(abi.encodePacked(msg.sender, _saltNonce, block.timestamp));
        }

        // Deploy using CREATE2
        appAddress = Create2.deploy(0, salt, initCode);

        if (appAddress == address(0)) revert DeploymentFailed();
        if (_appIdByAddress[appAddress] != 0) revert AppAlreadyExists(appAddress);

        _registerApp(appAddress, msg.sender, 0, initCode);
    }

    /// @notice Create a new dApp from a registered template
    /// @param templateId The template ID to use
    /// @param initData The initialization data (constructor arguments / initializer params)
    /// @param salt Optional salt for CREATE2 (use bytes32(0) for auto-generated)
    /// @return appAddress The address of the deployed dApp
    function createAppFromTemplate(
        uint256 templateId,
        bytes calldata initData,
        bytes32 salt
    ) external nonReentrant returns (address appAddress) {
        if (templateId == 0) revert InvalidTemplate();

        // In a full implementation, we would fetch the template's bytecode from the registry
        // and concatenate it with initData. For this factory, we expect the caller to provide
        // the full init code or use a proxy pattern.
        // Here we deploy a minimal proxy (EIP-1167) pointing to the template implementation.

        // For simplicity, we use a minimal proxy pattern:
        // The template registry stores a templateAddress which is the implementation.
        // We deploy a clone using CREATE2.

        // This is a placeholder for the actual template bytecode retrieval.
        // In production, the template registry would store the bytecode or implementation address.
        // We emit the event and register the app.

        // Generate deterministic salt if not provided
        if (salt == bytes32(0)) {
            _saltNonce++;
            salt = keccak256(abi.encodePacked(msg.sender, _saltNonce, block.timestamp, templateId));
        }

        // Deploy a minimal proxy (EIP-1167) to the template implementation
        // The template address is fetched from the registry
        // For this implementation, we use a generic minimal proxy deployment

        // NOTE: In a real deployment, you'd call ITemplateRegistry(templateRegistry).getTemplate(templateId)
        // and use the templateAddress field. Here we deploy a generic minimal proxy.

        // Minimal proxy bytecode: 3d602d80600a3d3981f3363d3d373d3d3d363d73...address...5af43d82803e903d91602b57fd5bf3
        // For now, we deploy a simple contract using the initData as the full init code
        if (initData.length == 0) revert EmptyInitData();

        appAddress = Create2.deploy(0, salt, initData);

        if (appAddress == address(0)) revert DeploymentFailed();
        if (_appIdByAddress[appAddress] != 0) revert AppAlreadyExists(appAddress);

        _registerApp(appAddress, msg.sender, templateId, initData);
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Register a deployed app in the factory's state
    function _registerApp(
        address appAddress,
        address deployer,
        uint256 templateId,
        bytes memory initData
    ) internal {
        _appCount++;
        uint256 appId = _appCount;

        _deployedApps[appId] = DeployedApp({
            appId: appId,
            appAddress: appAddress,
            deployer: deployer,
            templateId: templateId,
            initData: initData,
            deployedAt: block.timestamp
        });

        _deployerApps[deployer].push(appId);
        _appIdByAddress[appAddress] = appId;

        emit AppCreated(appId, appAddress, deployer, templateId, initData, block.timestamp);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get total number of deployed apps
    /// @return count The app count
    function getDeployedAppCount() external view returns (uint256) {
        return _appCount;
    }

    /// @notice Get deployed app info by ID
    /// @param appId The app ID
    /// @return DeployedApp struct
    function getDeployedApp(uint256 appId) external view returns (DeployedApp memory) {
        return _deployedApps[appId];
    }

    /// @notice Get all deployed apps (paginated)
    /// @param offset Starting index (0-based)
    /// @param limit Maximum number of results
    /// @return apps Array of DeployedApp
    function getDeployedApps(uint256 offset, uint256 limit) external view returns (DeployedApp[] memory apps) {
        if (offset >= _appCount) return new DeployedApp[](0);
        uint256 end = offset + limit;
        if (end > _appCount) end = _appCount;
        uint256 resultCount = end - offset;
        apps = new DeployedApp[](resultCount);
        for (uint256 i = 0; i < resultCount; i++) {
            apps[i] = _deployedApps[offset + i + 1];
        }
    }

    /// @notice Get all apps deployed by a specific deployer
    /// @param deployer The deployer address
    /// @return apps Array of DeployedApp
    function getDeployerApps(address deployer) external view returns (DeployedApp[] memory apps) {
        uint256[] storage appIds = _deployerApps[deployer];
        uint256 len = appIds.length;
        apps = new DeployedApp[](len);
        for (uint256 i = 0; i < len; i++) {
            apps[i] = _deployedApps[appIds[i]];
        }
    }

    /// @notice Get app ID by deployed address
    /// @param appAddress The deployed dApp address
    /// @return appId The app ID (0 if not found)
    function getAppIdByAddress(address appAddress) external view returns (uint256) {
        return _appIdByAddress[appAddress];
    }

    /// @notice Predict the address of a CREATE2 deployment
    /// @param initCodeHash keccak256 of the init code
    /// @param salt The salt
    /// @return predictedAddress The predicted address
    function predictAppAddress(bytes32 initCodeHash, bytes32 salt) external view returns (address) {
        return Create2.computeAddress(salt, initCodeHash);
    }

    /// @notice Predict the address of a CREATE2 deployment with a specific deployer
    /// @param initCodeHash keccak256 of the init code
    /// @param salt The salt
    /// @param deployer The deployer address
    /// @return predictedAddress The predicted address
    function predictAppAddressByDeployer(
        bytes32 initCodeHash,
        bytes32 salt,
        address deployer
    ) external view returns (address) {
        return Create2.computeAddress(salt, initCodeHash, deployer);
    }
}
