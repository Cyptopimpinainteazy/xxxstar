# Title & ID
TESTNET-ARCH-001 — Multi-Region Testnet Architecture and Launch Plan

# Status
DRAFT

# Authors
- lojak
- codex

# Summary
Define the minimum viable multi-region testnet topology, chain spec generation flow, launch procedure, telemetry stack, faucet/explorer setup, and structured test plan (including GPU acceleration validation) to de-risk consensus integrity and performance claims before a public devnet.

# Motivation
Running a geographically diverse testnet is the only practical way to uncover latency asymmetry, finality stalls, and GPU/consensus regressions that will not appear on local or single-region setups. This proposal standardizes topology, hardware baselines, and launch procedures so the team can execute repeatable benchmarks and adversarial tests with confidence.

# Design
## Topology (Minimum Viable Real Testnet)
- 4 to 7 validators, multi-region (US West, US East, Europe, Asia)
- 2 archive nodes
- 2 public RPC nodes
- 1 telemetry server
- 1 faucet server
- 1 block explorer

## Hardware Baseline (Validators)
- 8 to 16 vCPU
- 32 to 64 GB RAM
- NVMe SSD
- 1 Gbps network
- Ubuntu 22.04 LTS

## GPU Baseline (When Testing Acceleration)
- Dedicated NVIDIA GPU
- Locked driver version
- CUDA version pinned

## Chain Spec and Genesis Flow
1. Generate a custom chain spec and raw file on a trusted machine:
   - `./x3-node build-spec --chain dev > x3-testnet.json`
   - `./x3-node build-spec --chain x3-testnet.json --raw > x3-testnet-raw.json`
2. Distribute `x3-testnet-raw.json` to all validators.
3. No edits after launch; any change requires a new network.

## Launch Procedure (Bootnode + Validators)
- Bootnode (single public entrypoint):
  - `--chain x3-testnet-raw.json`
  - `--validator`
  - `--name "X3-BOOT-01"`
  - `--port 30333 --rpc-port 9933 --ws-port 9944`
- Capture the bootnode peer ID.
- Validators join via `--bootnodes /ip4/<boot-ip>/tcp/30333/p2p/<peer-id>`.
- Verify:
  - Peers > 3
  - Finalization progressing
  - No fork warnings

## Telemetry and Observability
- Prometheus + Grafana
- Substrate telemetry backend
- Log aggregation (Loki or ELK)
- Track: block time, finality time, propagation latency, CPU/memory, GPU usage, fork count

## Faucet and Explorer
- Faucet: Node.js service + hot wallet, rate-limited, CAPTCHA required
- Explorer: Polkadot.js Apps custom config, Subscan (if compatible), or Blockscout if EVM pallet

## Structured Testing
1. Consensus Integrity
   - Fraud proof re-execution matches
   - Invalid commitments rejected
   - Slashing triggers
2. Adversarial Scenarios
   - Kill a validator mid-block
   - Add 300ms latency
   - Drop 20% packets
   - Restart during finality
3. Load Testing
   - 100 / 500 / 1000 tx/sec; compare GPU on/off
   - Measure submit TPS, finalized TPS, reorg rate, CPU cost per tx

## GPU Acceleration Validation Layer
- Compare CPU-only baseline vs GPU-enabled scheduling
- Cross-region propagation deltas under GPU load
- Validator stability under sustained GPU pressure

# Integration Points
- `deployment/chain-specs/x3-testnet-raw.json`
- `deployment/chain-specs/x3-testnet-plain.json`
- `deployment/genesis/x3-testnet-allocations.json`
- `scripts/testnet/run-7-validators-local.sh`
- `scripts/testnet/status-7-validators.sh`
- `scripts/testnet/load-remarks-tps.js`
- `scripts/testnet/verify-testnet.sh`
- `scripts/testnet/run-chainbench-stack.sh`
- `deployment/manage-testnet.sh`
- `docker-compose.monitoring.yml`
- `prometheus.yml`
- `docs/testnet-config/testnet-config.json`
- `docs/testnet-config/TESTNET-VERIFICATION.md`

# Invariants
Add the following to `tests/invariants/registry.toml`:
- `INFRA-TESTNET-001`: bootnode reachability and peer count threshold
- `INFRA-TESTNET-002`: finality progression within agreed SLA
- `INFRA-TESTNET-003`: telemetry ingestion and dashboard availability
- `INFRA-TESTNET-004`: load test meets configured TPS with bounded error rate

# Testing Strategy
- Local 7-validator dry run (scripts/testnet)
- Multi-region cloud run with telemetry + alerts
- Adversarial fault injection (latency, packet loss, node kill)
- GPU A/B benchmarks with identical workloads
- Publish a benchmark report and logs per run

# Rollout Plan
1. Produce production chain spec and raw file
2. Spin up 4 cloud validators + bootnode
3. Verify finality and peer counts
4. Deploy telemetry stack
5. Deploy faucet and explorer
6. Run a 20-minute baseline benchmark
7. Expand to 7 validators and repeat with GPU enabled
8. Open public RPC and invite external validators

# Risks & Mitigations
- Single-region bias: enforce minimum 3 regions
- Chain spec drift: raw file is immutable post-launch
- Faucet abuse: rate limit + CAPTCHA + spend caps
- Slashing misconfig: dry run on local 7-validator network
- GPU nondeterminism: require CPU/GPU parity checks
- Telemetry blind spots: require dashboards before load tests

# Open Questions
- Final chain ID, token symbol, and address prefix
- Exact slashing and staking params for testnet
- Preferred explorer stack (Polkadot.js vs Subscan vs Blockscout)
- Faucet rate limits and CAPTCHA provider
- Telemetry retention and log storage policies
