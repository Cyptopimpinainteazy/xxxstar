# EXEC-PREDICT-002 — Predictive Parallel Execution with On-Device ML

| Field       | Value |
|-------------|-------|
| **ID**      | EXEC-PREDICT-002 |
| **Status**  | DRAFT |
| **Authors** | X3 Core Team |
| **Created** | 2026-02-13 |
| **Priority** | P1 — Performance edge |

---

## Summary

Train a lightweight ML model (~10–20 MB transformer) that runs on the same
GPU as the execution engine to predict read/write contention **before**
transaction execution. The model outputs a contention heatmap that the
orchestrator uses to dynamically partition transaction batches into
independent shards for fully parallel GPU execution. This pushes real-world
parallelism from ~60–80% to 90–95%, dramatically reducing rollbacks and
state thrashing — making X3 the first L1 with **adaptive parallelism**
instead of static.

---

## Motivation

### Problem

Even the best parallel execution engines (Sui/Aptos object-model, Monad
optimistic parallelism) hit hotspots when:

- A viral app concentrates writes on a small set of storage keys (e.g.,
  AMM pool state during a token launch).
- Coordinated attacks intentionally create contention to degrade throughput.
- Batch composition varies wildly block-to-block, making static sharding
  suboptimal.

The current X3 pipeline (SigVerifier → PoH → GPU batch execution)
processes transactions in a fixed parallel topology. When contention
occurs, the rollback rate spikes and effective TPS drops.

### Opportunity

- The GPU is already present and underutilized during the SigVerifier/PoH
  stage — a small transformer inference adds <0.5ms per batch.
- Historical access patterns are available on-chain (state trie access logs,
  recent block traces) — perfect training data.
- No production chain does real-time contention prediction. This would be a
  genuine first-mover advantage.

### Expected Impact

| Metric | Current (Static) | With Prediction |
|--------|------------------|-----------------|
| Parallelizable TX ratio | 60–80% | 90–95% |
| Rollback rate under contention | 5–15% | <2% |
| Effective TPS under hotspot attack | Degrades 30–50% | Degrades <10% |
| Batch scheduling latency overhead | 0 | <0.5ms |

---

## Design

### Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                     Block Pipeline                            │
│                                                              │
│  ┌──────────┐   ┌──────────────┐   ┌─────────────────────┐  │
│  │ Mempool  │──▶│ SigVerifier  │──▶│ ContentionPredictor │  │
│  │ Batch    │   │ + PoH        │   │ (GPU Transformer)   │  │
│  └──────────┘   └──────────────┘   └────────┬────────────┘  │
│                                              │               │
│                                    ┌─────────▼──────────┐   │
│                                    │ ShardPlanner       │   │
│                                    │ (partition batch    │   │
│                                    │  into independent   │   │
│                                    │  execution groups)  │   │
│                                    └─────────┬──────────┘   │
│                                              │               │
│                                    ┌─────────▼──────────┐   │
│                                    │ GPU Parallel       │   │
│                                    │ Executor           │   │
│                                    │ (one CUDA stream   │   │
│                                    │  per shard)        │   │
│                                    └─────────┬──────────┘   │
│                                              │               │
│                                    ┌─────────▼──────────┐   │
│                                    │ State Merge +      │   │
│                                    │ Conflict Detection │   │
│                                    └────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. `crates/contention-predictor/` (New Crate)

Core ML inference engine:

```rust
// crates/contention-predictor/src/lib.rs (sketch)

pub struct ContentionPredictor {
    model: TensorRtEngine,       // ONNX → TensorRT compiled model
    feature_extractor: FeatureExtractor,
    confidence_threshold: f32,   // below this, fall back to conservative
    device: GpuDevice,
}

pub struct ContentionHeatmap {
    /// Storage key → predicted access probability and type
    pub predictions: BTreeMap<StorageKey, AccessPrediction>,
    /// Confidence score for the entire batch [0.0, 1.0]
    pub confidence: f32,
    /// Suggested shard boundaries
    pub suggested_shards: Vec<ShardGroup>,
}

pub struct AccessPrediction {
    pub read_probability: f32,
    pub write_probability: f32,
    pub contention_score: f32,  // 0.0 = no contention, 1.0 = certain conflict
}

pub struct ShardGroup {
    pub tx_indices: Vec<usize>,
    pub estimated_parallelism: f32,
    pub max_contention_pair: Option<(usize, usize)>,
}

impl ContentionPredictor {
    /// Predict contention for a batch of transactions.
    /// Runs on GPU; latency budget: <0.5ms for 10K TXs.
    pub fn predict(&self, batch: &[TransactionMetadata]) -> ContentionHeatmap {
        let features = self.feature_extractor.extract(batch);
        let raw_output = self.model.infer(&features);
        ContentionHeatmap::from_raw(raw_output, self.confidence_threshold)
    }
}
```

#### 2. Feature Extraction

Input features per transaction (fed to transformer):

| Feature | Size | Source |
|---------|------|--------|
| Sender address (hashed) | 8 bytes | TX header |
| Contract/pallet ID | 4 bytes | TX call data |
| Function selector | 4 bytes | TX call data |
| Estimated storage keys touched | Variable | Static analysis or recent trace cache |
| Recent access frequency per key | 4 bytes/key | Rolling window (last 100 blocks) |
| Gas/weight estimate | 4 bytes | Fee estimator |
| Nonce delta | 2 bytes | Mempool state |
| Time since last TX from sender | 4 bytes | Block timestamp |

Total feature vector per TX: ~64–128 bytes. For a 10K TX batch: ~1.28 MB —
trivially fits in GPU memory.

#### 3. Model Architecture

```
Input: [batch_size, seq_len=num_txs, feature_dim=64]
  │
  ▼
Positional Encoding (learned, size 64)
  │
  ▼
4× Transformer Encoder Layers
  - d_model=128, nhead=4, d_ff=256
  - Attention: TX-to-TX attention (captures pairwise contention)
  │
  ▼
Linear Head → [batch_size, seq_len, num_storage_keys]
  - Output: per-TX, per-key contention probability
  │
  ▼
Graph Coloring → ShardGroups
  - Greedy coloring on contention graph
  - TXs that don't conflict share a shard
```

Model size: ~12 MB (FP16 quantized). Inference: <0.3ms on A100 for 10K TXs.

#### 4. Training Pipeline

```
On-chain data (privacy-preserving):
  └─ Block traces: (tx_hash, accessed_keys[], read/write, rollback?)
      └─ Aggregate into access pattern histograms (no raw data)
          └─ Train model offline, push weights on-chain via governance
              └─ Validators download and load model
```

- **Training data**: last 1M blocks of access traces, aggregated.
- **Training frequency**: weekly, triggered by governance or automated.
- **Model distribution**: weights stored in IPFS, hash registered on-chain.
- **Privacy**: only aggregated access statistics, never raw TX content.

#### 5. Shard Planner

```rust
// crates/contention-predictor/src/shard_planner.rs (sketch)

pub struct ShardPlanner {
    pub max_shards: usize,           // bounded by CUDA streams (typ. 32)
    pub min_shard_size: usize,       // don't over-partition small batches
    pub contention_threshold: f32,   // pairs above this go to different shards
}

impl ShardPlanner {
    /// Given a contention heatmap, produce an optimal sharding of the batch.
    /// Uses greedy graph coloring: build conflict graph, color it.
    pub fn plan(&self, heatmap: &ContentionHeatmap, batch_size: usize) -> Vec<ShardGroup> {
        if heatmap.confidence < CONFIDENCE_FLOOR {
            // Low confidence → conservative: single shard, serial execution
            return vec![ShardGroup::serial(batch_size)];
        }
        let conflict_graph = self.build_conflict_graph(heatmap);
        self.greedy_color(conflict_graph, self.max_shards)
    }
}
```

#### 6. Fallback Strategy

When prediction confidence is below threshold (default 0.6):

1. Fall back to **conservative serial execution** (current behavior).
2. Log the batch signature for later training (improves model).
3. No performance regression — worst case is status quo.

---

## Integration Points

| Existing Component | Change Required |
|---|---|
| `crates/gpu-swarm/src/cu_kernels/` | Add transformer inference kernel (or use TensorRT) |
| `crates/x3-vm/src/vm.rs` | Wire shard groups into parallel execution dispatch |
| `crates/gpu-swarm/src/warden/` | Allocate GPU memory for predictor model |
| `crates/gpu-swarm/src/scheduler.rs` | Consume `ShardGroup` output for task assignment |
| `pallets/x3-kernel/` | Emit access traces for training data collection |
| `node/` | Model loading on startup; weight update mechanism |
| `runtime/` | Governance hook for model weight updates |

---

## Invariants

```toml
[[invariant]]
id = "EXEC-PREDICT-001"
description = "Prediction inference latency is ≤0.5ms for batches up to 10K transactions"
severity = "HIGH"
layer = "CONSENSUS"
tested_by = ["crates/contention-predictor/benches/inference_latency.rs::latency_under_budget"]
property = "predict(batch_10k).latency <= 500μs"

[[invariant]]
id = "EXEC-PREDICT-002"
description = "Fallback to serial execution when prediction confidence is below threshold"
severity = "CRITICAL"
layer = "CONSENSUS"
tested_by = ["crates/contention-predictor/tests/fallback.rs::low_confidence_serial"]
property = "confidence < threshold => serial_execution"

[[invariant]]
id = "EXEC-PREDICT-003"
description = "Shard groups are disjoint — no transaction appears in multiple shards"
severity = "CRITICAL"
layer = "VM"
tested_by = ["crates/contention-predictor/tests/shard_planner.rs::shards_disjoint"]
property = "forall i,j where i≠j: shard[i] ∩ shard[j] = ∅"

[[invariant]]
id = "EXEC-PREDICT-004"
description = "Predicted execution produces identical state root to serial execution"
severity = "CRITICAL"
layer = "VM"
tested_by = ["tests/integration/predictive_exec.rs::parallel_matches_serial"]
property = "state_root(parallel_sharded) == state_root(serial)"

[[invariant]]
id = "EXEC-PREDICT-005"
description = "Model weights are loaded only from governance-approved hash"
severity = "HIGH"
layer = "CONSENSUS"
tested_by = ["crates/contention-predictor/tests/model_integrity.rs::reject_unapproved_weights"]
property = "load_model(weights) => hash(weights) in approved_set"
```

---

## Testing Strategy

| Phase | Scope | Method |
|-------|-------|--------|
| Unit | Feature extraction correctness | Property tests with known TX patterns |
| Unit | Shard planner graph coloring | Exhaustive tests on small conflict graphs |
| Unit | Heatmap confidence calibration | Compare prediction vs actual on historical data |
| Integration | Predictor + GPU executor end-to-end | Replay historical blocks, compare state roots |
| Benchmark | Inference latency p99 < 0.5ms | CUDA event profiling, `criterion` benchmarks |
| Correctness | Parallel == serial state roots | Replay 10K blocks both ways, assert identical roots |
| Adversarial | Intentional hotspot attack | Synthetic workloads targeting single storage key |
| Regression | Model accuracy over time | Weekly accuracy report on testnet |

---

## Rollout Plan

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **Phase 1** | 2 weeks | `crates/contention-predictor` scaffolding; feature extractor; offline data pipeline |
| **Phase 2** | 3 weeks | Transformer model training on historical testnet data; TensorRT compilation |
| **Phase 3** | 2 weeks | Shard planner + integration with GPU executor; correctness tests |
| **Phase 4** | 1 week | Benchmarking; latency optimization; fallback tuning |
| **Phase 5** | 1 week | Testnet shadow mode (predict but don't act; log accuracy) |
| **Phase 6** | 1 week | Testnet active mode; monitor rollback rate improvement |

Total: **~10 weeks** to testnet-active.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Model predictions are wrong → increased rollbacks | Medium | High | Confidence threshold + automatic fallback to serial; shadow mode first |
| Inference latency exceeds 0.5ms on target hardware | Low | High | TensorRT quantization (INT8); reduce model size; batch across blocks |
| Training data doesn't generalize to new workloads | Medium | Medium | Continuous online learning from recent blocks; periodic retraining |
| Adversary poisons training data | Low | High | Aggregate stats only; outlier detection; governance-gated model updates |
| GPU memory pressure from model + execution | Low | Medium | Model is 12MB FP16; A100 has 80GB; negligible overhead |

---

## Open Questions

- [ ] Should the model run in the SigVerifier stage (overlapped with signature verification)
      or in a separate stage? (Propose: overlap with SigVerifier for zero added latency.)
- [ ] What's the minimum batch size where prediction is worthwhile?
      (Propose: skip prediction for batches < 100 TXs — insufficient signal.)
- [ ] Should model weights be stored on-chain or off-chain (IPFS + hash on-chain)?
      (Propose: IPFS + on-chain hash via governance.)
- [ ] How to handle contract upgrades that change access patterns?
      (Propose: invalidate model for affected contracts; retrain within 1 epoch.)
- [ ] Can the model also predict gas/weight more accurately?
      (Propose: secondary output head — low incremental cost, high value.)
