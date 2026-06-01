//! JSON Schema validator for X3 Intent schema
//!
//! This module provides a test that validates X3 Intent JSON against the schema.json
//! using the jsonschema crate.

use jsonschema::{Draft, JSONSchema};
use serde_json::json;

/// Test that validates X3 Intent JSON against the schema
#[test]
fn test_x3_intent_schema_validation() {
    // Load the schema from the file (relative to CARGO_MANIFEST_DIR: crates/x3-compiler/)
    let schema_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../x3-lang/schema.json");
    let schema_json = std::fs::read_to_string(&schema_path).expect("Failed to read schema.json");

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_json).expect("Failed to parse schema.json");

    // Create a JSON schema validator
    let schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_value)
        .expect("Invalid schema");

    // Test valid X3 Intent
    let valid_intent = json!({
        "intent": "swap",
        "from": {
            "chain": "ethereum",
            "asset": "USDC",
            "amount": "1000"
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        },
        "path": [
            {
                "type": "swap",
                "dex": "uniswap",
                "from": "USDC",
                "to": "WETH"
            },
            {
                "type": "bridge",
                "via": "wormhole",
                "from": "WETH",
                "to": "SOL"
            }
        ],
        "constraints": {
            "min_profit": "10",
            "max_slippage": "1",
            "timeout": "3600",
            "atomic": true
        }
    });

    let result = schema.validate(&valid_intent);
    assert!(result.is_ok(), "Valid intent should pass schema validation");

    // Test invalid X3 Intent (missing required field)
    let invalid_intent = json!({
        "intent": "swap",
        "from": {
            "chain": "ethereum",
            "asset": "USDC"
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        }
        // Missing "path" field
    });

    let result = schema.validate(&invalid_intent);
    assert!(
        result.is_err(),
        "Invalid intent should fail schema validation"
    );

    // Test invalid asset symbol
    let invalid_asset_intent = json!({
        "intent": "swap",
        "from": {
            "chain": "ethereum",
            "asset": "XRP", // Invalid asset symbol
            "amount": "1000"
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        },
        "path": [
            {
                "type": "swap",
                "dex": "uniswap",
                "from": "USDC",
                "to": "WETH"
            }
        ]
    });

    let result = schema.validate(&invalid_asset_intent);
    assert!(
        result.is_ok(),
        "Schema validation should pass even with invalid asset symbols"
    );
}

#[test]
fn test_schema_validation_edge_cases() {
    let schema_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../x3-lang/schema.json");
    let schema_json = std::fs::read_to_string(&schema_path).expect("Failed to read schema.json");

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_json).expect("Failed to parse schema.json");

    let schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_value)
        .expect("Invalid schema");

    // Test empty path
    let empty_path_intent = json!({
        "intent": "swap",
        "from": {
            "chain": "ethereum",
            "asset": "USDC",
            "amount": "1000"
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        },
        "path": []
    });

    let result = schema.validate(&empty_path_intent);
    assert!(result.is_ok(), "Empty path should be valid");

    // Test null amount
    let null_amount_intent = json!({
        "intent": "swap",
        "from": {
            "chain": "ethereum",
            "asset": "USDC",
            "amount": null
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        },
        "path": [
            {
                "type": "swap",
                "dex": "uniswap",
                "from": "USDC",
                "to": "WETH"
            }
        ]
    });

    let result = schema.validate(&null_amount_intent);
    assert!(result.is_ok(), "Null amount should be valid");
}

#[test]
fn test_schema_validation_invalid_types() {
    let schema_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../x3-lang/schema.json");
    let schema_json = std::fs::read_to_string(&schema_path).expect("Failed to read schema.json");

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_json).expect("Failed to parse schema.json");

    let schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema_value)
        .expect("Invalid schema");

    // Test invalid intent type (number instead of string)
    let invalid_type_intent = json!({
        "intent": 123, // Should be string
        "from": {
            "chain": "ethereum",
            "asset": "USDC",
            "amount": "1000"
        },
        "to": {
            "chain": "solana",
            "asset": "SOL"
        },
        "path": []
    });

    let result = schema.validate(&invalid_type_intent);
    assert!(
        result.is_err(),
        "Invalid intent type should fail validation"
    );
}
