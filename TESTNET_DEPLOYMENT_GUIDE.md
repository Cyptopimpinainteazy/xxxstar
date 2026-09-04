# Testnet Deployment Guide - Hub Fee

## Purpose

Validate hub fee extraction (250 bps) on testnet with runtime + event evidence.

## Preconditions

- Validator host access
- RPC available (default expected: `ws://localhost:9944`, `http://localhost:9933`)
- Runtime deploy procedure available in your environment

## Procedure

### 1. Deploy

1. Backup runtime artifact
2. Deploy updated runtime
3. Restart validator
4. Check health + block production

### 2. Functional Validation (Phase 1)

Execute the following in UI or scripted extrinsics:

1. Register dApp
2. Record revenue `1_000_000`
3. Verify `HubFeeCollected` fee = `25_000`
4. Record revenue `10_000_000`
5. Verify `HubFeeCollected` fee = `250_000`
6. Verify net earnings and withdrawal behavior

### 3. Continuous Monitoring (Phase 2)

Run:

```bash
./TESTNET_HUB_FEE_MONITOR.sh ws://localhost:9944 30
```

Capture:
- event count
- total fees
- malformed/missing fields

### 4. Decide

Complete checklist in `DEPLOYMENT_VERIFICATION_CHECKLIST_HUB_FEE.md` and mark GO/NO-GO.

## Rollback

If NO-GO:
1. stop validator
2. restore backed-up runtime artifact
3. restart validator
4. confirm chain health
