//! Atomic inventory mutation helpers.
//!
//! Each function operates inside a single `try_mutate` and therefore
//! inherits Substrate's storage-transaction rollback semantics.
//! The balance invariant `available + reserved + pending_out <= total_deposited`
//! is maintained by construction: we never add to more than one bucket at a time,
//! and we only move between buckets on explicit confirmation.

use crate::pallet::{Config, Error, Event, Pallet, Vaults};
use crate::types::VaultId;
use frame_support::pallet_prelude::*;
use sp_runtime::traits::Zero;

/// Reserve `amount` from `available_balance` into `reserved_balance`.
///
/// Rejects if:
/// - vault is frozen (`VaultFrozen`)
/// - available balance is insufficient (`InsufficientAvailableBalance`)
pub fn reserve_inventory<T: Config>(vault_id: VaultId, amount: T::Balance) -> DispatchResult {
    if amount.is_zero() {
        return Ok(());
    }

    Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
        let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

        ensure!(
            vault.status != crate::types::VaultStatus::Frozen,
            Error::<T>::VaultFrozen
        );
        ensure!(
            vault.available_balance >= amount,
            Error::<T>::InsufficientAvailableBalance
        );

        vault.available_balance = vault.available_balance - amount;
        vault.reserved_balance = vault.reserved_balance + amount;

        // Re-evaluate status after balance change.
        Pallet::<T>::refresh_vault_status(vault);
        Ok(())
    })?;

    Pallet::<T>::deposit_event(Event::InventoryReserved { vault_id, amount });
    Ok(())
}

/// Release `amount` from `reserved_balance` back to `available_balance`.
///
/// Rejects if reserved balance is insufficient.
pub fn release_inventory<T: Config>(vault_id: VaultId, amount: T::Balance) -> DispatchResult {
    if amount.is_zero() {
        return Ok(());
    }

    Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
        let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

        ensure!(
            vault.reserved_balance >= amount,
            Error::<T>::InsufficientReservedBalance
        );

        vault.reserved_balance = vault.reserved_balance - amount;
        vault.available_balance = vault.available_balance + amount;

        Pallet::<T>::refresh_vault_status(vault);
        Ok(())
    })?;

    Pallet::<T>::deposit_event(Event::InventoryReleased { vault_id, amount });
    Ok(())
}

/// Move `amount` from `available_balance` to `pending_out_balance` (funds in flight).
///
/// Represents capital that has left the vault but whose settlement is not yet confirmed.
///
/// Rejects if available balance is insufficient.
pub fn record_pending_out<T: Config>(vault_id: VaultId, amount: T::Balance) -> DispatchResult {
    if amount.is_zero() {
        return Ok(());
    }

    Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
        let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

        ensure!(
            vault.available_balance >= amount,
            Error::<T>::InsufficientAvailableBalance
        );

        vault.available_balance = vault.available_balance - amount;
        vault.pending_out_balance = vault.pending_out_balance + amount;

        Pallet::<T>::refresh_vault_status(vault);
        Ok(())
    })?;

    Pallet::<T>::deposit_event(Event::PendingOutRecorded { vault_id, amount });
    Ok(())
}

/// Confirm settlement: reduce `pending_out_balance` by `amount`.
///
/// Called when the on-chain settlement proof is received and the obligation
/// is considered discharged. The balance is removed from the vault entirely
/// (it has already left the chain).
///
/// Rejects if `pending_out_balance` is insufficient.
pub fn confirm_settlement<T: Config>(vault_id: VaultId, amount: T::Balance) -> DispatchResult {
    if amount.is_zero() {
        return Ok(());
    }

    Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
        let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

        ensure!(
            vault.pending_out_balance >= amount,
            Error::<T>::InsufficientPendingOutBalance
        );

        vault.pending_out_balance = vault.pending_out_balance - amount;
        // Note: available_balance is NOT restored — the funds have settled externally.
        Ok(())
    })?;

    Pallet::<T>::deposit_event(Event::SettlementConfirmed { vault_id, amount });
    Ok(())
}

/// Fund a vault: add `amount` directly to `available_balance`.
///
/// Called by treasury policy when deploying capital to a settlement float vault.
pub fn fund_vault<T: Config>(vault_id: VaultId, amount: T::Balance) -> DispatchResult {
    if amount.is_zero() {
        return Ok(());
    }

    Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
        let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

        vault.available_balance = vault.available_balance + amount;

        Pallet::<T>::refresh_vault_status(vault);
        Ok(())
    })?;

    Pallet::<T>::deposit_event(Event::VaultFunded { vault_id, amount });
    Ok(())
}
