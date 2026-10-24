# X3 External Toolchain — Gap Analysis (Phase 0, refreshed)

**Date:** 2026-10-01
**Commit:** `ff57d15dd6e93ba4bcc19081d18bcd5632954941` (master, unchanged)
**Companion artifact:** [`external-tool-inventory.json`](external-tool-inventory.json) — 90 tool entries with per-tool classification, status, evidence paths, gaps and next action.

**Why this file was regenerated.** The first pass was produced in a
**network-disabled** sandbox and recorded ~20 tools as absent or blocked because
they could not be installed. That pass is stale. This version was produced with
the network enabled and a full host re-probe (`command -v`, `--version`, and a
dependency-resolution test). No software was installed, removed, or
reconfigured by this pass. 23 tool statuses changed; every change is listed in
`refresh_summary.status_changes` inside the JSON.

---

## 1. Headline findings

1. **The host is now roughly as capable as the repo expects; the wiring is not.**
   Of 90 inventoried capabilities, 27 now have a working binary/dependency on the
   host, yet only **6 are IMPLEMENTED** in the sense the prompt requires
   (installed → configured → connected → exercised → evidenced → repeatable).
   The gap has moved from *"the tools are missing"* to *"the tools are installed
   and still not exercised."* That is the dominant risk now.

2. **Almost nothing is gated.** 37 of 39 GitHub workflows are
   `workflow_dispatch` only. Only `queue-drain.yml` and
   `swarm-reactor-pr519-verify.yml` run on push. Several workflows state in a
   header comment that *"GitHub-hosted CI is disabled for this repo; local
   pre-push CI is the gate of record"* — and `core.hooksPath` is unset, so
   `.githooks/{commit-msg,pre-commit,pre-push}` are **not wired**. The gate of
   record is a script a human must remember to run.

3. **The single biggest "wired but never run" asset is fuzzing, and it just
   unblocked.** 75 fuzz crates / 98 targets exist. `cargo-fuzz 0.13.2` is now
   installed and `libfuzzer-sys 0.4.13` resolves from crates.io (verified via
   `cargo metadata` on `crates/x3-integration/fuzz`). Nothing has been built or
   run, and only one corpus directory exists in the whole tree
   (`crates/x3-integration/fuzz/corpus`). **cargo-fuzz: BLOCKED → WIRED.**

4. **Mutation testing is still the only oracle with captured evidence — and it
   is misdirected.** `mutants.out/` records cargo-mutants 27.1.0: 94 caught,
   59 missed, 6 unviable (61.4% kill rate). It ran against `x3-fees`, not the
   security crates, and the 59 survivors are unmapped. The binary is back, so
   this is now repeatable — it just hasn't been pointed at the right code.

5. **The high-value EVM differential oracle still does not exist.** REVM has
   zero references anywhere in the workspace. Foundry/anvil 1.8.3 is installed
   and hosts the HTLC lifecycle, but nothing compares the X3 EVM's execution
   against an independent EVM implementation. This is the most important
   *missing* (not merely unexercised) integration.

6. **try-runtime remains hard-blocked.** The standalone CLI was removed from
   the pinned Polkadot SDK (`polkadot-stable2603`); `try-runtime-upgrade.yml`
   documents that it intentionally fails until a compatible state-replay runner
   exists. Runtime upgrades currently have no pre/post migration verification.

7. **Miri is not actually available.** `~/.cargo/bin/cargo-miri` exists as a
   symlink to rustup, but `cargo miri` reports the component is unavailable for
   stable `1.90.0`. It needs a nightly toolchain. Any "Miri runs in CI" claim is
   false today.

---

## 2. X3 EXTERNAL TOOLCHAIN STATUS

Only fields established from evidence are filled. `—` means no evidence exists;
it is not a stand-in for "probably fine". This table is the human-readable
summary; the JSON holds all 90 entries including REFERENCE/REJECT rows.

| Tool | Purpose | Class | Version | Status | X3 integration | Evidence | CI | Release | Compute | Next action |
|---|---|---|---|---|---|---|---|---|---|---|
| cargo-fuzz / libFuzzer | Fuzz parsers, packet decode, proof parse, X3VM, compiler | TEST_ENGINE | 0.13.2 | **PARTIAL** | 75 fuzz crates, 98 targets | 2 targets run + defect-detection proof | `--fuzz` gate (opt-in) | none | MEDIUM | extend to pallet targets; promote gate |
| proptest | Property tests: conservation, transitions, serialization | TEST_ENGINE | 1.4 | **IMPLEMENTED** | workspace dep, ~116 files | runs under `cargo test --workspace` | via unit gate | none | FAST | extract a named property gate + fixed seed set |
| cargo-mutants | Measure whether tests detect defects | TEST_ORACLE | 27.1.0 | **PARTIAL** | config names asset-kernel + atomic-trade | `mutants.out/` 94/59/6 | none | none | EXPENSIVE | re-run on security crates, ingest survivors |
| Kani | Bounded model checking of settlement/replay/authz | TEST_ORACLE | 0.68.0 | **INSTALLED_ONLY** | listed in tool-stack config only | no harness, no counterexample | formal-verification.yml does not invoke it | none | EXPENSIVE | write 6 harnesses, prove one end to end |
| try-runtime | Pre/post storage-migration checks | TEST_ENGINE | — | **BLOCKED** | scripts/run-try-runtime.sh | workflow header says it intentionally fails | try-runtime-upgrade.yml (manual, failing) | gate row with no producer | MEDIUM | highest-value substrate gap |
| Zombienet | Multi-validator topology, failure, partitions | TEST_ENGINE | 1.3.10 | **PARTIAL** | run script + 7-validator topology + finality smoke spec | none captured | zombienet-integration.yml (manual) | none | EXPENSIVE | execute topology once, capture finality |
| Chopsticks | Runtime state replay, upgrade testing | TEST_ENGINE | — | **PARTIAL** | scripts/run-chopsticks.sh | none captured | none | none | MEDIUM | pair with a try-runtime replacement |
| FRAME benchmarking | Weight generation and drift detection | TEST_ENGINE | workspace dep | **WIRED** | weights.rs + benchmarking.rs per pallet | no run captured | frame-benchmarking.yml (manual) | gate row exists | EXPENSIVE | run once, capture the diff |
| srtool | Reproducible runtime WASM | INFRASTRUCTURE | 0.13.2 | **PARTIAL** | release packaging scripts | release_hashes.txt exists | release-provenance.yml (manual) | yes, unverified | MEDIUM | two clean builds + hash diff |
| subwasm | Runtime WASM / metadata inspection | TEST_ORACLE | present | **INSTALLED_ONLY** | none | none | none | none | FAST | wire into release evidence |
| Foundry (forge/cast/anvil) | EVM dev, test, deterministic local EVM | TEST_ENGINE | 1.8.3 | **WIRED** | AtlasHTLC.sol + live lifecycle script | none on this commit | x3vm-evm-live-lifecycle.yml (manual) | none | FAST | run + capture lifecycle |
| Slither | Solidity static analysis | TEST_ORACLE | — | **PARTIAL** | slither.config.json (X3-contracts/evm + tests/security), crytic-export | last known install only, no scan output | none dedicated | none | MEDIUM | install (pipx), scan, justify exclusions |
| Echidna | Solidity property fuzzing | TEST_ENGINE | 2.3.3 | **PARTIAL** | config + EchidnaX3.sol invariants | none captured | referenced by economic gate | none | EXPENSIVE | fix config paths, prove a broken invariant is caught |
| Medusa | Solidity property fuzzing (independent) | TEST_ENGINE | 1.5.1 | **INSTALLED_ONLY** | tests/security/medusa.config.json | none | none | none | EXPENSIVE | compare coverage vs Echidna/Foundry |
| REVM | Independent EVM oracle for differential execution | TEST_ORACLE | — | **MISSING** | none | none | none | none | FAST | build the differential harness |
| LiteSVM / Mollusk | In-process SVM program testing | TEST_ENGINE | — | **MISSING** | config mention only | none | none | none | FAST | add one, not both |
| solana-test-validator | Live SVM lifecycle | TEST_ENGINE | 4.3.0 | **WIRED** | programs/svm + Anchor workspace | none on this commit | x3vm-svm-live-lifecycle.yml (manual) | none | MEDIUM | run + add restart/recovery scenario |
| tc/netem | Latency, loss, reorder, corruption | INFRASTRUCTURE | present | **WIRED** | rc5_chaos_harness.sh, adversarial tests | none captured | distributed-atomic-chaos.yml (manual) | none | MEDIUM | lab-only scenario generator |
| Toxiproxy | Application-level network faults | TEST_ENGINE | 2.12.0 | **PARTIAL** | tool-stack config (says 2.9.0) | none captured | distributed-atomic-chaos.yml (manual) | none | FAST | run one latency + one cutoff scenario |
| k6 | RPC/API/WS load | TEST_ENGINE | v2.3.0 | **PARTIAL** | benchmarks/k6/x3_rpc_load.js (+3 dupes) | none captured | none | none | MEDIUM | run, capture, dedupe |
| Criterion | Rust microbenchmarks | TEST_ENGINE | workspace dep | **WIRED** | 6 benches | none captured | benchmark-regression.yml (manual) | none | MEDIUM | run + commit baseline |
| Semgrep | Custom static rules | TEST_ORACLE | — | **PARTIAL** | 9 X3 rules (duplicated dir) | none captured | semgrep.yml (manual, `:latest`) | none | FAST | install, run rules, pin image |
| CodeQL | Semantic security analysis | TEST_ORACLE | 2.27.1 | **WIRED** | release-hardening.yml | no SARIF captured | codeql.yml (manual) | none | MEDIUM | run + record exact language coverage |
| OSV-Scanner / Trivy | Dependency + container vulns | TEST_ORACLE | 2.6.0 / 0.75.0 | **WIRED** | Dockerfiles as targets | none captured | osv-scan.yml, trivy.yml (manual) | none | FAST | run both, define the split |
| Syft → Grype | SBOM + vuln scan | TEST_ORACLE | 1.54.0 / 0.119.0 | **INSTALLED_ONLY** | none | none | release gate has an SBOM row | gate row | FAST | produce SBOM, scan it, wire the gate |
| cargo-audit / cargo-deny | Advisory + license policy | TEST_ORACLE | 0.22.2 / 0.20.2 | **PARTIAL** | deny.toml, check-dependency-audit.sh | one committed license report | local-ci gate 3, mainnet-readiness.yml | gate row | FAST | run + machine-readable allowlist |
| Gitleaks | Secret scanning | TEST_ORACLE | 8.30.1 | **INSTALLED_ONLY** | SECRET_MANAGEMENT_POLICY.md | none | mainnet-readiness.yml (manual) | none | FAST | add config, run, prove good/bad discrimination |
| TruffleHog | Secret scanning (independent) | TEST_ORACLE | — | **WIRED** | SECRET_MANAGEMENT_POLICY.md | none | mainnet-readiness.yml (manual) | none | FAST | run with redaction (or drop in favour of Gitleaks) |
| Prometheus / Grafana / Loki | Metrics + dashboards + logs | OBSERVABILITY | 3.15.0 / 13.2.3 / 3.7.8 | **PARTIAL** | 5 prometheus configs, 2 dashboards, router metrics.rs | none captured | none | none | MEDIUM | bring up one scrape, one panel, one log stream |
| Ansible | Reproducible validator lab | INFRASTRUCTURE | — | **MISSING** | none — infra is ad-hoc shell | none | none | none | MEDIUM | **7-validator prerequisite** |
| TLA+/Apalache | Formal protocol models | TEST_ORACLE | tla2tools.jar present | **PARTIAL** | formal-proofs/{tla,coq,k} | no model-check run captured | formal-verification.yml (manual) | none | EXPENSIVE | run TLC, capture a counterexample |
| Miri | Rust UB checks | TEST_ORACLE | — | **BLOCKED** | none | none | none | none | MEDIUM | needs a nightly toolchain first |
| REVM / Reth | Independent EVM execution oracle | TEST_ORACLE / REFERENCE | — | **MISSING / REFERENCE_ONLY** | none | none | none | none | FAST | build the differential harness |

**Status distribution (90 entries):** IMPLEMENTED 6 · WIRED 11 · PARTIAL 24 ·
INSTALLED_ONLY 8 · BLOCKED 4 · MISSING 29 · REFERENCE_ONLY 8.
**Host-installed:** 27 of 90 have a working binary or resolvable dependency.

---

## 3. WHAT CHANGED SINCE THE FIRST PASS

| Tool | Was | Now | Trigger |
|---|---|---|---|
| cargo-fuzz | BLOCKED (binary absent) | **WIRED** | installed 0.13.2; libfuzzer-sys 0.4.13 resolves |
| libfuzzer-sys | BLOCKED (unresolvable offline) | **WIRED** | resolves from crates.io |
| kani | MISSING | **INSTALLED_ONLY** | 0.68.0 installed; no harnesses |
| cargo-mutants | (binary gone) | **PARTIAL** | 27.1.0 back; evidence still x3-fees only |
| zombienet | (binary absent) | **PARTIAL** | 1.3.10 present |
| echidna | (binary absent) | **PARTIAL** | 2.3.3 present |
| medusa | MISSING | **INSTALLED_ONLY** | 1.5.1 present |
| solana-test-validator | (binary absent) | **WIRED** | 4.3.0 (Agave) present |
| toxiproxy | (binary absent) | **PARTIAL** | 2.12.0 present |
| k6 | (binary absent) | **PARTIAL** | v2.3.0 present |
| codeql / osv-scanner / trivy | (binary absent) | **WIRED** | 2.27.1 / 2.6.0 / 0.75.0 present |
| syft / grype | MISSING | **INSTALLED_ONLY** | 1.54.0 / 0.119.0 present |
| gitleaks | MISSING | **INSTALLED_ONLY** | 8.30.1 present |
| cargo-audit / cargo-deny | (binary absent) | **PARTIAL** | 0.22.2 / 0.20.2 present |
| prometheus / grafana / loki | (binary absent) | **PARTIAL / INSTALLED_ONLY** | 3.15.0 / 13.2.3 / 3.7.8 present |
| subwasm | MISSING | **INSTALLED_ONLY** | binary present |
| miri | INSTALLED_ONLY | **BLOCKED** | component unavailable for stable 1.90.0 |

No tool moved to IMPLEMENTED. Installing a binary is not an integration.

---

## 4. TOP 10 MISSING HIGH-VALUE INTEGRATIONS

Ranked by launch value per unit of work.

1. **REVM differential harness.** Zero references today; the only independent
   EVM execution oracle. The X3 EVM cannot be called verified without it.
2. **try-runtime replacement (frame-try-runtime based).** Blocks every runtime
   upgrade. Nothing else substitutes for pre/post migration verification.
3. **Kani harnesses** for claim-XOR-refund, no-replay, supply conservation,
   authorization, single settlement. Binary is now installed — only harnesses
   are missing.
4. **Zombienet execution** (captured 7-validator run). Highest-leverage item
   before the physical campaign; the topology file already exists.
5. **cargo-fuzz activation** — run at least the X3-language and bridge targets
   and persist corpora. 98 targets, 0 runs.
6. **Audit King normalization schema + first adapter (Slither).** Until one
   normalized format exists, "dedup" and "tool router" have nothing to operate on.
7. **Ansible for the validator lab.** Without it, the seven-validator campaign is
   not reproducible from version control.
8. **cargo-mutants re-run on the security crates** + survivor ingestion into
   Completion Intelligence.
9. **tools.lock.** Every release result is currently unreproducible against a
   fixed scanner set.
10. **Syft → Grype SBOM chain.** The release gate has an SBOM row with no
    producer.

---

## 5. TOP 10 BROKEN / PARTIAL INTEGRATIONS

1. **try-runtime** — manual workflow that intentionally fails; no producer for
   the release-gate row.
2. **Zombienet** — binary, topology and run script all exist; zero captured runs.
3. **cargo-fuzz** — 2 of 98 targets now run, evidenced, and defect-detection proven; 96 unbuilt, and the gate is opt-in only, not release-gated.
4. **Slither** — config committed, binary absent, 12 detectors excluded
   (including reentrancy and arbitrary-send-eth) with no written justification.
5. **Echidna** — config paths do not match the contract directory; the contract
   exists but has never been fuzzed.
6. **cargo-mutants** — evidence is from the wrong crate; 59 survivors unmapped.
7. **CodeQL / OSV / Trivy / Semgrep** — all `workflow_dispatch` with floating or
   absent tool pins and no committed SARIF.
8. **cargo-audit / cargo-deny** — policy exists mostly in comments; binaries only
   just installed; no committed report on this commit.
9. **srtool** — release hashes exist but were never compared against a node build.
10. **Kani** — named in the tool-stack config, invoked by no workflow and by no
    harness.

---

## 6. REDUNDANT TOOLS WE SHOULD NOT RUN (measure first, then cut)

- **wrk / wrk2 vs k6** — k6 covers HTTP/WS load; wrk only wins on micro-throughput
  and adds no transaction-level signal. Keep k6.
- **TruffleHog vs Gitleaks** — two secret scanners, one job. Pick Gitleaks
  (installed, configurable) unless TruffleHog demonstrates a unique catch on X3.
- **LiteSVM vs Mollusk** — pick one in-process SVM harness; both execute the same
  programs.
- **Medusa vs Echidna** — run both once on the same targets, compare unique
  findings, then keep only the one with unique coverage.
- **Mythril vs Slither vs Semgrep** — three static analyzers heavily overlapping
  on EVM; keep the two with the best signal-to-noise after a benchmark fixture.
- **ParityDB vs RocksDB** — benchmark only; do not migrate on a microbenchmark.
- **OpenTelemetry + Tempo** — do not stand up trace storage until a trace has
  actually resolved a bug.
- **Docker vs Podman** — choose one default; Podman only if a concrete need.

---

## 7. TOOLS BLOCKING 7-VALIDATOR READINESS

1. **Zombienet** — orchestration exists but no run is captured.
2. **Ansible** — the physical lab is ad-hoc shell; not reproducible.
3. **Four-validator mesh** — `four-validator-mesh.yml` is manual and has no
   captured run; the 7-validator tier needs the 4-validator tier proven first.
4. **Prometheus/Grafana** — no validator/finality dashboards producing data.
5. **netem / Toxiproxy** — fault injection is unwired, so partition/latency
   behaviour of a 7-validator set is unmeasured.
6. **Multi-validator EconomicHalt** — never triggered in a multi-node context.

---

## 8. TOOLS BLOCKING MAINNET READINESS

1. **try-runtime** — no pre/post migration verification for runtime upgrades.
2. **srtool reproducibility** — release hashes not reproduced against a build.
3. **cargo-audit / cargo-deny / OSV / Trivy** — supply-chain gate has no committed
   evidence on the release commit.
4. **Gitleaks** — no secret-scan evidence.
5. **Syft → Grype** — SBOM gate row has no producer.
6. **CodeQL / Semgrep** — static analysis is manual-only with no committed SARIF.
7. **tools.lock** — release results are not bound to a pinned scanner set.
8. **Evidence capture (§72)** — no single schema binds a tool run to
   commit/dirty-state/tool-version/seed as the release gate requires.

---

## 9. RECOMMENDED NEXT STEP

**Done for cargo-fuzz.** The binary and dependency are installed, the
X3-language targets build and run seeded campaigns, the corpus is captured, a
known-good/known-bad fixture proves libFuzzer detects defects, and the whole
thing is wired into the local CI of record as the opt-in `--fuzz` gate
(`scripts/local-ci.sh --fuzz`). The next family to integrate is **REVM** (the top
*missing* integration) or **Kani** (installed, harnesses missing) — see §4.
