# Deployment Quick Reference (Hub Fee)

## Step 1: Runtime Deploy Prep

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR_corrupt_backup_20260518-225645
```

Backup current runtime artifact (adjust to your host path):

```bash
cp /var/lib/x3-testnet/runtime.wasm /var/lib/x3-testnet/runtime.wasm.backup.$(date +%s)
```

## Step 2: Start Validator

Systemd:

```bash
sudo systemctl restart x3-testnet-validator
sudo systemctl status x3-testnet-validator --no-pager
```

Manual:

```bash
./quickstart-testnet.sh
```

Docker (example):

```bash
docker compose up -d
```

## Step 3: Health Check

```bash
curl http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'
```

## Step 4: Phase 1 Functional Validation

Perform extrinsics in UI or your automation:
- register dApp
- record revenue: `1_000_000`
- record revenue: `10_000_000`
- verify `HubFeeCollected` events
- verify post-fee earnings
- test withdrawal

Expected fee math:
- 1,000,000 -> 25,000
- 10,000,000 -> 250,000

## Step 5: Phase 2 Monitoring

```bash
chmod +x ./TESTNET_HUB_FEE_MONITOR.sh
./TESTNET_HUB_FEE_MONITOR.sh ws://localhost:9944 30
```

## Step 6: Decision

Use `DEPLOYMENT_VERIFICATION_CHECKLIST_HUB_FEE.md`.
- All pass -> TESTNET GO
- Any fail -> NO-GO and rollback/troubleshoot
