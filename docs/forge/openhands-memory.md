# X3 Repo Brain: OpenHands integration without Serena

X3 already has a local, incremental code index (`tools/x3-forge/index.py`),
context selection (`context.py`), and an append-only engineering incident
memory (`failure_memory.py`). This integration exposes **those existing tools**
through the installed X3 MCP server; there is no Serena prerequisite.

## Installation (Linux, OpenHands CLI)

Run from the checkout the agents will inspect:

```bash
python3 -m venv .venv-x3-mcp
. .venv-x3-mcp/bin/activate
python -m pip install -e tools/x3-mcp
python3 tools/x3-forge/index.py build

# Ensure every worker can access this same persistent directory.
mkdir -p "$HOME/.local/share/x3-forge"
chmod 700 "$HOME/.local/share/x3-forge"

openhands mcp add x3 --transport stdio \
  --env "X3_REPO_ROOT=$PWD" \
  --env "X3_FORGE_SHARED_MEMORY=$HOME/.local/share/x3-forge/memory.jsonl" \
  "$PWD/.venv-x3-mcp/bin/x3-mcp"

openhands mcp list
```

Restart the agent conversation and use `/mcp` to verify that the X3 MCP tools
were loaded. First try `x3_brain_context(query="settlement")` and then
`x3_brain_recall(query="refund")`. Empty recall results are normal before
incidents are recorded.

For **explicitly approved** persistent writing, recreate the MCP configuration
with the additional `--env "X3_FORGE_ALLOW_WRITES=1"`, or configure that
environment variable only on the trusted server process. With writing enabled,
`x3_brain_remember` can append `failure`, `success`, and `dead_end`
observations. Keep human review of anything marked "success"; these are notes,
**not CI evidence**.

### Shared memory across agents

- Each worktree or branch has its **own** `.x3-forge/index.json` and should
  run `python3 tools/x3-forge/index.py build` after a commit/change.
- Point every agent at the **same absolute** `X3_FORGE_SHARED_MEMORY` file on
  a trusted filesystem that supports POSIX advisory locks. On multiple
  machines, use a shared mounted filesystem with functioning distributed locks
  or keep one MCP process/service as the sole writer. Do not independently
  write separate local files and assume they will synchronize.
- The memory file is deliberately **not** committed. `.x3-forge/` is ignored
  by Git. Back up the configured shared file (and its audit history)
  separately; exclude credentials, wallet keys and private data from notes.
- An agent operating in a different checkout must launch its own X3 MCP
  instance with `X3_REPO_ROOT` pointing at **that** checkout. The shared
  memory location can stay the same.
- If the index is missing or HEAD differs, context retrieval **fails closed**.
  Rebuild the index. If a selected file changed while HEAD stayed the same,
  retrieval also refuses its stale hash.
- Records are lexical matches, not embeddings. Rust indexing is
  shape-based, not compiler-grade call-graph analysis. No semantic/vector
  retrieval or automatic full conversation ingestion is claimed.
- Enable OpenHands' approval gates for edits, shell commands, merges and
  deployment. Do not share credentials with untrusted agent sandboxes.

## Available tools

| MCP tool | Scope |
| --- | --- |
| `x3_brain_context(query, budget_tokens, max_files)` | Return bounded file/symbol context with exact HEAD + SHA-256 evidence |
| `x3_brain_recall(query, kind, limit)` | Search recorded failures, successes and dead ends; explicitly marked unverified |
| `x3_brain_remember(kind, component, observation, resolution, regression)` | Append a timestamped, commit-linked memory; opt-in writes only |

## Verification

```bash
python3 -m unittest discover -s tools/x3-mcp/tests -p 'test_*.py' -v
python3 -m compileall -q tools/x3-mcp/src/x3_mcp
python3 tools/x3-forge/index.py build
python3 tools/x3-forge/context.py --json "atomic swap refund"
```

These tests cover real Forge parsing/context selection, stale HEAD and changed
file detection, missing-index failure, write permissions, durable recall,
configurable shared memory location, and MCP tool registration. The GitHub
workflow runs the focused tests in isolation from the blockchain build.

### Troubleshooting OpenHands

OpenHands CLI 1.x stores MCP connections in `~/.openhands/mcp.json` and
supports `openhands mcp add`. Older OpenHands GUI installations use a different
MCP configuration format; do not assume a pasted `config.toml` applies to the
CLI. If tools are absent, use `openhands mcp list`, inspect the MCP server
startup logs, and check that the checkout path is accessible from the agent
runtime. Verify your CLI's installed syntax before making changes.

## Roadmap beyond this PR

1. Add optional semantic embeddings over code and approved memories, with
   measured retrieval quality; keep provenance filtering before serving results.
2. Ingest selected prior agent sessions only after a redaction and
   source-attribution policy is in place.
3. Add a centralized service and access controls for multi-host agent fleets,
   rather than assuming a filesystem mount is universally reliable.
