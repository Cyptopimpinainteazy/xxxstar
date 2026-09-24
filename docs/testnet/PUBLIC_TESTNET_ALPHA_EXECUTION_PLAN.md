# X3 Public Testnet Alpha Execution Plan

**Status:** execution plan  
**Target:** move X3 from internal/loopback proof to a publicly reachable, geographically distributed Alpha testnet  
**Authoritative scope:** `LAUNCH_SCOPE.md`  
**Existing gates reused:** RC5, RC6, public testnet gate, ceremony tooling, snapshot/restore, validator rotation, RPC policy

## Non-negotiable launch rule

Public Testnet Alpha does **not** launch because RC6 packaging is green.

Launch requires all of the following:

1. **X3Lang production cutover passes:** the Rust `x3-lang/compiler` + canonical X3Lang VM semantics are the single production `.x3` path and source-to-finality execution is proven on the validator network. See `docs/x3-lang/PRODUCTION_CUTOVER_GATE.md`.
2. RC5 72-hour internal alpha passes on a real multi-node environment.
3. RC6 package/readiness gate passes.
4. Public bootnodes are deployed and reachable.
5. At least 7 validators are online, with no single-rack/single-provider topology masquerading as decentralization.
6. Public RPC/WebSocket, faucet, explorer/indexer and monitoring are deployed separately from validator consensus hosts.
7. A signed ceremony manifest is published and verifies against the live network.
8. Live validator key rotation succeeds without loss of finality.
9. Validator restart, validator replacement, snapshot restore and minority-failure recovery drills pass.
10. Runtime-upgrade and rolling-node-upgrade rehearsals pass.
11. No conflicting finalized heads, unexplained supply drift or unrecoverable atomic-settlement state is observed.

## Target topology

### Credible Alpha topology

A single home rack is one physical failure domain even if it contains many servers. Use the home lab for strong local proof and some Alpha nodes, but spread the public validator set across remote hosts/providers.

Recommended Alpha authority set:

- 3 validators in the physical X3 lab
- 4 remote validators across at least 3 external regions and at least 2 providers/operators
- 2 public bootnodes in different regions/providers
- 2 public RPC nodes behind one load balancer/rate limiter
- 1 archive/indexer/explorer host
- 1 monitoring/alerting host
- 1 build/release runner kept off the validator hot path

### Local hardware assignment

| Host | Alpha role | Notes |
|---|---|---|
| Lenovo x3550 M5 #1 | Validator V1 | Primary local authority; newer server platform |
| Lenovo x3550 M5 #2 | Validator V2 | Primary local authority |
| HP DL380 G7 192 GB #1 | Validator V3 | Third local authority |
| HP DL380 G7 192 GB #2 | RPC node A / failover validator lab | Do not expose validator RPC if used as authority |
| HP DL380 G7 192 GB #3 | RPC node B / testnet load node | Public RPC behind gateway only |
| HP DL360 G7 48 GB | Bootnode / network test host | No authority signing keys |
| Dell R710 | Monitoring / Prometheus / Grafana / Loki | Separate observability failure domain inside lab |
| HP DL380p Gen8 | Recovery / snapshot / fresh-join drill node | Repeated destructive restore/state-sync target |
| Intel desktop, ~30 TB SATA | Archive + indexer + explorer DB | Capacity-oriented; benchmark IOPS before making archive promises |
| Threadripper workstation + GPUs | Build/release, benchmarking, GPU validation, load generation | Not a public validator dependency |
| Apple Xserve | Spare observer / external-health checker / log mirror | Do not make it consensus-critical |

The local rack still counts as one site. Public Alpha needs remote authorities even if local testing uses all 7 physical machines.

## Network layout

```text
Internet
   |
   +--> bootnode-a --------------------+
   |                                   |
   +--> bootnode-b --------------------+---- P2P validators
   |
   +--> TLS/LB/WAF/rate limiter
           |
           +--> RPC-A
           +--> RPC-B
                   |
                   +--> indexer/archive
                   +--> explorer
                   +--> faucet

Validators expose:
  TCP/30333 P2P as required
  admin/RPC/metrics only on management network or localhost
```

Public RPC must never be the same endpoint/process exposure used for validator administration.

---

# Dependency graph

```text
P0-00 X3Lang production cutover
  -> P0-01 release freeze / exact commit
  -> P0-02 RC5 72h
  -> P0-03 RC6 package
  -> P0-04 public bootnodes + DNS
  -> P0-05 chain spec + authority/gateway/treasury review
  -> P0-06 validator deployment
  -> P0-07 live ceremony
  -> P0-08 WAN consensus gate
  -> P0-09 key rotation
  -> P0-10 recovery drills
  -> P0-11 runtime + binary upgrade drills
  -> P0-12 public RPC isolation attack gate

P1 support plane may proceed after P0-04:
  RPC/LB -> faucet -> indexer/archive -> explorer -> monitoring

P1 cross-chain proof may proceed after P0-07:
  public EVM test path
  public SVM test path

P2:
  outside validator onboarding
  bug bounty
  extended soak
  performance tuning
```

---

# P0 — Launch blockers

## P0-00 — X3Lang production cutover

X3Lang is a **pre-launch product requirement**, not a later feature.

Public Alpha is blocked until `docs/x3-lang/PRODUCTION_CUTOVER_GATE.md` passes. The production target is the Rust `x3-lang/compiler` + canonical X3Lang VM semantic contract, connected through the root runtime integration boundary and proven all the way to GRANDPA-finalized execution evidence.

Current alternate compiler/interpreter paths may remain only as compatibility/test adapters if they cannot redefine production language semantics and their equivalence is proven.

### Hard PASS

- canonical Rust compiler authority is unambiguous
- canonical VM semantics are unambiguous
- production runtime consumes canonical artifacts
- no fixture/dry-run host in the launch proof
- differential VM conformance passes
- one X3-native `.x3` program reaches actual runtime state and GRANDPA finality
- one Trading Core `.x3` program reaches actual runtime state and GRANDPA finality
- finalized receipt binds source, bytecode, execution, state roots, inclusion and finality
- mutation/replay tests fail closed

If this gate is red, RC5/RC6 may continue as engineering rehearsals, but **Public Testnet Alpha remains NO-GO**.

---
## P0-01 — Freeze an exact Alpha candidate

### Goal

Every host runs the same reviewed source, runtime and binary.

### Commands

```bash
git fetch origin
git checkout master
git pull --ff-only
git rev-parse HEAD
git status --short

cargo build --release -p x3-chain-node
sha256sum target/release/x3-chain-node
```

Run the existing release/provenance gates as applicable.

### Evidence

```text
audit-artifacts/public-testnet-alpha/<commit>/release/
  commit.txt
  binary.sha256
  runtime-wasm.sha256
  toolchain.txt
  cargo-lock.sha256
  release-manifest.json
```

### PASS

- clean tree
- exact commit recorded
- binary hash recorded
- runtime WASM hash recorded
- no host uses an unpinned replacement binary

---

## P0-02 — RC5 72-hour internal alpha

### Command

```bash
bash scripts/mainnet/rc5_internal_alpha_72h.sh
```

Use the existing status mode during the run:

```bash
bash scripts/mainnet/rc5_internal_alpha_72h.sh --status
```

### During the 72 hours

Inject:

- validator restart
- one validator offline
- RPC outage
- GPU unavailable/reset on GPU-enabled hosts
- disk-pressure warning
- transaction load
- atomic lock/claim/refund traffic

### PASS

- zero conflicting finalized heads
- finality resumes after minority failure
- no unexplained invariant halt
- no unrecoverable settlement
- evidence bundle complete

---

## P0-03 — RC6 public-testnet package

### Command

```bash
bash scripts/mainnet/rc6_public_testnet_readiness.sh
```

### Required result

```text
RC6_PACKAGE_READY: PASS
```

A pending bootnode is acceptable for packaging only. It is **not** acceptable for Alpha launch.

### Verify generated artifacts

```bash
sha256sum chain-specs/x3-public-testnet-plain.json
sha256sum chain-specs/x3-public-testnet-raw.json
grep -n '"bootNodes"' chain-specs/x3-public-testnet-plain.json
```

### PASS

- node release build passes
- runtime WASM build passes
- public-safe chain spec generated
- dev-key scan passes
- external bridges remain disabled unless a separately approved testnet bridge gate is being exercised
- release artifact hashes recorded

---

## P0-04 — Deploy real public bootnodes

Use `scripts/testnet/public-node-id.sh` to create stable public peer identities.

### Example

```bash
NODE_BIN=target/release/x3-chain-node \
  scripts/testnet/public-node-id.sh \
    --host bootnode-a.testnet.x3-chain.io \
    --p2p-port 30333 \
    --key-file deployment/keys/bootnode-a.nodekey
```

Repeat for bootnode B on another region/provider.

### Firewall

Bootnodes:

- allow TCP/30333 from Internet
- admin SSH only from management/VPN addresses
- RPC bound to localhost or management network
- no validator signing keys

### Acceptance

From at least two external networks:

```bash
nc -vz bootnode-a.testnet.x3-chain.io 30333
nc -vz bootnode-b.testnet.x3-chain.io 30333
```

Verify each reported local peer ID matches the published `/p2p/<peer-id>`.

### PASS

- DNS resolves
- both bootnodes reachable
- peer IDs stable
- bootnodes are in generated spec
- at least one bootnode can disappear without isolating the network

---

## P0-05 — Build and review the final Alpha chain spec

### Inputs that must be explicit

- authority public keys
- council accounts
- treasury accounts/signers
- X3 atomic gateway account
- settlement gateway account
- EVM escrow address
- SVM escrow address
- public bootnodes
- chain ID/network ID
- any BTC checkpoint if BTC proof testing is enabled

Never use published development phrases.

### Build

```bash
PUBLIC_BOOTNODES='/dns4/bootnode-a.testnet.x3-chain.io/tcp/30333/p2p/<peer>,/dns4/bootnode-b.testnet.x3-chain.io/tcp/30333/p2p/<peer>' \
X3_NODE_BIN=target/release/x3-chain-node \
python3 scripts/testnet/build-x3-testnet-spec.py 7
```

Set the required production-safe account environment variables before running.

### Independent review

At least two people/operators verify:

- authority set
- issuance
- treasury
- governance
- gateways
- bootnodes
- runtime WASM
- chain ID
- disabled features

### PASS

The reviewed spec hash is the exact spec installed on every validator.

---

## P0-06 — Deploy seven validators

Use the maintained installer/systemd path rather than hand-written services.

### Preflight

```bash
sudo bash scripts/install-validator.sh \
  --binary ./target/release/x3-chain-node \
  --sha256 <PINNED_SHA256> \
  --chain ./x3-public-testnet-plain.json \
  --check
```

Then install without `--check`.

Apply the repository's validator hardening script/runbook before enabling the service.

### Network rule

Validators may expose P2P. Public RPC/admin/key methods must not be exposed to the Internet.

### PASS

For all seven:

- same genesis hash
- same runtime version
- same finalized head
- required peer count present
- authority keys recognized
- no dev seed dependency
- no unsafe public RPC

---

## P0-07 — Record and publish the ceremony

### Record

```bash
python3 scripts/testnet/testnet-ceremony.py record <spec> \
  --node-bin x3-chain-node \
  --rpc <management-rpc-list> \
  --out ceremony.json
```

### Verify

```bash
python3 scripts/testnet/testnet-ceremony.py verify ceremony.json \
  --rpc <management-rpc-list> \
  --node-bin x3-chain-node \
  --min-finalized 100
```

Sign and publish:

- ceremony.json
- ceremony hash
- chain-spec hash
- binary hash
- runtime WASM hash
- bootnode multiaddrs

### PASS

A third party can verify that the live chain matches the published manifest.

---

## P0-08 — WAN consensus proof

Run sustained traffic while collecting per-validator:

- best block
- finalized block
- peer count
- Aura misses
- GRANDPA lag
- block interval
- transaction inclusion/finality latency

### Failure matrix

1. kill 1 validator
2. restart it
3. kill 2 validators sequentially
4. 4|3 partition drill in a controlled environment
5. 2–5% packet loss
6. 50–150 ms added latency profiles
7. jitter and bandwidth limits

Use `tc netem` for controlled network impairment where real geography cannot reproduce a case deterministically.

### Hard PASS

- no conflicting finalized heads
- minority cannot finalize a competing chain
- finality resumes after recovery
- no finalized reorg
- every submitted test transaction reconciles to finalized/rejected state

---

## P0-09 — Live validator key rotation

Current code is not enough; execute the proof.

### Drill

On a running 3–4 validator subset first:

1. schedule rotation in `pallet_x3_custody`
2. execute the node `validator rotate` operator command
3. confirm signed `session.setKeys` inclusion
4. wait for activation boundary
5. confirm new keys author/vote
6. confirm old keys no longer do
7. repeat sequentially

### PASS

- finality never stalls beyond the declared threshold
- no equivocation
- next due rotation advances correctly
- on-chain registry and active session keys agree

Evidence:

```text
rotation-before.json
rotation-extrinsic.json
rotation-after.json
finality-series.csv
```

---

## P0-10 — Destructive recovery

Use the DL380p Gen8 recovery host or equivalent disposable node.

### Snapshot

```bash
bash scripts/snapshot-restore.sh backup /var/lib/x3
bash scripts/snapshot-restore.sh list
```

### Destructive test

- stop node
- archive evidence
- delete/corrupt node DB
- restore snapshot
- restart
- catch up to current finalized head

### PASS

- integrity verification succeeds
- node rejoins the same chain
- finality/head match peers
- no signing occurs from stale state before synchronization
- measured RTO recorded

Repeat once with a completely fresh base path and peer sync.

---

## P0-11 — Runtime and rolling binary upgrade

### Runtime upgrade rehearsal

Use the existing try-runtime/release-candidate rehearsal workflow and a staging/WAN chain snapshot.

Require:

- pre-upgrade checks
- migrations
- post-upgrade invariant checks
- supply equality
- settlement continuity
- runtime version update
- no finality loss

### Binary rolling upgrade

Upgrade exactly one authority at a time:

```text
V1 stop -> replace verified binary -> start -> catch up -> verify
V2 ...
```

Never remove quorum intentionally during an ordinary rolling upgrade.

### PASS

- mixed old/new node binaries interoperate for the supported transition window
- finality remains healthy
- all seven end on the intended binary/runtime hashes

---

## P0-12 — Public RPC cannot hurt consensus

Public Alpha uses **two RPC nodes behind a load balancer/rate limiter**, matching `docs/RPC_POLICY.md`.

Attack the RPC plane with:

- valid read floods
- malformed JSON
- oversized bodies
- batch requests
- WebSocket churn
- slow subscribers
- transaction spam
- heavy historical queries

While doing so, measure validators.

### Hard PASS

- unsafe methods unavailable publicly
- validator key/admin methods unreachable
- consensus hosts do not exhaust CPU/RAM/disk because of public RPC
- finalized TPS/finality latency remains inside declared degradation budget
- slow WebSocket clients are bounded/dropped

---

# P1 — Public service plane

## P1-01 — Public RPC + WebSocket

Deploy:

```text
rpc.testnet.x3-chain.io
wss://rpc.testnet.x3-chain.io
```

Use:

```text
DNS -> TLS/LB -> rate limiter -> RPC-A/RPC-B
```

Acceptance:

```bash
curl -s https://rpc.testnet.x3-chain.io \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}'
```

Run:

```bash
scripts/mainnet/public_testnet_gate.sh \
  --rpc-base-url https://rpc.testnet.x3-chain.io
```

---

## P1-02 — Monitoring

Monitoring host must observe the network independently.

Alert on:

- validator down
- finality stall
- best/finalized gap
- peer collapse
- missed slots
- high CPU/RAM
- swap
- disk full/latency/I/O error
- RPC saturation
- clock drift
- runtime/binary mismatch
- key rotation due

Reuse:

```text
monitoring/prometheus-rules.yml
monitoring/alertmanager.yml
monitoring/config/prometheus.yml
```

---

## P1-03 — Faucet

Faucet keys must be separate from treasury/governance/validator keys.

Requirements:

- per-IP/account limits
- bounded daily issuance
- abuse logging
- no privileged runtime authority
- explicit test-token labeling

Publish:

```text
faucet.testnet.x3-chain.io
```

---

## P1-04 — Indexer/archive/explorer

Use the capacity host for archive/indexer, but benchmark its disk IOPS.

Rules:

- indexer is derived data, never consensus truth
- finalized status comes from canonical finalized head
- orphaned pre-finality blocks are tracked separately
- indexer can rebuild from chain history

Minimum explorer fields:

- tx hash
- block number/hash
- finalized state
- events
- fee
- X3VM/EVM/SVM leg status where applicable
- atomic-intent lifecycle
- proof/finality references

---

## P1-05 — Public EVM test leg

After the internal Alpha chain itself is stable, use the existing protected testnet deploy workflow for the EVM gateway/verifier path.

Use Sepolia (or the repository-configured approved EVM testnet), not only Anvil.

Acceptance:

- deploy verifier/gateway
- verify source
- create X3 intent
- X3 lock finalizes
- external EVM action finalizes
- proof accepted by X3
- claim path succeeds
- timeout/refund path succeeds
- wrong proof/secret rejected

Store transaction hashes from both chains.

---

## P1-06 — Public SVM test leg

Move beyond local `solana-test-validator` proof.

Acceptance:

- deploy/use the approved X3 SBF program on the selected public Solana test environment
- create/claim/refund
- record signatures/slots/accounts
- X3 verifies the external evidence
- success and timeout paths complete
- wrong PDA/preimage/intent rejected

---

# P2 — Alpha expansion

## P2-01 — Independent validator onboarding

Give an operator only:

- signed binary/release manifest
- chain spec
- bootnode list
- onboarding guide

They should join without private assistance or receiving any launcher's secret material.

PASS: a new validator reaches healthy sync/peer state and registers/rotates keys through the documented path.

## P2-02 — Extended soak

After the initial 72-hour WAN proof, keep Alpha running continuously for 2–4 weeks.

Track:

- uptime
- finality
- fork/orphan rate
- RPC availability
- disk growth
- memory growth
- restart count
- rejected tx classes
- cross-chain lifecycle success/failure

## P2-03 — Public bug intake

Use the existing bug-report template and incident runbook.

All Critical/High findings affecting fund safety, consensus safety or key security block promotion.

---

# Evidence layout

Every run should land under:

```text
audit-artifacts/public-testnet-alpha/<commit>/<timestamp>/
  summary.json
  commit.txt
  binary.sha256
  runtime-wasm.sha256
  chain-spec.sha256
  ceremony.json
  bootnodes.json
  validators.json
  consensus.csv
  finality.csv
  latency.csv
  peer-topology.json
  key-rotation.json
  recovery.json
  runtime-upgrade.json
  binary-upgrade.json
  rpc-attack.json
  storage-growth.csv
  evm.json
  svm.json
  invariant-results.json
  commands.txt
  logs/
```

No PASS is accepted without an artifact or reproducible command.

---

# Alpha go/no-go matrix

| Gate | Must be green? |
|---|---|
| X3Lang production cutover | YES |
| RC5 72h | YES |
| RC6 package | YES |
| public bootnodes | YES |
| 7 validators live | YES |
| geographic/provider diversity | YES |
| ceremony manifest | YES |
| GRANDPA agreement | YES |
| WAN failure recovery | YES |
| live key rotation | YES |
| snapshot restore | YES |
| runtime upgrade rehearsal | YES |
| rolling binary upgrade | YES |
| public RPC isolation | YES |
| monitoring/alerts | YES |
| faucet | YES |
| explorer/indexer | YES |
| public EVM leg | Required for EVM-cross-chain Alpha claim |
| public SVM leg | Required for SVM-cross-chain Alpha claim |
| external bridges | Remain disabled unless their dedicated gate passes |
| unexplained supply drift | ZERO tolerated |
| conflicting finalized heads | ZERO tolerated |

## Promotion rule

If any mandatory row is red, the result is:

```text
PUBLIC_TESTNET_ALPHA: NO-GO
```

Do not rename, waive or soften the failed gate.

---

# First execution sequence

Run these in this order:

```bash
# 1. Exact head
git checkout master
git pull --ff-only
git rev-parse HEAD

# 2. Build
cargo build --release -p x3-chain-node

# 3. RC6 package preflight
bash scripts/mainnet/rc6_public_testnet_readiness.sh

# 4. RC5 72h on the local multi-node environment
bash scripts/mainnet/rc5_internal_alpha_72h.sh

# 5. While RC5 is running, provision two bootnodes and remote validator hosts.
# Do not expose validator RPC publicly.

# 6. Once public bootnode peer IDs are known, generate the final Alpha spec.
# scripts/testnet/public-node-id.sh
# scripts/testnet/build-x3-testnet-spec.py

# 7. Install validators using scripts/install-validator.sh and maintained systemd unit.

# 8. Record and verify the live ceremony.
# scripts/testnet/testnet-ceremony.py

# 9. Run WAN/failure/key-rotation/recovery/upgrade gates.

# 10. Only then enable the public RPC/faucet/explorer endpoints and run:
scripts/mainnet/public_testnet_gate.sh \
  --rpc-base-url https://rpc.testnet.x3-chain.io
```

## Immediate operator decision

The only infrastructure item that cannot be derived from the repository is where the four non-home-rack validators and two public bootnodes will run.

Everything else in this plan can proceed using the existing repo paths.
