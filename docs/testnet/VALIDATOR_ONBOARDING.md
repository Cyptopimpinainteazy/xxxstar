# X3 Public Testnet Validator Onboarding

## Scope

This guide is for RC6 public testnet package validation. It does not authorize mainnet operations.

## Requirements

- Ubuntu 22.04 or newer
- 8+ CPU cores recommended
- 32 GB RAM minimum
- 500 GB SSD/NVMe
- Stable public IP
- Open P2P port (default `30333`)
- NTP enabled and synchronized

## Install dependencies

```bash
sudo apt update
sudo apt install -y build-essential clang lld pkg-config libssl-dev protobuf-compiler jq curl git tmux
rustup toolchain install stable
rustup default stable
```

## Build node binary

```bash
git clone https://github.com/Cyptopimpinainteazy/x3-atomic-star.git
cd x3-atomic-star
cargo build --release -p x3-chain-node
```

## Obtain public testnet chain spec

Use artifacts produced by RC6 package process:

- `chain-specs/x3-public-testnet-plain.json`
- `chain-specs/x3-public-testnet-raw.json`

If you are onboarding from a release package, use the included raw spec directly.

## Start validator

```bash
./target/release/x3-chain-node \
  --chain chain-specs/x3-public-testnet-raw.json \
  --validator \
  --base-path /var/lib/x3 \
  --name "YOUR_VALIDATOR_NAME" \
  --port 30333 \
  --rpc-port 9944
```

## Verify sync and peer connectivity

```bash
journalctl -u x3-node -f
```

Or if running in shell, monitor process logs for block import and finality progress.

## Key safety

- Do not reuse mainnet keys.
- Keep validator keys isolated from faucet and treasury keys.
- Do not run external bridge services during initial public testnet scope.

## Troubleshooting quick checks

- Chain spec mismatch: confirm `x3-public-testnet-raw.json` hash from release artifacts.
- No peers: validate bootnodes and firewall rules.
- Time drift: ensure NTP is active and synchronized.

## Fresh-machine dry run

Use this on a clean validator host before public launch:

1. Verify the host can resolve the published bootnode hostname once DNS is assigned.
2. Confirm the node binary exists and prints a usable version.
3. Confirm the raw chain spec hash matches the RC6 release artifact.
4. Start the node with the published raw spec and `--validator`.
5. Confirm the node reaches peers and begins importing blocks.
6. Confirm the RPC port is only exposed according to the testnet plan.
7. Confirm keys are isolated from faucet and treasury material.

Expected outcome:

- Fresh host can join the network without manual intervention from the core team.
- If the bootnode hostname is not live yet, the dry run remains blocked at the DNS/bootstrap step and public launch stays blocked.
