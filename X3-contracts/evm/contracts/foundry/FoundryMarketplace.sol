// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title FoundryMarketplace — dApp Marketplace for X3 Foundry
/// @notice Marketplace listing contract for dApps with search, categories, and featured listings
/// @dev Stores all marketplace metadata on-chain
contract FoundryMarketplace is Ownable, ReentrancyGuard {
    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Full marketplace listing
    struct Listing {
        uint256 id;
        address dappAddress;
        address seller;
        string title;
        string description;
        string category;
        string tags;                // Comma-separated tags
        string metadataURI;         // URI to off-chain metadata
        uint256 price;              // Price in wei (0 = free)
        bool isActive;
        bool isFeatured;
        uint256 featuredUntil;      // Timestamp when featured expires
        uint256 createdAt;
        uint256 updatedAt;
        uint256 totalSales;
        uint256 totalRevenue;
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Listing ID => Listing
    mapping(uint256 => Listing) private _listings;

    /// @notice dApp address => listing ID
    mapping(address => uint256) private _listingIdByDapp;

    /// @notice Category => array of listing IDs
    mapping(string => uint256[]) private _listingsByCategory;

    /// @notice Featured listing IDs
    uint256[] private _featuredListings;

    /// @notice All active listing IDs
    uint256[] private _activeListingIds;

    /// @notice Incremental listing ID counter
    uint256 private _listingCount;

    /// @notice Platform fee basis points (e.g., 250 = 2.5%)
    uint256 public platformFeeBps;

    /// @notice Maximum basis points
    uint256 public constant MAX_BPS = 10000;

    /// @notice Maximum tags length
    uint256 public constant MAX_TAGS_LENGTH = 500;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a dApp is listed
    event AppListed(
        uint256 indexed listingId,
        address indexed dappAddress,
        address indexed seller,
        string title,
        string category,
        uint256 price,
        uint256 timestamp
    );

    /// @notice Emitted when a listing is updated
    event ListingUpdated(
        uint256 indexed listingId,
        string title,
        uint256 price,
        uint256 timestamp
    );

    /// @notice Emitted when a dApp is delisted
    event AppDelisted(
        uint256 indexed listingId,
        address indexed dappAddress,
        uint256 timestamp
    );

    /// @notice Emitted when a listing is featured or unfeatured
    event FeaturedUpdated(
        uint256 indexed listingId,
        bool isFeatured,
        uint256 featuredUntil,
        uint256 timestamp
    );

    /// @notice Emitted when a sale occurs
    event AppSold(
        uint256 indexed listingId,
        address indexed buyer,
        uint256 price,
        uint256 timestamp
    );

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error EmptyTitle();
    error ListingNotFound(uint256 listingId);
    error AlreadyListed(address dappAddress);
    error NotSeller(uint256 listingId);
    error ListingInactive(uint256 listingId);
    error TagsTooLong(uint256 length, uint256 max);
    error InvalidFee();

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor(uint256 _platformFeeBps) {
        _transferOwnership(msg.sender);
        if (_platformFeeBps > MAX_BPS) revert InvalidFee();
        platformFeeBps = _platformFeeBps;
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the platform fee basis points
    /// @param newFeeBps The new fee in basis points
    function setPlatformFeeBps(uint256 newFeeBps) external onlyOwner {
        if (newFeeBps > MAX_BPS) revert InvalidFee();
        platformFeeBps = newFeeBps;
    }

    /// @notice Toggle featured status for a listing
    /// @param listingId The listing ID
    /// @param isFeatured Whether to feature
    /// @param duration Duration in seconds (0 for permanent if featuring)
    function setFeatured(uint256 listingId, bool isFeatured, uint256 duration) external onlyOwner {
        if (listingId == 0 || listingId > _listingCount) revert ListingNotFound(listingId);
        Listing storage listing = _listings[listingId];

        listing.isFeatured = isFeatured;
        listing.featuredUntil = isFeatured ? (duration > 0 ? block.timestamp + duration : type(uint256).max) : 0;

        if (isFeatured) {
            _featuredListings.push(listingId);
        } else {
            _removeFromArray(_featuredListings, listingId);
        }

        emit FeaturedUpdated(listingId, isFeatured, listing.featuredUntil, block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice List a dApp on the marketplace
    /// @param dappAddress The dApp contract address
    /// @param title Listing title
    /// @param description Listing description
    /// @param category Category
    /// @param tags Comma-separated tags
    /// @param metadataURI URI to metadata
    /// @param price Price in wei (0 = free)
    /// @return listingId The new listing ID
    function listApp(
        address dappAddress,
        string calldata title,
        string calldata description,
        string calldata category,
        string calldata tags,
        string calldata metadataURI,
        uint256 price
    ) external nonReentrant returns (uint256 listingId) {
        if (dappAddress == address(0)) revert ZeroAddress();
        if (bytes(title).length == 0) revert EmptyTitle();
        if (_listingIdByDapp[dappAddress] != 0) revert AlreadyListed(dappAddress);
        if (bytes(tags).length > MAX_TAGS_LENGTH) revert TagsTooLong(bytes(tags).length, MAX_TAGS_LENGTH);

        _listingCount++;
        listingId = _listingCount;

        _listings[listingId] = Listing({
            id: listingId,
            dappAddress: dappAddress,
            seller: msg.sender,
            title: title,
            description: description,
            category: category,
            tags: tags,
            metadataURI: metadataURI,
            price: price,
            isActive: true,
            isFeatured: false,
            featuredUntil: 0,
            createdAt: block.timestamp,
            updatedAt: block.timestamp,
            totalSales: 0,
            totalRevenue: 0
        });

        _listingIdByDapp[dappAddress] = listingId;
        _listingsByCategory[category].push(listingId);
        _activeListingIds.push(listingId);

        emit AppListed(listingId, dappAddress, msg.sender, title, category, price, block.timestamp);
    }

    /// @notice Update an existing listing
    /// @param listingId The listing ID
    /// @param title New title
    /// @param description New description
    /// @param category New category
    /// @param tags New tags
    /// @param metadataURI New metadata URI
    /// @param price New price
    function updateListing(
        uint256 listingId,
        string calldata title,
        string calldata description,
        string calldata category,
        string calldata tags,
        string calldata metadataURI,
        uint256 price
    ) external nonReentrant {
        if (listingId == 0 || listingId > _listingCount) revert ListingNotFound(listingId);
        Listing storage listing = _listings[listingId];
        if (listing.seller != msg.sender && owner() != msg.sender) revert NotSeller(listingId);
        if (bytes(title).length == 0) revert EmptyTitle();
        if (bytes(tags).length > MAX_TAGS_LENGTH) revert TagsTooLong(bytes(tags).length, MAX_TAGS_LENGTH);

        listing.title = title;
        listing.description = description;
        listing.category = category;
        listing.tags = tags;
        listing.metadataURI = metadataURI;
        listing.price = price;
        listing.updatedAt = block.timestamp;

        emit ListingUpdated(listingId, title, price, block.timestamp);
    }

    /// @notice Delist a dApp from the marketplace
    /// @param listingId The listing ID to delist
    function delistApp(uint256 listingId) external nonReentrant {
        if (listingId == 0 || listingId > _listingCount) revert ListingNotFound(listingId);
        Listing storage listing = _listings[listingId];
        if (listing.seller != msg.sender && owner() != msg.sender) revert NotSeller(listingId);

        listing.isActive = false;
        listing.updatedAt = block.timestamp;

        _removeFromArray(_activeListingIds, listingId);

        emit AppDelisted(listingId, listing.dappAddress, block.timestamp);
    }

    /// @notice Buy/purchase a listed dApp
    /// @param listingId The listing ID
    function buyApp(uint256 listingId) external payable nonReentrant {
        if (listingId == 0 || listingId > _listingCount) revert ListingNotFound(listingId);
        Listing storage listing = _listings[listingId];
        if (!listing.isActive) revert ListingInactive(listingId);
        if (msg.value < listing.price) revert("INSUFFICIENT_PAYMENT");

        uint256 fee = (listing.price * platformFeeBps) / MAX_BPS;
        uint256 sellerProceeds = listing.price - fee;

        listing.totalSales++;
        listing.totalRevenue += listing.price;

        // Send fee to platform
        if (fee > 0) {
            (bool feeSuccess,) = payable(owner()).call{value: fee}("");
            require(feeSuccess, "FEE_TRANSFER_FAILED");
        }

        // Send proceeds to seller
        if (sellerProceeds > 0) {
            (bool sellerSuccess,) = payable(listing.seller).call{value: sellerProceeds}("");
            require(sellerSuccess, "SELLER_TRANSFER_FAILED");
        }

        // Refund excess payment
        uint256 excess = msg.value - listing.price;
        if (excess > 0) {
            (bool refundSuccess,) = payable(msg.sender).call{value: excess}("");
            require(refundSuccess, "REFUND_FAILED");
        }

        emit AppSold(listingId, msg.sender, listing.price, block.timestamp);
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// @notice Remove a value from an array
    function _removeFromArray(uint256[] storage arr, uint256 value) internal {
        uint256 len = arr.length;
        for (uint256 i = 0; i < len; i++) {
            if (arr[i] == value) {
                arr[i] = arr[len - 1];
                arr.pop();
                break;
            }
        }
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get listing by ID
    /// @param listingId The listing ID
    /// @return Listing struct
    function getListing(uint256 listingId) external view returns (Listing memory) {
        if (listingId == 0 || listingId > _listingCount) revert ListingNotFound(listingId);
        return _listings[listingId];
    }

    /// @notice Get listing by dApp address
    /// @param dappAddress The dApp address
    /// @return Listing struct
    function getListingByDapp(address dappAddress) external view returns (Listing memory) {
        uint256 listingId = _listingIdByDapp[dappAddress];
        if (listingId == 0) revert ListingNotFound(listingId);
        return _listings[listingId];
    }

    /// @notice Get total number of listings
    /// @return count The listing count
    function getListingCount() external view returns (uint256) {
        return _listingCount;
    }

    /// @notice Search apps by title or description (simple substring match)
    /// @param query The search query
    /// @param offset Starting index
    /// @param limit Max results
    /// @return listings Array of matching Listing structs
    function searchApps(
        string calldata query,
        uint256 offset,
        uint256 limit
    ) external view returns (Listing[] memory listings) {
        if (bytes(query).length == 0) {
            return getListings(offset, limit);
        }

        // Count matches
        uint256 matchCount;
        bytes32 queryLower = _toLower(keccak256(bytes(query)));

        // We do a simple scan - in production you'd use an index
        for (uint256 i = 1; i <= _listingCount; i++) {
            if (_matchesQuery(_listings[i], query, queryLower)) {
                matchCount++;
            }
        }

        if (offset >= matchCount) return new Listing[](0);

        uint256 end = offset + limit;
        if (end > matchCount) end = matchCount;
        uint256 resultCount = end - offset;

        listings = new Listing[](resultCount);
        uint256 idx;
        uint256 found;
        for (uint256 i = 1; i <= _listingCount && found < end; i++) {
            if (_matchesQuery(_listings[i], query, queryLower)) {
                found++;
                if (found > offset) {
                    listings[idx] = _listings[i];
                    idx++;
                }
            }
        }
    }

    /// @notice Check if a listing matches a search query
    function _matchesQuery(Listing storage listing, string calldata query, bytes32 queryLower) internal view returns (bool) {
        // Simple substring check on title and description
        bytes32 titleLower = _toLower(keccak256(bytes(listing.title)));
        bytes32 descLower = _toLower(keccak256(bytes(listing.description)));

        // For simplicity, we check if the keccak of the query is contained
        // In production, you'd use a more sophisticated search
        if (titleLower == queryLower) return true;
        if (descLower == queryLower) return true;

        // Check tags
        if (keccak256(bytes(listing.tags)) == queryLower) return true;

        // Fallback: check if query is substring of title (approximate)
        string memory title = listing.title;
        if (bytes(title).length >= bytes(query).length) {
            return _contains(title, query);
        }

        return false;
    }

    /// @notice Simple substring check
    function _contains(string memory a, string calldata b) internal pure returns (bool) {
        bytes memory aBytes = bytes(a);
        bytes memory bBytes = bytes(b);
        if (bBytes.length > aBytes.length) return false;
        for (uint256 i = 0; i <= aBytes.length - bBytes.length; i++) {
            bool isMatch = true;
            for (uint256 j = 0; j < bBytes.length; j++) {
                if (aBytes[i + j] != bBytes[j]) {
                    isMatch = false;
                    break;
                }
            }
            if (isMatch) return true;
        }
        return false;
    }

    /// @notice Convert a hash to lowercase (simplified - just returns the hash)
    function _toLower(bytes32 input) internal pure returns (bytes32) {
        return input;
    }

    /// @notice Get all listings (paginated)
    /// @param offset Starting index
    /// @param limit Max results
    /// @return listings Array of Listing structs
    function getListings(uint256 offset, uint256 limit) public view returns (Listing[] memory listings) {
        if (offset >= _listingCount) return new Listing[](0);
        uint256 end = offset + limit;
        if (end > _listingCount) end = _listingCount;
        uint256 resultCount = end - offset;
        listings = new Listing[](resultCount);
        for (uint256 i = 0; i < resultCount; i++) {
            listings[i] = _listings[offset + i + 1];
        }
    }

    /// @notice Get listings by category
    /// @param category The category
    /// @return listings Array of Listing structs
    function getListingsByCategory(string calldata category) external view returns (Listing[] memory listings) {
        uint256[] storage ids = _listingsByCategory[category];
        uint256 len = ids.length;
        listings = new Listing[](len);
        for (uint256 i = 0; i < len; i++) {
            listings[i] = _listings[ids[i]];
        }
    }

    /// @notice Get active listings (paginated)
    /// @param offset Starting index
    /// @param limit Max results
    /// @return listings Array of active Listing structs
    function getActiveListings(uint256 offset, uint256 limit) external view returns (Listing[] memory listings) {
        uint256 len = _activeListingIds.length;
        if (offset >= len) return new Listing[](0);
        uint256 end = offset + limit;
        if (end > len) end = len;
        uint256 resultCount = end - offset;
        listings = new Listing[](resultCount);
        for (uint256 i = 0; i < resultCount; i++) {
            listings[i] = _listings[_activeListingIds[offset + i]];
        }
    }

    /// @notice Get featured listings
    /// @return listings Array of featured Listing structs
    function getFeaturedApps() external view returns (Listing[] memory listings) {
        // Filter out expired featured listings
        uint256 activeFeatured;
        for (uint256 i = 0; i < _featuredListings.length; i++) {
            Listing storage l = _listings[_featuredListings[i]];
            if (l.isFeatured && (l.featuredUntil >= block.timestamp || l.featuredUntil == type(uint256).max)) {
                activeFeatured++;
            }
        }

        listings = new Listing[](activeFeatured);
        uint256 idx;
        for (uint256 i = 0; i < _featuredListings.length; i++) {
            Listing storage l = _listings[_featuredListings[i]];
            if (l.isFeatured && (l.featuredUntil >= block.timestamp || l.featuredUntil == type(uint256).max)) {
                listings[idx] = l;
                idx++;
            }
        }
    }

    /// @notice Get listings by seller
    /// @param seller The seller address
    /// @return listings Array of Listing structs
    function getListingsBySeller(address seller) external view returns (Listing[] memory listings) {
        uint256 count;
        for (uint256 i = 1; i <= _listingCount; i++) {
            if (_listings[i].seller == seller) count++;
        }
        listings = new Listing[](count);
        uint256 idx;
        for (uint256 i = 1; i <= _listingCount; i++) {
            if (_listings[i].seller == seller) {
                listings[idx] = _listings[i];
                idx++;
            }
        }
    }
}
