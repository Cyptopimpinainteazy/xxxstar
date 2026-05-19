//! Weight functions for the X3 Identity Verifier pallet.

use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
use sp_std::marker::PhantomData;

/// Weight information for the X3 Identity Verifier pallet.
pub trait WeightInfo {
    /// Weight for registering a verifier.
    fn register_verifier() -> Weight;
    /// Weight for submitting an identity proof.
    fn submit_identity_proof() -> Weight;
    /// Weight for confirming an identity proof.
    fn confirm_identity_proof() -> Weight;
    /// Weight for rejecting an identity proof.
    fn reject_identity_proof() -> Weight;
    /// Weight for deactivating a verifier.
    fn deactivate_verifier() -> Weight;
    /// Weight for reactivating a verifier.
    fn reactivate_verifier() -> Weight;
    /// Weight for renewing an identity.
    fn renew_identity() -> Weight;
}

/// Weights for the X3 Identity Verifier pallet using Substrate's weight system.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Register verifier: 1 DB write (verifier record) + 1 DB write (total count) + reserve
    fn register_verifier() -> Weight {
        Weight::from_parts(
            10_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Submit proof: 1 DB write (proof) + 1 DB write (total count)
    fn submit_identity_proof() -> Weight {
        Weight::from_parts(
            8_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Confirm proof (without reaching quorum): 1 DB write (confirmation) + proof update
    fn confirm_identity_proof() -> Weight {
        Weight::from_parts(
            12_000_000 + T::DbWeight::get().reads_writes(2, 2).1 * 1_000_000,
            0,
        )
    }

    /// Reject proof: proof update + verifier slash
    fn reject_identity_proof() -> Weight {
        Weight::from_parts(
            15_000_000 + T::DbWeight::get().reads_writes(3, 3).1 * 1_000_000,
            0,
        )
    }

    /// Deactivate verifier: balance unreserve + storage update
    fn deactivate_verifier() -> Weight {
        Weight::from_parts(
            8_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Reactivate verifier: balance reserve + storage update
    fn reactivate_verifier() -> Weight {
        Weight::from_parts(
            10_000_000 + T::DbWeight::get().reads_writes(1, 2).1 * 1_000_000,
            0,
        )
    }

    /// Renew identity: single storage update
    fn renew_identity() -> Weight {
        Weight::from_parts(
            5_000_000 + T::DbWeight::get().reads_writes(1, 1).1 * 1_000_000,
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
        assert!(SubstrateWeight::<()>::register_verifier() >= w);
        assert!(SubstrateWeight::<()>::submit_identity_proof() >= w);
        assert!(SubstrateWeight::<()>::confirm_identity_proof() >= w);
        assert!(SubstrateWeight::<()>::reject_identity_proof() >= w);
        assert!(SubstrateWeight::<()>::deactivate_verifier() >= w);
        assert!(SubstrateWeight::<()>::reactivate_verifier() >= w);
        assert!(SubstrateWeight::<()>::renew_identity() >= w);
    }
}