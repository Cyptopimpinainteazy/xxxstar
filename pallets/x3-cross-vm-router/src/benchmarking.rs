//! Performance benchmarking for x3-cross-vm-router hot paths.
//!
//! Measures throughput and latency of:
//! - do_initiate_transfer (nonce reservation + message build + ledger debit)
//! - do_complete_xvm_transfer (message validation + state machine advance + ledger credit)
//! - Nonce generation under contention

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
use frame_support::assert_ok;
use sp_runtime::traits::StaticLookup;

use crate::*;

/// BASELINE: Single nonce reservation (current: NextNonce::mutate per call)
#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn nonce_reservation_single() {
        let source = DomainId::X3Native;
        let sender = AccountBytes::new_native(vec![1u8; 32]);
        
        #[extrinsic_call]
        {
            let _nonce = NextNonce::<T>::mutate(source, sender.clone(), |n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });
        }

        // Verify nonce incremented
        assert_eq!(NextNonce::<T>::get(source, sender), 1);
    }

    #[benchmark]
    fn nonce_reservation_batch_100() {
        let source = DomainId::X3Native;
        let sender = AccountBytes::new_native(vec![1u8; 32]);
        
        // Prime the batch allocation
        #[extrinsic_call]
        {
            // Simulate 100 batch reservations
            for _ in 0..100 {
                let _nonce = NextNonce::<T>::mutate(source, sender.clone(), |n| {
                    let cur = *n;
                    *n = n.saturating_add(1);
                    cur
                });
            }
        }

        // Verify 100 nonces incremented
        assert_eq!(NextNonce::<T>::get(source, sender), 100);
    }

    #[benchmark]
    fn route_lookup_cached() {
        // Establish asset and route
        let asset_id = AssetId::Native;
        let source = DomainId::X3Native;
        let destination = DomainId::X3Evm;
        
        #[extrinsic_call]
        {
            let _route = T::Registry::route(&asset_id, source, destination);
        }
    }

    #[benchmark]
    fn ledger_debit_operation() {
        let asset_id = AssetId::Native;
        let source = DomainId::X3Native;
        let amount = 1_000_000u128;
        
        #[extrinsic_call]
        {
            let _result = T::Ledger::debit_source_to_pending(&asset_id, source, amount);
        }
    }
}

impl_benchmark_test_suite!(
    crate::pallet::Pallet::<T>,
    crate::tests::new_test_ext(),
    crate::tests::Test,
);
