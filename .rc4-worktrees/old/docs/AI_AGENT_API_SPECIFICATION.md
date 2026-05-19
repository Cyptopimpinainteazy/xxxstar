# AI Agent API — Canonical Specification

> **Status**: Canonical | **Version**: 1.0.0 | **Last Updated**: 2025-12-10

The AI Agent API defines how autonomous agents are declared, executed, evolved, and coordinated within the X3 Chain runtime. This specification covers agent anatomy, the runtime API, the evolution engine, and the multi-agent coordination model.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Agent Anatomy](#2-agent-anatomy)
3. [Program Unit Types](#3-program-unit-types)
4. [Agent Runtime API](#4-agent-runtime-api)
5. [Evolution Engine](#5-evolution-engine)
6. [Strategy System](#6-strategy-system)
7. [AI Primitives & Types](#7-ai-primitives--types)
8. [Predictive Execution](#8-predictive-execution)
9. [Multi-Agent Coordination](#9-multi-agent-coordination)
10. [Agent Lifecycle](#10-agent-lifecycle)
11. [Memory & State Model](#11-memory--state-model)
12. [Security Model](#12-security-model)
13. [Standard Agent Patterns](#13-standard-agent-patterns)

---

## 1. Overview

### 1.1 What Is an AI Agent?

An AI Agent in X3 Chain is an autonomous program unit that can:

- **Observe** — Monitor chain state, prices, events
- **Decide** — Use ML models to make decisions
- **Execute** — Perform cross-VM transactions
- **Evolve** — Mutate and improve through genetic algorithms
- **Coordinate** — Work with other agents in swarms

### 1.2 Design Principles

| Principle               | Description                                           |
| ----------------------- | ----------------------------------------------------- |
| **Deterministic Core**  | Agent logic compiles to deterministic bytecode        |
| **Probabilistic Layer** | AI predictions are explicit, bounded, auditable       |
| **Evolvable**           | All agent code can be mutated by the evolution engine |
| **Cross-VM Native**     | Agents operate across EVM and SVM seamlessly          |
| **Sandboxed**           | Agents run in isolated execution contexts             |

### 1.3 Agent vs Contract vs Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    PROGRAM UNIT HIERARCHY                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CONTRACT                                                   │
│  ├── Passive, waits for calls                              │
│  ├── Storage-centric                                       │
│  └── Traditional smart contract semantics                  │
│                                                             │
│  AGENT                                                      │
│  ├── Active, autonomous execution                          │
│  ├── ML model integration                                  │
│  ├── Risk profiles, decision loops                         │
│  └── Can spawn child agents                                │
│                                                             │
│  STRATEGY                                                   │
│  ├── Declarative trigger → action rules                    │
│  ├── Event-driven activation                               │
│  └── Stateless or minimal state                            │
│                                                             │
│  KERNEL                                                     │
│  ├── Pure computation units                                │
│  ├── Parallelizable, vectorizable                          │
│  └── Used by agents/contracts for heavy math               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Agent Anatomy

### 2.1 File Structure

An agent is stored as a bundle of files:

```
agent_0x1234/
├── code.x3bc          # Compiled bytecode
├── memory.jsonl       # Episodic memory (observations)
├── stats.bin          # Performance statistics
├── risk_profile.json  # Risk parameters
├── genome.seed        # Genetic seed for reproduction
└── manifest.json      # Metadata and configuration
```

### 2.2 Manifest Schema

```json
{
  "id": "0x1234...abcd",
  "name": "ArbSniper_v3",
  "version": "3.2.1",
  "generation": 847,
  "parent_id": "0xabcd...1234",
  "created_at": 1733846400,
  "model": {
    "type": "transformer",
    "size": "7b",
    "quantization": "int8"
  },
  "risk_profile": {
    "max_position": "1000000",
    "max_drawdown": 0.15,
    "risk_score": 0.72
  },
  "capabilities": [
    "evm.call",
    "svm.invoke",
    "dex.swap",
    "ai.predict"
  ],
  "fitness": {
    "total_pnl": "4582910000000",
    "sharpe_ratio": 2.41,
    "win_rate": 0.673,
    "trades": 12847
  }
}
```

### 2.3 Code Structure

```x3script
agent Sniper {
    // Model configuration
    model "llama3-70b"
    risk 0.72
    
    // Persistent state
    storage {
        total_trades: u64;
        total_profit: u128;
        last_action: u64;
    }
    
    // Observation function — called each block
    fn observe() -> Observation {
        let prices = dex.get_prices([tokenA, tokenB, tokenC]);
        let volumes = dex.get_volumes([pairAB, pairBC]);
        let health = aave.get_health_factors(top_borrowers);
        
        return Observation {
            prices: prices,
            volumes: volumes,
            health: health,
            timestamp: system.timestamp()
        };
    }
    
    // Decision function — uses ML model
    fn decide(obs: Observation) -> Action {
        let features = encode_features(obs);
        let prediction = ai.predict(features);
        
        if prediction.confidence > 0.85 {
            return Action::Execute(prediction.action);
        }
        return Action::Wait;
    }
    
    // Execution function — performs the action
    fn execute(action: Action) {
        match action {
            Action::Swap(params) => {
                dex.swap_exact_in(
                    params.amount,
                    params.min_out,
                    params.path
                );
            },
            Action::Liquidate(params) => {
                aave.liquidate(
                    params.borrower,
                    params.collateral,
                    params.debt
                );
            },
            Action::Wait => {}
        }
        
        storage.total_trades += 1;
        storage.last_action = system.timestamp();
    }
}
```

---

## 3. Program Unit Types

### 3.1 Contract

Traditional smart contract with storage and callable functions:

```x3script
contract Vault {
    storage {
        balance: u128;
        owner: address;
    }
    
    fn init(owner: address) {
        storage.owner = owner;
        storage.balance = 0;
    }
    
    fn deposit(amount: u64) {
        storage.balance += amount;
    }
    
    fn withdraw(amount: u64) {
        assert(system.caller() == storage.owner, "unauthorized");
        assert(amount <= storage.balance, "insufficient");
        storage.balance -= amount;
        evm.call(to: storage.owner, value: amount, data: []);
    }
}
```

### 3.2 Agent

Autonomous actor with ML integration:

```x3script
agent Arbitrageur {
    model "gpt-4-turbo"
    risk 0.65
    
    storage {
        profit: u128;
    }
    
    fn observe() -> MarketState { ... }
    fn decide(state: MarketState) -> Action { ... }
    fn execute(action: Action) { ... }
}
```

### 3.3 Strategy

Declarative trigger-action rules:

```x3script
strategy LiquidationBot {
    watch aave.v3
    
    when health_factor < 1.0 => {
        let borrower = event.borrower;
        let collateral = event.collateral;
        execute Liquidate(borrower, collateral);
    }
    
    when price_drop > 0.10 => {
        execute EmergencyExit();
    }
}
```

### 3.4 Kernel

Pure computation unit for heavy math:

```x3script
kernel PriceOracle {
    fn compute_twap(prices: array[u64; 24]) -> u64 {
        let sum: u64 = 0;
        for i in 0..24 {
            sum += prices[i];
        }
        return sum / 24;
    }
    
    fn compute_volatility(prices: array[u64; 24]) -> u64 {
        let mean = compute_twap(prices);
        let variance: u64 = 0;
        for i in 0..24 {
            let diff = if prices[i] > mean { 
                prices[i] - mean 
            } else { 
                mean - prices[i] 
            };
            variance += diff * diff;
        }
        return math.sqrt(variance / 24);
    }
}
```

---

## 4. Agent Runtime API

### 4.1 Core Runtime Methods

```x3script
// Agent identity
agent.id() -> bytes32              // Unique agent ID
agent.generation() -> u32          // Evolution generation
agent.parent_id() -> bytes32       // Parent agent ID
agent.fitness() -> u64             // Current fitness score

// Lifecycle
agent.spawn(genome: bytes) -> bytes32    // Create child agent
agent.terminate()                        // Self-terminate
agent.hibernate(blocks: u64)             // Sleep for N blocks

// Memory
agent.observe(key: bytes, value: bytes)  // Record observation
agent.recall(key: bytes) -> bytes        // Retrieve memory
agent.forget(key: bytes)                 // Delete memory
agent.memory_size() -> u64               // Current memory usage
```

### 4.2 AI Prediction API

```x3script
// Model invocation
ai.predict(features: tensor) -> Prediction
ai.classify(input: bytes) -> (class: u32, confidence: prob)
ai.embed(text: bytes) -> tensor<768>
ai.similarity(a: tensor, b: tensor) -> prob

// Model management
ai.load_model(model_id: bytes32) -> bool
ai.set_temperature(temp: prob)
ai.set_top_k(k: u32)

// Prediction result structure
struct Prediction {
    action: Action;
    confidence: prob;      // 0.0 to 1.0
    reasoning: bytes;      // Optional explanation
    alternatives: array[Action; 3];
}
```

### 4.3 Evolution API

```x3script
// Mutation
ai.mutate(fn_id: u32) -> u32              // Mutate function, return new ID
ai.crossover(parent_a: bytes32, parent_b: bytes32) -> bytes32
ai.evaluate(strategy_id: u32) -> u64      // Evaluate fitness

// Selection
ai.tournament(pool: array[bytes32], k: u32) -> bytes32
ai.roulette(pool: array[bytes32], fitness: array[u64]) -> bytes32

// Population management
ai.population_size() -> u32
ai.best_agent() -> bytes32
ai.worst_agent() -> bytes32
ai.average_fitness() -> u64
```

### 4.4 Coordination API

```x3script
// Swarm communication
swarm.broadcast(message: bytes)
swarm.receive() -> array[Message]
swarm.peers() -> array[bytes32]

// Consensus
swarm.propose(action: Action) -> bytes32
swarm.vote(proposal_id: bytes32, vote: bool)
swarm.execute_if_passed(proposal_id: bytes32)

// Resource sharing
swarm.share_memory(key: bytes, value: bytes)
swarm.request_memory(peer: bytes32, key: bytes) -> bytes
```

---

## 5. Evolution Engine

### 5.1 Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    EVOLUTION ENGINE                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  MUTATION   │───▶│  EVALUATION │───▶│  SELECTION  │     │
│  │   Engine    │    │    Arena    │    │   Filter    │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Genetic   │    │  Fitness    │    │ Population  │     │
│  │  Operators  │    │   Scoring   │    │   Control   │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Mutation Operators

| Operator           | Description               | MIR Impact         |
| ------------------ | ------------------------- | ------------------ |
| `point_mutation`   | Change single instruction | Opcode swap        |
| `block_swap`       | Swap code blocks          | CFG restructure    |
| `constant_tweak`   | Adjust numeric constants  | Immediate change   |
| `function_inline`  | Inline function call      | Code expansion     |
| `loop_unroll`      | Unroll loop iteration     | Block duplication  |
| `branch_flip`      | Invert branch condition   | Predicate negation |
| `dead_code_inject` | Add no-op code            | Neutral mutation   |
| `crossover`        | Combine two genomes       | Multi-parent merge |

### 5.3 Mutation Safety Rules

```x3script
// Mutations must preserve:
// 1. Type safety
// 2. Memory bounds
// 3. Gas bounds
// 4. Determinism

@mutation_safe
fn compute_spread(a: u64, b: u64) -> u64 {
    // This function can be safely mutated
    return b - a;
}

@mutation_frozen
fn security_check(caller: address) -> bool {
    // This function cannot be mutated
    return caller == ADMIN;
}
```

### 5.4 Fitness Functions

```x3script
// Default fitness function
fn default_fitness(agent: Agent) -> u64 {
    let pnl_score = agent.stats.total_pnl * 100;
    let risk_penalty = agent.stats.max_drawdown * 50;
    let efficiency = agent.stats.gas_used / agent.stats.trades;
    
    return pnl_score - risk_penalty - efficiency;
}

// Custom fitness function
strategy MyFitness {
    fn evaluate(agent: Agent) -> u64 {
        // Sharpe ratio focused
        let returns = agent.stats.returns;
        let volatility = agent.stats.volatility;
        
        if volatility == 0 {
            return 0;
        }
        
        return (returns * 1000) / volatility;
    }
}
```

### 5.5 Selection Strategies

```x3script
// Tournament selection
fn tournament_select(population: array[Agent], k: u32) -> Agent {
    let best: Agent = population[random() % population.len()];
    
    for i in 1..k {
        let candidate = population[random() % population.len()];
        if candidate.fitness > best.fitness {
            best = candidate;
        }
    }
    
    return best;
}

// Roulette wheel selection
fn roulette_select(population: array[Agent]) -> Agent {
    let total_fitness: u64 = 0;
    for agent in population {
        total_fitness += agent.fitness;
    }
    
    let threshold = random() % total_fitness;
    let cumulative: u64 = 0;
    
    for agent in population {
        cumulative += agent.fitness;
        if cumulative >= threshold {
            return agent;
        }
    }
    
    return population[0];
}
```

### 5.6 Evolution Cycle

```
┌─────────────────────────────────────────────────────────────┐
│                   EVOLUTION CYCLE                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. INITIALIZE                                              │
│     └── Create initial population (random or seeded)       │
│                                                             │
│  2. EVALUATE                                                │
│     └── Run each agent in simulation environment           │
│     └── Compute fitness scores                             │
│                                                             │
│  3. SELECT                                                  │
│     └── Choose parents based on fitness                    │
│     └── Apply elitism (keep top N)                         │
│                                                             │
│  4. REPRODUCE                                               │
│     └── Crossover: combine parent genomes                  │
│     └── Mutation: random modifications                     │
│                                                             │
│  5. REPLACE                                                 │
│     └── New generation replaces old                        │
│     └── Archive best performers                            │
│                                                             │
│  6. REPEAT                                                  │
│     └── Continue until convergence or generation limit     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Strategy System

### 6.1 Declarative Strategies

```x3script
strategy FlashLoanArb {
    // What to watch
    watch dex.uniswap.v3
    watch dex.sushiswap
    watch dex.curve
    
    // Trigger conditions
    when price_diff(uniswap, sushiswap) > 0.005 => {
        let profit = calculate_arb_profit(uniswap, sushiswap);
        if profit > MIN_PROFIT {
            execute FlashArb(uniswap, sushiswap, profit);
        }
    }
    
    when gas_price < 20 gwei => {
        // Low gas = good time for complex operations
        execute BatchRebalance();
    }
}
```

### 6.2 Event Triggers

| Trigger                 | Description                    |
| ----------------------- | ------------------------------ |
| `when condition => { }` | Execute when condition is true |
| `on event => { }`       | Execute when event is emitted  |
| `every N blocks => { }` | Execute periodically           |
| `at timestamp => { }`   | Execute at specific time       |
| `after delay => { }`    | Execute after duration         |

### 6.3 Strategy Composition

```x3script
// Base strategy
strategy BaseArb {
    fn find_opportunity() -> Option<Opportunity>;
    fn calculate_profit(opp: Opportunity) -> u64;
}

// Composed strategy
strategy AdvancedArb extends BaseArb {
    // Override with better implementation
    fn find_opportunity() -> Option<Opportunity> {
        // Use ML model for detection
        let prediction = ai.predict(market_state);
        if prediction.confidence > 0.9 {
            return Some(prediction.opportunity);
        }
        return None;
    }
    
    // Add new functionality
    fn hedge_position(opp: Opportunity) {
        // Implement hedging logic
    }
}
```

---

## 7. AI Primitives & Types

### 7.1 Type Definitions

```x3script
// Probability type (0.0 to 1.0, stored as u64 fixed-point)
type prob = u64;  // 1e18 = 1.0

// Tensor type (fixed-size multidimensional array)
type tensor<N> = array[f32; N];  // Only in simulation mode

// Action enum (agent decisions)
enum Action {
    Wait,
    Swap { amount: u64, path: array[address] },
    Liquidate { borrower: address, amount: u64 },
    Provide { token_a: u64, token_b: u64 },
    Remove { liquidity: u64 },
    Custom { data: bytes }
}

// Observation struct
struct Observation {
    timestamp: u64;
    block: u64;
    prices: array[u64; 16];
    volumes: array[u64; 16];
    features: tensor<256>;
}
```

### 7.2 VM-Specific Types

```x3script
// EVM types
type evm.slot = u256;       // Storage slot
type evm.log = bytes;       // Event log
type evm.calldata = bytes;  // Call data

// SVM types  
type svm.account = bytes32;  // Account pubkey
type svm.cpi = bytes;        // Cross-program invocation
type svm.lamports = u64;     // SOL amount
```

### 7.3 AI Annotation Types

```x3script
// Hint annotation
@ai.hint("optimize for gas")
fn expensive_operation() { ... }

// Model binding
@ai.model("llama3-70b")
fn make_decision() { ... }

// Mutation constraints
@ai.mutable(rate: 0.1)  // 10% mutation rate
fn evolvable_logic() { ... }

@ai.frozen  // Never mutate
fn critical_security() { ... }
```

---

## 8. Predictive Execution

### 8.1 Probability-Aware Branching

```x3script
fn liquidation_check(borrower: address) {
    // Get health factor with confidence
    let health = aave.get_health_factor(borrower);
    
    // AI prediction of imminent liquidation
    let p: prob = ai.predict_liquidation(borrower, health);
    
    // Probabilistic execution
    if p > 0.9 {
        // High confidence — execute immediately
        execute_liquidation(borrower);
    } elif p > 0.7 {
        // Medium confidence — prepare but wait
        prepare_liquidation(borrower);
        schedule_check(blocks: 1);
    } else {
        // Low confidence — just monitor
        add_to_watchlist(borrower);
    }
}
```

### 8.2 Speculative Branching

```x3script
fn speculative_arb() {
    // Execute multiple paths in parallel (simulation)
    branch speculative {
        path A {
            try_uniswap_arb();
        }
        path B {
            try_curve_arb();
        }
        path C {
            try_balancer_arb();
        }
    }
    
    // Runtime selects best successful path
    // Failed paths are discarded
}
```

### 8.3 Quantum-Like Superposition

```x3script
fn quantum_strategy() {
    // Create superposition of strategies
    let strategies = superposition [
        conservative_strategy(),
        moderate_strategy(),
        aggressive_strategy()
    ];
    
    // Collapse based on market conditions
    let selected = collapse(strategies, market_signal);
    
    // Execute selected strategy
    execute(selected);
}
```

---

## 9. Multi-Agent Coordination

### 9.1 Swarm Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SWARM TOPOLOGY                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│         ┌─────────┐                                        │
│         │ Leader  │                                        │
│         │ Agent   │                                        │
│         └────┬────┘                                        │
│              │                                             │
│     ┌────────┼────────┐                                   │
│     │        │        │                                   │
│  ┌──▼──┐  ┌──▼──┐  ┌──▼──┐                               │
│  │Scout│  │Scout│  │Scout│    ← Observation Layer         │
│  │  A  │  │  B  │  │  C  │                               │
│  └──┬──┘  └──┬──┘  └──┬──┘                               │
│     │        │        │                                   │
│     └────────┼────────┘                                   │
│              │                                             │
│         ┌────▼────┐                                        │
│         │Executor │    ← Action Layer                      │
│         │  Agent  │                                        │
│         └─────────┘                                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 9.2 Swarm Communication Protocol

```x3script
// Message types
enum SwarmMessage {
    Observation { data: bytes },
    Proposal { action: Action, confidence: prob },
    Vote { proposal_id: bytes32, approve: bool },
    Execute { action: Action },
    Heartbeat { timestamp: u64 }
}

// Leader agent
agent SwarmLeader {
    storage {
        scouts: array[bytes32; 8];
        proposals: map[bytes32, Proposal];
        votes: map[bytes32, array[bool]];
    }
    
    fn collect_observations() -> array[Observation] {
        let observations: array[Observation; 8];
        for i in 0..scouts.len() {
            observations[i] = swarm.request_memory(
                scouts[i], 
                "latest_observation"
            );
        }
        return observations;
    }
    
    fn aggregate_and_decide(obs: array[Observation]) -> Action {
        let consensus = ai.aggregate(obs);
        let action = ai.decide(consensus);
        
        // Propose to swarm
        let proposal_id = swarm.propose(action);
        
        // Wait for votes
        schedule_vote_collection(proposal_id, blocks: 2);
        
        return action;
    }
}
```

### 9.3 Consensus Mechanisms

```x3script
// Simple majority
fn simple_majority(votes: array[bool]) -> bool {
    let yes_count: u32 = 0;
    for vote in votes {
        if vote {
            yes_count += 1;
        }
    }
    return yes_count > votes.len() / 2;
}

// Weighted voting (by stake or fitness)
fn weighted_vote(votes: array[(bool, u64)]) -> bool {
    let yes_weight: u64 = 0;
    let total_weight: u64 = 0;
    
    for (vote, weight) in votes {
        total_weight += weight;
        if vote {
            yes_weight += weight;
        }
    }
    
    return yes_weight > total_weight / 2;
}

// Conviction voting (time-weighted)
fn conviction_vote(votes: array[(bool, u64, u64)]) -> bool {
    // (vote, weight, duration)
    let conviction_threshold: u64 = 1000000;
    let conviction: u64 = 0;
    
    for (vote, weight, duration) in votes {
        if vote {
            conviction += weight * duration;
        }
    }
    
    return conviction > conviction_threshold;
}
```

---

## 10. Agent Lifecycle

### 10.1 Lifecycle States

```
┌─────────────────────────────────────────────────────────────┐
│                    AGENT LIFECYCLE                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  CREATED ──▶ INITIALIZING ──▶ ACTIVE ──▶ TERMINATED       │
│                   │              │                         │
│                   │              ▼                         │
│                   │         HIBERNATING                    │
│                   │              │                         │
│                   │              ▼                         │
│                   └──────▶ EVOLVING ──▶ ACTIVE            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 10.2 State Transitions

| From         | To           | Trigger                                    |
| ------------ | ------------ | ------------------------------------------ |
| CREATED      | INITIALIZING | `agent.init()` called                      |
| INITIALIZING | ACTIVE       | Initialization complete                    |
| ACTIVE       | HIBERNATING  | `agent.hibernate(n)` called                |
| HIBERNATING  | ACTIVE       | N blocks passed                            |
| ACTIVE       | EVOLVING     | Evolution cycle triggered                  |
| EVOLVING     | ACTIVE       | Mutation complete                          |
| ACTIVE       | TERMINATED   | `agent.terminate()` or fitness < threshold |
| ANY          | TERMINATED   | Owner kills agent                          |

### 10.3 Lifecycle Hooks

```x3script
agent LifecycleDemo {
    // Called once on creation
    fn on_create() {
        log("Agent created");
        initialize_memory();
    }
    
    // Called each block when active
    fn on_tick() {
        let obs = observe();
        let action = decide(obs);
        execute(action);
    }
    
    // Called before hibernation
    fn on_hibernate() {
        save_state();
        log("Going to sleep");
    }
    
    // Called after waking from hibernation
    fn on_wake() {
        restore_state();
        log("Woke up");
    }
    
    // Called before evolution
    fn on_evolve_start() {
        backup_genome();
    }
    
    // Called after evolution
    fn on_evolve_complete(mutated: bool) {
        if mutated {
            log("Genome updated");
        }
    }
    
    // Called before termination
    fn on_terminate() {
        withdraw_funds();
        log("Goodbye");
    }
}
```

---

## 11. Memory & State Model

### 11.1 Agent Memory Tiers

```
┌─────────────────────────────────────────────────────────────┐
│                    AGENT MEMORY MODEL                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  EPISODIC MEMORY (memory.jsonl)                            │
│  ├── Recent observations                                   │
│  ├── Action history                                        │
│  ├── Outcome records                                       │
│  └── Decays over time                                      │
│                                                             │
│  SEMANTIC MEMORY (embeddings.bin)                          │
│  ├── Learned patterns                                      │
│  ├── Market embeddings                                     │
│  ├── Strategy embeddings                                   │
│  └── Persists across generations                           │
│                                                             │
│  PROCEDURAL MEMORY (code.x3bc)                             │
│  ├── Compiled bytecode                                     │
│  ├── Evolved through mutations                             │
│  └── Core decision logic                                   │
│                                                             │
│  WORKING MEMORY (runtime)                                  │
│  ├── Current observation                                   │
│  ├── Active computations                                   │
│  └── Cleared each tick                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 11.2 Memory Operations

```x3script
// Episodic memory
agent.remember(event: Event) {
    memory.episodic.append({
        timestamp: system.timestamp(),
        event: event,
        context: current_context()
    });
}

agent.recall_recent(n: u32) -> array[Event] {
    return memory.episodic.tail(n);
}

agent.forget_old(age: u64) {
    memory.episodic.prune(before: system.timestamp() - age);
}

// Semantic memory
agent.learn_pattern(pattern: tensor, label: bytes) {
    memory.semantic.store(pattern, label);
}

agent.recognize(input: tensor) -> (label: bytes, similarity: prob) {
    return memory.semantic.nearest(input);
}

// Cross-generation memory transfer
agent.inherit_memory(parent: bytes32, keys: array[bytes]) {
    for key in keys {
        let value = swarm.request_memory(parent, key);
        memory.semantic.store(key, value);
    }
}
```

### 11.3 Dual-State Model (EVM + SVM)

```x3script
// Agents see both VM states
agent CrossVMAgent {
    fn get_balances(user: address) -> (evm_balance: u256, svm_balance: u64) {
        // EVM state
        let evm_bal = evm.balance(user);
        
        // SVM state  
        let svm_account = derive_svm_account(user);
        let svm_bal = svm.get_lamports(svm_account);
        
        return (evm_bal, svm_bal);
    }
    
    fn sync_state(user: address) {
        let (evm_bal, svm_bal) = get_balances(user);
        
        // Write to unified state view
        storage.user_balance[user] = evm_bal + svm_bal * CONVERSION_RATE;
    }
}
```

---

## 12. Security Model

### 12.1 Capability System

```x3script
// Capabilities are explicitly granted
capabilities {
    evm.call: true,
    evm.sstore: false,      // No direct storage writes
    svm.invoke: true,
    svm.transfer: limited(1000),  // Max 1000 lamports per tx
    ai.mutate: true,
    ai.spawn: limited(3),   // Max 3 children
}

// Capability check at runtime
fn transfer_funds(amount: u64) {
    require_capability("evm.call");
    require_capability("svm.transfer", amount);
    
    // Execute if capabilities present
    ...
}
```

### 12.2 Sandbox Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    SANDBOX MODEL                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  AGENT SANDBOX                                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ✓ Own storage read/write                              │ │
│  │ ✓ Own memory access                                   │ │
│  │ ✓ Hostcall execution (if capable)                     │ │
│  │ ✓ Child agent spawning (if capable)                   │ │
│  │ ✗ Direct runtime access                               │ │
│  │ ✗ Other agent memory access                           │ │
│  │ ✗ Privileged syscalls                                 │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  SWARM SANDBOX                                              │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ ✓ Broadcast messages                                  │ │
│  │ ✓ Request shared memory (with permission)             │ │
│  │ ✓ Participate in voting                               │ │
│  │ ✗ Force other agents to act                           │ │
│  │ ✗ Read private agent state                            │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 12.3 Mutation Safety

```x3script
// Mutation safety annotations
@mutation_safe        // Can be mutated freely
@mutation_constrained // Can be mutated within bounds
@mutation_frozen      // Cannot be mutated

// Safety verification
fn verify_mutation_safety(original: MIR, mutated: MIR) -> bool {
    // Type safety preserved
    if !type_check(mutated) { return false; }
    
    // Memory bounds preserved
    if !memory_safe(mutated) { return false; }
    
    // Gas bounds preserved
    if gas_estimate(mutated) > gas_estimate(original) * 2 {
        return false;
    }
    
    // Determinism preserved
    if !is_deterministic(mutated) { return false; }
    
    // Security invariants preserved
    for invariant in security_invariants {
        if !check_invariant(mutated, invariant) {
            return false;
        }
    }
    
    return true;
}
```

---

## 13. Standard Agent Patterns

### 13.1 Arbitrage Agent

```x3script
agent ArbAgent {
    model "gpt-4-turbo"
    risk 0.70
    
    capabilities {
        dex.swap: true,
        system.flashloan: true,
    }
    
    storage {
        min_profit: u64;
        total_profit: u128;
    }
    
    fn observe() -> ArbOpportunity {
        let prices_uni = dex.get_prices("uniswap", tokens);
        let prices_sushi = dex.get_prices("sushiswap", tokens);
        
        for i in 0..tokens.len() {
            let spread = abs_diff(prices_uni[i], prices_sushi[i]);
            if spread > storage.min_profit {
                return Some(ArbOpportunity {
                    token: tokens[i],
                    buy_venue: if prices_uni[i] < prices_sushi[i] { "uniswap" } else { "sushiswap" },
                    sell_venue: if prices_uni[i] < prices_sushi[i] { "sushiswap" } else { "uniswap" },
                    spread: spread
                });
            }
        }
        return None;
    }
    
    fn execute(opp: ArbOpportunity) {
        system.flashloan(opp.token, LOAN_AMOUNT, || {
            dex.swap(opp.buy_venue, opp.token, LOAN_AMOUNT);
            let out = dex.swap(opp.sell_venue, opp.token, LOAN_AMOUNT);
            let profit = out - LOAN_AMOUNT;
            storage.total_profit += profit;
        });
    }
}
```

### 13.2 Liquidation Agent

```x3script
agent LiquidatorAgent {
    model "llama3-70b"
    risk 0.85
    
    storage {
        watched_protocols: array[address; 4];
        min_profit: u64;
    }
    
    fn observe() -> array[LiquidationTarget] {
        let targets: array[LiquidationTarget; 32];
        let count: u32 = 0;
        
        for protocol in storage.watched_protocols {
            let at_risk = protocol.get_at_risk_positions();
            for pos in at_risk {
                if pos.health_factor < 1.05 {
                    targets[count] = LiquidationTarget {
                        protocol: protocol,
                        borrower: pos.borrower,
                        collateral: pos.collateral,
                        debt: pos.debt,
                        health: pos.health_factor
                    };
                    count += 1;
                }
            }
        }
        
        return targets[0..count];
    }
    
    fn decide(targets: array[LiquidationTarget]) -> Option<LiquidationTarget> {
        // Use AI to predict which will become liquidatable first
        let predictions = ai.batch_predict(targets);
        
        // Sort by (probability * profit)
        let best = predictions
            .filter(|p| p.confidence > 0.8)
            .max_by(|p| p.expected_profit);
            
        return best;
    }
    
    fn execute(target: LiquidationTarget) {
        target.protocol.liquidate(
            target.borrower,
            target.collateral,
            target.debt
        );
    }
}
```

### 13.3 Market Maker Agent

```x3script
agent MarketMakerAgent {
    model "custom-mm-model"
    risk 0.50
    
    storage {
        pair: (address, address);
        spread: u64;
        inventory: (u64, u64);
        max_inventory: u64;
    }
    
    fn observe() -> MarketState {
        return MarketState {
            mid_price: dex.price(storage.pair),
            our_inventory: storage.inventory,
            order_flow: get_recent_trades(100),
            volatility: compute_volatility(24)
        };
    }
    
    fn decide(state: MarketState) -> (Option<Order>, Option<Order>) {
        // Adjust spread based on inventory and volatility
        let adjusted_spread = storage.spread * (1 + state.volatility / 100);
        
        // Skew prices based on inventory
        let skew = compute_inventory_skew(state.our_inventory, storage.max_inventory);
        
        let bid_price = state.mid_price - adjusted_spread / 2 - skew;
        let ask_price = state.mid_price + adjusted_spread / 2 - skew;
        
        let bid = if state.our_inventory.0 < storage.max_inventory {
            Some(Order::Bid { price: bid_price, size: ORDER_SIZE })
        } else { None };
        
        let ask = if state.our_inventory.1 < storage.max_inventory {
            Some(Order::Ask { price: ask_price, size: ORDER_SIZE })
        } else { None };
        
        return (bid, ask);
    }
    
    fn execute(bid: Option<Order>, ask: Option<Order>) {
        if let Some(b) = bid {
            dex.place_order(storage.pair, b);
        }
        if let Some(a) = ask {
            dex.place_order(storage.pair, a);
        }
    }
}
```

---

## Appendix A: Complete EBNF Grammar

```ebnf
Program          = { TopLevelDecl } ;

TopLevelDecl     = ContractDecl | AgentDecl | StrategyDecl | KernelDecl ;

AgentDecl        = "agent" Identifier "{" AgentBody "}" ;
AgentBody        = { AgentProp | StorageDecl | FunctionDecl | CapabilityDecl } ;
AgentProp        = "model" StringLiteral
                 | "risk" NumberLiteral ;

StrategyDecl     = "strategy" Identifier "{" StrategyBody "}" ;
StrategyBody     = { WatchDecl | TriggerDecl | FunctionDecl } ;
WatchDecl        = "watch" Identifier ("." Identifier)* ;
TriggerDecl      = "when" Expression "=>" Block
                 | "on" Identifier "=>" Block
                 | "every" NumberLiteral "blocks" "=>" Block ;

CapabilityDecl   = "capabilities" "{" { CapabilityItem } "}" ;
CapabilityItem   = Identifier ("." Identifier)* ":" CapabilityValue ;
CapabilityValue  = "true" | "false" | "limited" "(" NumberLiteral ")" ;

AIAnnotation     = "@ai.hint" "(" StringLiteral ")"
                 | "@ai.model" "(" StringLiteral ")"
                 | "@ai.mutable" "(" "rate" ":" NumberLiteral ")"
                 | "@ai.frozen"
                 | "@mutation_safe"
                 | "@mutation_constrained"
                 | "@mutation_frozen" ;

AIExpression     = "ai.predict" "(" Expression ")"
                 | "ai.classify" "(" Expression ")"
                 | "ai.embed" "(" Expression ")"
                 | "ai.mutate" "(" Expression ")"
                 | "ai.spawn" "(" Expression ")"
                 | "ai.eval" "(" Expression ")" ;

SwarmExpression  = "swarm.broadcast" "(" Expression ")"
                 | "swarm.receive" "(" ")"
                 | "swarm.propose" "(" Expression ")"
                 | "swarm.vote" "(" Expression "," Expression ")" ;

SpeculativeBranch = "branch" "speculative" "{" { PathDecl } "}" ;
PathDecl         = "path" Identifier Block ;

SuperpositionExpr = "superposition" "[" ExpressionList "]" ;
CollapseExpr      = "collapse" "(" Expression "," Expression ")" ;
```

---

## Appendix B: Quick Reference

```
┌─────────────────────────────────────────────────────────────┐
│                 AI AGENT API QUICK REFERENCE                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  PROGRAM UNITS                                              │
│  contract { storage, fn }       Traditional smart contract  │
│  agent { model, risk, fn }      Autonomous AI actor         │
│  strategy { watch, when }       Declarative triggers        │
│  kernel { fn }                  Pure computation            │
│                                                             │
│  AGENT RUNTIME                                              │
│  agent.id() agent.generation() agent.fitness()             │
│  agent.spawn() agent.terminate() agent.hibernate()         │
│  agent.observe() agent.recall() agent.forget()             │
│                                                             │
│  AI API                                                     │
│  ai.predict() ai.classify() ai.embed() ai.similarity()     │
│  ai.mutate() ai.crossover() ai.evaluate()                  │
│  ai.tournament() ai.roulette()                             │
│                                                             │
│  SWARM API                                                  │
│  swarm.broadcast() swarm.receive() swarm.peers()           │
│  swarm.propose() swarm.vote() swarm.execute_if_passed()    │
│                                                             │
│  EVOLUTION                                                  │
│  @mutation_safe    Can mutate freely                       │
│  @mutation_frozen  Cannot mutate                           │
│  @ai.mutable(rate) Controlled mutation rate                │
│                                                             │
│  LIFECYCLE                                                  │
│  CREATED → ACTIVE → HIBERNATING → EVOLVING → TERMINATED   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

**Document Version:** 1.0.0  
**Specification Status:** Canonical  
**Maintainer:** X3 Chain Core Engineering
