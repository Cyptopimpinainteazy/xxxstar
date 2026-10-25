//! Real FRAME storage, authenticated registrations, and canonical compiler output.
use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok};

fn source() -> &'static str {
    include_str!("../../../x3-lang/examples/native_trading.x3")
}
fn asset_key(symbol: &str, id: &str) -> Vec<u8> {
    format!(r#"{{"vm_family":"x3vm","chain":"x3","canonical_id":"{id}","symbol":"{symbol}","decimals":0}}"#).into_bytes()
}
fn setup() {
    for (id, symbol, identity, is_fee) in [(0, "A", "0x01", true), (78, "B", "0x02", false)] {
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            id,
            symbol.as_bytes().to_vec(),
            0
        ));
        assert_ok!(AtlasKernel::register_native_trading_asset(
            RuntimeOrigin::root(),
            asset_key(symbol, identity),
            id,
            is_fee
        ));
    }
    for (account, a, b) in [
        (ALICE, 0, 0),
        (BOB, 1000, 0),
        (10, 10000, 20000),
        (11, 10000, 10000),
    ] {
        CanonicalLedger::<Test>::insert(account, 0, a);
        CanonicalLedger::<Test>::insert(account, 78, b);
    }
    assert_ok!(AtlasKernel::register_native_trading_provider(
        RuntimeOrigin::signed(BOB),
        b"lender".to_vec()
    ));
    assert_ok!(AtlasKernel::register_native_trading_venue(
        RuntimeOrigin::signed(10),
        b"left".to_vec(),
        0,
        78
    ));
    assert_ok!(AtlasKernel::register_native_trading_venue(
        RuntimeOrigin::signed(11),
        b"right".to_vec(),
        0,
        78
    ));
}

#[test]
fn canonical_trade_moves_real_balances_and_repays_real_lender() {
    new_test_ext().execute_with(|| {
        setup();
        let bytes = x3_x3_integration::compiler_bridge::compile_source(source()).unwrap();
        let receipt = crate::native_trading::execute::<Test>(&ALICE, &bytes, 100_000, 0).unwrap();
        assert!(receipt.success);
        assert_eq!(CanonicalLedger::<Test>::get(BOB, 0), 1000);
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, 0), 94);
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, 78), 0);
        assert_eq!(CanonicalLedger::<Test>::get(10, 0), 10100);
        assert_eq!(CanonicalLedger::<Test>::get(10, 78), 19802);
        assert_eq!(CanonicalLedger::<Test>::get(11, 0), 9806);
        assert_eq!(CanonicalLedger::<Test>::get(11, 78), 10198);
        assert!(!receipt.return_data.is_empty());
    });
}

#[test]
fn rejected_profit_rolls_back_lender_pools_and_trader() {
    new_test_ext().execute_with(|| {
        setup();
        let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
        let source = source().replace("net_profit >= 1 A", "net_profit >= 1000 A");
        let bytes = x3_x3_integration::compiler_bridge::compile_source(&source).unwrap();
        assert!(crate::native_trading::execute::<Test>(&ALICE, &bytes, 100_000, 0).is_err());
        assert_eq!(sp_io::storage::root(sp_runtime::StateVersion::V1), before);
    });
}

#[test]
fn native_registration_requires_governance_and_preserves_asset_identity() {
    new_test_ext().execute_with(|| {
        setup();
        assert_err!(
            AtlasKernel::register_native_trading_asset(
                RuntimeOrigin::signed(ALICE),
                asset_key("A", "0x03"),
                0,
                false
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        assert!(AtlasKernel::register_native_trading_asset(
            RuntimeOrigin::root(),
            asset_key("B", "0x04"),
            0,
            false
        )
        .is_err());
        assert!(AtlasKernel::register_native_trading_venue(
            RuntimeOrigin::signed(ALICE),
            b"left".to_vec(),
            0,
            78
        )
        .is_err());
    });
}

#[test]
fn signed_canonical_submission_persists_receipt_and_rejects_replay() {
    new_test_ext().execute_with(|| {
        setup();
        let bytes = x3_x3_integration::compiler_bridge::compile_source(source()).unwrap();
        let id = H256::repeat_byte(91);
        let fee = 10;
        let initial_fee_balance = Balances::free_balance(ALICE);
        let gas = x3_lang_execution::artifact::decode_artifact(&bytes)
            .unwrap()
            .gas_used;
        let actual_fee = AtlasKernel::calculate_execution_fee_v2(0, 0, gas, 0).unwrap();
        let root = AtlasKernel::compute_prepare_root_v2(id, &[], &[], &bytes, 0, fee);
        assert_ok!(AtlasKernel::submit_comit_v2(
            RuntimeOrigin::signed(ALICE),
            id,
            vec![],
            vec![],
            bytes.clone(),
            0,
            fee,
            root
        ));
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, 0), 94);
        assert_eq!(Nonces::<Test>::get(ALICE), 1);
        assert_eq!(
            Balances::free_balance(ALICE),
            initial_fee_balance - actual_fee
        );
        let receipt = X3ExecutionReceipts::<Test>::get(id).unwrap();
        assert_eq!(receipt.gas_used, gas);
        assert_eq!(receipt.protocol_version, 2);
        let evidence: serde_json::Value = serde_json::from_slice(&receipt.return_data).unwrap();
        assert_eq!(evidence[5][0]["amount"], serde_json::json!(actual_fee));
        assert!(SubmittedComits::<Test>::contains_key(id));
        let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
        assert!(AtlasKernel::submit_comit_v2(
            RuntimeOrigin::signed(ALICE),
            id,
            vec![],
            vec![],
            bytes,
            0,
            fee,
            root
        )
        .is_err());
        assert_eq!(sp_io::storage::root(sp_runtime::StateVersion::V1), before);
    });
}

#[test]
fn late_prepare_rejection_rolls_back_trade_nonce_fee_and_events() {
    new_test_ext().execute_with(|| {
        setup();
        let bytes = x3_x3_integration::compiler_bridge::compile_source(source()).unwrap();
        let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
        assert!(AtlasKernel::submit_comit_v2(
            RuntimeOrigin::signed(ALICE),
            H256::repeat_byte(92),
            vec![],
            vec![],
            bytes,
            0,
            10,
            H256::zero()
        )
        .is_err());
        assert_eq!(sp_io::storage::root(sp_runtime::StateVersion::V1), before);
    });
}

#[test]
fn failures_preserve_the_entire_storage_state() {
    for case in [
        "gas",
        "deadline",
        "liquidity",
        "lender",
        "venue",
        "identity",
        "private",
        "fee",
    ] {
        new_test_ext().execute_with(|| {
            setup();
            let mut program = source().to_string();
            let mut gas_limit = 100_000;
            let mut fee = 0;
            match case {
                "gas" => gas_limit = 1,
                "deadline" => System::set_block_number(101),
                "liquidity" => CanonicalLedger::<Test>::insert(BOB, 0, 99),
                "lender" => program = program.replace("from lender", "from missing"),
                "venue" => program = program.replace("via right", "via missing"),
                "identity" => program = program.replace("x3vm.x3.0x02", "x3vm.x3.0x03"),
                "private" => program = program.replace("submission: false", "submission: true"),
                "fee" => fee = 11,
                _ => unreachable!(),
            }
            let bytes = x3_x3_integration::compiler_bridge::compile_source(&program).unwrap();
            let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
            assert!(
                crate::native_trading::execute::<Test>(&ALICE, &bytes, gas_limit, fee).is_err(),
                "{case}"
            );
            assert_eq!(
                sp_io::storage::root(sp_runtime::StateVersion::V1),
                before,
                "{case}"
            );
        });
    }
}

#[test]
fn governance_cannot_label_another_asset_as_the_fee_currency() {
    new_test_ext().execute_with(|| {
        assert_ok!(AtlasKernel::register_asset(
            RuntimeOrigin::root(),
            78,
            b"B".to_vec(),
            0
        ));
        assert!(AtlasKernel::register_native_trading_asset(
            RuntimeOrigin::root(),
            asset_key("B", "0x02"),
            78,
            true
        )
        .is_err());
        assert!(!NativeTradingFeeAsset::<Test>::exists());
        assert_eq!(NativeTradingAssets::<Test>::iter().count(), 0);
    });
}
