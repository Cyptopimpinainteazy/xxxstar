# RC6 Bootnode Config Review

## Goal

Confirm that bootnode configuration is either present in chain spec or explicitly tracked as pending deployment.

## Required

- Bootnode list included in raw spec, or
- Documented `BOOTNODE_DEPLOYMENT: PENDING` with launch block in place

## Repo-Backed Bootnode Addresses

- Canonical generated form from `deployment/generate-keys-only.sh`:
	- `/dns4/bootnode.testnet.x3-chain.io/tcp/30333/p2p/211d3541...bff90d9`
- Current internal seed used in Kubernetes config map:
	- `/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWP1XsE2tRWDVyAMyCxeDUqsCvCGFKt7ZoCZk7Wn8BKWjU`

## Deployment Reality

- DNS A-record values for `bootnode.testnet.x3-chain.io` are still `null` in `deployment/inventory.yaml`.
- The public bootnode host still needs a real IP before launch.
- RC6 package readiness can still pass with deployment pending, but public launch remains blocked until bootnodes are live.

## Status

- Raw spec bootnode entries: PENDING
- Deployment owner assigned: release operations / infrastructure owner
- RC6 package status: PENDING

## Launch Rule

Public testnet launch must not proceed until bootnodes are live and validated.
