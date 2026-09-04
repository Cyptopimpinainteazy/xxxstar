# Repository Cleanup Checklist

Generated: 2026-05-22T23:38:33.040751+00:00
Last audited: 2026-05-23T03:40:00+00:00

- [x] dup-1 DUPLICATES: 2 files with same content
    - cargo-check-v2.log / cargo-check.log
    - Resolution: stale CI log artifacts — acceptable, no code impact
- [x] todo-1 TODOS: node/src/service.rs — spawn_sidecar_service placeholder
    - Resolution: FIXED — real tokio::process::Command impl with exponential
      backoff + restart threshold at line ~1513. No TODO markers remain.
      Merged to main via PR #36 (2026-05-22).
- [x] todo-2 TODOS: docs/T5_PROPOSALS.md — precompile TODO markers
    - Resolution: TRACKED — RFC-t5-3-precompile-gas-addresses.md created.
      Requires runtime governance vote before code change. Blocked by design.
- [x] todo-3 TODOS: launch-gates/evidence/ci/embarrassment-raw-findings.txt
    - Resolution: FILE NO LONGER EXISTS — findings were resolved and the
      evidence file was not regenerated. No open items.
- [x] todo-4 TODOS: crates/x3-sidecar/src/main.rs — panic in data path
    - Resolution: FIXED — safe_str_prefix() replaces byte-slice truncation;
      both Client::build().unwrap() calls replaced with graceful error+exit.
      8 regression tests added. 46/46 tests pass. Committed f4492103 to main.
- [x] halfdone-1 HALF-DONE: node/src/service.rs — dead code suppressed
    - Resolution: ACCEPTABLE — #[allow(dead_code)] on experimental feature-gated
      paths (PoH, GPU) that are intentionally disabled for mainnet-v1.

## Open backlog (v2)

- [ ] poh-v2: PoH digest enforcement in block import
    - node/src/service.rs:886 — warning confirms enforcement not yet wired.
    - Shadow mode only for v1. 5 regression tests added to service.rs::tests
      to lock in shadow-mode behavior and prevent accidental v1 enforcement.
    - Tracked: implement verify_poh_digest() hook in block import pipeline.
- [ ] rfc-t5-3: Precompile gas benchmarking + address validation
    - docs/rfc/RFC-t5-3-precompile-gas-addresses.md — governance-gated.
    - No code action until runtime upgrade proposal passes council vote.

To remove selected items, run: python3 tools/agent_scanner/scan.py --delete ids.txt --yes
