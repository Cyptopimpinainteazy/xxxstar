# X3 Operator

Production-grade operator toolkit for the X3 blockchain network. Handles operator onboarding, stake bonding, slashing, agent supervision, storage deals, governance simulation, genesis ceremony, and telemetry — all with zero external dependencies (stdlib only).

## Quickstart

```bash
# 1. Health check
python -m x3_operator doctor

# 2. Initialize operator
python -m x3_operator init --role gpu --network devnet

# 3. Bond stake
python -m x3_operator bond 5000

# 4. Check status
python -m x3_operator status

# 5. Run governance simulation
python -m x3_operator simulate

# 6. Run genesis ceremony
python -m x3_operator genesis
```

## Architecture

```
x3_operator/
├── __init__.py         # Package root — 37 exports
├── __main__.py         # `python -m x3_operator` entry
├── cli.py              # Argparse CLI with 8 commands
├── command_center.py   # HTTP REST API server (20+ endpoints)
├── config.py           # Centralized config with validation, persistence, env overrides
├── identity.py         # Hardware-bound operator identity (deterministic IDs, keypair)
├── health.py           # Hardware preflight checks (disk, RAM, CPU, GPU, NTP, OS)
├── bonding.py          # Bond lifecycle: create → confirm → unbond → slash
├── slashing.py         # Deterministic slashing: severity × repetition × confidence
├── supervisor.py       # Agent process supervisor with kill-switch + rate limiting
├── storage.py          # Filecoin-style storage: CID, deals, proofs, SLA
├── governance.py       # 4 attack simulations (whale, sybil, bribery, speed)
├── genesis.py          # 7-step genesis ceremony with config freeze + hash anchor
└── telemetry.py        # Structured JSON logging + Prometheus-compatible metrics
```

### Dependencies

**None.** The entire package uses only Python 3.10+ standard library. No pip install needed.

### Module Dependency Graph

```
config.py ← (all modules)
identity.py ← cli.py, command_center.py
health.py ← cli.py, command_center.py
bonding.py ← cli.py, command_center.py
slashing.py ← command_center.py
supervisor.py ← command_center.py
storage.py ← command_center.py
governance.py ← cli.py, command_center.py
genesis.py ← cli.py, command_center.py
telemetry.py ← cli.py, command_center.py
```

No circular dependencies. All leaf modules import only from `config.py`.

## CLI Reference

### `doctor` — Hardware preflight

```bash
python -m x3_operator doctor [--role gpu] [--min-disk 100] [--min-ram 8] [--min-cpu 4]
```

Checks: disk space, RAM, CPU cores, GPU (nvidia-smi), NTP clock sync, OS compatibility.
Returns recommended roles based on hardware capabilities.

### `init` — Initialize operator

```bash
python -m x3_operator init --role <validator|gpu|storage|relayer> --network <devnet|testnet|mainnet> [--rpc-url ws://...]
```

Creates `config.json` (validated), `identity.json` (hardware-bound ID), and `operator.key` (chmod 600).

Operator ID is deterministic: `blake2b(hardware_fingerprint:role)`.

### `bond` — Bond stake

```bash
python -m x3_operator bond <amount>
```

Minimum bonds (configurable):
| Role | Min Bond |
|------|----------|
| Validator | 10,000 |
| GPU | 1,000 |
| Storage | 2,000 |
| Relayer | 5,000 |

### `start` — Run operator daemon

```bash
python -m x3_operator start [--metrics-port 9615]
```

Runs heartbeat loop, tracks uptime, exports Prometheus metrics.

### `status` — Show operator state

```bash
python -m x3_operator status
```

Displays operator ID, role, network, bond amount, effective stake, slash total.

### `simulate` — Governance capture simulation

```bash
python -m x3_operator simulate [--attack whale|sybil|bribery|speed] [--seed 42]
```

Runs deterministic (seeded) governance attack simulations. Default runs all 4.

### `genesis` — Genesis ceremony

```bash
python -m x3_operator genesis [--chain-id x3-devnet] [--chain-name "X3 Devnet"] [--validators addr1,addr2,...] [--anchor]
```

7 steps: configure → collect attestations → freeze → verify → generate chain spec → dry run → anchor hash.

### `exit-op` — Graceful exit

```bash
python -m x3_operator exit-op
```

Starts unbonding period (default: 86,400s / 24h). Bond amount released after cooldown.

## Command Center API

Start the HTTP server:

```bash
python -m x3_operator.command_center [--host 0.0.0.0] [--port 8900] [--data-dir ~/.x3_operator]
```

### GET Endpoints

| Endpoint | Description |
|----------|-------------|
| `/api/health` | Hardware health check |
| `/api/status` | Operator status + uptime |
| `/api/identity` | Operator identity details |
| `/api/bond` | Bond status |
| `/api/agents` | Registered agents list |
| `/api/storage/deals` | Storage deal list |
| `/api/metrics` | Metrics (JSON) |
| `/api/metrics/prometheus` | Metrics (Prometheus text format) |
| `/api/config` | Current configuration |

### POST Endpoints

| Endpoint | Body | Description |
|----------|------|-------------|
| `/api/simulate` | `{seed: 42}` | Run full governance simulation |
| `/api/simulate/whale` | `{seed, whale_stake_fraction}` | Whale attack |
| `/api/simulate/sybil` | `{seed, n_sybils}` | Sybil attack |
| `/api/simulate/bribery` | `{seed, bribe_budget}` | Bribery attack |
| `/api/simulate/speed` | `{seed}` | Speed attack |
| `/api/agents/register` | `{agent_id, operator_id, allowed_endpoints, ...}` | Register agent |
| `/api/agents/kill` | `{agent_id, reason}` | Kill agent |
| `/api/agents/kill-switch` | `{}` | Arm global kill switch |
| `/api/storage/register` | `{provider_id, capacity_bytes}` | Register provider |
| `/api/storage/propose` | `{client_id, provider_id, cid, size_bytes, ...}` | Propose deal |
| `/api/genesis/run` | `{validators, chain_id, chain_name}` | Run genesis ceremony |
| `/api/slash` | `{operator_id, fault_type, confidence, ...}` | Slash operator |

All endpoints return JSON. CORS enabled (`Access-Control-Allow-Origin: *`).

## Configuration

`X3Config` is the root configuration object with nested sections:

| Section | Key Fields |
|---------|------------|
| `chain` | `rpc_url`, `chain_id`, `network_phase` (devnet/testnet/mainnet) |
| `bonding` | `min_bond_*`, `unbonding_delay_seconds` (86400), `unbonding_delay_blocks` (14400) |
| `slashing` | `severity_table` (fault→fraction), `repetition_base` (1.5), `max_slash_fraction` (1.0) |
| `health` | `min_disk_gb`, `min_ram_gb`, `min_cpu_cores`, `gpu_required_for`, `heartbeat_interval_seconds` |
| `agent` | `max_agents_per_operator` (10), `max_calls_per_minute` (60), `kill_switch_delay_seconds` (5) |
| `storage` | `proof_interval_seconds` (60), `min_replication_factor` (3) |
| `telemetry` | `log_level`, `log_format`, `metrics_port` (9615) |

### Environment Overrides

```bash
X3_RPC_URL=ws://custom:9944
X3_NETWORK_PHASE=testnet
X3_MIN_BOND_VALIDATOR=50000
```

## Slashing Formula

```
slash_amount = base_severity × repetition_base^(count-1) × confidence × bond_amount
```

Capped at `max_slash_fraction × bond_amount`.

| Fault Type | Base Severity |
|-----------|---------------|
| `downtime` | 1% |
| `missed_heartbeat` | 0.5% |
| `sla_violation` | 5% |
| `invalid_proof` | 10% |
| `agent_violation` | 10% |
| `governance_abuse` | 25% |
| `equivocation` | 50% |
| `data_corruption` | 100% |

Repeat offenses scale exponentially: `1.5^(count-1)`.

## Testing

```bash
python -m pytest tests/x3_operator/ -v
```

**111 tests** across 11 test files covering all modules:

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `test_config.py` | 9 | Defaults, validation, save/load, env overrides |
| `test_identity.py` | 8 | Attestation, fingerprint, generate/load, determinism, key perms |
| `test_health.py` | 6 | Pass/fail checks, OS, roles, GPU optional |
| `test_bonding.py` | 11 | Bond lifecycle, slash, top-up, save/load, history |
| `test_slashing.py` | 12 | Severity, repetition, confidence, cap, evidence hash |
| `test_supervisor.py` | 11 | Agent lifecycle, rate limiter, kill switch, policy hash |
| `test_storage.py` | 11 | CID, deals, proofs, capacity, providers |
| `test_governance.py` | 11 | All 4 attacks, determinism, summary |
| `test_genesis.py` | 11 | Full ceremony, freeze, verify, dry run, chain spec |
| `test_telemetry.py` | 8 | Counter, gauge, Prometheus, JSON, logging |
| `test_cli.py` | 10 | All CLI commands, full lifecycle |

## Devnet Scripts

```bash
scripts/devnet/launch_devnet.sh          # Init 3 operators, bond, genesis, simulate
scripts/devnet/adversarial_week.sh       # Multi-seed governance stress tests
scripts/devnet/key_ceremony_rehearsal.sh  # Dry-run genesis with verification
scripts/devnet/mainnet_countdown.sh      # 23-check pre-launch audit
```

## Known Limitations

- **Keypair generation is simulated** — uses `sha256(random_seed)` instead of real Ed25519. Production should use ed25519-dalek via FFI or PyNaCl.
- **Operator key stored in plaintext** — `operator.key` is raw hex with `chmod 0600`. Production needs KDF + passphrase encryption.
- **Bond tx_hash is simulated** — generated from `sha256(op:role:amount:nonce:time)`, not from an actual chain extrinsic.
- **Anchor hash is simulated** — `sha256(frozen_hash:time)` instead of an on-chain transaction.
- **MetricsRegistry is not thread-safe** — counter increments and gauge sets have no locking. Use `threading.Lock` for concurrent access.

## License

Part of the X3 Chain project.
