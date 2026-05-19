# X3 Break-The-Chain Simulator

## Attack 1 - Asset Kernel Supply Drift

Try to make:
`canonical_supply != native + evm + svm + external_locked + pending`

Expected:
- invariant test fails
- transaction rejected
- no silent accounting drift

## Attack 2 - Cross-VM Partial Commit

Make EVM leg succeed and SVM/X3VM leg fail.

Expected:
- all legs rollback
- no partial state remains

## Attack 3 - Bridge Replay

Replay same bridge message twice.

Expected:
- second execution rejected
- nonce/message hash marked used

## Attack 4 - Expired Bridge Message

Execute message after expiry/deadline.

Expected:
- rejected

## Attack 5 - Wrong Chain ID / Domain

Execute message signed for a different chain/domain.

Expected:
- rejected

## Attack 6 - DEX Reserve Drift

Force swap/liquidity operation that makes reserves inconsistent.

Expected:
- invariant/test failure

## Attack 7 - Liquidity Lock Bypass

Try withdrawal before lock expiry.

Expected:
- rejected

## Attack 8 - Launchpad Anti-Rug Bypass

Try owner/admin path to bypass restrictions.

Expected:
- rejected

## Attack 9 - Runtime Panic Path

Trigger unwrap/expect/panic in consensus/runtime-critical code.

Expected:
- no panic reachable in production path

## Attack 10 - Genesis/Mainnet Config Footgun

Detect unsafe defaults, mock keys, fake balances, dev chain spec flags.

Expected:
- launch gate blocks
