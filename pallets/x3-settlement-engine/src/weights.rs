//! Weight information for X3 Settlement Engine extrinsics

use frame_support::traits::Get;
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

/// Weight functions trait
pub trait WeightInfo {
    fn create_intent() -> Weight;
    fn lock_escrow() -> Weight;
    fn claim_settlement() -> Weight;
    fn finalize_intent() -> Weight;
    fn refund_intent() -> Weight;
    fn verify_btc_proof() -> Weight;
    fn update_btc_block_header() -> Weight;
    fn submit_external_proof() -> Weight;
    fn create_bond() -> Weight;
    fn claim_bond() -> Weight;
    fn update_finality_config() -> Weight;
    fn report_violation() -> Weight;
    fn settle_transfer() -> Weight;
    fn trigger_refund() -> Weight;
}

/// Default weights for testing
impl WeightInfo for () {
    fn create_intent() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }
    fn lock_escrow() -> Weight {
        Weight::from_parts(75_000_000, 0)
    }
    fn claim_settlement() -> Weight {
        Weight::from_parts(150_000_000, 0)
    }
    fn finalize_intent() -> Weight {
        Weight::from_parts(200_000_000, 0)
    }
    fn refund_intent() -> Weight {
        Weight::from_parts(100_000_000, 0)
    }
    fn verify_btc_proof() -> Weight {
        Weight::from_parts(250_000_000, 0)
    }
    fn update_btc_block_header() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }
    fn submit_external_proof() -> Weight {
        Weight::from_parts(180_000_000, 0)
    }
    fn create_bond() -> Weight {
        Weight::from_parts(75_000_000, 0)
    }
    fn claim_bond() -> Weight {
        Weight::from_parts(100_000_000, 0)
    }
    fn update_finality_config() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }
    fn report_violation() -> Weight {
        Weight::from_parts(50_000_000, 0)
    }
    fn settle_transfer() -> Weight {
        Weight::from_parts(120_000_000, 0)
    }
    fn trigger_refund() -> Weight {
        Weight::from_parts(80_000_000, 0)
    }
}

/// Substrate weight implementation (derived from benchmarks)
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_intent() -> Weight {
        Weight::from_parts(50_000_000, 3500)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn lock_escrow() -> Weight {
        Weight::from_parts(75_000_000, 4500)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn claim_settlement() -> Weight {
        Weight::from_parts(150_000_000, 8000)
            .saturating_add(T::DbWeight::get().reads(6))
            .saturating_add(T::DbWeight::get().writes(4))
    }

    fn finalize_intent() -> Weight {
        // Finalizing may involve merkle proof verification
        Weight::from_parts(200_000_000, 10000)
            .saturating_add(T::DbWeight::get().reads(7))
            .saturating_add(T::DbWeight::get().writes(5))
    }

    fn refund_intent() -> Weight {
        Weight::from_parts(100_000_000, 5500)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn verify_btc_proof() -> Weight {
        // Higher weight for SPV proof verification (merkle + PoW check)
        Weight::from_parts(250_000_000, 15000)
            .saturating_add(T::DbWeight::get().reads(9))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn update_btc_block_header() -> Weight {
        Weight::from_parts(50_000_000, 3000)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn submit_external_proof() -> Weight {
        Weight::from_parts(180_000_000, 9000)
            .saturating_add(T::DbWeight::get().reads(5))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn create_bond() -> Weight {
        Weight::from_parts(75_000_000, 4000)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn claim_bond() -> Weight {
        Weight::from_parts(100_000_000, 5000)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(3))
    }

    fn update_finality_config() -> Weight {
        Weight::from_parts(50_000_000, 2500)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn report_violation() -> Weight {
        Weight::from_parts(50_000_000, 2500)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn settle_transfer() -> Weight {
        Weight::from_parts(120_000_000, 6000)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn trigger_refund() -> Weight {
        Weight::from_parts(80_000_000, 4500)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}
