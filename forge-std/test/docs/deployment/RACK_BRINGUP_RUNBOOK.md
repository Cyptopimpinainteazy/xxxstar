# X3 Rack Bring-Up Runbook

## Purpose

This runbook turns the rack plan into a host-by-host bring-up sequence. It assumes the hardware allocation in `docs/deployment/HARDWARE_ROLE_PLAN.md` is accepted and that the operator wants reusable systemd-managed services instead of one-off shell commands.

## Current Reality

The repo now contains a reusable node launcher at `deployment/systemd/run-x3-node.sh`, a matching systemd unit template at `deployment/systemd/x3-chain-node@.service`, and rack-specific example env files under `deployment/systemd/examples/`. These artifacts are sufficient for the first real local-lab deployment on the Lenovo pair, the Threadripper workstation, and the DL380p without needing to guess service command lines during the install.

## Verified Inputs

The rack role mapping lives in `docs/deployment/HARDWARE_ROLE_PLAN.md`. The hardware-backed inventory lives in `deployment/inventory.yaml`. The multi-server SSH target template lives in `deployment/servers.env.example`. The shell syntax of the new deployment helpers has been checked with `bash -n`.

## Host Roles

Use these instance names for the first lab bring-up:

| Host | Instance env file | systemd instance |
| --- | --- | --- |
| `x3-lab-val-01` | `/etc/x3-chain/nodes/x3-lab-val-01.env` | `x3-chain-node@x3-lab-val-01` |
| `x3-lab-val-02` | `/etc/x3-chain/nodes/x3-lab-val-02.env` | `x3-chain-node@x3-lab-val-02` |
| `x3-lab-val-03` | `/etc/x3-chain/nodes/x3-lab-val-03.env` | `x3-chain-node@x3-lab-val-03` |
| `x3-lab-rpc-01` | `/etc/x3-chain/nodes/x3-lab-rpc-01.env` | `x3-chain-node@x3-lab-rpc-01` |

The first validator keeps private RPC enabled so the operator can inspect node state locally during bring-up. The other validators keep RPC disabled in the sample env files. The RPC node runs as a non-validator full node with local-only RPC until a reverse proxy or allowlist is in place.

## Install Layout

Install the binary and service files once on each host. The service template expects the repo checkout at `/opt/x3-chain`, the node binary at `/usr/local/bin/x3-chain-node`, and writable state under `/var/lib/x3-chain`.

Use this layout on every X3 host:

```bash
sudo useradd --system --home /var/lib/x3-chain --shell /usr/sbin/nologin x3 || true
sudo mkdir -p /etc/x3-chain/nodes
sudo mkdir -p /var/lib/x3-chain/node-keys
sudo mkdir -p /var/lib/x3-chain
sudo chown -R x3:x3 /etc/x3-chain /var/lib/x3-chain

sudo install -D -m 0755 /opt/x3-chain/deployment/systemd/run-x3-node.sh /opt/x3-chain/deployment/systemd/run-x3-node.sh
sudo install -D -m 0644 /opt/x3-chain/deployment/systemd/x3-chain-node@.service /etc/systemd/system/x3-chain-node@.service
sudo systemctl daemon-reload
```

Copy the example env file that matches the host role, then edit only the fields that are truly host-specific:

```bash
sudo cp /opt/x3-chain/deployment/systemd/examples/x3-lab-val-01.env.example /etc/x3-chain/nodes/x3-lab-val-01.env
sudo chown x3:x3 /etc/x3-chain/nodes/x3-lab-val-01.env
sudo chmod 0640 /etc/x3-chain/nodes/x3-lab-val-01.env
```

The values that always need review are `chain_spec`, `base_path`, `node_key_file`, and `bootnodes`. On `x3-lab-val-02`, `x3-lab-val-03`, and `x3-lab-rpc-01`, leave the service stopped until the first validator is up and you have the real bootnode multiaddress.

If you want the repo to stamp out the local-lab env files after the bootnode identity is known, use `deployment/scripts/render-rack-config.sh`. It takes `BOOTNODE_MULTIADDR` and writes rendered env files under `deployment/systemd/rendered/`, and it can also write `deployment/servers.env` if you export the rack SSH targets at the same time.

```bash
BOOTNODE_MULTIADDR='/ip4/203.0.113.10/tcp/30333/p2p/<peer-id>' \
BOOTNODE_HOST='x3@bootnode-host' \
VALIDATOR_01_HOST='x3@lenovo-01' \
VALIDATOR_02_HOST='x3@lenovo-02' \
VALIDATOR_03_HOST='x3@threadripper' \
RPC_01_HOST='x3@dl380p' \
./deployment/scripts/render-rack-config.sh
```

The rendered files are still operator-reviewed artifacts. The script removes the repetitive hand editing, but it does not replace the final review.

## Firewall Bring-Up

The firewall helper no longer opens SSH to the world by default. It requires `ADMIN_CIDR` and opens only the ports appropriate for the selected role.

Examples:

```bash
ADMIN_CIDR=198.51.100.10/32 \
METRICS_CIDR=192.168.10.0/24 \
P2P_CIDR=0.0.0.0/0 \
sudo bash /opt/x3-chain/deployment/configure-firewall.sh validator
```

```bash
ADMIN_CIDR=198.51.100.10/32 \
RPC_CIDR=192.168.10.0/24 \
METRICS_CIDR=192.168.10.0/24 \
sudo bash /opt/x3-chain/deployment/configure-firewall.sh rpc
```

If the RPC node really must expose port `9944` directly, set `PUBLIC_RPC=1` explicitly. Do not do that by accident.

## Bring-Up Order

Start `x3-lab-val-01` first. Confirm that the node comes up cleanly, that the node key file is readable, and that the logs print the local node identity. Build the bootnode multiaddress from that identity and insert it into the env files for the other three hosts before you start them.

Bring up the remaining nodes in this order: `x3-lab-val-02`, `x3-lab-val-03`, then `x3-lab-rpc-01`. That order keeps the authority set stable before the support node joins.

Use these commands on each host:

```bash
sudo systemctl enable --now x3-chain-node@x3-lab-val-01
sudo journalctl -u x3-chain-node@x3-lab-val-01 -f
```

```bash
sudo systemctl enable --now x3-chain-node@x3-lab-val-02
sudo systemctl enable --now x3-chain-node@x3-lab-val-03
sudo systemctl enable --now x3-chain-node@x3-lab-rpc-01
```

## Verification

The first verification target is not public RPC. It is consensus health inside the rack. Check that all three authorities are connected, that best and finalized block height advance, and that the RPC node syncs without participating in authority duty.

Run local checks from the hosts:

```bash
curl -s http://127.0.0.1:9944 -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}'
```

```bash
curl -s http://127.0.0.1:9615/metrics | grep substrate_block_height
```

Do not expose the RPC endpoint publicly until the rack passes restart, rejoin, and restore drills.

## Next Required Work

After the local rack is stable, the next step is to add the real remote validators to `deployment/inventory.yaml`, create `deployment/servers.env`, and produce the final public-testnet bootnode and authority plan. At that point the Threadripper host should move out of the authority set and into GPU canary or build-host duty.