# X3 Testnet Verification

This document describes the config-driven verification flow for the local or cloud X3 testnet.

## Entry Points

- `make testnet-verify`
- `scripts/testnet/verify-testnet.sh`

Both commands read optional defaults from `docs/testnet-config/testnet-config.json` under the `verification` key.

## Verification Checks

The wrapper runs these checks in order:

1. Peer count threshold via `scripts/testnet/status-7-validators.sh --check-peers`
2. Finality progression via `scripts/testnet/status-7-validators.sh --check-finality`
3. Optional Prometheus and Grafana health checks
4. Optional single-process or multiprocess load baseline

The script exits non-zero on the first failed invariant.

## Config Schema

`docs/testnet-config/testnet-config.json`

```json
{
  "verification": {
    "min_peers": 3,
    "peer_timeout_sec": 120,
    "finality_window_sec": 12,
    "max_finality_lag_mult": 2,
    "check_telemetry": false,
    "prom_host": "127.0.0.1",
    "prom_port": 9090,
    "grafana_host": "127.0.0.1",
    "grafana_port": 3001,
    "run_load": false,
    "run_multiprocess": false,
    "load_duration_sec": 1200,
    "min_finalized_tps": 0,
    "max_error_rate": 0.01
  }
}
```

## Precedence

Verification settings resolve in this order:

1. Environment variables
2. `TESTNET_CONFIG` JSON file
3. Built-in script defaults

Example:

```bash
RUN_LOAD=1 MIN_FINALIZED_TPS=1000 make testnet-verify
```

## Common Commands

Basic peer and finality checks:

```bash
make testnet-verify
```

Enable telemetry checks from the shell:

```bash
CHECK_TELEMETRY=1 make testnet-verify
```

Run the 20-minute multiprocess baseline:

```bash
RUN_LOAD=1 RUN_MULTIPROCESS=1 LOAD_DURATION_SEC=1200 MIN_FINALIZED_TPS=1000 MAX_ERROR_RATE=0.01 make testnet-verify
```

Point at a different config file:

```bash
TESTNET_CONFIG=deployment/testnet/verification.json make testnet-verify
```

## CI Workflow

GitHub Actions workflow to exercise the local 7-validator path (manual or continuous):

`testnet-verify.yml` runs `scripts/testnet/run-7-validators-local.sh` followed by `make testnet-verify`.

## Self-Hosted Continuous Runner

For true continuous verification beyond GitHub’s 5-minute schedule limit, run the loop script on a self-hosted runner or dedicated host:

```bash
bash scripts/testnet/continuous-verify.sh --interval-sec 60
```

Set any of these as environment variables to tune the loop:

`INTERVAL_SEC`, `RUN_LOAD`, `RUN_MULTIPROCESS`, `LOAD_DURATION_SEC`, `MIN_FINALIZED_TPS`, `MAX_ERROR_RATE`, `CHECK_TELEMETRY`, `TESTNET_CONFIG`, `LOG_FILE`, `STATUS_FILE`, `LOG_MAX_BYTES`, `LOG_BACKUPS`, `START_LOCAL_VALIDATORS`, `LOCAL_BASE_DIR`, `LOCAL_LOG_DIR`, `LOCAL_COUNT`

### Systemd Unit (Optional)

Copy `scripts/testnet/x3-testnet-verify.service` to `/etc/systemd/system/` and adjust the paths if your repo lives elsewhere.

Example:

```bash
sudo cp scripts/testnet/x3-testnet-verify.service /etc/systemd/system/x3-testnet-verify.service
sudo systemctl daemon-reload
sudo systemctl enable --now x3-testnet-verify.service
sudo systemctl status x3-testnet-verify.service
```

### Logs and Status

The continuous loop writes logs to `logs/testnet-verify.log` and rotates at `LOG_MAX_BYTES` with `LOG_BACKUPS` retained.
It also writes a JSON status file at `logs/testnet-verify-status.json` with the last run timestamp and status.

### Local Validator Auto-Start

If `START_LOCAL_VALIDATORS=1`, the loop will start the local 7-validator testnet when nodes are down.
It uses `scripts/testnet/run-7-validators-local.sh` with `LOCAL_BASE_DIR`, `LOCAL_LOG_DIR`, and `LOCAL_COUNT`.

## Mapped Invariants

- `INFRA-TESTNET-001`: peer threshold
- `INFRA-TESTNET-002`: finality progression
- `INFRA-TESTNET-003`: telemetry health
- `INFRA-TESTNET-004`: baseline load success
