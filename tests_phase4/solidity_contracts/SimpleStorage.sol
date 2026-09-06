// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

/// @title SimpleStorage
/// @notice Minimal solidity contract used to smoke-test X3 Chain's Frontier EVM integration.
/// @dev The contract interacts with the X3 Kernel canonical ledger through a designated precompile.
///      Native X3 tokens (the chain's base asset) are forwarded to the ledger so that balances stay
///      in sync between the EVM and the native runtime.
contract SimpleStorage {
    /// @dev Interface that reflects the X3 Kernel canonical ledger precompile surface.
    ///      The precompile is expected to be an address in the EVM universe that delegates calls to
    ///      the X3 Kernel pallet in the native runtime. Implementations may return a boolean success
    ///      flag for state-modifying calls and expose view functions for reads.
    interface IAtlasLedger {
        function creditAtlasBalance(address account, uint256 amount) external returns (bool);

        function canonicalBalanceOf(address account) external view returns (uint256);
    }

    /// @dev Emitted whenever the stored value is updated.
    event ValueUpdated(uint256 newValue, address indexed updater, uint256 timestamp);

    /// @dev Emitted when the contract successfully syncs a deposit with the canonical ledger.
    event AtlasDeposit(address indexed sender, uint256 amount, uint256 timestamp);

    /// @notice Address of the X3 Ledger precompile used to synchronize native balances.
    /// @dev This address should point to a precompile or a contract shim that bridges EVM calls to
    ///      the X3 Kernel pallet. It must be configured in the chain's runtime.
    address public immutable atlasLedger;

    /// @notice Contract owner – receives elevated permissions for certain administrative actions.
    address public immutable owner;

    /// @notice Current stored value.
    uint256 private storedValue;

    /// @notice Tracks the address that last updated the stored value.
    address public lastUpdater;

    /// @notice Timestamp for the most recent value update.
    uint256 public lastUpdatedAt;

    /// @param atlasLedgerAddress The address of the precompile/shim that forwards calls into X3 Kernel.
    /// @param initialValue Initial stored value for smoke-testing.
    constructor(address atlasLedgerAddress, uint256 initialValue) {
        require(atlasLedgerAddress != address(0), "SimpleStorage: x3 ledger is zero address");

        atlasLedger = atlasLedgerAddress;
        owner = msg.sender;
        storedValue = initialValue;
        lastUpdater = msg.sender;
        lastUpdatedAt = block.timestamp;

        emit ValueUpdated(initialValue, msg.sender, block.timestamp);
    }

    /// @notice Updates the stored value with an arbitrary number.
    /// @dev Requires the new value to differ from the existing one to avoid redundant writes.
    function setValue(uint256 newValue) external {
        require(newValue != storedValue, "SimpleStorage: value unchanged");

        storedValue = newValue;
        lastUpdater = msg.sender;
        lastUpdatedAt = block.timestamp;

        emit ValueUpdated(newValue, msg.sender, block.timestamp);
    }

    /// @notice Convenience helper that lets the contract owner bump the value by a delta.
    /// @dev Demonstrates that state-changing operations can enforce role-based access control.
    function incrementValue(uint256 delta) external {
        require(msg.sender == owner, "SimpleStorage: unauthorized");
        uint256 newValue = storedValue + delta;
        storedValue = newValue;
        lastUpdater = msg.sender;
        lastUpdatedAt = block.timestamp;

        emit ValueUpdated(newValue, msg.sender, block.timestamp);
    }

    /// @notice Returns the currently stored value.
    function getValue() external view returns (uint256) {
        return storedValue;
    }

    /// @notice Previews the value after applying a hypothetical delta without modifying contract state.
    function previewValueAfterDelta(uint256 delta) external view returns (uint256) {
        return storedValue + delta;
    }

    /// @notice Retrieves the canonical ledger balance tracked for an account.
    /// @dev Delegates to the X3 ledger precompile. This demonstrates how read-only interactions
    ///      remain consistent across the dual-VM execution environment.
    function getCanonicalBalance(address account) external view returns (uint256) {
        return IAtlasLedger(atlasLedger).canonicalBalanceOf(account);
    }

    /// @notice Accepts native X3 tokens and notifies the X3 ledger to keep balances in sync.
    /// @dev The canonical ledger is updated via a precompile call, ensuring that the native runtime
    ///      sees the same balance changes that occurred inside the EVM. If the ledger rejects the
    ///      credit, the entire transaction reverts to maintain consistency.
    function depositAndSync() external payable {
        require(msg.value > 0, "SimpleStorage: zero deposit");
        _doSyncDeposit(msg.sender, msg.value);
    }

    /// @notice Fallback to accept plain transfers to this contract and automatically sync with ledger.
    /// @dev This allows native X3 transfers to this contract address to be recognized by the
    ///      canonical ledger without requiring an explicit depositAndSync call from users.
    receive() external payable {
        require(msg.value > 0, "SimpleStorage: zero deposit");
        _doSyncDeposit(msg.sender, msg.value);
    }

    /// @notice Reject arbitrary calls without data (keeps behavior explicit).
    fallback() external payable {
        // If caller sent value, attempt to sync. Otherwise, revert to avoid accidental interactions.
        if (msg.value > 0) {
            _doSyncDeposit(msg.sender, msg.value);
            return;
        }
        revert("SimpleStorage: fallback not supported");
    }

    /// @dev Internal helper that performs the low-level call into the atlasLedger precompile and
    ///      handles revert message bubbling and decoded boolean results.
    function _doSyncDeposit(address sender, uint256 amount) private {
        // Forward the native value to the precompile and request a canonical credit for `sender`.
        (bool success, bytes memory returndata) = atlasLedger.call{value: amount}(
            abi.encodeWithSelector(IAtlasLedger.creditAtlasBalance.selector, sender, amount)
        );

        // If the low-level call reverted, attempt to bubble up a useful revert message.
        if (!success) {
            revert(_bubbleRevert(returndata));
        }

        // Some implementations return a boolean success flag; check and fail if false.
        if (returndata.length != 0) {
            bool ok = abi.decode(returndata, (bool));
            require(ok, "SimpleStorage: ledger sync failed");
        }

        emit AtlasDeposit(sender, amount, block.timestamp);
    }

    /// @notice Helper used internally to bubble revert messages from low-level calls.
    /// @dev When a precompile or shim reverts with a reason string, this utility extracts and
    ///      returns that reason so the outer transaction can revert with the same message.
    function _bubbleRevert(bytes memory returndata) private pure returns (string memory) {
        // If returndata is too short to contain a revert reason, provide a default message.
        if (returndata.length < 68) {
            return "SimpleStorage: ledger call reverted";
        }
        assembly {
            // Skip the function selector and length prefix of the revert reason.
            returndata := add(returndata, 0x04)
        }
        return abi.decode(returndata, (string));
    }
}
