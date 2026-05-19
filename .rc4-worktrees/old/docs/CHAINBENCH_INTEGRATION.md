# Chainbench Integration (Testnet + Live Mode)

This repo now wires `chainbench-ultimate(1).html` into a backend control/API plane for X3 Chain/X3 testing.

## What Was Wired

1. Dashboard mode control is API-backed.
2. Cross-chain onboarding form is API-backed and persisted.
3. RPC sweep and RPC benchmark panels call backend APIs.
4. Admin mode controls are added to the dashboard.
5. Harness smoke and remark runs are callable from dashboard admin actions.
6. Network status and adapter placeholder checks are exposed for UI consumption.

## New Files

- `scripts/testnet/chainbench_server.py`
- `scripts/testnet/run-chainbench-stack.sh`
- `scripts/testnet/stop-chainbench-stack.sh`

## Dashboard File (kept visually the same)

- `chainbench-ultimate(1).html`

## Start Stack

```bash
bash scripts/testnet/run-chainbench-stack.sh
```

Default endpoints:

- Dashboard: `http://127.0.0.1:7788/`
- Health: `http://127.0.0.1:7788/health`

## Stop Stack

```bash
bash scripts/testnet/stop-chainbench-stack.sh
```

## API Surface

- `GET /api/mode`
- `POST /api/mode`
- `POST /api/rpc/sweep`
- `POST /api/rpc/bench`
- `GET /api/rpc/bench`
- `GET /api/network/status`
- `GET /api/onboarding/chains`
- `POST /api/onboarding/chains`
- `GET /api/adapters/status`
- `GET /api/admin/state`
- `POST /api/admin/toggle` (requires `X-Admin-Key`)
- `POST /api/harness/smoke`
- `POST /api/harness/remark`

## Admin Mode

Set the key when launching:

```bash
export CHAINBENCH_ADMIN_KEY='your-admin-key'
bash scripts/testnet/run-chainbench-stack.sh
```

Then use the same value in the dashboard `Admin Key` input.

## Notes

- `SVM` and `BTC` are wired as adapter placeholders in this phase (status checks only).
- `EVM` path is marked active via RPC-backed checks.
- Onboarding stores masked credentials only in persisted records.
