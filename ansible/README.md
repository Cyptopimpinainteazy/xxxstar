# X3 testnet deployment (Ansible)

Runs the 7-validator X3 testnet as systemd **user** units on the hosts in
`inventory/hosts.local.yml`. Nothing needs root; the two host settings that do are printed by
preflight as exact commands for the operator.

```bash
cd ansible
cp inventory/hosts.seven.example.yml inventory/hosts.local.yml
# Set actual LAN/VPN addresses and SSH users in hosts.local.yml before continuing.
ansible-playbook playbooks/00-genesis.yml     # once: seeds, node keys, spec; bootNodes -> inventory IPs
ansible-playbook playbooks/10-preflight.yml   # linger, NTP, disk, ports, arch; prints sudo fixes
ansible-playbook playbooks/20-deploy.yml      # binary, spec, keys, units; start
ansible-playbook playbooks/30-verify.yml      # one genesis, peers, finality advancing
ansible-playbook playbooks/90-stop.yml        # -l <host>, -e x3_only=<index>, -e x3_purge=true
```

- The node binary defaults to `target/release/x3-chain-node` in this checkout; point
  elsewhere with `-e x3_binary_src=/path/to/x3-chain-node`.
- Genesis output (seeds, node keys, staged keystores) stays on the controller in the
  gitignored `deployment/chain-specs/fresh/generated/`. Re-running 00-genesis reuses it;
  `-e x3_genesis_force=true` mints a new network and must never be used on a live one.
- Validator N listens on p2p `30340+N` (LAN), RPC `9960+N` and Prometheus `9640+N`
  (loopback only; 30-verify reads RPC over SSH).
- Deploy to some hosts with `-e x3_hosts=<host>[,<host>]`. `--limit` is refused: it would
  skip the controller play that stages keys and runs the double-sign check.
- **Double-sign guard.** 20-deploy asks every inventory host which validators it runs and
  refuses to start an index that is active on a different host. To move validator N:
  `90-stop.yml -l <old> -e x3_only=N`, edit the inventory, `20-deploy.yml`.

The checked-in `hosts.yml` is the older three-machine topology. The default now
requires a local inventory so stale addresses cannot deploy a different topology.
The seven-machine example assigns one authority per machine. The local inventory
is ignored by Git. SSH host keys must already be trusted; deployment does not
accept unknown keys automatically.

Verification compares canonical block hashes at the minimum finalized height,
requires a non-null genesis hash, and requires every validator to advance finality.
It is RPC observation, not independent cryptographic verification of GRANDPA votes.
For physical-host evidence, run `scripts/testnet/physical-seven-preflight.py` from
the repository root using a filled-in copy of
`docs/testnet-config/physical-seven-inventory.example.json`. That inventory must
name seven distinct machines and absolute installed artifact/data paths. The gate
collects two observations (12 seconds apart by default), refuses stalled finality
or changing artifacts/identities, and compares hashes at a shared finalized height.
The output records both observations. A seven-process local test is not physical
seven-host certification.

Controller dependency: `requirements-testnet.txt` pins the tested Ansible version.
Test dependencies are in `requirements-dev.txt`. Install into the controller virtual
environment; no Ansible installation is needed on validator machines.
