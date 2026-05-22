# Repository Cleanup Checklist

Generated: 2026-05-22T23:38:33.040751+00:00

- [ ] dup-1 DUPLICATES: 2 files with same content
    - cargo-check-v2.log
    - cargo-check.log
- [ ] todo-1 TODOS: node/src/service.rs (last used: 2026-05-22T21:37:31+00:00)
    - TODO:                // Perform health check (TODO(t5-1): implement actual process detecti
    - todo: ection + RPC probe — tracked in session todo 't5-1')
    - TODO: hour sleep loop.
- [ ] todo-2 TODOS: docs/T5_PROPOSALS.md (last used: 2026-05-22T21:07:46+00:00)
    - TODO: 1 (node/src/service.rs:1355)
    - TODO: iew required) Precompile setup includes TODO markers — may affect gas accounting or 
- [ ] todo-3 TODOS: launch-gates/evidence/ci/embarrassment-raw-findings.txt (last used: 2026-05-15T01:03:58.687948+00:00)
    - TODO: tches
- [ ] todo-4 TODOS: crates/x3-sidecar/src/main.rs (last used: 2026-05-22T21:38:13+00:00)
    - panic!(: s[3]]) >> 2) as usize,
- [ ] halfdone-1 HALF-DONE: node/src/service.rs (last used: 2026-05-22T21:37:31+00:00)
    - Dead code allowed (suppressed warning)

To remove selected items, run: python3 tools/agent_scanner/scan.py --delete ids.txt --yes
