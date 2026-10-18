//! Regression guard: every table the gateway's SQL references must be created
//! by the migrations `Database::connect` applies.
//!
//! 2026-10-01: `crates/x3-gateway/migrations/` was missing 0001-0005, so a real
//! Postgres `Database::connect` (and `connect_isolated_for_test`) created none
//! of the benchmark/orchestra tables and every DB-backed endpoint returned 500
//! (`relation "benchmark_reports" does not exist`). The unit tests skipped
//! silently without `DATABASE_URL`, so CI stayed green. This test needs no
//! database: it statically proves the schema coverage that a live DB would
//! enforce, and fails if a future query targets a table no migration creates.
//!
//! Tables the gateway only *reads* are owned by `x3-indexer` and live in its
//! migration set (`crates/x3-indexer/migrations/`); they are allowlisted here.

use std::collections::BTreeSet;
use std::fs;

const SOURCES: &[&str] = &[
    include_str!("../src/funding_discovery.rs"),
    include_str!("../src/db.rs"),
    include_str!("../src/orchestra.rs"),
    include_str!("../src/rest.rs"),
];

/// Tables created by `x3-indexer`'s schema, which the gateway reads but does
/// not own. Adding a gateway-owned table here defeats the guard.
const INDEXER_OWNED: &[&str] = &[
    "blocks",
    "extrinsics",
    "events",
    "accounts",
    "comit_transactions",
];

fn extract_table_names(text: &str) -> BTreeSet<String> {
    let mut tables = BTreeSet::new();
    let mut in_string = false;
    let mut literal = String::new();
    for ch in text.chars() {
        if ch == '"' {
            if in_string {
                scan_literal(&literal, &mut tables);
                literal.clear();
            }
            in_string = !in_string;
        } else if in_string {
            literal.push(ch);
        }
    }
    tables
}

fn scan_literal(literal: &str, tables: &mut BTreeSet<String>) {
    let mut rest = literal;
    while let Some(pos) = rest.find(|c: char| c.is_ascii_uppercase()) {
        let upper = &rest[pos..];
        let (verb, tail) = if let Some(tail) = upper
            .strip_prefix("FROM ")
            .or_else(|| upper.strip_prefix("FROM\n"))
        {
            ("FROM", tail)
        } else if let Some(tail) = upper.strip_prefix("INTO ") {
            ("INTO", tail)
        } else if let Some(tail) = upper.strip_prefix("UPDATE ") {
            ("UPDATE", tail)
        } else if let Some(tail) = upper.strip_prefix("JOIN ") {
            ("JOIN", tail)
        } else {
            rest = &upper[1..];
            continue;
        };
        let tail = tail.trim_start();
        // A table name can end the literal ("... FROM benchmark_reports"); without the
        // fallback to the literal's end those were skipped and the guard missed them.
        let end = tail
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .unwrap_or(tail.len());
        let ident = &tail[..end];
        if !ident.is_empty() {
            tables.insert(ident.to_string());
        }
        rest = &tail[end..];
        let _ = verb;
    }
}

#[test]
fn every_table_the_gateway_queries_is_created_by_a_migration() {
    let migrations_dir = fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
        .expect("migrations directory must exist");
    let mut created: BTreeSet<String> = BTreeSet::new();
    for entry in migrations_dir {
        let path = entry.expect("read migration entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }
        let sql = fs::read_to_string(&path).expect("read migration file");
        let mut rest = sql.as_str();
        while let Some(pos) = rest.find("CREATE TABLE ") {
            let tail = &rest[pos + "CREATE TABLE ".len()..];
            let tail = tail.strip_prefix("IF NOT EXISTS ").unwrap_or(tail);
            let ident: String = tail
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '_')
                .collect();
            assert!(
                !ident.is_empty(),
                "migration {} has a CREATE TABLE without a name",
                path.display()
            );
            created.insert(ident.clone());
            rest = &tail[ident.len()..];
        }
    }
    assert!(
        created.len() >= 8,
        "expected the gateway migrations to create its core tables; found {created:?}"
    );

    let mut referenced = BTreeSet::new();
    for source in SOURCES {
        referenced.extend(extract_table_names(source));
    }
    assert!(
        referenced.len() >= 10,
        "table extraction must not be vacuous; found {referenced:?}"
    );

    let mut uncovered: Vec<String> = referenced
        .iter()
        .filter(|table| !created.contains(*table) && !INDEXER_OWNED.contains(&table.as_str()))
        .cloned()
        .collect();
    uncovered.sort();
    assert!(
        uncovered.is_empty(),
        "gateway queries tables that no migration creates (indexer-owned tables must be \
         allowlisted in INDEXER_OWNED): {uncovered:?}\n\
         migrations create: {created:?}\n\
         referenced tables: {referenced:?}"
    );
}
