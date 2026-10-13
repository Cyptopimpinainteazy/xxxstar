# X3 cluster node tooling

Standard-library Python, so a fresh Ubuntu node can run it before anything else is installed.

| Command | Does |
| --- | --- |
| `x3-node-bootstrap --role ROLE [--apply] [--lan]` | Writes `~/.config/x3-cluster/node.env` (non-secret). For `gpu`: ranks GPUs (VRAM, then compute capability), gives rank *n* port `11434 + 2n`, adopts an existing Ollama already pinned to that GPU's UUID, plans (`--apply`: creates) `x3-ollama-gpu<n>.service` user units, and writes a router registration to `~/.config/x3-cluster/registration/<node>.json`. |
| `x3-cluster-health [--json]` | Table of every node in `inventory.json`: reachability and SSH key auth for all; for remote nodes CPU threads/load, free disk and identity (hostname, `node.env` name and role); for this node also RAM/GPU, services, router, Ollama workers, clock and repo commit. |
| `scripts/x3-cluster-gate.sh` | Health + checks (every declared service running, router health, remote identity matching the inventory); evidence in `audit-artifacts/x3-cluster/<commit>/`; nonzero on any failure. |
| `x3cluster.py bench` | CPU (SHA-256), disk (1 GiB `O_DIRECT` dd), LAN latency, clean Rust build of one crate. |

Nothing here runs `sudo`. Privileged steps are written to
`~/.config/x3-cluster/staged-privileged.sh` for an operator to review and run. With
`--lan`, that script enables ufw (SSH from the LAN first, GPU ports only from the
control/ops hosts) and only then rebinds the workers to `0.0.0.0`; workers this tool
creates always start on loopback.

Only workers known to be pinned to their GPU and running (adopted, or created and
seen answering) are registered. A worker with no advertised model gets an
`ollama pull` in the staged script and no registration until bootstrap is re-run;
if no worker qualifies, no registration file is written. Without `--lan` the
registration points at `127.0.0.1` and is marked `scope: local`, for a router on
the same node.

## Joining a GPU node to the router (no router code edits)

1. On the GPU node: `x3-node-bootstrap --role gpu --apply --lan`, then run the staged script.
2. Copy `~/.config/x3-cluster/registration/<node>.json` to the router host's
   `~/.config/x3-router/providers.d/` and restart `x3-ai-router`.
   The router appends those providers after the configured ones and logs
   `registered worker ...`. A drop-in cannot carry API keys or prices, override an
   existing provider, or reference another file's providers.

Results are labelled `LOCAL` (this node) or `PHYSICAL` (measured on real hardware).
A node with no IP in DNS/mDNS/`inventory.json` is reported as `UNKNOWN`, never
as passing.

Tests: `python3 -m unittest scripts/x3-cluster/test_x3cluster.py` (all system calls mocked).
