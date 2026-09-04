// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";

/// @title FoundryTemplateRegistry — dApp Template Registry for X3 Foundry
/// @notice Registry for dApp templates that can be used to deploy new dApps
/// @dev Stores template metadata and supports categorization
contract FoundryTemplateRegistry is Ownable {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Full metadata for a dApp template
    struct Template {
        uint256 id;
        address author;
        string name;
        string description;
        string category;
        string version;
        string metadataURI;
        bytes32 bytecodeHash;       // keccak256 of the init code
        address templateAddress;    // Reference implementation address
        bool isActive;
        bool isDeprecated;
        uint256 createdAt;
        uint256 updatedAt;
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Incremental ID counter
    uint256 private _templateCount;

    /// @notice Template ID => Template
    mapping(uint256 => Template) private _templates;

    /// @notice Template name => ID (for uniqueness check)
    mapping(bytes32 => uint256) private _templateIdByName;

    /// @notice Category => array of template IDs
    mapping(string => uint256[]) private _templatesByCategory;

    /// @notice Active template IDs
    uint256[] private _activeTemplateIds;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a new template is registered
    event TemplateRegistered(
        uint256 indexed templateId,
        address indexed author,
        string name,
        string category,
        uint256 timestamp
    );

    /// @notice Emitted when a template is updated
    event TemplateUpdated(
        uint256 indexed templateId,
        string name,
        string version,
        uint256 timestamp
    );

    /// @notice Emitted when a template is deprecated
    event TemplateDeprecated(uint256 indexed templateId, uint256 timestamp);

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyName();
    error TemplateNotFound(uint256 templateId);
    error TemplateAlreadyExists(string name);
    error TemplateDeprecatedError(uint256 templateId);
    error NotTemplateAuthor(uint256 templateId);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Admin / Write Functions ──────────────────────────────────────────────

    /// @notice Register a new dApp template
    /// @param name Template name (must be unique)
    /// @param description Short description
    /// @param category Category (e.g., "DeFi", "NFT", "Gaming")
    /// @param version Version string
    /// @param metadataURI URI to off-chain metadata
    /// @param bytecodeHash keccak256 hash of the init code
    /// @param templateAddress Reference implementation address
    /// @return templateId The assigned template ID
    function registerTemplate(
        string calldata name,
        string calldata description,
        string calldata category,
        string calldata version,
        string calldata metadataURI,
        bytes32 bytecodeHash,
        address templateAddress
    ) external returns (uint256 templateId) {
        if (bytes(name).length == 0) revert EmptyName();

        bytes32 nameHash = keccak256(bytes(name));
        if (_templateIdByName[nameHash] != 0) revert TemplateAlreadyExists(name);

        _templateCount++;
        templateId = _templateCount;

        Template storage tmpl = _templates[templateId];
        tmpl.id = templateId;
        tmpl.author = msg.sender;
        tmpl.name = name;
        tmpl.description = description;
        tmpl.category = category;
        tmpl.version = version;
        tmpl.metadataURI = metadataURI;
        tmpl.bytecodeHash = bytecodeHash;
        tmpl.templateAddress = templateAddress;
        tmpl.isActive = true;
        tmpl.isDeprecated = false;
        tmpl.createdAt = block.timestamp;
        tmpl.updatedAt = block.timestamp;

        _templateIdByName[nameHash] = templateId;
        _templatesByCategory[category].push(templateId);
        _activeTemplateIds.push(templateId);

        emit TemplateRegistered(templateId, msg.sender, tmpl.name, tmpl.category, block.timestamp);
    }

    /// @notice Update an existing template
    /// @param templateId The template ID
    /// @param description New description
    /// @param version New version
    /// @param metadataURI New metadata URI
    /// @param bytecodeHash New bytecode hash
    /// @param templateAddress New reference address
    function updateTemplate(
        uint256 templateId,
        string calldata description,
        string calldata version,
        string calldata metadataURI,
        bytes32 bytecodeHash,
        address templateAddress
    ) external {
        if (templateId == 0 || templateId > _templateCount) revert TemplateNotFound(templateId);
        Template storage tmpl = _templates[templateId];
        if (tmpl.author != msg.sender && owner() != msg.sender) revert NotTemplateAuthor(templateId);
        if (tmpl.isDeprecated) revert TemplateDeprecatedError(templateId);

        tmpl.description = description;
        tmpl.version = version;
        tmpl.metadataURI = metadataURI;
        tmpl.bytecodeHash = bytecodeHash;
        tmpl.templateAddress = templateAddress;
        tmpl.updatedAt = block.timestamp;

        emit TemplateUpdated(templateId, tmpl.name, version, block.timestamp);
    }

    /// @notice Deprecate a template (soft-delete)
    /// @param templateId The template ID to deprecate
    function deprecateTemplate(uint256 templateId) external {
        if (templateId == 0 || templateId > _templateCount) revert TemplateNotFound(templateId);
        Template storage tmpl = _templates[templateId];
        if (tmpl.author != msg.sender && owner() != msg.sender) revert NotTemplateAuthor(templateId);
        if (tmpl.isDeprecated) revert TemplateDeprecatedError(templateId);

        tmpl.isDeprecated = true;
        tmpl.isActive = false;
        tmpl.updatedAt = block.timestamp;

        // Remove from active list
        _removeFromActiveList(templateId);

        emit TemplateDeprecated(templateId, block.timestamp);
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Remove a template ID from the active list
    function _removeFromActiveList(uint256 templateId) internal {
        uint256 len = _activeTemplateIds.length;
        for (uint256 i = 0; i < len; i++) {
            if (_activeTemplateIds[i] == templateId) {
                _activeTemplateIds[i] = _activeTemplateIds[len - 1];
                _activeTemplateIds.pop();
                break;
            }
        }
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get template by ID
    /// @param templateId The template ID
    /// @return Template struct
    function getTemplate(uint256 templateId) external view returns (Template memory) {
        if (templateId == 0 || templateId > _templateCount) revert TemplateNotFound(templateId);
        return _templates[templateId];
    }

    /// @notice Get template ID by name
    /// @param name The template name
    /// @return templateId The template ID (0 if not found)
    function getTemplateIdByName(string calldata name) external view returns (uint256) {
        return _templateIdByName[keccak256(bytes(name))];
    }

    /// @notice Get total number of templates
    /// @return count The template count
    function getTemplateCount() external view returns (uint256) {
        return _templateCount;
    }

    /// @notice List all templates (paginated)
    /// @param offset Starting index (0-based)
    /// @param limit Maximum number of results
    /// @return templates Array of Template structs
    function listTemplates(uint256 offset, uint256 limit) external view returns (Template[] memory templates) {
        if (offset >= _templateCount) return new Template[](0);
        uint256 end = offset + limit;
        if (end > _templateCount) end = _templateCount;
        uint256 resultCount = end - offset;
        templates = new Template[](resultCount);
        for (uint256 i = 0; i < resultCount; i++) {
            templates[i] = _templates[offset + i + 1];
        }
    }

    /// @notice List all active (non-deprecated) templates
    /// @return templates Array of active Template structs
    function listActiveTemplates() external view returns (Template[] memory templates) {
        uint256 len = _activeTemplateIds.length;
        templates = new Template[](len);
        for (uint256 i = 0; i < len; i++) {
            templates[i] = _templates[_activeTemplateIds[i]];
        }
    }

    /// @notice List templates by category
    /// @param category The category to filter by
    /// @return templates Array of Template structs in that category
    function listByCategory(string calldata category) external view returns (Template[] memory templates) {
        uint256[] storage ids = _templatesByCategory[category];
        uint256 len = ids.length;
        templates = new Template[](len);
        for (uint256 i = 0; i < len; i++) {
            templates[i] = _templates[ids[i]];
        }
    }

    /// @notice Get all template IDs for a given category
    /// @param category The category
    /// @return ids Array of template IDs
    function getTemplateIdsByCategory(string calldata category) external view returns (uint256[] memory ids) {
        ids = _templatesByCategory[category];
    }
}
