// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {IX3FlashloanReceiver} from "../interfaces/IX3FlashloanReceiver.sol";

/// @notice Minimal ERC20 surface used by the flashloan core. Kept inline to
///         avoid pulling a full token library into a launch-critical contract.
interface IERC20Like {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

/// @title X3Flashloan
/// @notice Repay-or-revert flashloan core. Mirrors the SVM `x3_core::flashloan`
///         program. The behavior contract is defined by
///         `X3-contracts/shared/test-vectors/flashloan_repay_or_revert.json`.
///
/// Invariants enforced here:
///   I1 (atomicity)     : terminal balance must be >= pre-balance + fee, else revert.
///   I2 (no reentrancy) : a flashloan call cannot recursively borrow the same asset.
///   I3 (fee monotonic) : `fee` is purely additive; the protocol never owes the borrower.
contract X3Flashloan {
    /// @dev Returned by `IX3FlashloanReceiver.onFlashloan` on success.
    bytes32 public constant CALLBACK_OK = keccak256("X3Flashloan.onFlashloan.OK");

    /// @dev Fee is in basis points of `amount`. 9 bps == 0.09%.
    uint256 public immutable feeBps;

    /// @dev Reentrancy lock keyed per asset.
    mapping(address => bool) private _locked;

    error AlreadyEntered(address asset);
    error CallbackFailed();
    error NotRepaid(uint256 expected, uint256 got);
    error TransferFailed();

    event Flashloan(
        address indexed asset,
        address indexed borrower,
        uint256 amount,
        uint256 fee
    );

    constructor(uint256 _feeBps) {
        // Cap at 1000 bps (10%) defensively; X3 default is 9 bps.
        require(_feeBps <= 1000, "fee too high");
        feeBps = _feeBps;
    }

    /// @notice Compute the fee for a flashloan amount. Rounds up so the
    ///         protocol cannot be drained by repeated 1-wei loans.
    function quoteFee(uint256 amount) public view returns (uint256) {
        uint256 num = amount * feeBps;
        return (num + 9_999) / 10_000;
    }

    /// @notice Atomically lend `amount` of `asset` to `borrower`, invoke its
    ///         `onFlashloan` hook, and require repayment in the same call.
    function flashloan(
        address asset,
        uint256 amount,
        address borrower,
        bytes calldata data
    ) external {
        if (_locked[asset]) revert AlreadyEntered(asset);
        _locked[asset] = true;

        IERC20Like token = IERC20Like(asset);
        uint256 fee = quoteFee(amount);
        uint256 preBalance = token.balanceOf(address(this));

        // Lend.
        if (!token.transfer(borrower, amount)) revert TransferFailed();

        // Borrower must return amount + fee before this call returns.
        bytes32 ack = IX3FlashloanReceiver(borrower).onFlashloan(
            asset,
            amount,
            fee,
            data
        );
        if (ack != CALLBACK_OK) revert CallbackFailed();

        uint256 postBalance = token.balanceOf(address(this));
        uint256 minRepayment = preBalance + fee;
        if (postBalance < minRepayment) revert NotRepaid(minRepayment, postBalance);

        emit Flashloan(asset, borrower, amount, fee);

        _locked[asset] = false;
    }
}
