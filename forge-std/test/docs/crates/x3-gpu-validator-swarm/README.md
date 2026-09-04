# X3 GPU Validator Swarm

A deterministic GPU validator swarm with CPU verification, replay mode, and quarantine/fallback mechanisms for production deployments.

## Features

- **Deterministic GPU Execution**: Bit-for-bit deterministic outputs guaranteed by CPU verification
- **CPU Verification**: Every GPU result is verified by CPU with replay mode
- **Quarantine System**: Automatic isolation of misbehaving validators
- **Fallback Mechanism**: Automatic CPU fallback on divergence detection
- **Swarm Orchestration**: Coordinate multiple validators with load balancing
- **Easy Onboarding**: One-command install/run/join/bench flows
- **JSON Benchmarks**: Machine-readable performance reports
- **Full Telemetry**: Prometheus metrics and health monitoring

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    X3 GPU Validator Swarm                                │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │                     Swarm Orchestrator                               │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐│ │
│  │  │ Task Queue  │  │ Scheduler   │  │ Verification Engine         ││ │
│  │  └──────┬──────┘  └──────┬──────┘  └───────────┬───────────────┘│ │
│  └─────────┼────────────────┼─────────────────────┼──────────────────┘ │
│            │                │                     │                     │
│      ┌─────▼─────┬──────────▼──────────┬──────────▼─────┐            │
│      │           │                     │                │            │
│  ┌───▼───┐   ┌───▼───┐            ┌────▼────┐      ┌────▼────┐       │
│  │Validator│  │Validator│   ...    │Validator│      │Validator│       │
│  │GPU:A   │  │GPU:B   │            │GPU:X    │      │GPU:Y    │       │
│  └───┬───┘   └───┬───┘            └────┬────┘      └────┬────┘       │
│      │           │                     │                │             │
│      └───────────┴─────────────────────┴────────────────┘             │
│                          │                                             │
│               ┌──────────▼───────────┐                                 │
│               │  CPU Verification    │                                 │
│               │  + Replay Mode      │                                 │
│               └─────────────────────┘                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### One-Command Installation

```bash
# Clone the repository
git clone https://github.com/x3-chain/x3-chain.git
cd x3-chain

# Install and build
cd crates/x3-gpu-validator-swarm
./scripts/x3-swarm.sh install
```

### One-Command Run

```bash
# Run the validator
./scripts/x3-swarm.sh run --validator-id my-validator
```

### One-Command Join

```bash
# Join the swarm
./scripts/x3-swarm.sh join --orchestrator https://orchestrator.x3.io
```

### One-Command Benchmark

```bash
# Run benchmarks
./scripts/x3-swarm.sh bench
```

## Usage

### Validator Commands

```bash
# Install dependencies
cargo build --release

# Run validator
cargo run --release --bin x3-validator -- run

# Run benchmark
cargo run --release --bin x3-bench -- run

# Show status
cargo run --release --bin x3-validator -- status

# Run tests
cargo test --release
```

### Orchestrator Commands

```bash
# Run orchestrator
cargo run --release --bin x3-swarm-orchestrator -- run

# Show swarm status
cargo run --release --bin x3-swarm-orchestrator -- status

# Run swarm benchmark
cargo run --release --bin x3-swarm-orchestrator -- benchmark
```

## Configuration

### Validator Configuration

Create a `config.toml` file:

```toml
[identity]
keypair_path = "~/.x3-validator/validator.key"
display_name = "My Validator"
region = "us-west"

[network]
listen_addresses = ["/ip4/0.0.0.0/tcp/30334"]
bootstrap_nodes = []
max_peers = 50

[gpu]
enable_cuda = true
deterministic_mode = true

[verification]
cpu_verification_enabled = true
replay_mode_enabled = true
max_replay_attempts = 3
verification_level = "Standard"

[quarantine]
enabled = true
quarantine_duration_secs = 1800
max_divergence_count = 3
auto_fallback_cpu = true

[telemetry]
enabled = true
endpoint = "https://telemetry.x3.io"
interval_secs = 30
```

## Benchmark Reports

Benchmarks produce JSON reports with detailed performance metrics:

```bash
# Run benchmark
cargo run --release --bin x3-bench -- run

# View report
cargo run --release --bin x3-bench -- report
```

Sample report:

```json
{
  "version": "1.0",
  "benchmark": "x3-gpu-validator",
  "hardware": {
    "cpu_cores": 8,
    "memory_mb": 16384,
    "gpu_available": true
  },
  "summary": {
    "total_operations": 50500,
    "total_time_ms": 1250.5,
    "avg_throughput": 40360.5,
    "peak_throughput": 125000.0
  }
}
```

## Testing

Run all tests:

```bash
cargo test --release
```

Run specific test:

```bash
cargo test test_deterministic_engine_basic
```

## Modules

- `config` - Configuration management
- `crypto` - Cryptographic operations (keccak256, sha256, blake2b)
- `deterministic` - Deterministic execution engine with CPU verification
- `quarantine` - Validator isolation on divergence
- `metrics` - Metrics collection and reporting
- `health` - Health monitoring
- `telemetry` - Telemetry and event recording
- `validator` - Individual validator implementation
- `orchestrator` - Swarm orchestration
- `network` - P2P networking

## API

### Validator API

```rust
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    deterministic::{DeterministicTask, TaskType},
    crypto::HashAlgorithm,
    validator::Validator,
};

// Create validator
let config = SwarmConfig::default();
let validator = Validator::new(config, "validator-1".to_string());

// Initialize
validator.initialize().unwrap();

// Process task
let task = DeterministicTask::new(
    TaskType::BatchHash,
    vec![b"hello world".to_vec()],
    HashAlgorithm::Keccak256,
);
let result = validator.process_task(task);

// Get metrics
let metrics = validator.get_metrics();
```

### Orchestrator API

```rust
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    orchestrator::SwarmOrchestrator,
    deterministic::{DeterministicTask, TaskType},
    crypto::HashAlgorithm,
    validator::Validator,
};
use std::sync::Arc;

// Create orchestrator
let config = SwarmConfig::default();
let mut orchestrator = SwarmOrchestrator::new(config);

// Register validators
for i in 0..4 {
    let validator = Arc::new(Validator::new(
        SwarmConfig::default(),
        format!("validator-{}", i),
    ));
    validator.initialize().unwrap();
    orchestrator.register_validator(validator);
}

// Submit tasks
for i in 0..100 {
    let task = DeterministicTask::new(
        TaskType::BatchHash,
        vec![format!("data {}", i).into_bytes()],
        HashAlgorithm::Keccak256,
    );
    orchestrator.submit_task(task);
}

// Process tasks
orchestrator.process_pending_tasks();

// Get metrics
let metrics = orchestrator.get_swarm_metrics();
```

## Performance

Typical performance on modern hardware:

| Batch Size | Time (ms) | Throughput |
|------------|-----------|------------|
| 1          | 0.05      | 20,000/s   |
| 10         | 0.15      | 66,000/s   |
| 100        | 0.8       | 125,000/s  |
| 1000       | 7.5       | 133,000/s  |
| 10000      | 75        | 133,000/s  |

## Security

- All GPU outputs are verified by CPU before acceptance
- Divergence triggers automatic quarantine
- Automatic CPU fallback ensures continuity
- Cryptographic signing of all task results

## License

MIT
