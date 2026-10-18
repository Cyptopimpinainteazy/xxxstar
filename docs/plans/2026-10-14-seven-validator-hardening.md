# X3 Seven-Validator Testnet Implementation Plan

> **For agentic workers:** Use the host's available task-by-task implementation workflow. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Certify seven independent physical X3 validators through reproducible negative tests, safety and liveness evidence, deterministic state execution, and auditable release artifacts.

**Architecture:** Keep fast source-level checks in local CI and drive long-running physical tests from `x3star1`, using SSH to access loopback-only validator RPC. Distinguish local multi-process tests from seven independent Linux machines. Pin all evidence to the exact git commit, binary digest, chain-spec digest, runtime version, and fault scenario. Missing evidence or an unhealthy node must yield a nonzero exit and `FAIL`, not an optimistic `SKIP` or `PASS`.

**Tech Stack:** Python 3 standard library, Bash, Rust/Cargo, FRAME/Substrate, Aura + GRANDPA, Ansible/SSH, network fault injection (`tc`/`nftables` on isolated lab), GitHub Actions/local CI, Zombienet, x3-sim, EVM/SVM/X3VM lifecycle tests.

## Global Constraints

- Never erase validator chain data, alter firewall rules, kill services, or inject network faults during read-only preflight.
- Never copy validator seeds/private keys to CI logs or JSON artifacts. The seven physical authorities must have unique keys and peer identities.
- Do not use `--alice`/`--bob` or embedded dev seeds in a public-eligible chain spec.
- Never use `--rpc-external --rpc-methods Unsafe` as a shortcut to make test RPC accessible. Use authenticated SSH to loopback RPC; explicitly review RPC access controls separately.
- Distinguish a local seven-process demonstration, live seven-machine evidence, and cryptographically verified finality. None implies another automatically.
- Before destructive drills, obtain host-specific targets, maintenance authorization, persistent evidence destination, backups, and rollback/rejoin procedures.
- Reject missing metrics, uncollected tests, malformed reports, stale commit hashes, process exit, and unconverged nodes; never mark skipped safety gates green.
- Keep experimental PoH/Flash Finality, GPU signing, and 200-ms blocks behind independently tested feature flags until production consensus semantics are verified.

---

## Existing foundations and gaps (master observed 2026-10-14)

- `scripts/testnet/run-7-validators-local.sh`: local validator startup and restart.
- `scripts/testnet/build-x3-testnet-spec.py`: generated testnet authority spec.
- `scripts/testnet/validator-failure-drill.sh`: local threshold kill/recovery drill.
- `scripts/testnet/consensus-soak.sh`: local soak, stalls, peer counts, RSS and optional load.
- `scripts/testnet/public-testnet-gate-drill.sh`, `scripts/mainnet/public_testnet_gate.sh`: release/readiness checks.
- `.github/workflows/zombienet-integration.yml`: a **four**-validator smoke, not physical seven-host certification.
- `tests/test_validator_launcher.py`, `tests/test_rc_gates.py`, and `tests/zombienet/`: existing test coverage.
- `scripts/testnet/status-7-validators.sh`: monitor on master still requires the proposed changes in PR #592 for divergent finality handling; coordinate to avoid conflicting edits.
- `docs/design/x3-monte-carlo-lab.md`: coverage proposal, not evidence that its campaigns ran.
- There is not yet a verified seven-machine topology, signed finality proof gate, nor a complete 72-hour hardware soak report in the inspected evidence.

## Test catalog and release criteria

| Area | Tests to implement / run | Mandatory PASS evidence |
|---|---|---|
| Supply-chain / build | Exact-head reproducible release build, locked toolchains, SHA256 artifacts, SBOM, vulnerabilities and Rust advisories, signed release provenance | Rebuilt binary/spec digests, compile/test logs, no unresolved critical issues |
| Hardware bootstrap | Seven SSH-known hosts, seven machine IDs, distinct libp2p keys, matching binary/spec/genesis/runtime, disk and NTP checks, firewall validation | `physical-seven-preflight` JSON with seven non-loopback machines and no errors |
| Genesis and authority keys | Exactly seven distinct Aura/GRANDPA authorities, no dev seeds or duplicate session keys, same chain ID, well-formed endowments, chain-spec loader acceptance | Validated spec, public-key attestations, no private keys in artifacts |
| Cold start / networking | Cold start permutations, full 7-node convergence, peer discovery, bootnode loss, churn, high latency, jitter, packet loss, asymmetrical partitions | Per-node peer IDs and finality traces, recovered connectivity |
| Consensus safety | 5/2 and 4/3 partitions, two nodes killed (5 remain), three killed (4 remain), delayed/reordered messages, stale proposals, equivocation | Never two conflicting finalized hashes at same height; 5 may finalize, 4 must not finalize alone |
| Consensus liveness | Single/dual validator restart, long offline catch-up, bootnode outage, authority rotation, signing-key restart, full rejoin | Finalization resumes within measured SLO; all nodes agree after healing |
| Transaction/mempool | Sequential nonces, 96+ concurrent senders, duplicate nonce, replay, tx ordering, fee priority, malformed signatures, expired tx, spam and oversized payloads | No invalid included tx, exact balances, finalized TPS and rejection attribution |
| State/invariants | Deterministic execution and replay from genesis, state root equality across validators, resource exhaustion, supply ledger conservation | Identical state roots; no mint without authorization or negative asset balances |
| Dual VM / atomic | Native X3VM, EVM, SVM, X3↔EVM and X3↔SVM live HTLCs, timeout, refund, duplicate callback, wrong domain, replay, reordered claims | `claim XOR refund`, no double settlement, no partial unauthorized result; on-chain receipts/proofs |
| Storage/recovery | Power-loss equivalents (`SIGKILL`), dirty DB restart, journal replay, checksum/corruption rejection, snapshot restore/state sync, full disk and inode exhaustion, growth/compaction | State roots and balances preserved, corruption rejected, downtime and RTO measured |
| Runtime upgrades | Old-node/new-node mixed rollout, storage migration, interrupt/restart at activation, rollback if safe, validator membership changes | Same post-upgrade state root and conservation invariants; never partial migration |
| RPC/explorer | RPC concurrency/timeouts/bounds, request authentication and rate limiting, finalized proof endpoints, indexer reorg and restart, malicious JSON | No panic/leak, correct finalized results, indexer matches canonical chain |
| Security / adversarial | Cargo fuzz/coverage/mutants, property-based tests, dependency and secret scanners, malformed P2P, peer eclipse/Sybil simulations, resource limits | No critical unresolved finding; reproducible fuzz corpus; red tests for known vulnerabilities |
| Performance | Fixed + randomized EVM/SVM/X3VM load, TPS offered/accepted/included/finalized, p50/p95/p99 latency, CPU/RSS/IO/network | Acceptance thresholds set from seven-machine baseline; no silent throughput collapse or unbounded queues |
| Long soak | 24h baseline, 72h loaded seven-host finalization, fault injection during load and after healing | Exact-commit time series; 0 safety violations, 0 unexplained stalls, 0 unbounded leaks |
| Operations | Telemetry scraping, alert delivery, backup/restore, log retention, disk alert, validator rotation, upgrade/rollback runbook | Real alert receipts, archived manifests and operator sign-off |

**Safety gate recommendation:** For 7 equal-weight GRANDPA voters, >2/3 means 5 signatures are needed. Five live validators should allow finality; only four remaining must not produce new valid GRANDPA justifications. The network must never finalize competing hashes, including during asymmetric partitions or rejoin. A timer-based `stalled` result is not by itself cryptographic proof: inspect the justifications and validator set.

**Performance gate recommendation:** Do not claim 2,000+ finalized TPS from a single validator or simulated load. First measure a real sustained seven-host baseline, then set documented floor/ceiling values; measure p99 finality and queue drain, not only accepted submissions.

## Task 1: Independent physical-topology preflight (first implementation batch)

**Files:**
- Create: `scripts/testnet/physical-seven-preflight.py` (standalone read-only SSH/RPC gate).
- Create: `tests/test_physical_seven_preflight.py` (behavioral regression tests without requiring a live cluster).
- Create: `docs/testnet-config/physical-seven-inventory.example.json` (safe non-routable example).

**Interfaces:**
- Consumes: `--inventory JSON`, `--commit 40HEX`, `--out JSON`, optional expected binary/spec/genesis SHA and safety thresholds; per-node `name`, `ssh_host`, `rpc_port`, `binary_path`, `chain_spec_path`, `data_dir`.
- Produces: JSON `{schema, commit, timestamp_utc, status, snapshots, shared_finalized_height, canonical_hashes_at_shared_height, errors, limits}`; exit 0 only on complete PASS.

- [x] **Step 1: Write focused failing tests:** 19 initial cases for count/unique hosts, path/option injection, missing or malformed evidence, binary/genesis/runtime mismatch, duplicate machine/peer identity, lag/finality problems, low disk, skew, fork, expected digests, and failure artifact.
- [x] **Step 2: Observe red:** Unit tests failed before implementation (inventory first; assessor and SSH/CLI after).
- [x] **Step 3: Implement gate:** Use strict known-host SSH, loopback RPC from inside each host, validate seven unique hardware identities and current state; compare every node's canonical hash at the *minimum* finalized height. Never log secret keys.
- [x] **Step 4: Focused green:** `python3 -m unittest discover -s tests -p test_physical_seven_preflight.py -v` → 23 tests passing locally.
- [ ] **Step 5: Exact-commit live hardware run:** Use actual SSH fingerprints and real inventory; archive reports by commit. Do not call PASS before physical evidence exists.
- [ ] **Step 6: Review/merge the isolated implementation PR** after required exact-head checks.

## Task 2: Signed-finality safety and negative chaos tests

**Files:**
- Extend existing `scripts/testnet/validator-failure-drill.sh` in a separate PR after tests protect the current behavior.
- Proposed: `scripts/testnet/physical-seven-chaos.py`, `tests/test_physical_seven_chaos.py` and a scenario JSON under `tests/chaos/`.

**Interfaces:**
- Consumes: same physical inventory, authorized test campaign, ordered fault injection scenarios, exact commit and expected GRANDPA authorities.
- Produces: per-scenario finalized headers/justifications, sampled signatures, fault timeline, `PASS/FAIL`, recovered tip and node rejoin times.

- [ ] **Step 1:** Red tests: two killed → finality keeps moving; three killed → no valid 4-voter justification; 5/2 partition → only majority can finalize; 4/3 → neither side has quorum; healing → a single chain.
- [ ] **Step 2:** Assert a script cannot target a host absent from the authorized inventory, cannot kill outside lab unit names, and cannot run when maintenance mode is false.
- [ ] **Step 3:** Implement opt-in stop/start and `tc netem` adapters with dry-run, rollback traps, root privilege checks, and immutable fault log.
- [ ] **Step 4:** Run deterministic local tests, then one seven-host rehearsal, then randomized fault campaigns; preserve reproducer seeds and raw GRANDPA proof data.

## Task 3: State, recovery, cross-VM and throughput campaigns

**Files:**
- Use and extend `scripts/testnet/consensus-soak.sh`, `scripts/testnet/run-multiprocess-load.py`, `scripts/testnet/cross-vm-harness.py`, `scripts/testnet/runtime_upgrade_rehearsal.sh`.
- Proposed: `tests/e2e/physical-seven-recovery.py`, `tests/invariants/seven-host-ledger.py` and replayable scenario manifests.

**Interfaces:**
- Consumes: tested binary/spec + validator inventory, uniquely funded test accounts, campaign seed and resource limits.
- Produces: canonical receipts, state-root diffs, balance conservation, accepted vs finalized throughput, p99 inclusion/finality latency, replay artifacts.

- [ ] **Step 1:** Fault-based red tests for disk-full, restart during snapshot, interrupted migration, tx nonce storms and wrong-domain cross-VM proofs.
- [ ] **Step 2:** Add live assertions (not mock-only) of exact account deltas, per-VM gas, finality roots, uniqueness of settlement, timeouts and queue recovery.
- [ ] **Step 3:** Run short local reproductions; then 24-hour baseline and 72-hour loaded seven-host campaign with recorded failure thresholds.

## Task 4: Continuous release gate and signing of evidence

**Files:**
- Reuse `scripts/testnet/testnet_rc_gate.sh`, `.github/workflows/mainnet-readiness.yml`, `.github/workflows/zombienet-integration.yml` and `scripts/local-ci.sh`; integrate only after active PRs touching those files are reconciled.
- Proposed: `scripts/testnet/seven-validator-verdict.py` and `tests/test_seven_validator_verdict.py`.

**Interfaces:**
- Consumes: a manifest of signed/hashed scenario evidence all pinned to one immutable commit and one expected binary, chain spec, runtime and genesis.
- Produces: `PASS/FAIL` release record with zero-exception required gates, machine identities, result hashes, timestamps and human approval metadata.

- [ ] **Step 1:** Negative verdict tests: missing file, fabricated green string, mismatched commit, stale report, unsafe `SKIP`, no raw proof, or failed subgate must fail entire release.
- [ ] **Step 2:** Execute source tests in local PR CI; run bounded four/seven-process virtual tests on self-hosted CI; gate physical deployment on seven-node checks and explicitly authorized chaos campaign.
- [ ] **Step 3:** Verify every artifact hash, collect external operator review, and produce one final gate with no status inflation.

## Execution from x3star1 when seven machines are reachable

1. Copy `docs/testnet-config/physical-seven-inventory.example.json` outside the repo and replace example targets, paths, and ports with verified data. Install each machine's known SSH host key using out-of-band fingerprints; this test will **not** auto-trust them.
2. Make sure every validator has a **local-only** JSON-RPC listener, unique libp2p identity, generated authority keys and a matching complete seven-validator chain spec.
3. Run source-level tests: `python3 -m unittest discover -s tests -p 'test_physical_seven_preflight.py' -v`.
4. Run the independent read-only physical gate (after the testnet is live):

```bash
COMMIT="$(git rev-parse HEAD)"
python3 scripts/testnet/physical-seven-preflight.py \
  --inventory /secure/path/physical-seven-inventory.json \
  --commit "$COMMIT" \
  --out "audit-artifacts/physical-seven/$COMMIT/preflight.json" \
  --min-peers 3 --min-free-gib 5 --max-clock-skew-sec 3
```

To prove you are running the *expected release* and not merely seven identical copies, supply `--expected-binary-sha256`, `--expected-spec-sha256` and `--expected-genesis` from trusted release manifests.

### Trust boundary and limitations

The first read-only gate relies on SSH host-key trust and the RPC responses from those machines. Its machine-ID check detects many accidental duplicate targets but cannot attest actual hardware, signer custody, independent operators, or prove the binary corresponds to the git commit. It does **not** validate GRANDPA vote signatures, exact active authority weights, malicious validator behavior, or actual finality progression over a time window. Those require Task 2 evidence and a second monitor sample. A successful preflight is only **ready to test**, never mainnet-certified.

### Decisions to approve before destructive testing

- Choose the seven actual hosts and agree whether all are independent physical machines or VMs on shared failure domains. This plan assumes independent machines.
- Decide the minimum disk reserve and sustained TPS/latency SLA after recording a seven-machine baseline. Defaults in preflight are conservative *recommendations*, not verified resource sizing.
- Approve a maintenance window and target service names for process kills, network partitions, and storage fault injection.
- Decide who independently verifies SSH host fingerprints, validator public keys, the release binary/build manifest and final launch verdict.
