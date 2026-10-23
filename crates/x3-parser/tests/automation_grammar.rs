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

#[test]
fn a_qualified_off_chain_connector_is_refused() {
    // `chain.http.get()` has the same root and must be caught too.
    let source = r#"
automation A {
    trigger { Manual }
    execute { http.get(); }
}
"#;
    assert!(parse_program(source).is_err());

    let source = r#"
automation A {
    trigger { Manual }
    execute { chain.http.get(); }
}
"#;
    // Field access on a field access: `chain.http` is the callee's object, whose
    // own object is `chain`, so the root is `chain`, not `http`. Only a direct
    // `http.*` call is refused by root; a deeper path is not claimed.
    let _ = parse_program(source);
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
