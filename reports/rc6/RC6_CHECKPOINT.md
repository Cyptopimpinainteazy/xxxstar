# RC6 Checkpoint

## Snapshot

- Date: 2026-05-16
- Commit: `719845d950d4506577798fcfb99d73235024a51c`
- RC5 status: still intended to run separately as the 72-hour internal alpha on server
- RC6 status: package files created, readiness script present, current verdict `FAIL`

## What Exists

- RC6 readiness script exists at `scripts/mainnet/rc6_public_testnet_readiness.sh`
- Public testnet docs exist under `docs/testnet/`
- RC6 report/checklist files exist under `reports/rc6/`
- External bridge disable gate is documented and currently checks `TESTNET_FEATURE_FLAGS.toml`

## Current RC6 Failure Cause

The last RC6 run failed because build tooling was unavailable in the execution environment:

- `cargo` was not available in `PATH`
- release node build did not run
- runtime WASM build did not run
- public chain spec generation did not run
- dev-key scrub check could not run because specs were not generated

This is an environment blocker, not a documentation/package-structure blocker.

## Files To Reuse On Resume

- `scripts/mainnet/rc6_public_testnet_readiness.sh`
- `reports/rc6/rc6_public_testnet_readiness_report.md`
- `reports/rc6/release_artifacts.sha256`
- `reports/rc6/node_release_build.log`
- `reports/rc6/runtime_wasm_build.log`

## Resume Sequence

1. Run RC5 72-hour alpha on server and let it continue independently.
2. Resume local/package work in an environment with Rust toolchain available.
3. Re-run:

```bash
cd /home/lojak/Desktop/X3_ATOMIC_STAR
bash scripts/mainnet/rc6_public_testnet_readiness.sh
```

4. If build gates pass, verify generated files:

- `chain-specs/x3-public-testnet-plain.json`
- `chain-specs/x3-public-testnet-raw.json`

5. If RC6 turns green, then commit RC6 package changes.

## Best Next Build Targets

If you want to keep building in parallel while RC5 runs, the next highest-value items are:

1. Replace `PENDING_BEFORE_LAUNCH` placeholders in RPC/faucet/explorer docs with real endpoints or named owners.
2. Add real bootnode addresses once deployment hosts exist.
3. Validate that the `testnet` chain alias produces a public-safe spec with no dev authorities.
4. Add a fresh-machine validator dry-run using the RC6 docs.
5. Commit RC6 package once the build environment issue is removed and the script passes.

## Launch Guard

RC6 may reach:

- `RC6_PACKAGE_READY: PASS`
- `BOOTNODE_DEPLOYMENT: PENDING`

But public testnet launch still stays blocked until bootnodes are live and RC5 stability is complete.