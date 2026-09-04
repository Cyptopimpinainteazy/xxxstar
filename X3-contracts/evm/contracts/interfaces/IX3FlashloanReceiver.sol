// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title IX3FlashloanReceiver
/// @notice Borrower contract interface for X3 flashloans. The X3 invariant is
///         repay-or-revert: the borrower MUST return `amount + fee` of `asset`
///         to the lender within the same call frame. If it does not, the
///         entire transaction reverts. There is no partial repayment path.
interface IX3FlashloanReceiver {
    /// @notice Called by the lender after transferring `amount` of `asset`.
    /// @dev    Must approve or transfer `amount + fee` back to `msg.sender`
    ///         before returning. Reverting here cancels the flashloan.
    function onFlashloan(
        address asset,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external returns (bytes32);
}
