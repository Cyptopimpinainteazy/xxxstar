// SPDX-License-Identifier: MIT
pragma solidity 0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title FoundrySubscriptionManager — Subscription Manager for X3 Foundry
/// @notice Subscription management for SaaS dApps with billing cycle tracking
/// @dev Handles subscriptions, renewals, cancellations, and billing cycles
contract FoundrySubscriptionManager is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ── Types ────────────────────────────────────────────────────────────────

    /// @notice Represents a subscription
    struct Subscription {
        address subscriber;
        address dapp;
        uint256 price;                  // Price per billing cycle (in wei or token units)
        uint256 billingPeriod;          // Billing period in seconds (e.g., 30 days)
        uint256 startTime;              // When the subscription started
        uint256 endTime;                // When the current billing period ends
        uint256 lastPaymentTime;        // Timestamp of last payment
        uint256 totalPaid;              // Total amount paid all-time
        bool isActive;                  // Whether the subscription is active
        bool autoRenew;                 // Whether auto-renewal is enabled
        address paymentToken;           // address(0) for native, otherwise ERC20
    }

    // ── State ────────────────────────────────────────────────────────────────

    /// @notice Subscription ID => Subscription
    mapping(uint256 => Subscription) private _subscriptions;

    /// @notice Subscriber address => dApp address => subscription ID
    mapping(address => mapping(address => uint256)) private _subscriptionId;

    /// @notice dApp address => default subscription price
    mapping(address => uint256) public defaultPrices;

    /// @notice dApp address => default billing period
    mapping(address => uint256) public defaultBillingPeriods;

    /// @notice dApp address => subscriber count
    mapping(address => uint256) public subscriberCount;

    /// @notice dApp address => total revenue from subscriptions
    mapping(address => uint256) public dappSubscriptionRevenue;

    /// @notice Incremental subscription ID counter
    uint256 private _subscriptionCounter;

    /// @notice Total active subscriptions
    uint256 public totalActiveSubscriptions;

    // ── Events ───────────────────────────────────────────────────────────────

    /// @notice Emitted when a user subscribes to a dApp
    event Subscribed(
        uint256 indexed subscriptionId,
        address indexed subscriber,
        address indexed dapp,
        uint256 price,
        uint256 billingPeriod,
        uint256 startTime,
        uint256 endTime
    );

    /// @notice Emitted when a subscription is cancelled
    event Cancelled(
        uint256 indexed subscriptionId,
        address indexed subscriber,
        address indexed dapp,
        uint256 timestamp
    );

    /// @notice Emitted when a subscription is renewed
    event Renewed(
        uint256 indexed subscriptionId,
        address indexed subscriber,
        address indexed dapp,
        uint256 amount,
        uint256 newEndTime
    );

    /// @notice Emitted when a dApp's default subscription price is set
    event SubscriptionPriceSet(
        address indexed dapp,
        uint256 price,
        uint256 billingPeriod,
        uint256 timestamp
    );

    // ── Errors ───────────────────────────────────────────────────────────────

    error ZeroAddress();
    error ZeroPrice();
    error ZeroBillingPeriod();
    error AlreadySubscribed(address subscriber, address dapp);
    error NotSubscribed(address subscriber, address dapp);
    error SubscriptionInactive(uint256 subscriptionId);
    error SubscriptionExpired(uint256 subscriptionId);
    error TransferFailed();
    error InsufficientPayment(uint256 required, uint256 provided);

    // ── Constructor ──────────────────────────────────────────────────────────

    constructor() {
        _transferOwnership(msg.sender);
    }

    // ── Admin Functions ──────────────────────────────────────────────────────

    /// @notice Set the default subscription price and billing period for a dApp
    /// @param dapp The dApp address
    /// @param price The price per billing cycle
    /// @param billingPeriod The billing period in seconds
    function setSubscriptionPrice(
        address dapp,
        uint256 price,
        uint256 billingPeriod
    ) external onlyOwner {
        if (dapp == address(0)) revert ZeroAddress();
        if (price == 0) revert ZeroPrice();
        if (billingPeriod == 0) revert ZeroBillingPeriod();

        defaultPrices[dapp] = price;
        defaultBillingPeriods[dapp] = billingPeriod;

        emit SubscriptionPriceSet(dapp, price, billingPeriod, block.timestamp);
    }

    // ── Core Functions ───────────────────────────────────────────────────────

    /// @notice Subscribe to a dApp (pay with native currency)
    /// @param dapp The dApp address to subscribe to
    /// @param billingPeriod The billing period in seconds (0 to use default)
    /// @param autoRenew Whether to auto-renew
    /// @return subscriptionId The new subscription ID
    function subscribe(
        address dapp,
        uint256 billingPeriod,
        bool autoRenew
    ) external payable nonReentrant returns (uint256 subscriptionId) {
        if (dapp == address(0)) revert ZeroAddress();
        if (_subscriptionId[msg.sender][dapp] != 0) {
            revert AlreadySubscribed(msg.sender, dapp);
        }

        uint256 price = defaultPrices[dapp];
        if (price == 0) revert ZeroPrice();

        if (msg.value < price) revert InsufficientPayment(price, msg.value);

        uint256 period = billingPeriod > 0 ? billingPeriod : defaultBillingPeriods[dapp];
        if (period == 0) revert ZeroBillingPeriod();

        _subscriptionCounter++;
        subscriptionId = _subscriptionCounter;

        uint256 startTime = block.timestamp;
        uint256 endTime = startTime + period;

        _subscriptions[subscriptionId] = Subscription({
            subscriber: msg.sender,
            dapp: dapp,
            price: price,
            billingPeriod: period,
            startTime: startTime,
            endTime: endTime,
            lastPaymentTime: startTime,
            totalPaid: price,
            isActive: true,
            autoRenew: autoRenew,
            paymentToken: address(0)
        });

        _subscriptionId[msg.sender][dapp] = subscriptionId;
        subscriberCount[dapp]++;
        totalActiveSubscriptions++;
        dappSubscriptionRevenue[dapp] += price;

        // Forward payment to dApp
        (bool success,) = payable(dapp).call{value: price}("");
        if (!success) revert TransferFailed();

        // Refund excess payment
        uint256 excess = msg.value - price;
        if (excess > 0) {
            (bool refundSuccess,) = payable(msg.sender).call{value: excess}("");
            require(refundSuccess, "REFUND_FAILED");
        }

        emit Subscribed(subscriptionId, msg.sender, dapp, price, period, startTime, endTime);
    }

    /// @notice Subscribe to a dApp using ERC20 tokens
    /// @param dapp The dApp address
    /// @param token The ERC20 token address
    /// @param billingPeriod The billing period in seconds
    /// @param autoRenew Whether to auto-renew
    /// @return subscriptionId The new subscription ID
    function subscribeWithToken(
        address dapp,
        IERC20 token,
        uint256 billingPeriod,
        bool autoRenew
    ) external nonReentrant returns (uint256 subscriptionId) {
        if (dapp == address(0)) revert ZeroAddress();
        if (address(token) == address(0)) revert ZeroAddress();
        if (_subscriptionId[msg.sender][dapp] != 0) {
            revert AlreadySubscribed(msg.sender, dapp);
        }

        uint256 price = defaultPrices[dapp];
        if (price == 0) revert ZeroPrice();

        uint256 period = billingPeriod > 0 ? billingPeriod : defaultBillingPeriods[dapp];
        if (period == 0) revert ZeroBillingPeriod();

        token.safeTransferFrom(msg.sender, address(this), price);

        _subscriptionCounter++;
        subscriptionId = _subscriptionCounter;

        uint256 startTime = block.timestamp;
        uint256 endTime = startTime + period;

        _subscriptions[subscriptionId] = Subscription({
            subscriber: msg.sender,
            dapp: dapp,
            price: price,
            billingPeriod: period,
            startTime: startTime,
            endTime: endTime,
            lastPaymentTime: startTime,
            totalPaid: price,
            isActive: true,
            autoRenew: autoRenew,
            paymentToken: address(token)
        });

        _subscriptionId[msg.sender][dapp] = subscriptionId;
        subscriberCount[dapp]++;
        totalActiveSubscriptions++;
        dappSubscriptionRevenue[dapp] += price;

        // Forward tokens to dApp
        token.safeTransfer(dapp, price);

        emit Subscribed(subscriptionId, msg.sender, dapp, price, period, startTime, endTime);
    }

    /// @notice Cancel an active subscription
    /// @param subscriptionId The subscription ID to cancel
    function cancelSubscription(uint256 subscriptionId) external nonReentrant {
        Subscription storage sub = _subscriptions[subscriptionId];
        if (sub.subscriber != msg.sender) revert NotSubscribed(msg.sender, sub.dapp);
        if (!sub.isActive) revert SubscriptionInactive(subscriptionId);

        sub.isActive = false;
        sub.autoRenew = false;
        subscriberCount[sub.dapp]--;
        totalActiveSubscriptions--;

        emit Cancelled(subscriptionId, msg.sender, sub.dapp, block.timestamp);
    }

    /// @notice Renew a subscription (pay with native currency)
    /// @param subscriptionId The subscription ID to renew
    function renewSubscription(uint256 subscriptionId) external payable nonReentrant {
        Subscription storage sub = _subscriptions[subscriptionId];
        if (sub.subscriber != msg.sender) revert NotSubscribed(msg.sender, sub.dapp);
        if (!sub.isActive) revert SubscriptionInactive(subscriptionId);

        if (block.timestamp > sub.endTime) {
            // Subscription expired, treat as new period
            sub.startTime = block.timestamp;
        }

        if (msg.value < sub.price) revert InsufficientPayment(sub.price, msg.value);

        sub.endTime = block.timestamp + sub.billingPeriod;
        sub.lastPaymentTime = block.timestamp;
        sub.totalPaid += sub.price;
        dappSubscriptionRevenue[sub.dapp] += sub.price;

        // Forward payment to dApp
        (bool success,) = payable(sub.dapp).call{value: sub.price}("");
        if (!success) revert TransferFailed();

        // Refund excess
        uint256 excess = msg.value - sub.price;
        if (excess > 0) {
            (bool refundSuccess,) = payable(msg.sender).call{value: excess}("");
            require(refundSuccess, "REFUND_FAILED");
        }

        emit Renewed(subscriptionId, msg.sender, sub.dapp, sub.price, sub.endTime);
    }

    /// @notice Renew a subscription using ERC20 tokens
    /// @param subscriptionId The subscription ID to renew
    /// @param token The ERC20 token address
    function renewSubscriptionWithToken(uint256 subscriptionId, IERC20 token) external nonReentrant {
        Subscription storage sub = _subscriptions[subscriptionId];
        if (sub.subscriber != msg.sender) revert NotSubscribed(msg.sender, sub.dapp);
        if (!sub.isActive) revert SubscriptionInactive(subscriptionId);
        if (address(token) != sub.paymentToken) revert("WRONG_TOKEN");

        if (block.timestamp > sub.endTime) {
            sub.startTime = block.timestamp;
        }

        token.safeTransferFrom(msg.sender, address(this), sub.price);

        sub.endTime = block.timestamp + sub.billingPeriod;
        sub.lastPaymentTime = block.timestamp;
        sub.totalPaid += sub.price;
        dappSubscriptionRevenue[sub.dapp] += sub.price;

        token.safeTransfer(sub.dapp, sub.price);

        emit Renewed(subscriptionId, msg.sender, sub.dapp, sub.price, sub.endTime);
    }

    // ── View Functions ───────────────────────────────────────────────────────

    /// @notice Get subscription details by ID
    /// @param subscriptionId The subscription ID
    /// @return Subscription struct
    function getSubscription(uint256 subscriptionId) external view returns (Subscription memory) {
        return _subscriptions[subscriptionId];
    }

    /// @notice Get subscription ID for a subscriber-dApp pair
    /// @param subscriber The subscriber address
    /// @param dapp The dApp address
    /// @return subscriptionId The subscription ID (0 if none)
    function getSubscriptionId(address subscriber, address dapp) external view returns (uint256) {
        return _subscriptionId[subscriber][dapp];
    }

    /// @notice Get subscription for a subscriber-dApp pair
    /// @param subscriber The subscriber address
    /// @param dapp The dApp address
    /// @return Subscription struct
    function getSubscriptionByUserAndDapp(address subscriber, address dapp) external view returns (Subscription memory) {
        uint256 sid = _subscriptionId[subscriber][dapp];
        if (sid == 0) revert NotSubscribed(subscriber, dapp);
        return _subscriptions[sid];
    }

    /// @notice Get the number of active subscribers for a dApp
    /// @param dapp The dApp address
    /// @return count The subscriber count
    function getSubscriberCount(address dapp) external view returns (uint256) {
        return subscriberCount[dapp];
    }

    /// @notice Check if a subscription is expired
    /// @param subscriptionId The subscription ID
    /// @return True if expired
    function isExpired(uint256 subscriptionId) external view returns (bool) {
        return block.timestamp > _subscriptions[subscriptionId].endTime;
    }

    /// @notice Get total subscription revenue for a dApp
    /// @param dapp The dApp address
    /// @return revenue Total revenue
    function getDappRevenue(address dapp) external view returns (uint256) {
        return dappSubscriptionRevenue[dapp];
    }

    /// @notice Get total number of subscriptions created
    /// @return count Total subscriptions
    function getTotalSubscriptions() external view returns (uint256) {
        return _subscriptionCounter;
    }
}
