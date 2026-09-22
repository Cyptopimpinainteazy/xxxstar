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

1. `x3_inspect_intent`
2. `x3_atomic_state`
3. `x3_validator_status`
4. `x3_consensus_status`
5. `x3_run_failure_matrix`
6. `x3_compile_x3lang`
7. `x3_collect_production_evidence`


Each must bind to actual X3 interfaces. No fake responses or placeholder success paths.
