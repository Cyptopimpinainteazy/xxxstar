# X3 7-VALIDATOR READINESS AUDIT — PHASE 0 INVENTORY

Captured: 2026-10-01T04:46Z
Base revision: `2685f7d89` on `feat/x3-guardian` (26 working-tree entries)
Companion data: `repository-state.json` (same directory)
Method: repository inspection only; every statement below names its evidence.
No hardware was touched.

## 1. Current X3 software status

| Area | State | Evidence |
| --- | --- | --- |
| Toolchain | Rust/Cargo 1.90.0; polkadot-sdk `stable2512` | Cargo.toml, `rustc --version` |
| Runtime | spec_version 20, impl 1, tx 1; node 0.1.0 | runtime/src/lib.rs:395 |
| Local CI (32+ fast gates) | **RUNNING at capture**; result appended when it exits | `scripts/local-ci.sh` session |
| Audit matrix consistency | **PASS** | `python3 scripts/x3_audit_matrix.py --check` |
| Forge completion scan | 153 features: 2 high + 27 medium + 2 low unfinished | `tools/x3-forge/completion.py scan` |
| Router | v1 complete; 126 tests PASS; live and serving | `.ai/reports/router-gap-analysis-20260930.md` |
| Loopback 7-validator network | Works: 7/7 identical finalized heads, 2000/2000 remarks, 110.6 finTPS, 8/8 cold starts, 7/7 single-loss survival | TESTNET_GAP_LEDGER.md ("Confirmed solid", reserved full-mesh) |
| Release binary | `target/release/x3-chain-node`, SHA-256 recorded, **built 2026-09-28 — predates HEAD; not candidate-bound** | repository-state.json |

Not proven by this audit (require their own runs before any campaign): fresh
`cargo test --workspace`, WASM runtime build at this commit, release build at
this commit, and a launch of 7 validators from a candidate-bound artifact set.

## 2. Existing deployment automation

Present:
- `scripts/testnet/run-7-validators-local.sh` — 7 local validators, per-validator
  stable node keys, authority/bootnode preflight, `--only N` rejoin path.
- `scripts/testnet/build-x3-testnet-spec.py` — derives authorities, seeds, node
  keys, bootnodes; writes a plain Live spec.
- `scripts/testnet/run-mesh.py` — reserved full-mesh launcher with deterministic
  node keys; `cycles` and `kills` drill modes.
- `scripts/testnet/inject-keystore.sh`, `generate-node-keys`, `subkey-js-shim.cjs`.
- `scripts/bootstrap-validator.sh`, `scripts/harden-validator.sh`, `scripts/deploy-all.sh`.
- `packaging/systemd/x3-validator.service`, `x3-bootnode.service` (hardened unit,
  no containers, no unsafe RPC flags).
- `scripts/testnet/{x3_testnet_up,down,health}.sh`, `status-7-validators.sh`
  (peer/finality checks with fail conditions), `public-testnet-gate-drill.sh`,
  `testnet_rc_gate.sh`, `consensus-soak.sh`, `runtime_upgrade_rehearsal.sh`.
- `.github/workflows/testnet-deploy.yml` exists (never run for a real deploy per
  TESTNET_GAP_LEDGER 2026-09-22).

Absent (bare metal): no Ansible, no autoinstall/cloud-init, no PXE, no hardware
profiles, no BIOS/firmware policy, no storage maps, no OS baseline, no network
plan, no `x3-lab` operator CLI. Verified by filesystem search across the repo.

## 3. Existing validator tooling

- Bring-up: `run-7-validators-local.sh`, `run-fresh-validators.sh`,
  `run-fresh-mesh.py`, `run-solo-join.py`, `x3_testnet_up.sh`.
- Drills: `validator-failure-drill.sh`, `validator-rotation-drill.sh`,
  `testnet-ceremony.py` + `testnet-ceremony-drill.sh`,
  `scripts/drills/{node_restart_drill,halt_recovery_live,...}.sh`.
- Observability/status: `status-7-validators.sh`, `x3_testnet_health.sh`,
  `scripts/testnet/continuous-verify.sh`.
- Key operations: `generate-node-keys`, `inject-keystore.sh`,
  `scripts/generate-bootnode-keys.py`, on-chain rotation via
  `pallet_x3_custody` (TESTNET_GAP_LEDGER, 2026-09-22).

## 4. Chain-spec / genesis state

- Committed: `deployment/chain-specs/x3-testnet-raw.json` (+ baseline),
  `deployment/chain-specs/fresh/x3-testnet-plain.json` (keys under
  `fresh/generated/` are gitignored), `chain-specs/x3-local3-*.json`.
- Generators: `scripts/testnet/build-x3-testnet-spec.py`,
  `scripts/testnet/generate_testnet_chain_spec.sh`,
  `scripts/mainnet/generate_mainnet_chain_spec.sh`.
- Integrity gates: `scripts/ci/verify_chain_spec_baseline.sh`,
  `scripts/check-runtime-hash-freshness.py` (in local-ci).
- Missing: a frozen `x3-7val-testnet.json` artifact with recorded SHA-256 bound
  to an exact commit, plus binary/runtime hashes in one manifest (PHASE 3).

## 5. Existing monitoring

- Prometheus scrape config and rules; Alertmanager config
  (`monitoring/config/prometheus.yml`, `monitoring/prometheus-rules.yml`,
  `monitoring/alertmanager.yml`).
- Grafana dashboard for local3 validator networks with 7 panels: finalized
  height, peer count, node role, best height, block production rate, GRANDPA
  rounds, runtime build (`monitoring/local3/grafana-x3-validators.json`).
- Benchmark dashboard (`monitoring/grafana/dashboards/x3-benchmark-dashboard.json`).
- Node metrics via `--prometheus` on port 9615 (validator unit sets it).
- Missing: central log aggregation (Loki is not installed), and no automated
  per-validator evidence collector writing to `audit-artifacts/`.

## 6. Existing external testing tools

From `reports/toolchain/external-tool-gap-analysis.md` (generated 2026-09-30):
GATED 45 / EVIDENCED 7 / WIRED 3 / INSTALLED_ONLY 2 / REFERENCE_ONLY 3 /
MISSING 6. The report states plainly that GATED means wiring only — it is not
proof a tool ran, and EXERCISED/EVIDENCED/REPEATABLE are `not_determinable`
without commit-bound runs.

Notable for this campaign:
- Installed and gated: zombienet, kani, cargo-fuzz, cargo-mutants, proptest,
  foundry/anvil, slither, semgrep, cargo-audit, cargo-deny, prometheus, miri,
  loom, solana-test-validator, trivy, srtool.
- Installed but **not gated**: ansible, halmos.
- Gated but **not installed**: echidna, grafana, k6, try-runtime, toxiproxy,
  revm, litesvm, chaos-mesh, codeql, osv-scanner, podman, subwasm, tempo,
  tla+/apalache, trufflehog, trivy(installed yes).
- Missing entirely: mollusk, proptest-state-machine, syft, pumba,
  defihacklabs, grype (as container), honggfuzz (wired only), trident (wired only).

## 7. P0 blockers (fix before any physical deployment)

1. No candidate-bound release artifact for the current HEAD. The only release
   binary is 3 days older than `2685f7d89`; no `x3-7val-testnet.json`, no
   binary/runtime hash manifest for a candidate. (PHASE 3, PHASE 15, STEP 4.)
2. Completion-scan high findings, unclosed:
   - `CURRENT_MAINNET_STATUS.md` claims `tested=75` but no test symbol surveyed
     in its own paths → a claim the evidence cannot back (X3-CLAIM-002).
   - `pallets/x3-supply-ledger` — 1 of 4 required tests absent from the index
     (X3-ECO-002).
3. `x3-lang/vm/src/btc_adapter.rs` carries `TODO: Implement ...` markers and no
   real production behavior in the paths five features rely on (X3-LANG-007,
   X3-MEV-002..005) — a "code exists but behaviour does not" blocker.
4. Fresh full-workspace verification is not yet recorded at this commit
   (local-ci RUNNING at capture; `cargo test --workspace` not re-run).

## 8. P1 blockers

1. Nine fuzz-target files are TODO stubs ("Add specific structure decoding
   tests for pallet-x-..."); six of them are cited by features (completion scan).
2. `scripts/mainnet_release_gate.py` asserts a 65% mainnet score while no
   manifest depends on `generate-node-keys` (X3-SEC-003/010 gap).
3. Ansible is installed but belongs to no gate; even a lint-only gate does not
   exist yet.
4. try-runtime, k6, echidna not installed although gated; GUI/observability
   tooling (grafana binary, loki) absent from the host.
5. Key distribution/rotation for the artifact- and receipt-root keys remains an
   operational gap (recorded in agent memory, 2026-09-29).

## 9. Missing bare-metal automation (PHASE 10/11/12/37)

- `infra/autoinstall/` — unattended Ubuntu (user-data/meta-data, USB flow).
- `infra/ansible/` — inventory, group_vars, host_vars, roles, playbooks.
- `infra/hardware/{hp,dell,lenovo}/` — BIOS/firmware profiles per model, and a
  hardware discovery script producing `hardware-matrix.md`.
- Storage maps, OS baseline document, network plan, time-sync checks.
- `scripts/x3-lab` + subcommands (inventory, preflight, deploy, start, stop,
  status, health, benchmark, restart, logs, verify-hashes, collect-evidence).
- `docs/validator-lab/00..15`.
- Artifact/spec freeze + hash verification step in the deploy path.

## 10. Exact files/scripts that should be created or modified

Create: `infra/ansible/**`, `infra/autoinstall/**`, `infra/hardware/**`,
`scripts/x3-lab`, `scripts/validator-lab/{preflight,health,collect-evidence,verify-hashes}.sh`,
`docs/validator-lab/*.md`, `monitoring/validator-lab/*`,
`audit-artifacts/7-validator/{x3-7val-testnet.json,artifact-manifest.json}`.

Modify: `packaging/systemd/x3-validator.service` (add ExecStartPre hash check
against the frozen manifest), `scripts/local-ci.sh` (add a `--lab` gate family
once they exist), `TESTNET_GAP_LEDGER.md` (record campaign evidence).

## 11. Recommended first bounded implementation task

**Freeze a candidate-bound 7-validator artifact set at `2685f7d89` and prove it
launches locally.** Concretely: build the release node + WASM runtime at this
commit, generate a 7-authority plain Live spec with `build-x3-testnet-spec.py`,
record binary/runtime/spec SHA-256 into
`audit-artifacts/7-validator/artifact-manifest.json`, then run the reserved
full-mesh 7-validator launch from exactly those files and capture status +
finality evidence. This is STEP 4 of the prompt and the cheapest way to turn the
"predates HEAD" gap into a candidate-bound baseline the bare-metal work can
deploy. It blocks nothing downstream and everything downstream reads it.
