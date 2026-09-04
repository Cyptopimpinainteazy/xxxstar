# Workflow: Cross-VM Review

## When To Use
Before claiming any cross-VM (EVM/SVM/BTC/CosmWasm/X3VM) feature is working.

## Steps
1. Identify all VMs involved in the operation.
2. Trace the message flow: source chain → bridge/adapter → destination chain.
3. Verify message format compatibility across VMs.
4. Verify timeout and refund paths exist on both ends.
5. Verify finality handling is correct (not 1-block assumption).
6. Verify replay protection across chains.
7. Run cross-VM integration tests if available.
8. Simulate failure scenarios: timeout, reorg, malformed message, gas spike.
9. Run `scripts/x3-detect-stubs.sh` on bridge/adapter paths.
10. File findings in proof report.

## Required Checks
- Two-phase commit or HTLC for atomicity.
- Timeout + refund tested.
- Finality assumptions correct per chain.
- Replay protection present.
- Malformed message handling exists.

## Proof Commands
- Cross-VM integration tests.
- `scripts/x3-detect-stubs.sh` on bridge paths.
- Failure injection tests.

## Exit Criteria
- Cross-VM integration tests pass.
- Failure paths tested.
- No stubs in bridge/adapter production paths.