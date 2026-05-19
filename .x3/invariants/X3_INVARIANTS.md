# X3 Critical Invariants

## Universal Asset Kernel

`canonical_supply == native + evm + svm + external_locked + pending`

Required proof:
- accounting tests
- overflow tests
- rollback tests
- external locked supply tests

## Atomic Cross-VM Execution

Either all VM legs commit or all VM legs roll back.

Required proof:
- EVM success + SVM failure rolls back all legs
- SVM success + X3VM failure rolls back all legs
- expiry/deadline rejection
- replay rejection
- domain separation tests

## Bridge Safety

No message may execute twice.
No expired message may execute.
No message for a different chain/domain may execute.
No unaudited external bridge may be enabled by default.

Required proof:
- nonce uniqueness tests
- message hash replay tests
- expiry/deadline tests
- chain ID/domain tests
- mainnet bridge gate

## DEX Accounting

Pool reserves, LP supply, fees, and locked liquidity must remain consistent.

Required proof:
- swap invariant tests
- liquidity add/remove tests
- fee accounting tests
- slippage/TWAP tests
- lock bypass tests

## Runtime Safety

Consensus-critical runtime paths must not panic.

Required proof:
- panic/unwrap audit
- runtime tests
- try-runtime/runtime-upgrade rehearsal where available
- chain spec/genesis review
