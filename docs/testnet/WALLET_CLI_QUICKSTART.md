# X3 Wallet / CLI Quickstart

## Scope

This quickstart verifies user-level transfer flow on RC6 public testnet package.

## Create account

Use your preferred wallet or CLI tooling compatible with X3 testnet endpoints.

- Record seed phrase offline.
- Store account address for faucet requests.

## Request faucet funds

1. Use the public faucet URL once the release team publishes it alongside RPC and explorer endpoints.
2. If faucet is not yet published, treat faucet funding as a launch blocker for public-user testing.
3. Submit your testnet account address.
4. Wait for transaction hash confirmation.

## Check balance

Use one of the following, depending on environment:

- Planned public RPC: `https://rpc.x3star.net`
- Planned public WebSocket: `wss://ws.x3star.net`
- Operator-local validator RPC: `http://127.0.0.1:9933`

## Send transfer

1. Select recipient testnet address.
2. Enter transfer amount and fee.
3. Submit transaction.
4. Record transaction hash.

## View transaction

Use the explorer endpoint once published. Repo-backed planned hostnames are:

- `https://explorer.x3star.net`
- `https://blockexplorer.x3star.net`

Do not treat those hostnames as launch-ready until they point at a live explorer deployment.

Use explorer to verify:

- Inclusion block
- Success/failure status
- Sender/recipient and amount

## Internal settlement route smoke test

- Run one transfer from faucet-funded account to second account.
- Run a return transfer.
- Confirm both appear on explorer and in account history.
