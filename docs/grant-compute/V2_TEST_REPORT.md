# Funding integrity milestone verification

Base: `70423054e2d9560a212ab763d4f265f9cdc448d9`. Work branch:
`codex/grant-compute-audit-20261018`. Results observed 2026-10-18.

| Command / experiment | Result | Scope |
|---|---|---|
| Execute baseline production UPDATE SQL on disposable PostgreSQL | Expected regression FAIL | Discovery could jump to approved; seeded awarded amount was 120000 USD |
| `CARGO_TARGET_DIR=... CARGO_BUILD_JOBS=8 bash scripts/gateway-postgres-integration.sh` | PASS, exit 0 | Explicitly runs real PostgreSQL test, not the ignored default path |
| `cargo test --locked -p x3-gateway --lib` with shared target and 8 jobs | PASS, exit 0; 52 tests | Gateway unit suite; no full-workspace inference |
| `rustfmt --check --edition 2021 crates/x3-gateway/src/db.rs crates/x3-gateway/tests/postgres_integration.rs` | PASS | Changed Rust formatting |
| `git diff --check` | PASS | Source diff whitespace |
| `cargo clippy --locked -p x3-gateway --all-targets -- -D warnings` | PASS, exit 0 | Gateway and all its test targets |

The PostgreSQL suite covers fresh and legacy migrations, gateway/indexer shared
migration histories, real grant intake/read-back, rejection of skipped approval
stages and unsupported financial writes, exactly one committed concurrent
transition, injected audit insertion failure with state rollback, reconnect
persistence, unknown grants and corrupted stages. Historical seed records remain
available to administrators but are absent from public grants/timeline/award
totals. The new migration is shared with the indexer via the existing symlink
convention. No historical migration checksum changed.

The integration build began before the final production patch, and Cargo picked
up the changed source before executing the passing tests. The baseline failure
evidence is specifically `sql-before.log`, not the full Rust integration log
named `before-postgres.log`. Do not misrepresent the latter as a red result.

No award is marked verified by this migration or API. The legacy award-paid
handler now fails through the restricted database method. An evidence-controlled
award/submission workflow remains required. Approval audit events are
transactional; they are not yet a tamper-evident, per-user approval system.

Clippy and subsequent dashboard/discovery work are separately verified; no
RUNNING command is treated as PASS here. Full workspace tests/clippy, full CI,
production runtime WASM, live GPU execution, public sandbox safety and end-to-end
research funding are NOT PROVEN by these commands. The PostgreSQL script sets
`SKIP_WASM_BUILD=1` for its dependency build; this is DB proof only.

Proof files: `.ai/runlogs/grant-compute-20261018/sql-before.log`,
`before-postgres.log`, `gateway-unit.log`, plus PR/CI/source snapshots in that
directory. The baseline volunteer tests independently passed 23 cases at #626;
strict-input hardening on #627's base passes 28 cases and is draft PR #629 at
`53fd11bb584c12524ace858b22801775a5686bc7`. Its CI was queued, not green, at last
inspection. No changes were merged.
