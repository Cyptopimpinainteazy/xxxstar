# Hook: Stub Detector

## When To Run
After every code change, before committing.

## Maps To
`scripts/x3-detect-stubs.sh`

## What It Blocks
Committing or claiming completion when stubs exist in runtime, adapter, consensus, bridge, verifier, execution, security, or rollback paths.

## Gate
HARD GATE on critical paths. Stubs in non-critical paths are warnings only.
Critical paths: `runtime/`, `pallets/`, `bridges/`, `adapters/`, `crates/x3-gateway/`, `crates/cross-vm-*/`, `crates/atomic-*/`, `X3-contracts/`