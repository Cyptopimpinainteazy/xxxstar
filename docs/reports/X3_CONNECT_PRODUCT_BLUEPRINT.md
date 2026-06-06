# X3 Connect Product Blueprint

## Positioning Statement

X3 is a throughput coprocessor and interoperability fabric for chains.

The product is not "replace your chain with X3." It is "keep your chain, connect to X3, prove the gain, and route only the workloads that benefit."

## Product Summary

X3 Connect is a multi-tier product family composed of:

- `X3 Benchmark Cloud`
- `X3 Connect SDK`
- `X3 Turbo Lanes`
- `X3 Shared Settlement`

The first revenue engine should be benchmarking and sidecar integration. Shared settlement is the later premium tier.

## Customer Promise

Integrate X3 in shadow mode first. X3 ingests the partner chain's traffic, identifies bottlenecks, replays the workload through X3 execution lanes, and produces a signed report showing the gain before deeper commitment.

## Core Offers

### X3 Benchmark Cloud

Purpose:

- analyze real workloads
- replay traces through X3 models
- generate signed benchmark reports

Outputs:

- hotspot analysis
- contention map
- throughput ceiling estimate
- sidecar vs turbo-lane recommendation

### X3 Connect SDK

Purpose:

- make integration real and low-friction

Capabilities:

- messaging APIs
- route discovery
- intent execution hooks
- receipt verification
- benchmark API client

### X3 Turbo Lanes

Purpose:

- accelerate specific partner workloads

Typical workloads:

- swaps
- payments
- orderflow
- NFT mints
- game actions
- bot traffic
- bridge settlement

### X3 Shared Settlement

Purpose:

- deeper safety and performance integration

Capabilities:

- proof commitments
- shared sequencing
- shared execution
- dispute and replay support

## Integration Tiers

### Tier 0 - Benchmark Only

Best for chains evaluating X3 with minimal risk.

### Tier 1 - Sidecar Mode

Best for chains that want routing, messaging, soft-confirm aids, and benchmark-backed visibility without changing consensus.

### Tier 2 - Turbo Lane Mode

Best for chains that want selected high-volume workloads accelerated by X3.

### Tier 3 - Shared Settlement Mode

Best for chains that want X3 integrated into execution and settlement itself.

## Architecture Overview

### X3 Root

Canonical control chain for:

- partner registry
- billing
- proof commitments
- disputes
- cross-lane accounting

### X3 Turbo Fabric

Execution layer containing:

- execution cells
- partner-specific lanes
- conflict-aware schedulers
- GPU assist services
- receipt generation
- proof packaging

### X3 Connect Adapters

Adapter layer for:

- EVM
- OP Stack style chains
- Substrate/Cosmos style chains
- custom appchains

### X3 Proof Bus

Shared backbone carrying:

- receipt roots
- state delta roots
- dependency digests
- replay seeds
- witness references

### X3 Benchmark Harness

Benchmarking layer carrying:

- passive analysis
- trace replay
- shadow-mode comparison
- signed certification reports

## Benchmark Report Schema

Every partner-facing report should include:

### Identity

- partner chain name
- environment
- benchmark window
- signer identity
- config hash

### Baseline

- p50/p95/p99 latency
- block fullness
- failed tx rate
- hotspot contracts/accounts
- contention summary

### X3 Replay

- replayed transaction count
- projected throughput by tier
- latency delta
- failure-rate delta
- route-quality or bridge-latency delta

### Recommendation

- benchmark only
- sidecar mode
- turbo lane mode
- shared settlement mode

## Revenue Model

### Benchmarking

Flat fee for analysis and signed report generation.

### Integration

One-time fee for adapters, SDK support, workload mapping, and shadow-mode setup.

### Ongoing Usage

Metered pricing on:

- receipts
- proof commitments
- messages
- reserved throughput
- GPU bursts

### Premium Enterprise

Reserved lanes, latency guarantees, regional deployment, advanced reporting, and dedicated support.

## Onboarding Experience

### Day 1

- register chain
- connect RPC/config
- upload or stream traces
- run baseline benchmark

### Day 2

- receive signed report
- inspect dashboard
- choose sidecar or turbo lane path

### Week 1

- deploy testnet adapter
- run shadow mode
- compare live baseline vs X3 outputs

### Later

- enable workload routing
- reserve lanes
- expand to shared settlement

## Repo Module Mapping

Current crates that can anchor this product:

- `crates/x3-rpc`
- `crates/x3-sidecar`
- `crates/x3-gateway`
- `crates/x3-bridge-adapters`
- `crates/parallel-proposer`
- `crates/contention-predictor`

Planned additions:

- `x3-connect-sdk`
- `x3-benchmark-cloud`
- `x3-turbo-lane`
- `x3-proof-bus`
- `x3-metering`

## Rollout Plan

### Phase 1

- benchmark harness
- EVM adapter
- sidecar proxy
- metrics portal
- signed report generation

### Phase 2

- turbo lane engine
- receipt bridge
- proof-bus artifacts
- GPU assist plane

### Phase 3

- shared settlement interfaces
- partner proofs
- dispute engine
- partner registry and metering

### Phase 4

- regional lane clusters
- dedicated partner lanes
- hierarchical proof aggregation

## Success Test

X3 Connect is succeeding only if partner chains can measure at least one of these clearly:

- materially better hot-path throughput
- lower failure rate under load
- faster soft confirmations
- cleaner congestion isolation
- better cross-chain routing or message latency

## Final View

The manageable product is not "the billion TPS chain." The manageable product is:

`X3 Connect + Benchmark Cloud + Turbo Lanes`

That product can be sold before deep consensus integration, proven in shadow mode, and expanded into shared settlement only after the partner sees the gain.
