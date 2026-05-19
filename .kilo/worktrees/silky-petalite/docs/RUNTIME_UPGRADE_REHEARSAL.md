# RUNTIME UPGRADE REHEARSAL

Target: v0.4 Internal-Only Mainnet RC

## Procedure

1. Launch old runtime
- Start node network from previous runtime release and confirm block production.

2. Submit upgrade
- Build new runtime WASM and submit governance-authorized runtime upgrade extrinsic.

3. Verify migration
- Confirm migration events emitted and no migration errors in logs.

4. Verify storage version
- Confirm pallet storage versions increment as expected.

5. Verify chain continues
- Confirm blocks continue finalizing after upgrade and internal routes still execute.

6. Rollback and emergency process
- If migration breaks invariants or liveness, use emergency governance process:
  - pause new transfers if required
  - execute rollback runtime upgrade
  - resume only after invariant and route checks pass

## Rehearsal Script Placeholder

- scripts/mainnet/runtime_upgrade_rehearsal.sh (to be implemented with environment-specific keys and endpoints)
