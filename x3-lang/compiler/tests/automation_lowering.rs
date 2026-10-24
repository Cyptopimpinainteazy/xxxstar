//! An `automation` declaration compiles into the chain's own task vocabulary.
//!
//! End to end: source text → canonical parser → `AutomationDecl` →
//! `x3_automation::{Condition, Action}` — the exact types
//! `pallet-x3-automation` executes. Nothing here is a mock of the target shape.

use x3_automation::{Action, Condition};
use x3_lang_ast::ast::Item;
use x3_lang_compiler::automation_lowering::{lower, AutomationPlan};
use x3_lang_compiler::parser::parse_source;

fn plan(source: &str) -> AutomationPlan {
    let program = parse_source(source).expect("source parses");
    let decl = program
        .items
        .iter()
        .find_map(|item| match &item.node {
            Item::Automation(decl) => Some(decl),
            _ => None,
        })
        .expect("an automation declaration");
    lower(decl).expect("the declaration lowers")
}

fn lower_error(source: &str) -> String {
    let program = parse_source(source).expect("source parses");
    let decl = program
        .items
        .iter()
        .find_map(|item| match &item.node {
            Item::Automation(decl) => Some(decl),
            _ => None,
        })
        .expect("an automation declaration");
    match lower(decl) {
        Ok(plan) => panic!("expected a refusal, got {plan:?}"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn a_full_declaration_lowers_into_the_pallets_own_types() {
    let plan = plan(
        r#"
automation "X3 Autonomous Trading" {
    trigger { price_above(1, 1000) }
    permissions {
        wallet: "trading-vault",
        max_trade: 1000,
        daily_loss_limit: 250,
        allowed_chain: "X3",
        allowed_chain: "Ethereum"
    }
    execute { transfer_native(7, 5) }
    on_failure { no_change() }
}
"#,
    );

    assert_eq!(plan.name, "X3 Autonomous Trading");
    assert_eq!(
        plan.condition,
        Condition::PriceThreshold {
            asset_id: 1,
            threshold: 1000,
            above: true,
        }
    );
    assert_eq!(
        plan.action,
        Action::TransferNative {
            recipient: {
                let mut key = [0u8; 32];
                key[..16].copy_from_slice(&7u128.to_le_bytes());
                key
            },
            amount: 5,
        }
    );
    assert_eq!(plan.budgets.wallet.as_deref(), Some("trading-vault"));
    assert_eq!(plan.budgets.max_trade, Some(1000));
    assert_eq!(plan.budgets.daily_loss_limit, Some(250));
    assert_eq!(plan.budgets.allowed_chains, vec!["X3", "Ethereum"]);
}

#[test]
fn every_supported_trigger_maps_to_a_decidable_condition() {
    let cases = [
        ("manual()", Condition::Manual),
        ("block_number(100)", Condition::BlockNumber(100)),
        ("timestamp(1700000000000)", Condition::Timestamp(1_700_000_000_000)),
        (
            "price_below(2, 42)",
            Condition::PriceThreshold {
                asset_id: 2,
                threshold: 42,
                above: false,
            },
        ),
        (
            "price_band(3, 10, 20)",
            Condition::PriceBand {
                asset_id: 3,
                lower: 10,
                upper: 20,
            },
        ),
    ];
    for (trigger, expected) in cases {
        let source = format!("automation A {{ trigger {{ {trigger} }} execute {{ no_change() }} }}");
        assert_eq!(plan(&source).condition, expected, "trigger `{trigger}`");
    }
}

#[test]
fn every_supported_action_maps_to_a_whitelisted_verb() {
    assert_eq!(
        plan("automation A { trigger { manual() } execute { no_change() } }").action,
        Action::NoChange
    );

    let with_asset = plan("automation A { trigger { manual() } execute { transfer_asset(9, 4, 100) } }");
    assert_eq!(
        with_asset.action,
        Action::TransferLedgerAsset {
            asset: 9,
            recipient: {
                let mut key = [0u8; 32];
                key[..16].copy_from_slice(&4u128.to_le_bytes());
                key
            },
            amount: 100,
        }
    );
}

#[test]
fn a_hex_recipient_is_decoded_exactly() {
    let hex = "0101010101010101010101010101010101010101010101010101010101010101";
    let plan = plan(&format!(
        "automation A {{ trigger {{ manual() }} execute {{ transfer_native(\"0x{hex}\", 1) }} }}"
    ));
    let Action::TransferNative { recipient, .. } = plan.action else {
        panic!("expected a native transfer");
    };
    assert_eq!(recipient, [1u8; 32]);
}

#[test]
fn an_inverted_price_band_is_refused() {
    let error = lower_error("automation A { trigger { price_band(1, 20, 10) } execute { no_change() } }");
    assert!(error.contains("lower bound"), "got: {error}");
    assert!(error.contains("never fire"), "got: {error}");
}

#[test]
fn an_unknown_trigger_is_refused_naming_the_construct() {
    let error = lower_error("automation A { trigger { http_price(1) } execute { no_change() } }");
    assert!(error.contains("http_price"), "got: {error}");
    assert!(error.contains("cannot be decided on chain"), "got: {error}");
}

#[test]
fn a_trigger_that_is_not_a_call_is_refused() {
    let error = lower_error("automation A { trigger { 5 } execute { no_change() } }");
    assert!(error.contains("not a call"), "got: {error}");
}

#[test]
fn an_off_chain_connector_in_execute_is_refused_rather_than_dropped() {
    let error = lower_error("automation A { trigger { manual() } execute { http_notify(1) } }");
    assert!(error.contains("http_notify"), "got: {error}");
    assert!(error.contains("not an executable action"), "got: {error}");
}

#[test]
fn a_multi_statement_execute_body_is_refused() {
    // The chain executes one whitelisted action. A body of several statements
    // would need a state machine the pallet does not have, so it must not appear
    // to have run.
    let error = lower_error("automation A { trigger { manual() } execute { no_change() no_change() } }");
    assert!(error.contains("more than one statement"), "got: {error}");
}

#[test]
fn an_empty_execute_body_is_refused() {
    let error = lower_error("automation A { trigger { manual() } execute { } }");
    assert!(error.contains("empty"), "got: {error}");
}

#[test]
fn a_missing_trigger_section_is_a_parse_error() {
    let error = parse_source("automation A { execute { no_change() } }").unwrap_err();
    assert!(error.to_string().contains("trigger"), "got: {error}");
}

#[test]
fn a_missing_execute_section_is_a_parse_error() {
    let error = parse_source("automation A { trigger { manual() } }").unwrap_err();
    assert!(error.to_string().contains("execute"), "got: {error}");
}

#[test]
fn an_unknown_permission_is_refused_at_parse_time() {
    let error =
        parse_source("automation A { trigger { manual() } permissions { max_leverage: 10 } execute { no_change() } }")
            .unwrap_err();
    assert!(error.to_string().contains("max_leverage"), "got: {error}");
}

#[test]
fn a_non_integer_budget_is_refused() {
    let error = lower_error(
        r#"automation A { trigger { manual() } permissions { max_trade: "1000" } execute { no_change() } }"#,
    );
    assert!(error.contains("non-negative integer"), "got: {error}");
}

#[test]
fn a_duplicate_permission_is_refused() {
    let error = lower_error(
        "automation A { trigger { manual() } permissions { max_trade: 1, max_trade: 2 } execute { no_change() } }",
    );
    assert!(error.contains("twice"), "got: {error}");
}

#[test]
fn on_failure_is_optional_and_does_not_change_the_action() {
    let without = plan("automation A { trigger { manual() } execute { no_change() } }");
    let with = plan("automation A { trigger { manual() } execute { no_change() } on_failure { no_change() } }");
    assert_eq!(without.action, with.action);
    assert_eq!(without.condition, with.condition);
}

#[test]
fn an_asset_id_outside_the_oracle_space_is_refused() {
    let error = lower_error("automation A { trigger { price_above(4294967296, 1) } execute { no_change() } }");
    assert!(error.contains("u32 asset space"), "got: {error}");
}
