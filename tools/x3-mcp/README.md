# X3 MCP Server

Local MCP server for Codex/VS Code to inspect a running X3 node and execute explicit repository verification gates.

## Security model

- stdio only by default
- read-only JSON-RPC allowlist
- no arbitrary shell tool
- subprocesses use argv directly, never `shell=True`
- unknown RPC methods fail closed
- node/RPC failures are reported as failures, never converted to success
- no wallet keys, signing, transfers, governance writes, or mainnet mutations

## Tools

| Tool | Purpose |
|---|---|
| `x3_chain_health` | system health + best/finalized heads |
| `x3_get_finalized_head` | exact finalized hash/header |
| `x3_inspect_block` | exact block inspection by hash |
| `x3_rpc` | allow-listed read-only JSON-RPC |
| `x3_repo_status` | branch, HEAD, dirty state |
| `x3_run_readiness_gate` | focused/tests/local-ci verification |
| `x3_run_atomic_tests` | real x3-atomic-swap tests |
| `x3_collect_evidence` | repository + live-node evidence bundle |
| `x3_trace_atomic_swap` | trace one intent from durable proof-ledger evidence |
| `x3_verify_proof` | require complete verified persisted proof evidence |
| `x3_verify_receipt` | run X3Lang receipt verification; optional trusted Ed25519 signer map |
| `x3_inspect_intent` | inspect live node intent RPC capability; fails closed if runtime RPC is not exposed |
| `x3_atomic_state` | bind atomic-state reporting to live chain evidence |
| `x3_validator_status` | node health plus author/GRANDPA capability evidence |
| `x3_consensus_status` | best/finalized heads and GRANDPA round state when exposed |
| `x3_run_failure_matrix` | existing validator failure drill or atomic negative suite |

## Install

From the repository root:

```bash
python3 -m venv .venv-x3-mcp
. .venv-x3-mcp/bin/activate
pip install -e tools/x3-mcp
```

Set the local node and repo:

```bash
export X3_RPC_URL=http://127.0.0.1:9944
export X3_REPO_ROOT="$PWD"
export X3_PROOF_LEDGER="$PWD/audit-artifacts/x3vm-proof-ledger.json"
```

## Codex

```bash
codex mcp add x3 -- env \
  X3_REPO_ROOT="$PWD" \
  X3_RPC_URL=http://127.0.0.1:9944 \
  "$PWD/.venv-x3-mcp/bin/x3-mcp"

codex mcp list
```

Restart Codex/VS Code after changing MCP configuration.

## VS Code

A local `.vscode/mcp.json` can point its stdio command at:

```text
<workspace>/.venv-x3-mcp/bin/x3-mcp
```

and set:

```text
X3_REPO_ROOT=<workspace>
X3_RPC_URL=http://127.0.0.1:9944
```

Do not commit secrets in MCP configuration.

## Next tools

Next phase should bind these to real X3 interfaces:

1. expose `GovernanceSettlementApi::get_settlement` through node JSON-RPC so `x3_inspect_intent` can return decoded live intent state
2. expose escrow-leg state through a runtime API/RPC rather than guessed SCALE storage keys
3. `x3_compile_x3lang`
4. `x3_collect_production_evidence`
5. extend the failure matrix with live EVM/SVM/native lifecycle profiles


Each must bind to actual X3 interfaces. No fake responses or placeholder success paths.
