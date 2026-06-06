# X3 GPU Validator Swarm - Operator's Manual

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Operation](#operation)
6. [Monitoring](#monitoring)
7. [Troubleshooting](#troubleshooting)
8. [Security](#security)
9. [API Reference](#api-reference)
10. [Advanced Topics](#advanced-topics)

---

## 1. Introduction

The X3 GPU Validator Swarm is a distributed computing network that provides:

- **Deterministic GPU Execution**: Bit-for-bit reproducible results
- **CPU Verification**: Every GPU result verified by CPU
- **Replay Mode**: Detects divergence between GPU executions
- **Quarantine System**: Isolates misbehaving validators
- **Automatic Fallback**: CPU mode when GPU fails

### Core Concepts

| Term | Description |
|------|-------------|
| Validator | Individual node that processes tasks |
| Orchestrator | Coordinator that distributes tasks |
| Task | Unit of work (hash, sign, verify) |
| Divergence | GPU output differs from CPU |
| Quarantine | Isolation of faulty validator |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                      Swarm Orchestrator                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐│
│  │ Task Queue  │  │ Scheduler   │  │ Verification Engine         ││
│  └──────┬──────┘  └──────┬──────┘  └───────────┬───────────────┘│
└─────────┼────────────────┼─────────────────────┼──────────────────┘
          │                │                     │
    ┌─────▼─────┐    ┌─────▼─────┐        ┌─────▼─────┐
    │ Validator │    │ Validator │  ...   │ Validator │
    │   GPU:A   │    │   GPU:B   │        │   GPU:N   │
    └─────┬─────┘    └─────┬─────┘        └─────┬─────┘
          │                │                     │
          └────────────────┴─────────────────────┘
                          │
              ┌───────────▼───────────┐
              │  CPU Verification    │
              │  + Replay Mode       │
              └─────────────────────┘
```

### Components

1. **Swarm Orchestrator**
   - Distributes tasks to validators
   - Monitors validator health
   - Manages task lifecycle
   - Collects results

2. **Validator**
   - Executes GPU workloads
   - Performs CPU verification
   - Reports metrics
   - Handles failures

3. **Deterministic Engine**
   - Ensures reproducible outputs
   - Runs GPU + CPU comparisons
   - Detects divergence
   - Triggers fallback

4. **Quarantine Manager**
   - Tracks divergence events
   - Isolates faulty validators
   - Manages recovery
   - Enforces penalties

---

## 3. Installation

### Prerequisites

- Rust 1.70+
- CUDA Toolkit (optional, for GPU support)
- 4GB RAM minimum
- 10GB disk space

### Quick Install

```bash
# Clone repository
git clone https://github.com/x3-chain/x3-chain.git
cd x3-chain

# Install and build
cd crates/x3-gpu-validator-swarm
./scripts/x3-swarm.sh install

# Verify installation
cargo run --release --bin x3-validator -- status
```

### Manual Build

```bash
# Build release binary
cargo build --release

# Or with CPU-only (no GPU)
cargo build --release --no-default-features
```

---

## 4. Configuration

### Configuration File

Create `config.toml`:

```toml
[identity]
keypair_path = "~/.x3-validator/validator.key"
display_name = "My Validator"
region = "us-west"
stake_amount = 1000

[network]
listen_addresses = ["/ip4/0.0.0.0/tcp/30334"]
bootstrap_nodes = [
    "/ip4/1.2.3.4/tcp/30334/p2p/..."
]
max_peers = 50
enable_mdns = true
enable_dht = true

[gpu]
enable_cuda = true
device_indices = [0, 1]  # Use GPUs 0 and 1
deterministic_mode = true
max_memory_mb = 8192

[verification]
cpu_verification_enabled = true
replay_mode_enabled = true
max_replay_attempts = 3
verification_level = "Standard"  # Basic, Standard, Strict

[quarantine]
enabled = true
quarantine_duration_secs = 1800  # 30 minutes
max_divergence_count = 3
auto_fallback_cpu = true

[telemetry]
enabled = true
endpoint = "https://telemetry.x3.io"
interval_secs = 30

[benchmark]
enabled = true
output_dir = "./benchmark-results"
iterations = 100
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `X3_VALIDATOR_ID` | Validator identifier | Auto-generated |
| `X3_CONFIG_PATH` | Config file path | `./config.toml` |
| `X3_LOG_LEVEL` | Logging level | `info` |
| `CUDA_VISIBLE_DEVICES` | GPU devices | All available |

---

## 5. Operation

### Starting the Validator

```bash
# Basic run
cargo run --release --bin x3-validator -- run

# With custom ID
cargo run --release --bin x3-validator -- run --validator-id my-validator

# CPU-only mode
cargo run --release --bin x3-validator -- run --cpu-only
```

### Starting the Orchestrator

```bash
# Run orchestrator
cargo run --release --bin x3-swarm-orchestrator -- run

# Add validators
cargo run --release --bin x3-swarm-orchestrator -- add-validator

# Run benchmark
cargo run --release --bin x3-swarm-orchestrator -- benchmark
```

### One-Command Flows

```bash
# Install
./scripts/x3-swarm.sh install

# Run validator
./scripts/x3-swarm.sh run --validator-id my-validator

# Join swarm
./scripts/x3-swarm.sh join --orchestrator https://orchestrator.x3.io

# Run benchmarks
./scripts/x3-swarm.sh bench

# Check status
./scripts/x3-swarm.sh status

# Run tests
./scripts/x3-swarm.sh test
```

---

## 6. Monitoring

### Status Command

```bash
# Validator status
cargo run --release --bin x3-validator -- status

# Example output:
# X3 GPU Validator Status
# ========================
# Validator ID: validator-123
# State: Running
# Uptime: 2h 34m 15s
# Health: Healthy
#
# Metrics:
#   Total tasks: 15000
#   Successful: 14995
#   Failed: 3
#   Divergent: 2
#   CPU fallbacks: 5
```

### Metrics

The system exports Prometheus-compatible metrics:

| Metric | Description |
|--------|-------------|
| `x3_tasks_total` | Total tasks processed |
| `x3_tasks_success` | Successful tasks |
| `x3_tasks_failed` | Failed tasks |
| `x3_tasks_divergent` | Divergent results |
| `x3_cpu_fallbacks` | CPU fallback count |
| `x3_task_latency_ms` | Average latency |
| `x3_validators_total` | Total validators |
| `x3_validators_active` | Active validators |

### JSON Reports

```bash
# Export metrics as JSON
cargo run --release --bin x3-validator -- status | jq

# Export orchestrator state
cargo run --release --bin x3-swarm-orchestrator -- status | jq
```

---

## 7. Troubleshooting

### Common Issues

#### Issue: GPU Not Detected

```bash
# Check CUDA
nvcc --version

# Check devices
nvidia-smi

# Set visible devices
export CUDA_VISIBLE_DEVICES=0
```

#### Issue: Divergence Detected

1. Check GPU hardware
2. Enable CPU fallback: `auto_fallback_cpu = true`
3. Review divergence logs
4. Consider GPU replacement

#### Issue: Validator Quarantined

```bash
# Check quarantine status
cargo run --release --bin x3-validator -- status

# If temporary, wait for expiry
# If permanent, investigate root cause
```

#### Issue: High Latency

- Check network connectivity
- Reduce concurrent tasks
- Enable CPU fallback
- Upgrade hardware

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
cargo run --release --bin x3-validator -- run
```

---

## 8. Security

### Key Management

- Store keys securely (HSM recommended)
- Use encrypted key storage
- Rotate keys regularly

### Network Security

- Use TLS for remote connections
- Enable peer authentication
- Restrict bootstrap nodes

### Monitoring

- Monitor for anomalous behavior
- Set up alerts for divergences
- Track validator reputation

---

## 9. API Reference

### Validator API

```rust
// Create validator
let config = SwarmConfig::default();
let validator = Validator::new(config, "validator-1".to_string());

// Initialize
validator.initialize().unwrap();

// Process task
let task = DeterministicTask::new(
    TaskType::BatchHash,
    vec![b"data".to_vec()],
    HashAlgorithm::Keccak256,
);
let result = validator.process_task(task);

// Get metrics
let metrics = validator.get_metrics();

// Check health
let health = validator.health_status();
```

### Orchestrator API

```rust
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
let task = DeterministicTask::new(
    TaskType::BatchHash,
    vec![b"data".to_vec()],
    HashAlgorithm::Keccak256,
);
let task_id = orchestrator.submit_task(task);

// Process tasks
orchestrator.process_pending_tasks();

// Get result
let result = orchestrator.get_task_result(&task_id);
```

---

## 10. Advanced Topics

### Custom Verification Levels

```toml
[verification]
verification_level = "Strict"  # Verifies all with multiple algorithms
```

| Level | Description |
|-------|-------------|
| Basic | Verify first and last only |
| Standard | Verify all results |
| Strict | Verify with multiple algorithms |

### Load Balancing Strategies

```rust
// Set assignment strategy
orchestrator.set_assignment_strategy(AssignmentStrategy::LeastLoaded);
```

| Strategy | Description |
|----------|-------------|
| RoundRobin | Distribute evenly |
| LeastLoaded | Assign to lowest load |
| Random | Random assignment |
| Priority | Priority-based |

### Benchmarking

```bash
# Run benchmark
cargo run --release --bin x3-bench -- run

# View report
cargo run --release --bin x3-bench -- report
```

### Integration

```rust
use x3_gpu_validator_swarm::{
    config::SwarmConfig,
    deterministic::{DeterministicTask, TaskType},
    crypto::HashAlgorithm,
    validator::Validator,
};

fn main() {
    let config = SwarmConfig::from_file("config.toml").unwrap();
    let validator = Validator::new(config, "my-validator".to_string());
    validator.initialize().unwrap();
    
    // Your integration code here
}
```

---

## Appendix: Command Reference

| Command | Description |
|---------|-------------|
| `x3-validator run` | Start validator |
| `x3-validator status` | Show status |
| `x3-validator benchmark` | Run benchmark |
| `x3-validator test` | Run tests |
| `x3-swarm-orchestrator run` | Start orchestrator |
| `x3-swarm-orchestrator status` | Show swarm status |
| `x3-bench run` | Run benchmark |
| `x3-bench report` | Show report |

---

## Support

- GitHub Issues: https://github.com/x3-chain/x3-chain/issues
- Discord: https://discord.gg/x3-chain
- Email: support@x3-chain.io
