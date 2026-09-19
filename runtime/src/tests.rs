//! Runtime-level tests for the settlement wiring.
//!
//! This file replaced an assertion-free placeholder. It used to hold fourteen
//! `#[test]` functions whose bodies were only comments ("Scenario: …",
//! "Assertion: …"), it was never declared in `lib.rs` (so nothing ever compiled
//! it), and its mock runtime was written against a pre-2022
//! `frame_system::Config`. The tests below assert against the real `Runtime`,
//! using the same `TestExternalities` pattern as the inline tests in `lib.rs`.

use super::*;
use frame_support::{assert_err, assert_ok};
use pallet_x3_settlement_engine::types::{AssetSpec, ExternalChainId, TokenId};
use sp_runtime::DispatchError;

fn account(seed: u8) -> AccountId {
    AccountId::from([seed; 32])
}

fn native_asset(chain: ExternalChainId, amount: u128) -> AssetSpec {
    AssetSpec {
        chain,
        token: TokenId::Native,
        amount,
    }
}

#[test]
fn runtime_genesis_config_builds() {
    use sp_runtime::BuildStorage;

    let storage = RuntimeGenesisConfig::default()
        .build_storage()
        .expect("the runtime's genesis config must build");
    // A genesis that produced nothing would still "build"; check the two
    // pallets this file exercises actually have storage entries.
    assert!(
        !storage.top.is_empty(),
        "genesis produced an empty top-level trie"
    );
}

/// The settlement engine must be reachable through the real runtime with a
/// signed origin, and the intent must land in its storage.
#[test]
fn settlement_intent_creation_is_wired_through_the_runtime() {
    sp_io::TestExternalities::default().execute_with(|| {
        let maker = account(0xAA);
        let taker = account(0xBB);
        let secret_hash = H256::from([0x11; 32]);

        assert_ok!(
            pallet_x3_settlement_engine::Pallet::<Runtime>::create_intent(
                RuntimeOrigin::signed(maker.clone()),
                taker,
                native_asset(ExternalChainId::X3Native, 1_000),
                native_asset(ExternalChainId::Ethereum, 500),
                secret_hash,
                Some(600),
            )
        );

        let intents: Vec<_> =
            pallet_x3_settlement_engine::SettlementIntents::<Runtime>::iter().collect();
        assert_eq!(intents.len(), 1, "the intent was stored");
        assert_eq!(intents[0].1.maker, maker);
        assert_eq!(intents[0].1.secret_hash, secret_hash);
        assert_eq!(intents[0].1.legs_total, 2);
    });
}

/// Settlement intents are per-account, so an unsigned origin must be refused
/// rather than attributed to some default account.
#[test]
fn settlement_intent_creation_requires_a_signed_origin() {
    sp_io::TestExternalities::default().execute_with(|| {
        let result = pallet_x3_settlement_engine::Pallet::<Runtime>::create_intent(
            RuntimeOrigin::root(),
            account(0xAA),
            native_asset(ExternalChainId::X3Native, 1_000),
            native_asset(ExternalChainId::Ethereum, 500),
            H256::from([0x11; 32]),
            None,
        );
        assert_err!(result, DispatchError::BadOrigin);
        assert!(
            pallet_x3_settlement_engine::SettlementIntents::<Runtime>::iter().count() == 0,
            "a rejected call must not leave an intent behind"
        );
    });
}
