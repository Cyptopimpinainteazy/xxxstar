# X3 Atomic Star — Deployment Runbook

**Status:** Draft — documentation only; not an authorization to deploy production/mainnet
**Owner:** Release/operator owner designated for the target environment
**Operator:** Named operator must be assigned before execution
**Last verified:** 2026-08-31
**Target environment:** RC-1 internal staged testnet (5–7 validators) unless an approved change record explicitly names another environment
**Expected duration:** 2–4 hours for a clean staging deployment, excluding image/build delays and verification drills
**Change identifier:** Record the release tag, commit SHA, and change/ticket ID before execution

> **Authoritative scope:** X3 is currently documented as a **v0.4 Internal Testnet Candidate / internal staged testnet**. This runbook does not authorize public testnet or mainnet launch. External bridges, parallel execution, appzone factory, PQ experimental features, advanced DEX routing, AI optimizer, and GPU-critical validator acceleration remain gated out of RC-1. See `LAUNCH_SCOPE.md`.

## Objective

Deploy a reproducible X3 RC-1 staged testnet using signed release binaries, systemd-managed validators/bootnodes, and containerized support services. Prove block production, finality, RPC health, cross-VM behavior, supply invariants, observability, backup/restore, and failure recovery before accepting the environment.

## Scope

### Included
- 5–7 validator staging topology.
- Bootnode(s) using systemd.
- Optional/public-RPC support node as approved by the change record.
- Explorer, indexer, faucet, PostgreSQL/Redis, and monitoring services using Docker/Kubernetes.
- Chain specification/genesis for the approved staging network.
- Release binary checksum verification.
- Session/node-key installation and service startup.
- Functional, consensus, observability, and recovery verification.

### Excluded
- Mainnet deployment.
- Public-value settlement.
- Enabling external EVM/SVM/Bitcoin bridges or relayers.
- Enabling `parallel-executor`, `appzone-factory`, `pq-experimental`, `advanced-dex`, `ai-optimizer`, or `gpu-acceleration`.
- Ad-hoc production changes outside the approved release and target hosts.

### Invariants
- External bridges remain disabled at genesis.
- Only the approved signed release artifact is installed.
- No seed phrase, private key, token, API key, or credential is written into Git, logs, tickets, or evidence.
- Validators are not run in Docker.
- No destructive database or chain-state operation occurs without an explicit approval gate and recoverable backup where possible.

## Preconditions

1. Approved change record exists with release tag, commit SHA, target environment, operator, and rollback owner.
2. A signed release artifact and checksum are published and independently verified.
3. Staging infrastructure exists and matches the approved topology.
4. Validator storage is mounted at the approved chain-data location and has sufficient free space.
5. Network/firewall rules are approved: P2P 30333; RPC/WebSocket only where required; outbound HTTPS for release retrieval.
6. Backups/snapshots exist for any pre-existing staging state that must be preserved.
7. Operators have access to the repository, hosts, systemd, Docker/Kubernetes, monitoring, and approved secret-management mechanism.
8. CI/release gates required by the launch scope have passed for the exact release being deployed.
9. The operator has read the current `LAUNCH_SCOPE.md`, deployment policy, staging setup guide, and failure/TODO ledger.
10. If any prerequisite is false or contradictory, **STOP**; do not improvise around it.

## Risk and stop conditions

### Immediate stop conditions
Stop deployment immediately if:
- The target host, release tag, chain spec, or commit differs from the approved change record.
- A checksum/signature cannot be verified.
- A secret/private key appears in command output, logs, Git, or evidence.
- Unexpected access to another user's data occurs.
- Chain data becomes corrupted or is unexpectedly deleted.
- Finality stalls beyond the approved tolerance or validators disagree on chain state.
- RPC reports contradictory chain identity/state.
- Monitoring cannot distinguish a healthy network from a failed one.
- Recovery requires an unapproved destructive operation.
- A supposedly disabled RC-1 feature becomes reachable or enabled.

Do not turn a failed verification into an improvised repair. Contain, preserve evidence, and obtain a new decision.

## Evidence plan

Record only sanitized evidence: UTC timestamps, host/node role, release tag, commit SHA, service status, block/finality observations, test IDs, checksum results, and incident/deviation IDs. Store evidence in the approved change record or release evidence location. Never record seed phrases, private keys, bearer tokens, passwords, full environment dumps, or sensitive customer data.

## Procedure

### Phase 0 — Freeze scope and capture baseline

**Action**
1. Record the approved release tag, commit SHA, chain-spec identifier/hash, target node count, and topology.
2. Confirm the environment is staging/internal and not a public-value network.
3. Capture baseline host disk, memory, CPU, service, and network health.

**Expected result**
The deployment target and exact artifact are unambiguous and baseline health is recorded.

**Verify**
- Approved identifiers match the deployment record.
- No unexpected nodes/services are in scope.
- Required capacity and connectivity are healthy.

**If verification fails**
STOP and correct the change record or infrastructure before continuing.

**Approval required:** Gate 2 approval for the named operator and rollback owner.

### Phase 1 — Verify release artifact

**Action**
On an approved staging host, retrieve the exact signed release binary and checksum using the repository release process. Do not copy credentials into shell history.

Example from the repository's staging guide, after replacing only approved release variables:

```bash
RELEASE_TAG="<APPROVED_RELEASE_TAG>"
REPO="Cyptopimpinainteazy/xxxstar"
wget "https://github.com/${REPO}/releases/download/${RELEASE_TAG}/x3-chain-node"
wget "https://github.com/${REPO}/releases/download/${RELEASE_TAG}/x3-chain-node.sha256"
sha256sum -c x3-chain-node.sha256
```

**Expected result**
The binary checksum matches the published checksum and the release/tag is the approved one.

**Verify**
- `sha256sum -c` succeeds.
- Release provenance/signature evidence matches the approved release.
- Binary version output matches the expected release.

**If verification fails**
STOP. Do not install the binary. Obtain a corrected release artifact.

### Phase 2 — Provision validator hosts

**Action**
1. Use Ubuntu 24.04 LTS or the platform explicitly approved for the staging environment.
2. Mount the approved NVMe/storage path for chain data.
3. Install the verified binary at `/usr/local/bin/x3-chain-node`.
4. Copy the approved `packaging/systemd/x3-validator.service`.
5. Run the repository hardening script.
6. Configure only approved P2P/RPC ports.

**Expected result**
Each validator host is hardened, has the correct binary, and has no unnecessary public services.

**Verify**
```bash
/usr/local/bin/x3-chain-node --version
sudo systemctl daemon-reload
sudo systemctl status x3-validator --no-pager
```
Confirm the service points to the approved binary, chain, data directory, and bootnode configuration.

**If verification fails**
Do not start consensus. Fix the host configuration or roll the host back to its pre-change state.

### Phase 3 — Configure keys and chain specification

**Action**
1. Generate validator/session keys using the approved isolated procedure.
2. Store sensitive material only in the approved offline/secret-management location.
3. Insert keys into the correct node keystore using the exact chain and key types required by the current runtime.
4. Generate or install the approved staging chain spec.
5. Independently compare the final raw chain-spec hash with the approved artifact.
6. Confirm `external_bridges_enabled = false` and all RC-1 gated features remain disabled.

**Expected result**
Every validator has the correct authority configuration for the same genesis and chain identity.

**Verify**
- Chain-spec hash matches the approved value.
- Validator identity list matches the approved topology.
- No secret material appears in files intended for Git or evidence.
- Feature gates match `LAUNCH_SCOPE.md`.

**If verification fails**
STOP. Do not start the validator set. Rebuild the configuration from approved inputs.

**Approval required:** Gate 2 approval immediately before starting consensus if genesis/configuration changed from the approved artifact.

### Phase 4 — Deploy bootnode and validators

**Action**
1. Start the bootnode service.
2. Verify its peer identity and P2P listener.
3. Start validators one at a time.
4. Enable the approved systemd units only after each node passes local health checks.
5. Keep the deployment inside the approved 5–7 validator scope.

**Expected result**
Validators discover peers, produce blocks, and converge on the same chain.

**Verify**
On each node:
```bash
sudo systemctl is-active x3-validator
sudo journalctl -u x3-validator -n 100 --no-pager
```
From an approved RPC endpoint, query best and finalized heads and peer count.

**If verification fails**
STOP adding nodes. Isolate the failing node, preserve logs, and determine whether the problem is binary, chain spec, keys, P2P, or storage. Do not alter multiple variables simultaneously.

### Phase 5 — Deploy support services

**Action**
On the approved support host, deploy only non-consensus services with Docker/Kubernetes according to the repository deployment policy.

```bash
docker compose -f docker/docker-compose.yml up -d
docker compose -f docker/docker-compose.yml ps
```

**Expected result**
Explorer, indexer, faucet, database/cache, and monitoring services are healthy and connected to the staging network.

**Verify**
- Compose/Kubernetes health checks pass.
- Indexer height follows the chain.
- Explorer shows current blocks.
- Prometheus scrapes all approved targets.
- Alerting is operational.

**If verification fails**
Keep validators running if consensus is healthy, but stop exposure of the unhealthy support service. Do not rebuild a database destructively without a backup and approval.

### Phase 6 — Consensus and functional verification

**Action**
Run the approved test suite and live-node verification.

Required checks:
1. Continuous block production.
2. GRANDPA finality progresses.
3. RPC transaction submission and query work.
4. Internal cross-VM transfers work for the approved native/EVM/SVM routes.
5. Supply-ledger invariant holds.
6. Replay protection and timeout/refund behavior pass their tests.
7. No externally gated bridge path is reachable.

**Verify**
Run the repository's CI-equivalent tests where appropriate:

```bash
cargo test -p pallet-x3-cross-vm-router -- --nocapture
cargo test -p pallet-x3-supply-ledger -- --nocapture
cargo test -p pallet-x3-settlement-engine -- --nocapture
cargo test --workspace
```

For a live RPC endpoint, verify best/finalized heads with the chain RPC methods and record sanitized results.

**If verification fails**
STOP promotion. Mark the affected verification as failed; do not claim deployment success.

### Phase 7 — Resilience and recovery drills

**Action**
Run only on the approved staging network:
1. Validator process crash/restart drill.
2. Controlled P2P partition and recovery drill.
3. Disk-space alert drill without risking host exhaustion.
4. Snapshot backup and restore drill.

**Expected result**
Failures are detected, services recover through the documented mechanism, and consensus/finality returns to healthy operation.

**Verify**
- systemd restarts only the intended service.
- Finality recovers after a controlled partition.
- Alerts fire and clear as expected.
- Restored node rejoins without corrupting the network.

**If verification fails**
STOP. Preserve logs and snapshots. Do not repeat a destructive drill until the recovery path is corrected and approved.

### Phase 8 — Final acceptance gate

**Action**
Review every acceptance item against evidence.

**Expected result**
All required gates are PASS, no stop condition is active, and all deviations have an owner and disposition.

**Verify**
- Exact release artifact deployed.
- Required validator count healthy.
- Block production and finality healthy.
- Cross-VM and invariant tests pass.
- Monitoring/indexer/explorer healthy.
- Restore drill passes.
- Disabled features remain disabled.
- No unresolved critical failure/TODO blocks the stated environment.

**Approval required:** Explicit release/change owner acceptance before declaring the staging deployment complete.

## Rollback

### Rollback triggers
Rollback when the approved release cannot achieve healthy consensus/finality, the deployed artifact/configuration differs from the approved scope, a critical security issue appears, chain state is corrupted, or required recovery cannot be completed safely.

### Rollback decision owner
The named change/release owner decides whether to roll back. If the owner is unavailable and a mandatory stop condition is active, stop the rollout and contain the affected node(s); do not invent a destructive recovery.

### Standard rollback — binary/configuration
1. Stop the affected validator service.
2. Preserve sanitized logs and the exact deployed binary/checksum.
3. Restore the previously approved binary and service configuration.
4. Restore the approved chain-spec/configuration if it was changed.
5. Start the validator and verify chain identity before allowing it back into service.
6. Confirm peer connectivity, block production, and finality.

### Standard rollback — support services
1. Stop only the affected support service.
2. Preserve logs and database metadata.
3. Restore the last known-good image/configuration.
4. Restore database state from the approved backup when necessary.
5. Verify indexer height and application health.

### Chain-state restore
Use snapshot restore only when the approved recovery procedure calls for it. Before deleting or replacing chain state, obtain Gate 3 approval, verify the target data directory, and confirm the backup/snapshot is readable. Never use a broad recursive delete against an ambiguous path.

**Rollback verification:**
- Chain identity and genesis remain correct.
- Best/finalized heads advance.
- Validator rejoins the intended peer set.
- No duplicate/foreign chain state is introduced.
- Support services resume from the expected height.

**Rollback limit:** If rollback would require an unapproved genesis replacement, destructive migration, credential rotation, or other irreversible action, STOP and escalate for a new approved recovery plan.

## Completion criteria

Deployment is complete only when all are true:

- The exact approved release is installed on all intended nodes.
- 5–7 staging validators are healthy according to the approved topology.
- Block production and finality are continuously progressing.
- RPC and peer health are verified.
- Required internal cross-VM tests pass.
- Supply and settlement invariants pass.
- Explorer/indexer/monitoring services are healthy.
- Backup/restore and at least the required failure drills pass.
- All RC-1 gated features remain disabled.
- No critical stop condition or unresolved release-blocking failure remains.
- Evidence and deviations are recorded in the change record.

## Communications

- **Start:** Notify the approved project/release channel with release tag, environment, operator, and change ID.
- **Failure:** Notify the same channel immediately for any stop condition; include only sanitized evidence and the affected phase.
- **Rollback:** Announce the rollback decision, scope, and current service state before destructive recovery actions.
- **Completion:** Publish the acceptance result, release identifier, validator count, verification summary, and known deviations.

## Record

Record:
- UTC start/end timestamps.
- Operator and approving owner.
- Release tag and commit SHA.
- Chain-spec identifier/hash.
- Target topology.
- Verification results and evidence references.
- Stop conditions encountered.
- Rollback actions, if any.
- Deviations and follow-up issues.
- Next verification/rehearsal date.

## Authoritative repository references

- `LAUNCH_SCOPE.md` — authoritative launch scope.
- `CURRENT_MAINNET_STATUS.md` — subsystem scoreboard, subject to the authoritative scope above.
- `docs/X3_DEPLOYMENT_POLICY.md` — validator/systemd and support-service deployment policy.
- `docs/STAGING_TESTNET_SETUP.md` — staging topology and setup procedure.
- `docs/current/FAILURES_AND_TODOS.md` — failure/TODO ledger.
- `MAINNET_LAUNCH_CHECKLIST.md` — release gate tracker.
