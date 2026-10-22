//! Benchmarking setup for pallet-x3-kernel
//!
//! This module provides benchmarks for all extrinsics in the X3 Kernel pallet.
//! Run benchmarks with:
//! ```bash
//! cargo build --release --features runtime-benchmarks
//! ./target/release/x3-chain-node benchmark pallet \
//!     --chain=dev \
//!     --pallet=pallet_x3_kernel \
//!     --extrinsic='*' \
//!     --steps=50 \
//!     --repeat=20 \
//!     --output=pallets/x3-kernel/src/weights.rs \
//!     --template=.maintain/frame-weight-template.hbs
//! ```

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_std::vec;

/// Helper to create a valid EVM payload for benchmarking
fn create_evm_payload(size: u32) -> Vec<u8> {
    // A *packet*, not raw calldata. The previous body returned `0xa9059cbb` followed by zeros, and
    // `submit_comit` refuses a non-empty payload that does not deserialize as a packet carrying the
    // EVM domain bit — measured 2026-09-25 on this very benchmark: `InvalidEvmPacket`. It had been
    // registered and never run, so nothing said so.
    crate::test_helpers::wrap_evm_payload(&vec![0xAAu8; size.max(8) as usize])
}

/// Helper to create a valid SVM payload for benchmarking
fn create_svm_payload(size: u32) -> Vec<u8> {
    // Same correction as the EVM helper above: a packet, not a stub header.
    crate::test_helpers::wrap_svm_payload(&vec![0xBBu8; size.max(8) as usize])
}

/// Helper to register an asset for testing
fn setup_asset<T: Config>(asset_id: T::AssetId, symbol: &[u8]) {
    let bounded_symbol: BoundedVec<u8, T::MaxAssetSymbolLength> =
        symbol.to_vec().try_into().expect("Symbol too long");
    let metadata = AssetMetadata {
        symbol: bounded_symbol,
        decimals: 18,
    };
    AssetRegistry::<T>::insert(asset_id, metadata);
}

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Benchmark submit_comit with varying payload sizes
    ///
    /// This benchmark measures the cost of:
    /// - Authorization check
    /// - Payload validation
    /// - Nonce verification and increment
    /// - Dual-VM execution (mock)
    /// - State merging
    /// - Fee accounting
    /// - Event emission
    #[benchmark]
    fn submit_comit() -> Result<(), BenchmarkError> {
        // Setup: Create funded and authorized account
        let caller: T::AccountId = whitelisted_caller();
        let amount = T::Currency::minimum_balance() * 1_000_000u32.into();
        let _ = T::Currency::make_free_balance_be(&caller, amount);
        AuthorizedAccounts::<T>::insert(&caller, ());

        // Create payloads at reasonable size (1KB each)
        let evm_payload = create_evm_payload(1024);
        let svm_payload = create_svm_payload(1024);

        let comit_id = H256::from_low_u64_be(1);
        let nonce = Nonces::<T>::get(&caller);
        let fee: T::Balance = 1_000u32.into();
        // `verify_dual_vm_with_receipts` refuses a zero `prepare_root` (that is the deliberate
        // `dev-bypass` arm), so this benchmark presents the commitment the chain computes — the
        // same correction `submit_comit_v2` already carries. With a zero root this benchmark failed
        // with `ComitVerificationFailed` on every chain that builds without the bypass feature,
        // which is why `pallet_x3_kernel`'s weights could not be regenerated at all.
        let prepare_root =
            Pallet::<T>::compute_prepare_root(comit_id, &evm_payload, &svm_payload, nonce, fee);

        #[extrinsic_call]
        submit_comit(
            RawOrigin::Signed(caller.clone()),
            comit_id,
            evm_payload,
            svm_payload,
            nonce,
            fee,
            prepare_root,
        );

        // Verify: Comit was recorded
        assert!(SubmittedComits::<T>::contains_key(comit_id));
        Ok(())
    }

    /// Benchmark `submit_comit_v2` — the triple-VM path, including the X3 execution and the execution
    /// receipt the pallet persists.
    ///
    /// Added 2026-09-25. This extrinsic had no benchmark at all, which is why its entry in
    /// `weights.rs` is a hand-written copy of `submit_comit`'s cost with a comment saying so: there
    /// was nothing to re-run. The X3 payload is a *compiled artifact*
    /// (`crate::bench_fixtures::X3_PROGRAM_FIXTURE`), because the runtime's adapter validates the
    /// envelope — a synthetic payload would have been accepted by the pallet's mock, whose
    /// `TestX3Adapter::validate` answers `Ok(())` for anything, and refused on the chain.
    #[benchmark]
    fn submit_comit_v2() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let amount = T::Currency::minimum_balance() * 1_000_000u32.into();
        let _ = T::Currency::make_free_balance_be(&caller, amount);
        AuthorizedAccounts::<T>::insert(&caller, ());

        let evm_payload = create_evm_payload(1024);
        let svm_payload = create_svm_payload(1024);
        let x3_payload = crate::bench_fixtures::X3_PROGRAM_FIXTURE.to_vec();

        let comit_id = H256::from_low_u64_be(2);
        let nonce = Nonces::<T>::get(&caller);
        let fee: T::Balance = 1_000u32.into();
        // The dispatch checks this against the payloads it was handed, so the benchmark presents the
        // same commitment the chain would compute.
        let prepare_root = Pallet::<T>::compute_prepare_root_v2(
            comit_id,
            &evm_payload,
            &svm_payload,
            &x3_payload,
            nonce,
            fee,
        );

        #[extrinsic_call]
        submit_comit_v2(
            RawOrigin::Signed(caller.clone()),
            comit_id,
            evm_payload,
            svm_payload,
            x3_payload,
            nonce,
            fee,
            prepare_root,
        );

        assert!(SubmittedComits::<T>::contains_key(comit_id));
        Ok(())
    }

    /// Running an X3 program on the chain's engine, as a function of the gas it spends.
    ///
    /// `submit_comit_v2` executes its X3 payload for up to `DefaultX3GasLimit` gas, and the cross-VM
    /// extrinsics execute an X3 call for up to the same bound; their weight was a constant that paid
    /// for none of that execution, so a block could be filled with work its weight did not count.
    /// This measures the engine the runtime runs — `WasmX3Adapter`, which the native runtime's
    /// adapter delegates to — on a program that spends exactly `g` gas, so the weight is linear in
    /// the gas an extrinsic is allowed.
    #[benchmark]
    fn x3_execute(g: Linear<1_000, 6_000_000>) -> Result<(), BenchmarkError> {
        use crate::X3ExecutorAdapter;
        let receipt;
        #[block]
        {
            receipt = crate::wasm_adapters::WasmX3Adapter::execute(
                crate::bench_fixtures::X3_LOOP_FIXTURE,
                g as u64,
            );
        }
        let receipt = receipt.map_err(|_| BenchmarkError::Stop("the loop fixture must execute"))?;
        assert!(!receipt.success, "the loop only ends by exhausting its gas");
        assert_eq!(receipt.gas_used, g as u64);
        Ok(())
    }

    /// Benchmark register_asset
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Symbol validation
    /// - Storage write to AssetRegistry
    /// - Event emission
    #[benchmark]
    fn register_asset() -> Result<(), BenchmarkError> {
        let asset_id: T::AssetId = Default::default();
        let symbol = b"X3".to_vec();
        let decimals = 18u8;

        // The development genesis already registers the default asset, so the precondition of
        // `register_asset` (this id is free) has to be established or the benchmark stops at
        // `AssetAlreadyRegistered` before it measures anything.
        AssetRegistry::<T>::remove(asset_id);

        #[extrinsic_call]
        register_asset(RawOrigin::Root, asset_id, symbol.clone(), decimals);

        // Verify: Asset was registered
        assert!(AssetRegistry::<T>::contains_key(asset_id));
        Ok(())
    }

    /// Benchmark update_canonical_balance
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Asset existence check
    /// - Storage write to CanonicalLedger
    /// - Optional event emission
    #[benchmark]
    fn update_canonical_balance() -> Result<(), BenchmarkError> {
        // Setup: Register asset first
        let asset_id: T::AssetId = Default::default();
        setup_asset::<T>(asset_id, b"X3");

        let account: T::AccountId = whitelisted_caller();
        let new_balance: T::Balance = 1_000_000u32.into();
        let comit_id = Some(H256::from_low_u64_be(1));

        #[extrinsic_call]
        update_canonical_balance(
            RawOrigin::Root,
            account.clone(),
            asset_id,
            new_balance,
            comit_id,
        );

        // Verify: Balance was updated
        assert_eq!(CanonicalLedger::<T>::get(&account, asset_id), new_balance);
        Ok(())
    }

    /// Benchmark authorize_account
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage write to AuthorizedAccounts
    /// - Event emission
    #[benchmark]
    fn authorize_account() -> Result<(), BenchmarkError> {
        let account: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        authorize_account(RawOrigin::Root, account.clone());

        // Verify: Account was authorized
        assert!(AuthorizedAccounts::<T>::contains_key(&account));
        Ok(())
    }

    /// Benchmark deauthorize_account
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage removal from AuthorizedAccounts
    /// - Event emission
    #[benchmark]
    fn deauthorize_account() -> Result<(), BenchmarkError> {
        let account: T::AccountId = whitelisted_caller();
        // Pre-authorize the account
        AuthorizedAccounts::<T>::insert(&account, ());

        #[extrinsic_call]
        deauthorize_account(RawOrigin::Root, account.clone());

        // Verify: Account was deauthorized
        assert!(!AuthorizedAccounts::<T>::contains_key(&account));
        Ok(())
    }

    /// Benchmark add_authority
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Existence check in BoundedVec
    /// - BoundedVec push operation
    /// - Event emission
    #[benchmark]
    fn add_authority() -> Result<(), BenchmarkError> {
        let authority: T::AccountId = whitelisted_caller();

        // Initialize authorities with some entries (worst case: near max)
        let initial_count = (T::MaxAuthorities::get() / 2) as usize;
        let mut initial_authorities: Vec<T::AccountId> = Vec::with_capacity(initial_count);
        for i in 0..initial_count {
            let acc = frame_benchmarking::account::<T::AccountId>("authority", i as u32, 0);
            initial_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            initial_authorities.try_into().expect("Within bounds");
        Authorities::<T>::put(bounded);

        #[extrinsic_call]
        add_authority(RawOrigin::Root, authority.clone());

        // Verify: Authority was added
        assert!(Authorities::<T>::get().contains(&authority));
        Ok(())
    }

    /// Benchmark remove_authority
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Linear search in BoundedVec
    /// - Minimum authorities check
    /// - BoundedVec remove operation
    /// - Event emission
    #[benchmark]
    fn remove_authority() -> Result<(), BenchmarkError> {
        let authority: T::AccountId = whitelisted_caller();

        // Initialize authorities with the target authority and some others
        let initial_count = (T::MaxAuthorities::get() / 2) as usize;
        let mut initial_authorities: Vec<T::AccountId> = Vec::with_capacity(initial_count);
        initial_authorities.push(authority.clone()); // Add target first
        for i in 0..(initial_count - 1) {
            let acc = frame_benchmarking::account::<T::AccountId>("authority", i as u32, 0);
            initial_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            initial_authorities.try_into().expect("Within bounds");
        Authorities::<T>::put(bounded);

        #[extrinsic_call]
        remove_authority(RawOrigin::Root, authority.clone());

        // Verify: Authority was removed
        assert!(!Authorities::<T>::get().contains(&authority));
        Ok(())
    }

    /// Benchmark schedule_authority_change
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Bounds validation
    /// - BoundedVec conversion
    /// - Storage write to PendingAuthorities
    /// - Event emission
    #[benchmark]
    fn schedule_authority_change() -> Result<(), BenchmarkError> {
        // Create a new authority set at half capacity
        let count = (T::MaxAuthorities::get() / 2) as usize;
        let mut new_authorities: Vec<T::AccountId> = Vec::with_capacity(count);
        for i in 0..count {
            let acc = frame_benchmarking::account::<T::AccountId>("new_authority", i as u32, 0);
            new_authorities.push(acc);
        }

        #[extrinsic_call]
        schedule_authority_change(RawOrigin::Root, new_authorities);

        // Verify: Pending changes were stored
        assert!(PendingAuthorities::<T>::get().is_some());
        Ok(())
    }

    /// Benchmark enact_authority_change
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - Storage read and take from PendingAuthorities
    /// - BoundedVec conversion
    /// - Storage write to Authorities
    /// - Event emission
    #[benchmark]
    fn enact_authority_change() -> Result<(), BenchmarkError> {
        // Setup: Schedule pending changes first
        let count = (T::MaxAuthorities::get() / 2) as usize;
        let mut new_authorities: Vec<T::AccountId> = Vec::with_capacity(count);
        for i in 0..count {
            let acc = frame_benchmarking::account::<T::AccountId>("pending_authority", i as u32, 0);
            new_authorities.push(acc);
        }
        let bounded: BoundedVec<T::AccountId, T::MaxAuthorities> =
            new_authorities.clone().try_into().expect("Within bounds");
        PendingAuthorities::<T>::put(Some(bounded));

        #[extrinsic_call]
        enact_authority_change(RawOrigin::Root);

        // Verify: Authorities were updated and pending cleared
        assert!(PendingAuthorities::<T>::get().is_none());
        assert_eq!(Authorities::<T>::get().len(), count);
        Ok(())
    }

    /// Benchmark emergency_pause
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - `ProtocolPaused` read (the already-paused guard) and write
    /// - Event emission
    ///
    /// The three emergency calls (`emergency_pause`, `emergency_unpause`,
    /// `emergency_halt`) charged literals of 10_000/15_000 picoseconds — a freeze
    /// of the chain at essentially zero cost. They are measured here instead.
    #[benchmark]
    fn emergency_pause() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
        emergency_pause(RawOrigin::Root);

        assert!(ProtocolPaused::<T>::get());
        Ok(())
    }

    /// Benchmark emergency_unpause
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - `ProtocolPaused` read and write
    /// - Event emission
    #[benchmark]
    fn emergency_unpause() -> Result<(), BenchmarkError> {
        // Setup: the chain has to be paused for the unpause arm to do its work.
        ProtocolPaused::<T>::put(true);

        #[extrinsic_call]
        emergency_unpause(RawOrigin::Root);

        assert!(!ProtocolPaused::<T>::get());
        Ok(())
    }

    /// Benchmark emergency_halt
    ///
    /// Measures cost of:
    /// - Governance origin check
    /// - The `EmergencyHaltController::trigger` call
    /// - Event emission
    #[benchmark]
    fn emergency_halt() -> Result<(), BenchmarkError> {
        #[extrinsic_call]
        emergency_halt(RawOrigin::Root);

        Ok(())
    }

    /// Benchmark `submit_trade`: the compiled flagship arbitrage (flash borrow, two swaps, repay,
    /// oracle-checked) committed against the market `TradingHost::benchmark_market` builds.
    ///
    /// The call is charged `trade_weight()`, a model: the measured `x3_execute` at the gas limit
    /// plus database weight per structure the section can name (`TradeMeter::upper_bound`). This
    /// measures what the model does not, the section verification and settlement compute, so the
    /// figure it produces is the one to compare against the model.
    #[benchmark]
    fn submit_trade() -> Result<(), BenchmarkError> {
        use crate::trading::{
            KernelTradingHost, TradingConfig, BENCHMARK_EXECUTED_AT, BENCHMARK_SUBMITTED_AT,
            BENCHMARK_TRADE_MODULE, BENCHMARK_USDX,
        };
        let trader: T::AccountId = whitelisted_caller();
        let collector: T::AccountId = account("fee-collector", 0, 0);
        frame_system::Pallet::<T>::set_block_number(BENCHMARK_EXECUTED_AT.into());
        T::TradingHost::benchmark_market(&trader).map_err(BenchmarkError::Stop)?;
        TradingConfiguration::<T>::put(TradingConfig {
            fee_asset: BENCHMARK_USDX,
            gas_price: 10,
            settlement_fee: 200_000,
            relay_fee: 300_000,
            protocol_fee: 100_000,
            oracle_fee: 10_000,
            solver_fee: 0,
            cross_domain_fee: 0,
            fee_collector: collector,
        });
        let module: BoundedVec<u8, ConstU32<{ crate::trading::MAX_TRADE_MODULE_BYTES }>> =
            BENCHMARK_TRADE_MODULE
                .to_vec()
                .try_into()
                .map_err(|_| BenchmarkError::Stop("the benchmark module exceeds the bound"))?;

        #[extrinsic_call]
        submit_trade(
            RawOrigin::Signed(trader),
            module,
            BENCHMARK_SUBMITTED_AT.into(),
        );

        // Committed, not rolled back: a rolled-back trade would measure the cheap path.
        let (_, record) = TradeReceipts::<T>::iter()
            .next()
            .ok_or(BenchmarkError::Stop("the trade left no receipt"))?;
        if !record.committed {
            return Err(BenchmarkError::Stop("the benchmark trade rolled back"));
        }
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
