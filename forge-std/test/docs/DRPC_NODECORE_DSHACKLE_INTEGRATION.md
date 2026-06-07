# dRPC Nodecore + Dshackle Integration

This phase integrates the two stacks you linked:

- `https://github.com/drpcorg/nodecore`
- `https://github.com/drpcorg/dshackle`

into the X3 Chain / Chainbench test workflow.

## Added in this repo

- `infra/drpc/docker-compose.drpc.yml`
- `infra/drpc/nodecore.yml`
- `infra/drpc/dshackle.yaml`
- `scripts/drpc/setup-drpc-stack.sh`
- `scripts/drpc/stop-drpc-stack.sh`
- `GET /api/drpc/status` in `scripts/testnet/chainbench_server.py`

## What this gives you

1. Local dRPC provider plane for benchmarking and failover tests.
2. Nodecore query endpoint for dashboard integration and synthetic TPS/RPC runs.
3. Dshackle routing/proxy endpoint for chain-aware upstream handling.
4. Chainbench API-level visibility into Nodecore and Dshackle health/latency.

## Quick start

### 1) Set provider key (if needed)

```bash
export DRPC_KEY='your-drpc-key'
```

### 2) Start dRPC stack

```bash
bash scripts/drpc/setup-drpc-stack.sh
```

Expected local endpoints:

- `http://127.0.0.1:9090/queries/ethereum` (nodecore)
- `http://127.0.0.1:8545/eth` (dshackle)

### 3) Start chainbench stack pointing at these endpoints

```bash
CHAINBENCH_NODECORE_QUERY_URL='http://127.0.0.1:9090/queries/ethereum' \
CHAINBENCH_DSHACKLE_PROXY_URL='http://127.0.0.1:8545/eth' \
bash scripts/testnet/run-chainbench-stack.sh
```

### 4) Verify from Chainbench API

```bash
curl -s http://127.0.0.1:7788/api/drpc/status | jq
```

## Stop

```bash
bash scripts/drpc/stop-drpc-stack.sh
bash scripts/testnet/stop-chainbench-stack.sh
```

## Next phase recommendations

1. Add Nodecore auth policy with scoped keys per partner chain.
2. Add dshackle multi-upstream routes for EVM + Solana + Bitcoin test targets.
3. Extend Chainbench UI to render `/api/drpc/status` directly in a dedicated provider panel.
4. Add synthetic load profiles that compare direct RPC vs nodecore-routed RPC.
5. Persist benchmark snapshots (`/api/rpc/bench`) into a DB for long-run trend analysis.
