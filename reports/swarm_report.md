# X3 Swarm Report

- API path: crates/x3-swarm-core/services/x3-swarm-api
- Worker path: crates/x3-swarm-core/services/x3-swarm-worker
- Memory path: data/agent-memory
- Generated at: 2026-05-04T21:02:53Z

## Swarm Health Summary
- API /health: ok
- API URL: http://127.0.0.1:8787

## API Snapshot
```json
{"agents":0,"kill_switch":false,"service":"x3-swarm-api","status":"ok","tasks":9}
```

### Current Tasks
```json
[{"id":"x3-task-0007","title":"Audit core runtime path guard","feature":"swarm-forbidden-path","agent":"swarm-guard","permission_tier":"constrained","allowed_paths":["crates/x3-swarm-core/src","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["cargo test -p x3-swarm-core -- --nocapture"],"status":"Pending","approval_required":"manual","risk":"medium"},{"id":"x3-task-0008","title":"Collect agent memory snapshot","feature":"memory-store","agent":"swarm-memory","permission_tier":"read-only","allowed_paths":["data/agent-memory"],"forbidden_paths":["./.git","*secret*","*.key"],"required_commands":["cat data/agent-memory/*.jsonl"],"status":"Pending","approval_required":"auto","risk":"low"},{"id":"x3-task-0002","title":"Collect agent memory snapshot","feature":"memory-store","agent":"swarm-memory","permission_tier":"read-only","allowed_paths":["data/agent-memory"],"forbidden_paths":["./.git","*secret*","*.key"],"required_commands":["cat data/agent-memory/*.jsonl"],"status":"Pending","approval_required":"auto","risk":"low"},{"id":"x3-task-0001","title":"Audit core runtime path guard","feature":"swarm-forbidden-path","agent":"swarm-guard","permission_tier":"constrained","allowed_paths":["crates/x3-swarm-core/src","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["cargo test -p x3-swarm-core -- --nocapture"],"status":"Pending","approval_required":"manual","risk":"medium"},{"id":"x3-task-0004","title":"Audit core runtime path guard","feature":"swarm-forbidden-path","agent":"swarm-guard","permission_tier":"constrained","allowed_paths":["crates/x3-swarm-core/src","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["cargo test -p x3-swarm-core -- --nocapture"],"status":"Pending","approval_required":"manual","risk":"medium"},{"id":"x3-task-0005","title":"Collect agent memory snapshot","feature":"memory-store","agent":"swarm-memory","permission_tier":"read-only","allowed_paths":["data/agent-memory"],"forbidden_paths":["./.git","*secret*","*.key"],"required_commands":["cat data/agent-memory/*.jsonl"],"status":"Pending","approval_required":"auto","risk":"low"},{"id":"x3-task-0006","title":"Generate swarm report","feature":"swarm-report","agent":"swarm-analyst","permission_tier":"read-only","allowed_paths":["reports/","scripts/swarm/","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["scripts/swarm/swarm_report.sh"],"status":"Pending","approval_required":"auto","risk":"low"},{"id":"x3-task-0009","title":"Generate swarm report","feature":"swarm-report","agent":"swarm-analyst","permission_tier":"read-only","allowed_paths":["reports/","scripts/swarm/","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["scripts/swarm/swarm_report.sh"],"status":"Pending","approval_required":"auto","risk":"low"},{"id":"x3-task-0003","title":"Generate swarm report","feature":"swarm-report","agent":"swarm-analyst","permission_tier":"read-only","allowed_paths":["reports/","scripts/swarm/","crates/x3-swarm-core/services/x3-swarm-api/src"],"forbidden_paths":["./.git","secrets/","node_modules/"],"required_commands":["scripts/swarm/swarm_report.sh"],"status":"Pending","approval_required":"auto","risk":"low"}]
```

### Memory Entries
```json
[]
```

### Recent Events
```json
[]
```

## Local Scan
- Generated file list from scripts/swarm/swarm_scan.sh

/home/lojak/Desktop/X3_ATOMIC_STAR/.legion/AUDITOR.md
/home/lojak/Desktop/X3_ATOMIC_STAR/.legion/FIXER.md
/home/lojak/Desktop/X3_ATOMIC_STAR/.legion/SCANNER.md
/home/lojak/Desktop/X3_ATOMIC_STAR/.legion/INTEGRATOR.md
/home/lojak/Desktop/X3_ATOMIC_STAR/.legion/ARCHITECT.md
/home/lojak/Desktop/X3_ATOMIC_STAR/stakeholder_comms/SECURITY_TEAM_SPRINT_PLAN.md
/home/lojak/Desktop/X3_ATOMIC_STAR/stakeholder_comms/CTO_BRIEF_META_BLOCKERS.md
/home/lojak/Desktop/X3_ATOMIC_STAR/stakeholder_comms/ENGINEERING_TEAM_ANNOUNCEMENT.md
/home/lojak/Desktop/X3_ATOMIC_STAR/TODO.md
/home/lojak/Desktop/X3_ATOMIC_STAR/FASTEST_MAINNET_PLAN.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/guard_tests.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/marketing_claims_audit.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/feature_gap_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/tauri_wiring_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_health_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/service_health_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_scan_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-operator-20260427T225228Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-operator-20260427T225412Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-operator-20260427T225554Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-full-20260427T225413Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-full-20260427T225705Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-full-20260427T225228Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/prompt-runs/prompt-operator-20260427T225325Z.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/SUBSTRATE_PROOF_PACK_LATEST.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/substrate-proof-pack-20260426-121738.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/README.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/substrate-proof-pack-20260426-121648.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/FRAME_WEIGHTS_BLOCKER_20260426.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/substrate/substrate-proof-pack-20260426-115335.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/missing_tests_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/reactor_benchmark_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_secret_warning.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_build_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/scripts/x3/crates/x3-feature-registry/README.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/grant_pipeline_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/panic_unwrap_audit.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/btc_gateway_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/swarm_task_queue.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/gpu_inventory_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/mainnet_rc_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/testnet_readiness_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/reports/dead_buttons_report.md
/home/lojak/Desktop/X3_ATOMIC_STAR/Cargo.toml
/home/lojak/Desktop/X3_ATOMIC_STAR/web/mainnet-progress/README.md
/home/lojak/Desktop/X3_ATOMIC_STAR/monitor-builds.sh
/home/lojak/Desktop/X3_ATOMIC_STAR/V0_4_INTERNAL_MAINNET_STATUS.md
/home/lojak/Desktop/X3_ATOMIC_STAR/node_modules/xmlchars/README.md
/home/lojak/Desktop/X3_ATOMIC_STAR/node_modules/nanoid/README.md
