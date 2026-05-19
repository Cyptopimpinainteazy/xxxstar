//! Weight functions for the X3 Keyring pallet.

use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
use sp_std::marker::PhantomData;

/// Weight information for the X3 Keyring pallet.
pub trait WeightInfo {
    /// Weight for registering an attestor.
    fn register_attestor() -> Weight;
    /// Weight for submitting a keyring proof.
    fn submit_keyring_proof() -> Weight;
    /// Weight for confirming a keyring proof.
    fn confirm_keyring_proof() -> Weight;
    /// Weight for rejecting a keyring proof.
    fn reject_keyring_proof() -> Weight;
    /// Weight for deactivating an attestor.
    fn deactivate_attestor() -> Weight;
    /// Weight for reactivating an attestor.
    fn reactivate_attestor() -> Weight;
}

/// Weights for the X3 Keyring pallet using Substrate's weight system.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Register attestor: 1 DB write (attestor record) + 1 DB write (total count) + reserve
    fn register_attestor() -> Weight {
        Weight::from_parts(
            // Base computation: DB writes + balance operations
            10_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Submit proof: 1 DB write (proof) + 1 DB write (total count)
    fn submit_keyring_proof() -> Weight {
        Weight::from_parts(
            8_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Confirm proof (without reaching quorum): 1 DB write (confirmation) + proof update
    fn confirm_keyring_proof() -> Weight {
        Weight::from_parts(
            12_000_000 + T::DbWeight::get().reads_writes(2, 2).1 * 1_000_000,
            0,
        )
    }

    /// Confirm proof (with quorum, successful verification): includes reward distribution
    /// This is the heaviest operation due to verification + reward logic
    // fn confirm_keyring_proof_full() -> Weight {
    //     Weight::from_parts(30_000_000, 0)
    // }

    /// Reject proof: proof update + attestor slash
    fn reject_keyring_proof() -> Weight {
        Weight::from_parts(
            15_000_000 + T::DbWeight::get().reads_writes(3, 3).1 * 1_000_000,
            0,
        )
    }

    /// Deactivate attestor: balance unreserve + storage update
    fn deactivate_attestor() -> Weight {
        Weight::from_parts(
            8_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Reactivate attestor: balance reserve + storage update
    fn reactivate_attestor() -> Weight {
        Weight::from_parts(
            10_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::weights::Weight;

    #[test]
    fn weights_are_sane() {
        // Verify all weights are non-zero and reasonable
        let w = Weight::from_parts(1_000_000, 0);
        assert!(SubstrateWeight::<()>::register_attestor() >= w);
        assert!(SubstrateWeight::<()>::submit_keyring_proof() >= w);
        assert!(SubstrateWeight::<()>::confirm_keyring_proof() >= w);
        assert!(SubstrateWeight::<()>::reject_keyring_proof() >= w);
        assert!(SubstrateWeight::<()>::deactivate_attestor() >= w);
        assert!(SubstrateWeight::<()>::reactivate_attestor() >= w);
    }
}