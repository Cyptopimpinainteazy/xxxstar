//! Benchmarking setup for `pallet-x3-cross-vm-router`.
//!
//! This file replaces an earlier one that was never declared as a module — `lib.rs` had no
//! `pub mod benchmarking;`, so nothing ever compiled it — and that measured helper functions
//! (`nonce_reservation_single`, `route_lookup_cached`, `ledger_debit_operation`) rather than the
//! pallet's dispatchables. The three calls below are the ones a benchmark can actually reach: each
//! is root-gated, and its preconditions are other governance calls in this same pallet.
//!
//! The other five are recorded in `docs/reports/benchmark-exceptions.md` and enforced by
//! `UNMEASURABLE_CALL_WEIGHTS` in `scripts/swarm/x3_repo_scan.py`. Four are gated on the
//! custody-backed gateway origin, whose `try_successful_origin` returns `Err` because no account is
//! always authorized. The fifth (`register_external_root`) reaches a verifier whose evidence is
//! runtime-specific finalized-header state; the generic pallet benchmark cannot fabricate that
//! state without bypassing the production verifier, so the call charges its declared compute plus
//! the verifier's storage reads rather than pretending to measure a path the chain never executes.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::vec;
use sp_std::vec::Vec;

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Reserved for X3Native, so any non-zero id names an external chain.
    const EXTERNAL_CHAIN_ID: u32 = 1;

    /// Open the external bridge surface through the pallet's own governance calls, in the order the
    /// pallet requires: the audit gate first, then the toggle that checks it.
    fn enable_external_bridges<T: Config>() -> Result<(), BenchmarkError> {
        Pallet::<T>::set_external_bridge_audit_gate(RawOrigin::Root.into(), true)
            .map_err(|_| BenchmarkError::Weightless)?;
        Pallet::<T>::set_external_bridges_enabled(RawOrigin::Root.into(), true)
            .map_err(|_| BenchmarkError::Weightless)
    }

    #[benchmark]
    fn set_external_bridge_audit_gate() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
        set_external_bridge_audit_gate(RawOrigin::Root, true);

        assert!(ExternalBridgeAuditGate::<T>::get());
        Ok(())
    }

    #[benchmark]
    fn set_external_bridges_enabled() -> Result<(), BenchmarkError> {
        Pallet::<T>::set_external_bridge_audit_gate(RawOrigin::Root.into(), true)
            .map_err(|_| BenchmarkError::Weightless)?;

        #[extrinsic_call]
        set_external_bridges_enabled(RawOrigin::Root, true);

        assert!(ExternalBridgesEnabled::<T>::get());
        Ok(())
    }

    #[benchmark]
    fn emergency_pause_bridge() -> Result<(), BenchmarkError> {
        enable_external_bridges::<T>()?;
        let reason: Vec<u8> = vec![b'p', b'a', b'u', b's', b'e'];

        #[extrinsic_call]
        emergency_pause_bridge(RawOrigin::Root, EXTERNAL_CHAIN_ID, reason);

        assert!(BridgePaused::<T>::contains_key(EXTERNAL_CHAIN_ID));
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
