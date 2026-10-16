//! Whole-runtime fuzzing harness for `x3-chain-runtime`.
//!
//! Adapted from SRLabs' `substrate-runtime-fuzzer` (solochain template,
//! <https://github.com/srlabs/substrate-runtime-fuzzer>, Apache-2.0 OR MIT,
//! Copyright (c) SRLabs). See `NOTICE` in this crate.
//!
//! An input is a SCALE stream of `(advance_block, origin, RuntimeCall)` triples. Each
//! call is dispatched as a signed call from one of the dev genesis accounts inside real
//! `Executive` block initialisation and finalisation, starting from the runtime's own
//! `dev` genesis preset. After the run, invariants are asserted; any violation panics,
//! which is what a fuzzer (or the seeded smoke test) reports.
//!
//! Invariants:
//! - a call's pre-dispatch weight is never below its post-dispatch weight (ref time and
//!   proof size): undercharged weight is a denial-of-service vector;
//! - no block takes more than 2 s to execute;
//! - no account has consumers without providers;
//! - frozen balance equals the largest lock, and holds never exceed reserved balance;
//! - total issuance equals the sum of free + reserved over all accounts, and never grows;
//! - every pallet's `integrity_test`, and with the `try-runtime` feature `try_state`.

use codec::{DecodeLimit, Encode};
use frame_support::{
    dispatch::GetDispatchInfo, pallet_prelude::Weight, traits::IntegrityTest,
    weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use frame_system::Account;
use pallet_balances::{Holds, TotalIssuance};
use sp_consensus_aura::{Slot, AURA_ENGINE_ID};
use sp_runtime::{
    traits::{Dispatchable, Header as _},
    BuildStorage, Digest, DigestItem, Storage,
};
use sp_state_machine::BasicExternalities;
use std::time::{Duration, Instant};
use x3_chain_runtime::{
    AccountId, AllPalletsWithSystem, Balance, Balances, Executive, Header, Runtime, RuntimeCall,
    RuntimeGenesisConfig, RuntimeOrigin, Timestamp, SLOT_DURATION,
};

/// The runtime's own `dev` preset, the same genesis a `--dev` node starts from.
const DEV_PRESET: &[u8] = include_bytes!("../../../runtime/genesis-presets/dev.json");

/// Maximum SCALE nesting accepted while decoding calls from fuzz input.
const DECODE_DEPTH: u32 = 64;

/// Genesis storage plus the accounts that act as call origins.
pub struct Genesis {
    pub storage: Storage,
    pub accounts: Vec<AccountId>,
    /// Top-level sections of the preset this runtime build does not have (for example
    /// `evm`/`ethereum` in a build without the `frontier` feature). Recorded, not hidden.
    pub dropped_sections: Vec<String>,
}

/// Build genesis from the `dev` preset. Sections the compiled runtime does not know are
/// dropped one at a time and reported, so the harness works on every runtime variant.
pub fn genesis() -> Genesis {
    let mut json: serde_json::Value =
        serde_json::from_slice(DEV_PRESET).expect("dev preset is JSON");
    let accounts: Vec<AccountId> = json["balances"]["balances"]
        .as_array()
        .expect("dev preset endows accounts")
        .iter()
        .map(|entry| {
            let ss58 = entry[0].as_str().expect("endowed account is SS58");
            <AccountId as sp_core::crypto::Ss58Codec>::from_ss58check(ss58).expect("valid SS58")
        })
        .collect();
    assert!(
        !accounts.is_empty(),
        "the dev preset endows no account to fuzz with"
    );

    let mut dropped_sections = Vec::new();
    loop {
        match serde_json::from_value::<RuntimeGenesisConfig>(json.clone()) {
            Ok(config) => {
                let storage = config.build_storage().expect("dev preset builds storage");
                return Genesis {
                    storage,
                    accounts,
                    dropped_sections,
                };
            }
            Err(err) => {
                // serde reports `unknown field `evm``: drop that section and retry.
                let message = err.to_string();
                let field = message
                    .split('`')
                    .nth(1)
                    .filter(|f| json.get(*f).is_some())
                    .unwrap_or_else(|| panic!("dev preset does not fit this runtime: {message}"))
                    .to_string();
                json.as_object_mut()
                    .expect("preset is an object")
                    .remove(&field);
                dropped_sections.push(field);
            }
        }
    }
}

/// Decode an input into `(advance_block, origin_index, call)` triples.
pub fn decode_input(mut data: &[u8]) -> Vec<(bool, u8, RuntimeCall)> {
    std::iter::from_fn(|| DecodeLimit::decode_with_depth_limit(DECODE_DEPTH, &mut data).ok())
        // `System` calls (set_code, kill_storage, ...) are root-only and would only test
        // that root is refused; they also make runs non-comparable.
        .filter(|(_, _, call): &(bool, u8, RuntimeCall)| !matches!(call, RuntimeCall::System(_)))
        .collect()
}

/// Encode one triple the way [`decode_input`] reads it (for building seed inputs).
pub fn encode_step(advance_block: bool, origin: u8, call: &RuntimeCall) -> Vec<u8> {
    (advance_block, origin, call).encode()
}

/// Run one input against a fresh copy of genesis. Panics on any invariant violation.
/// Returns the number of calls dispatched.
pub fn process_input(genesis: &Genesis, data: &[u8]) -> usize {
    let steps = decode_input(data);
    if steps.is_empty() {
        return 0;
    }
    let mut dispatched = 0;
    BasicExternalities::execute_with_storage(&mut genesis.storage.clone(), || {
        let initial_total_issuance = TotalIssuance::<Runtime>::get();
        let mut block: u32 = 1;
        let mut weight = Weight::zero();
        let mut elapsed = Duration::ZERO;
        initialize_block(block);

        for (advance_block, origin, call) in steps {
            if advance_block {
                finalize_block(elapsed);
                block += 1;
                weight = Weight::zero();
                elapsed = Duration::ZERO;
                initialize_block(block);
            }
            let pre_weight = call.get_dispatch_info().call_weight;
            let cumulative = weight.saturating_add(pre_weight);
            if cumulative.ref_time() >= 2 * WEIGHT_REF_TIME_PER_SECOND {
                continue; // would not fit in a block
            }
            weight = cumulative;

            let who = genesis.accounts[origin as usize % genesis.accounts.len()].clone();
            let started = Instant::now();
            let result = call.clone().dispatch(RuntimeOrigin::signed(who));
            elapsed += started.elapsed();
            dispatched += 1;

            let post = result
                .unwrap_or_else(|e| e.post_info)
                .actual_weight
                .unwrap_or_default();
            assert!(
                pre_weight.ref_time() >= post.ref_time(),
                "undercharged ref time: pre {} < post {} for {call:?}",
                pre_weight.ref_time(),
                post.ref_time()
            );
            assert!(
                pre_weight.proof_size() >= post.proof_size(),
                "undercharged proof size: pre {} < post {} for {call:?}",
                pre_weight.proof_size(),
                post.proof_size()
            );
        }
        finalize_block(elapsed);
        check_invariants(block, initial_total_issuance);
    });
    dispatched
}

fn initialize_block(block: u32) {
    Executive::initialize_block(&Header::new(
        block,
        Default::default(),
        Default::default(),
        Default::default(),
        Digest {
            logs: vec![DigestItem::PreRuntime(
                AURA_ENGINE_ID,
                Slot::from(u64::from(block)).encode(),
            )],
        },
    ));
    Timestamp::set(RuntimeOrigin::none(), u64::from(block) * SLOT_DURATION)
        .expect("timestamp inherent accepted");
}

fn finalize_block(elapsed: Duration) {
    assert!(elapsed.as_secs() <= 2, "block execution took {elapsed:?}");
    Executive::finalize_block();
}

fn check_invariants(block: u32, initial_total_issuance: Balance) {
    let mut counted_free: Balance = 0;
    let mut counted_reserved: Balance = 0;
    for (account, info) in Account::<Runtime>::iter() {
        assert!(
            !(info.consumers > 0 && info.providers == 0),
            "account {account:?} has consumers without providers"
        );
        counted_free += info.data.free;
        counted_reserved += info.data.reserved;
        let max_lock: Balance = Balances::locks(&account)
            .iter()
            .map(|l| l.amount)
            .max()
            .unwrap_or_default();
        assert_eq!(
            max_lock, info.data.frozen,
            "frozen balance != largest lock for {account:?}"
        );
        let held: Balance = Holds::<Runtime>::get(&account)
            .iter()
            .map(|h| h.amount)
            .sum();
        assert!(
            held <= info.data.reserved,
            "holds {held} exceed reserved {} for {account:?}",
            info.data.reserved
        );
    }
    let total_issuance = TotalIssuance::<Runtime>::get();
    assert_eq!(
        total_issuance,
        counted_free + counted_reserved,
        "total issuance != sum of free + reserved balances"
    );
    assert!(
        total_issuance <= initial_total_issuance,
        "total issuance grew from {initial_total_issuance} to {total_issuance}"
    );
    AllPalletsWithSystem::integrity_test();
    #[cfg(feature = "try-runtime")]
    {
        use frame_support::traits::{TryState, TryStateSelect};
        AllPalletsWithSystem::try_state(block, TryStateSelect::All).expect("try_state");
    }
    let _ = block;
}
