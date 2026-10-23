//! The `automation` declaration: grammar, structure, and the determinism boundary.

use x3_ast::{Item, PermissionKind};
use x3_parser::parse_program;

const ARBITRAGE: &str = r#"
automation "X3 Autonomous Trading" {
    trigger {
        PriceThreshold { asset_id: 1, threshold: 1000, above: true }
    }

    permissions {
        wallet: "trading-vault",
        max_trade: 1000,
        daily_loss_limit: 250,
        allowed_chains: ["X3", "Ethereum"]
    }

    execute {
        let ok = check();
        if ok {
            settle();
        }
    }

    on_failure {
        notify();
    }
}
"#;

#[test]
fn a_full_automation_parses_into_its_sections() {
    let module = parse_program(ARBITRAGE).expect("automation parses");
    assert_eq!(module.items.len(), 1);
    let Item::Automation(automation) = &module.items[0] else {
        panic!("expected an automation item, got {:?}", module.items[0]);
    };

    assert_eq!(automation.name.name, "X3 Autonomous Trading");
    assert_eq!(automation.permissions.len(), 4);
    assert_eq!(
        automation
            .permissions
            .iter()
            .map(|p| p.kind)
            .collect::<Vec<_>>(),
        vec![
            PermissionKind::Wallet,
            PermissionKind::MaxTrade,
            PermissionKind::DailyLossLimit,
            PermissionKind::AllowedChains,
        ]
    );
    assert!(!automation.execute.statements.is_empty());
    assert!(automation.on_failure.is_some());
}

#[test]
fn on_failure_is_optional() {
    let source = r#"
automation A {
    trigger { Manual }
    execute { settle(); }
}
"#;
    let module = parse_program(source).expect("parses without on_failure");
    let Item::Automation(automation) = &module.items[0] else {
        panic!("expected an automation item");
    };
    assert!(automation.on_failure.is_none());
    assert!(automation.permissions.is_empty());
}

#[test]
fn a_trigger_section_is_required() {
    let source = r#"
automation A {
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("a trigger is mandatory");
    assert!(
        err.to_string().contains("trigger"),
        "the error should name the missing section: {err}"
    );
}

#[test]
fn an_execute_section_is_required() {
    let source = r#"
automation A {
    trigger { Manual }
}
"#;
    let err = parse_program(source).expect_err("execute is mandatory");
    assert!(
        err.to_string().contains("execute"),
        "the error should name the missing section: {err}"
    );
}

#[test]
fn an_unknown_permission_is_refused() {
    let source = r#"
automation A {
    trigger { Manual }
    permissions { max_leverage: 10 }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("unknown permissions must not be silently ignored");
    assert!(
        err.to_string().contains("max_leverage"),
        "the error should name the offending permission: {err}"
    );
}

#[test]
fn a_duplicate_permission_is_refused() {
    let source = r#"
automation A {
    trigger { Manual }
    permissions {
        max_trade: 100,
        max_trade: 200
    }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("a capability declared twice is ambiguous");
    assert!(err.to_string().contains("twice"), "got: {err}");
}

#[test]
fn an_off_chain_connector_is_refused_in_execute() {
    for connector in ["http", "ai", "gpu"] {
        let source = format!(
            r#"
automation A {{
    trigger {{ Manual }}
    execute {{ {connector}.get(); }}
}}
"#
        );
        let err = parse_program(&source)
            .expect_err("an on-chain workflow may not call an off-chain connector");
        let message = err.to_string();
        assert!(
            message.contains(connector) && message.contains("off-chain"),
            "expected a determinism-boundary error naming `{connector}`, got: {message}"
        );
    }
}

#[test]
fn an_off_chain_connector_is_refused_in_on_failure_too() {
    let source = r#"
automation A {
    trigger { Manual }
    execute { settle(); }
    on_failure { http.notify(); }
}
"#;
    let err = parse_program(source).expect_err("on_failure runs on-chain as well");
    assert!(err.to_string().contains("on_failure"), "got: {err}");
}

/// Every spelling of an off-chain connector must be refused. An earlier version
/// checked only a bare `Call` at statement level, so all of these passed.
#[test]
fn no_spelling_of_an_off_chain_connector_gets_through() {
    let bodies = [
        // wrapped in a binding
        "let r = http.get(url);",
        // wrapped in a call argument
        "log(http.get(url));",
        // returned
        "return ai.classify(x);",
        // inside a conditional
        "if c { gpu.submit(job); }",
        // inside a loop body
        "while c { http.post(u); }",
        // inside an atomic block
        "atomic { http.get(u); }",
        // qualified path, connector not the root
        "chain.http.get(u);",
        // nested in a binary expression
        "let n = 1 + ai.score(x);",
    ];
    for body in bodies {
        let source = format!(
            r#"
automation A {{
    trigger {{ Manual }}
    execute {{ {body} }}
}}
"#
        );
        let err = match parse_program(&source) {
            Err(err) => err,
            Ok(_) => panic!("`{body}` must be refused"),
        };
        assert!(
            err.to_string().contains("off-chain"),
            "expected a determinism-boundary error for `{body}`, got: {err}"
        );
    }
}

/// The trigger decides the on-chain condition, so it is held to the same rule.
#[test]
fn an_off_chain_connector_in_the_trigger_is_refused() {
    let source = r#"
automation A {
    trigger { http.price() }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("the trigger is chain-side too");
    assert!(err.to_string().contains("off-chain"), "got: {err}");
}

/// A refusal in `on_failure` is covered elsewhere; this pins that the walk
/// descends into nested blocks there as well.
#[test]
fn an_off_chain_connector_nested_in_on_failure_is_refused() {
    let source = r#"
automation A {
    trigger { Manual }
    execute { settle(); }
    on_failure { if failed { gpu.retry(); } }
}
"#;
    let err = parse_program(source).expect_err("nested connector in on_failure");
    assert!(err.to_string().contains("off-chain"), "got: {err}");
}

#[test]
fn a_deterministic_body_parses() {
    let source = r#"
automation A {
    trigger { BlockNumber(100) }
    permissions { max_trade: 10 }
    execute {
        let x = compute(1, 2);
        emit Result { value: x };
    }
}
"#;
    let module = parse_program(source).expect("a deterministic workflow is allowed");
    assert!(matches!(module.items[0], Item::Automation(_)));
}

// ─────────────────────────────────────────────────────────────────────────────
// Capabilities are budgets, so their values are checked, not coerced.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a_negative_max_trade_is_refused() {
    let source = r#"
automation A {
    trigger { Manual }
    permissions { max_trade: -5 }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("a negative budget is meaningless");
    assert!(err.to_string().contains("negative"), "got: {err}");
}

#[test]
fn a_non_integer_loss_limit_is_refused() {
    let source = r#"
automation A {
    trigger { Manual }
    permissions { daily_loss_limit: "250" }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("a string is not an amount");
    assert!(
        err.to_string().contains("non-negative integer"),
        "got: {err}"
    );
}

#[test]
fn allowed_chains_must_be_a_list_of_names() {
    let ok = r#"
automation A {
    trigger { Manual }
    permissions { allowed_chains: ["X3", "Ethereum"] }
    execute { settle(); }
}
"#;
    assert!(parse_program(ok).is_ok());

    let bad = r#"
automation A {
    trigger { Manual }
    permissions { allowed_chains: 42 }
    execute { settle(); }
}
"#;
    let err = parse_program(bad).expect_err("chains must be a list");
    assert!(
        err.to_string().contains("array of chain names"),
        "got: {err}"
    );

    let non_string_element = r#"
automation A {
    trigger { Manual }
    permissions { allowed_chains: ["X3", 7] }
    execute { settle(); }
}
"#;
    let err = parse_program(non_string_element).expect_err("chain names are strings");
    assert!(
        err.to_string().contains("array of chain names"),
        "got: {err}"
    );
}

#[test]
fn a_wallet_may_be_a_name_or_an_identifier() {
    for value in ["\"trading-vault\"", "vault"] {
        let source = format!(
            r#"
automation A {{
    trigger {{ Manual }}
    permissions {{ wallet: {value} }}
    execute {{ settle(); }}
}}
"#
        );
        assert!(
            parse_program(&source).is_ok(),
            "wallet `{value}` should be accepted"
        );
    }

    let source = r#"
automation A {
    trigger { Manual }
    permissions { wallet: 12 }
    execute { settle(); }
}
"#;
    let err = parse_program(source).expect_err("a number is not a wallet");
    assert!(err.to_string().contains("wallet"), "got: {err}");
}
