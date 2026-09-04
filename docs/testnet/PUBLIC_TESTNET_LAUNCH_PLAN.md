# X3 Public Testnet Launch Plan

## Scope

- X3 internal settlement path only
- X3Native / X3EVM / X3SVM paths as currently enabled in testnet build
- External bridges disabled
- Public RPC and faucet planned
- Validator onboarding published

## Entry Criteria

- RC5 72-hour internal alpha passed
- RC6 public testnet package passed
- Release artifacts hashed and published
- Bootnodes online and validated
- Public docs published

## Exit Criteria

- 2-4 week public run completed
- No invariant failure
- At least one runtime upgrade rehearsal completed
- Validator restart drill completed
- Public bug reports triaged and tracked

## RC6 Packaging Rule

RC6 is packaging only. It does not authorize public testnet launch by itself.

## Bootnode Rule

If bootnodes are not live yet:

- `RC6_PACKAGE_READY: PASS`
- `BOOTNODE_DEPLOYMENT: PENDING`

Public testnet launch remains blocked until bootnodes are deployed and reachable.
