# X3 cluster node tooling

Standard-library Python, so a fresh Ubuntu node can run it before anything else is installed.

| Command | Does |
| --- | --- |
| `x3-node-bootstrap --role ROLE [--apply] [--lan]` | Writes `~/.config/x3-cluster/node.env` (non-secret). For `gpu`: ranks GPUs (VRAM, then compute capability), gives rank *n* port `11434 + 2n`, adopts an existing Ollama already pinned to that GPU's UUID, plans (`--apply`: creates) `x3-ollama-gpu<n>.service` user units, and writes a router registration to `~/.config/x3-cluster/registration/<node>.json`. |
| `x3-cluster-health [--json]` | Table of every node in `inventory.json`: reachability, SSH key auth, CPU/RAM/GPU, services, router, Ollama workers, disk, clock, repo commit. |
| `scripts/x3-cluster-gate.sh` | Health + checks; evidence in `audit-artifacts/x3-cluster/<commit>/`; nonzero on any failure. |
| `x3cluster.py bench` | CPU (SHA-256), disk (1 GiB `O_DIRECT` dd), LAN latency, clean Rust build of one crate. |
| `x3cluster.py job --class C [--priority P] [--node N] [--ref R] -- CMD` | Runs `CMD` at the exact commit `R` in a throwaway worktree on the node that owns class `C` (least loaded, with a free-disk guard). Evidence: `audit-artifacts/x3-cluster/<commit>/jobs/<id>/{job.json,log.txt,artifacts/}`. |
| `x3cluster.py pipeline [--spec pipeline.json]` | Stages in order, jobs within a stage in parallel; a failed stage skips the rest. Summary in `audit-artifacts/x3-cluster/<commit>/pipeline-<time>.json`. |
| `x3cluster.py ssh-config [--write]` | `Host` aliases for every inventory node with an IP that `~/.ssh/config` does not already define. |

Nothing here runs `sudo`. Privileged steps are written to
`~/.config/x3-cluster/staged-privileged.sh` for an operator to review and run. With
`--lan`, that script enables ufw (SSH from the LAN first, GPU ports only from the
control/ops hosts) and only then rebinds the workers to `0.0.0.0`.

## Joining a GPU node to the router (no router code edits)

1. On the GPU node: `x3-node-bootstrap --role gpu --apply --lan`, then run the staged script.
2. Copy `~/.config/x3-cluster/registration/<node>.json` to the router host's
   `~/.config/x3-router/providers.d/` and restart `x3-ai-router`.
   The router appends those providers after the configured ones and logs
   `registered worker ...`. A drop-in cannot carry API keys or prices, override an
   existing provider, or reference another file's providers.

Results are labelled `LOCAL` (this node) or `PHYSICAL` (measured on another real
machine over the LAN). A node with no IP in DNS/mDNS/`inventory.json` is reported
as `UNKNOWN`, never as passing.

## Jobs

| Class | Owning role | Stand-in when the owner is missing | Min free disk |
| --- | --- | --- | --- |
| BUILD, TEST | build | control (`LOCAL-FALLBACK`) | 30 GB |
| FUZZ | sim | control | 30 GB |
| SIMULATION | sim | control | 20 GB |
| GPU, INFERENCE | gpu | none: `REJECTED` | 8 / 1 GB |
| NETWORK, INTEGRATION | net | control | 5 / 20 GB |
| DATABASE | data | control | 5 GB |

Priority maps to `nice` (CRITICAL 0, HIGH 5, NORMAL 10, BACKGROUND 19). Commits are fetched
from `canonical_repo` in `inventory.json` (GitHub `xxxstar`), so a job's commit must be pushed
before a remote worker can run it. Each record carries commit, worker, label, kernel, `rustc`/
`cargo`/`cc` versions, start/end, exit code, result (`PASS`/`FAIL`/`TIMEOUT`/`REJECTED`),
the log's SHA-256 and every artifact's SHA-256. Whatever the command writes to `$X3_JOB_OUT`
(fuzz crashes, simulator failure packets) is copied back before it is deleted on the worker.

A worker's `node.env` may set `X3_REPO` (its checkout) and `X3_CARGO_TARGET_DIR` (reuse a warm
target dir instead of `~/.cache/x3-cluster/target`); `bootstrap` keeps those keys.

Tests: `python3 -m unittest scripts/x3-cluster/test_x3jobs.py`.

## Ansible

`x3-ansible-inventory` is an Ansible dynamic inventory built from `inventory.json` plus the IPs
`discover` found: one group per role (`control`, `gpu`, `build`, `sim`, `data`, `net`, `ops`) under
`cluster`. Nodes without an IP are left out; the control node uses a local connection.

    ansible-inventory -i scripts/x3-cluster/x3-ansible-inventory --graph
    ansible -i scripts/x3-cluster/x3-ansible-inventory gpu -m ping

It carries no secrets (user from `X3_ANSIBLE_USER`, default `lojak`). The validator-net playbooks
under `ansible/` keep their own inventory; this one covers the build cluster.
