# GPU Swarm + X3 Master Architecture

## Current Implementation Status

### ✅ COMPLETE - X3 Compiler Pipeline (44,750+ LOC)

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  x3-lexer   │───▶│  x3-parser  │───▶│   x3-ast    │───▶│   x3-hir    │
│   Tokens    │    │   Grammar   │    │    AST      │    │  High-IR    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                                                                │
┌─────────────┐    ┌─────────────┐    ┌─────────────┐           ▼
│  x3-typeck  │◀───│ x3-semantics│◀───│   x3-mir    │◀──────────┘
│  Type Check │    │   Resolver  │    │   Mid-IR    │
└─────────────┘    └─────────────┘    └─────────────┘
       │
       ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   x3-opt    │───▶│ x3-backend  │───▶│   x3-vm     │───▶│ x3-verifier │
│  Optimizer  │    │  Bytecode   │    │  Execution  │    │  Gas/Rules  │
│ 20+ passes  │    │   Emit      │    │   Runtime   │    │             │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### ✅ COMPLETE - GPU Swarm Core (4,284 LOC)

```
crates/gpu-swarm/
├── scheduler.rs     # Job scheduling & priority queues
├── coordinator.rs   # Swarm coordination & consensus
├── protocol.rs      # P2P messaging & discovery
├── verification.rs  # Result verification & fraud proofs
├── node.rs          # Node client runtime
├── network.rs       # Networking layer
├── task.rs          # Task definitions & lifecycle
├── config.rs        # Configuration management
└── error.rs         # Error types
```

### ✅ COMPLETE - Cross-VM Bridge

```
crates/cross-vm-bridge/   # EVM ↔ SVM atomic execution
crates/evm-integration/   # Frontier EVM adapter
crates/svm-integration/   # Solana BPF adapter
```

### ✅ COMPLETE - X3 Evolution Engine (~2,500 LOC)

```
crates/x3-evolution/
├── lib.rs            # EvolutionEngine main implementation
├── chromosome.rs     # Gene/Chromosome types for X3 bytecode
├── mutation.rs       # 5 mutation operators (Parameter, Logic, Gaussian, Swap, Composite)
├── crossover.rs      # 5 crossover operators (Uniform, SinglePoint, TwoPoint, Arithmetic, Adaptive)
├── fitness.rs        # PnL-based fitness scoring (Sharpe, Sortino, Calmar, Drawdown)
├── selection.rs      # 6 selection strategies (Tournament, Roulette, Elite, Rank, SUS, Truncation)
├── population.rs     # Population management with elitism
├── simulator.rs      # Strategy simulation with X3 VM integration
└── error.rs          # 17 error variants
```

### ✅ COMPLETE - Swarm Job Executors (~3,500 LOC)

```
crates/gpu-swarm/src/jobs/
├── mod.rs               # SwarmJob trait, JobType enum, JobOutput, JobSubmission, JobReceipt
├── x3_simulation.rs     # X3 strategy simulation with PnL fitness metrics
├── mev_discovery.rs     # MEV opportunity detection (arbitrage, sandwich, liquidations)
├── zk_proving.rs        # ZK proof generation (Groth16, PLONK, STARK, Halo2, Nova)
├── model_training.rs    # ML model training (PnL Reward Model, Evolution-Core, RL agents)
├── mempool_analysis.rs  # Mempool aggregation, gas prediction, whale tracking
└── chain_indexing.rs    # Multi-chain block/event/transfer indexing
```

**Job Type Reward Multipliers:**
| Job Type         | Multiplier | Default Timeout |
| ---------------- | ---------- | --------------- |
| ZK Proving       | 3.0x       | 600s            |
| MEV Discovery    | 2.0x       | 30s             |
| Model Training   | 1.5x       | 600s            |
| X3 Simulation    | 1.0x       | 60s             |
| Mempool Analysis | 0.8x       | 15s             |
| Chain Indexing   | 0.5x       | 120s            |

### ✅ COMPLETE - Warden: GPU Swarm Brain (~3,500 LOC)

The Warden is the Master Control Intelligence that orchestrates GPU allocation across compute lanes, balancing four pillars.

```
crates/gpu-swarm/src/warden/
├── mod.rs          # Warden main controller - decision cycles, state management
├── policy.rs       # ComputeLane enum (9 lanes), AllocationPolicy, LaneConstraints
├── signals.rs      # LaneSignal, SignalAggregator, LaneMetrics, AlertSeverity
├── allocator.rs    # GpuAllocator, AllocationPlan, LaneAllocation, anti-thrashing
├── predictor.rs    # LoadPredictor, SwarmForecast, LoadTrend, PredictionHorizon
├── governance.rs   # GovernanceEngine, EmergencyOverride, GuardBot, ThreatLevel
└── metrics.rs      # Four Pillars (P↑/I↑/S↑/E↑), PillarScores, HealthStatus
```

**Four Pillars:**
| Pillar         | Symbol | Description         | Contributing Lanes            |
| -------------- | ------ | ------------------- | ----------------------------- |
| Profit         | P↑     | MEV, trading, fees  | Strategy, AiAgents, Overflow  |
| Intelligence   | I↑     | Research, training  | Research, AiAgents, Evolution |
| Infrastructure | S↑     | Stability, security | ChainOps, Security, Storage   |
| Ecosystem      | E↑     | Growth, adoption    | Ecosystem, Overflow           |

**Compute Lanes (9 total):**
| Lane      | Priority | Min Alloc | Max Alloc | Critical |
| --------- | -------- | --------- | --------- | -------- |
| Security  | 100      | 15%       | 80%       | ✅        |
| ChainOps  | 90       | 10%       | 40%       | ✅        |
| Strategy  | 80       | 5%        | 35%       |          |
| AiAgents  | 70       | 0%        | 25%       |          |
| Research  | 60       | 5%        | 40%       |          |
| Evolution | 50       | 0%        | 15%       | Disabled |
| Ecosystem | 40       | 0%        | 20%       |          |
| Storage   | 30       | 0%        | 15%       |          |
| Overflow  | 10       | 0%        | 20%       |          |

**Threat Levels & Security Multipliers:**
| Level    | Multiplier | Response                  |
| -------- | ---------- | ------------------------- |
| None     | 1.0x       | Normal operations         |
| Low      | 1.2x       | Increased monitoring      |
| Elevated | 1.5x       | Activate secondary guards |
| High     | 2.0x       | Emergency reallocation    |
| Critical | 4.0x       | Full defensive mode       |

### ✅ COMPLETE - The Crown: Meta-Governor (~3,550 LOC)

The Crown is the immune system that watches everything - it evaluates the Warden, monitors chain health, prevents drift, and recycles failed modules for knowledge.

```
crates/gpu-swarm/src/crown/
├── mod.rs          # Crown main controller - evaluate cycles, verdict system
├── auditor.rs      # Auditor daemon - chain health, profit flows, security threats
├── prophet.rs      # AI forecast engine - market cycles, volatility, predictions
└── scrapyard.rs    # Failure recycling - quarantine, disassembly, knowledge extraction
```

**The Big Three (Hierarchy):**
| Component | Role       | Responsibility                           |
| --------- | ---------- | ---------------------------------------- |
| Crown     | Brain Stem | Survival regulation, Warden oversight    |
| Warden    | Arms       | GPU allocation, compute distribution     |
| Auditor   | Eyes       | Monitoring everything, signal collection |
| Prophet   | Foresight  | Market prediction, threat forecasting    |

**Crown Verdicts:**
| Verdict  | Severity | Action                                       |
| -------- | -------- | -------------------------------------------- |
| Healthy  | 0        | Commendations, no intervention               |
| Caution  | 1        | Issue tracking, recommendations              |
| Warning  | 2        | Required governance actions                  |
| Override | 3        | Crown takeover, Warden suspension, emergency |

**Issue Categories Detected:**
- WardenDrift - Allocation diverging from optimal
- AllocationBias - Over-concentration in single lane
- ProfitDecline - Revenue dropping below thresholds
- ChainHealth - Block time, error rate, consensus issues
- SecurityThreat - Active attacks, vulnerabilities
- EvolutionGaming - Evo Babies manipulating signals
- ResourceExhaustion - Storage, memory, CPU critical
- MissionCreep - Scope expanding beyond mandate
- ModelInstability - AI models producing dangerous outputs
- ChainStress - Potential DoS conditions

**Prophet Market Cycles:**
| Cycle         | Strategy Bias | Security Mult |
| ------------- | ------------- | ------------- |
| Accumulation  | 30%           | 1.0x          |
| Bull          | 45%           | 1.0x          |
| Distribution  | 20%           | 1.5x          |
| Bear          | 10%           | 1.3x          |
| Consolidation | 25%           | 1.0x          |

**Scrapyard Pipeline:**
```
┌─────────────────────────────────────────────────────────────────────┐
│                   SCRAPYARD FAILURE RECYCLING                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Stage 1: QUARANTINE                                                │
│  ├── Sandbox isolation (no chain/API/GPU access)                    │
│  ├── Behavioral observation                                          │
│  ├── Study why it failed                                            │
│  └── Identify potential innovations                                 │
│                                                                      │
│  Stage 2: DISASSEMBLY                                               │
│  ├── Parse architecture and logic                                   │
│  ├── Detect exploits and vulnerabilities                            │
│  ├── Extract useful heuristics                                      │
│  └── Calculate danger level & recyclability                         │
│                                                                      │
│  Stage 3: VERDICT                                                   │
│  ├── Recycle → Extract useful parts as genome fragments             │
│  ├── Execute → Destroy weights, wipe code, blacklist ID             │
│  ├── Rehabilitate → Return to service with conditions               │
│  └── Extend → More observation needed                               │
│                                                                      │
│  Stage 4: MEMORY INJECTION                                          │
│  ├── Feed knowledge to Evo Baby mutation logic                      │
│  ├── Update Warden allocation heuristics                            │
│  ├── Enhance Prophet forecasting models                             │
│  └── Strengthen security pattern detection                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Recyclable Knowledge Types:**
- Algorithm - Novel optimization techniques
- Pattern - Useful behavioral patterns
- SecurityInsight - Vulnerability knowledge
- StrategyFragment - Trading strategy pieces
- ModelWeights - Partial model parameters
- TrainingData - Useful training examples
- ErrorHandling - Recovery techniques

---

## 🔴 REMAINING COMPONENTS (Priority Build Order)

### Phase 3: Proof-of-Useful-Work

```text
crates/proof-of-useful-work/
├── scoring.rs        # Work scoring algorithm
├── verification.rs   # PoUW verification
├── rewards.rs        # Reward calculation
├── difficulty.rs     # Adaptive difficulty
└── lib.rs
```

### Phase 4: Compute Marketplace

```text
pallets/compute-marketplace/
├── lib.rs            # Pallet logic
├── pricing.rs        # Dynamic pricing
├── matching.rs       # Job-worker matching
├── escrow.rs         # Payment escrow
└── reputation.rs     # Node reputation
```

### Phase 5: JIT Acceleration

```text
crates/x3-jit/
├── cranelift.rs      # Cranelift JIT backend
├── cache.rs          # Compiled code cache
├── profiler.rs       # Hot path detection
└── lib.rs
```

---

## System Architecture Overview

```
                         ┌─────────────────────────────────────────────────────────────┐
                         │                    X3-SPHERE CHAIN                       │
                         │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
                         │  │ X3 Kernel │ │   X3 Pallet  │ │ Compute Marketplace  │ │
                         │  │  (Comit Tx)  │ │ (On-chain VM)│ │  (Job Escrow/Pay)    │ │
                         │  └──────────────┘ └──────────────┘ └──────────────────────┘ │
                         └─────────────────────────────────────────────────────────────┘
                                                    │
                                                    │ RPC / Events
                                                    ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                  THE CROWN (META-GOVERNOR)                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                     │
│  │   Auditor   │  │   Prophet   │  │  Scrapyard  │  │   Verdict   │                     │
│  │   (Eyes)    │  │ (Foresight) │  │  (Recycler) │  │   Engine    │                     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘                     │
│                            │ Oversight │ Override │ Recalibrate                          │
└────────────────────────────┼───────────┴──────────┼──────────────────────────────────────┘
                             ▼                      ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              THE WARDEN (GPU ALLOCATOR)                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Policy    │  │  Allocator  │  │  Predictor  │  │  Governance │  │   Signals   │   │
│  │   (Lanes)   │  │   (GPU)     │  │   (Load)    │  │  (Guards)   │  │ (Telemetry) │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────────┘
                                                    │
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              GPU SWARM COORDINATOR                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │    Job      │  │   Result    │  │   Node      │  │   Reward    │  │  Mempool    │   │
│  │  Scheduler  │  │ Aggregator  │  │  Registry   │  │ Calculator  │  │  Stitcher   │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────────┘
                                                    │
                    ┌───────────────────────────────┼───────────────────────────────┐
                    │                               │                               │
                    ▼                               ▼                               ▼
┌─────────────────────────────┐  ┌─────────────────────────────┐  ┌─────────────────────────────┐
│       GPU NODE (A)          │  │       GPU NODE (B)          │  │       GPU NODE (C)          │
│  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │
│  │     X3 VM Sandbox     │  │  │  │     X3 VM Sandbox     │  │  │  │     X3 VM Sandbox     │  │
│  │  ┌─────┐ ┌─────────┐  │  │  │  │  ┌─────┐ ┌─────────┐  │  │  │  │  ┌─────┐ ┌─────────┐  │  │
│  │  │ JIT │ │Strategy │  │  │  │  │  │ JIT │ │Strategy │  │  │  │  │  │ JIT │ │Strategy │  │  │
│  │  │Cache│ │Simulator│  │  │  │  │  │Cache│ │Simulator│  │  │  │  │  │Cache│ │Simulator│  │  │
│  │  └─────┘ └─────────┘  │  │  │  │  └─────┘ └─────────┘  │  │  │  │  └─────┘ └─────────┘  │  │
│  └───────────────────────┘  │  │  └───────────────────────┘  │  │  └───────────────────────┘  │
│  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │
│  │     Job Executors     │  │  │  │     Job Executors     │  │  │  │     Job Executors     │  │
│  │ ┌─────┐┌─────┐┌─────┐ │  │  │  │ ┌─────┐┌─────┐┌─────┐ │  │  │  │ ┌─────┐┌─────┐┌─────┐ │  │
│  │ │ MEV ││ ZK  ││Model│ │  │  │  │ │ MEV ││ ZK  ││Model│ │  │  │  │ │ MEV ││ ZK  ││Model│ │  │
│  │ │Path ││Prove││Train│ │  │  │  │ │Path ││Prove││Train│ │  │  │  │ │Path ││Prove││Train│ │  │
│  │ └─────┘└─────┘└─────┘ │  │  │  │ └─────┘└─────┘└─────┘ │  │  │  │ └─────┘└─────┘└─────┘ │  │
│  └───────────────────────┘  │  │  └───────────────────────┘  │  │  └───────────────────────┘  │
│         GPU: 4090           │  │         GPU: H100           │  │         GPU: A100           │
│         VRAM: 24GB          │  │         VRAM: 80GB          │  │         VRAM: 80GB          │
└─────────────────────────────┘  └─────────────────────────────┘  └─────────────────────────────┘
```

---

## Job Types & Rewards

| Job Type               | GPU Intensity | Base Reward | Difficulty Multiplier |
| ---------------------- | ------------- | ----------- | --------------------- |
| X3 Strategy Simulation | High          | 10 X3    | 1.0x - 5.0x           |
| MEV Path Discovery     | Very High     | 25 X3    | 1.5x - 10.0x          |
| ZK Proof Generation    | Extreme       | 50 X3    | 2.0x - 20.0x          |
| Model Training         | High          | 15 X3    | 1.0x - 8.0x           |
| Chain Indexing         | Medium        | 5 X3     | 0.5x - 2.0x           |
| Mempool Analysis       | Medium        | 8 X3     | 1.0x - 3.0x           |
| Attack Simulation      | Very High     | 30 X3    | 2.0x - 15.0x          |

---

## X3 Strategy Evolution Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         X3 STRATEGY EVOLUTION PIPELINE                        │
└──────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   GENERATION 0  │    │   MUTATION      │    │   SIMULATION    │
│                 │    │                 │    │                 │
│ Initial X3 Code │───▶│ • Param tweak   │───▶│ • 1M+ replays   │
│ (User-provided  │    │ • Logic mutate  │    │ • Multi-market  │
│  or seeded)     │    │ • Crossover     │    │ • GPU parallel  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                      │
                                                      ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   VERIFICATION  │    │   SELECTION     │    │   SCORING       │
│                 │    │                 │    │                 │
│ • ZK proof      │◀───│ • Top 10%       │◀───│ • PnL fitness   │
│ • Deterministic │    │ • Tournament    │    │ • Sharpe ratio  │
│ • Fraud check   │    │ • Elite keep    │    │ • Max drawdown  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │
         ▼
┌─────────────────┐    ┌─────────────────┐
│   ON-CHAIN      │    │   NEXT GEN      │
│   SUBMISSION    │    │                 │
│                 │    │ Repeat until    │
│ • Comit TX      │    │ convergence or  │
│ • Strategy NFT  │    │ N generations   │
└─────────────────┘    └─────────────────┘
```

---

## Monetization Model

### Revenue Streams

```
┌─────────────────────────────────────────────────────────────────────┐
│                      X3 CHAIN ECONOMY                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────┐      ┌──────────────────────┐            │
│  │   COMPUTE SELLERS    │      │   COMPUTE BUYERS     │            │
│  │   (GPU Node Ops)     │      │   (Developers)       │            │
│  │                      │      │                      │            │
│  │ • Earn X3 tokens  │◀────▶│ • Pay X3 for jobs │            │
│  │ • Stake for priority │      │ • Submit X3 code     │            │
│  │ • Uptime bonuses     │      │ • Buy trained models │            │
│  └──────────────────────┘      └──────────────────────┘            │
│              │                           │                          │
│              ▼                           ▼                          │
│  ┌──────────────────────────────────────────────────────┐          │
│  │              COMPUTE MARKETPLACE                      │          │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │          │
│  │  │ Escrow  │ │ Pricing │ │ Matching│ │ Dispute │    │          │
│  │  │ System  │ │ Oracle  │ │ Engine  │ │ Resolution│   │          │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘    │          │
│  └──────────────────────────────────────────────────────┘          │
│                              │                                      │
│                              ▼                                      │
│  ┌──────────────────────────────────────────────────────┐          │
│  │           PROOF-OF-USEFUL-WORK REWARDS               │          │
│  │                                                       │          │
│  │  Score = (ComputeUnits × Difficulty × Reliability)   │          │
│  │  Reward = Score × EpochPool / TotalScore             │          │
│  └──────────────────────────────────────────────────────┘          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Anti-Cheat Mechanisms

1. **Result Verification**: Multiple nodes compute same job, consensus required
2. **ZK Receipts**: Provable computation without revealing strategy
3. **Stake Slashing**: Malicious results = lose staked tokens
4. **Reputation Decay**: Inactive or unreliable nodes lose priority
5. **Deterministic Replay**: Any result can be re-verified on-chain

---

## Immediate Implementation Priority

### Week 1: X3 Evolution Engine
- [ ] `crates/x3-evolution/` - Mutation operators
- [ ] Fitness scoring with PnL metrics
- [ ] Population management

### Week 2: Swarm Job Executors
- [ ] `crates/gpu-swarm/src/jobs/` - Job type implementations
- [ ] X3 simulation job with sandboxing
- [ ] MEV path discovery job

### Week 3: Proof-of-Useful-Work
- [ ] Scoring algorithm
- [ ] Verification protocol
- [ ] Reward distribution

### Week 4: Compute Marketplace Pallet
- [ ] Job submission/bidding
- [ ] Escrow and payments
- [ ] Reputation system

---

## API Endpoints (Planned)

```rust
// Swarm Coordinator RPC
#[rpc(server)]
pub trait SwarmApi {
    /// Submit a compute job
    #[method(name = "swarm_submitJob")]
    async fn submit_job(&self, job: SwarmJob) -> Result<JobId>;
    
    /// Get job status
    #[method(name = "swarm_getJobStatus")]
    async fn get_job_status(&self, job_id: JobId) -> Result<JobStatus>;
    
    /// Get node statistics
    #[method(name = "swarm_getNodeStats")]
    async fn get_node_stats(&self, node_id: NodeId) -> Result<NodeStats>;
    
    /// Get network overview
    #[method(name = "swarm_getNetworkOverview")]
    async fn get_network_overview(&self) -> Result<NetworkOverview>;
    
    /// Submit X3 strategy for evolution
    #[method(name = "swarm_evolveStrategy")]
    async fn evolve_strategy(&self, config: EvolutionConfig) -> Result<EvolutionJobId>;
}
```

---

## Security Model

```
┌─────────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: SANDBOXING                                            │
│  ├── WASM isolation for X3 VM                                   │
│  ├── Memory limits per job                                      │
│  ├── CPU time limits                                            │
│  └── No network access from sandbox                             │
│                                                                  │
│  Layer 2: VERIFICATION                                          │
│  ├── Deterministic execution                                    │
│  ├── Multi-node consensus                                       │
│  ├── ZK proof for sensitive jobs                                │
│  └── Merkle receipts for all outputs                            │
│                                                                  │
│  Layer 3: ECONOMIC                                              │
│  ├── Stake requirements for nodes                               │
│  ├── Slashing for fraud                                         │
│  ├── Reputation scoring                                         │
│  └── Insurance pool for disputes                                │
│                                                                  │
│  Layer 4: CHAIN SAFETY                                          │
│  ├── Off-chain compute, on-chain verify                         │
│  ├── Atomic cross-VM with rollback                              │
│  ├── Rate limiting per account                                  │
│  └── Gas metering for all operations                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Next Command

Ready to implement. Which component first?

1. **X3 Evolution Engine** - Strategy mutation & fitness
2. **Job Executors** - MEV/ZK/Training jobs
3. **PoUW System** - Reward calculation
4. **Marketplace Pallet** - On-chain job market
5. **JIT Compiler** - Cranelift acceleration

Type the number or component name to begin.
