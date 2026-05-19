# FRAME Weights Blocker - 2026-04-26

## Status

FRAME weight replacement is blocked. Do not claim `FRAME Weights: generated and committed`.

## Completed

- Installed Substrate proof tools:
  - `srtool-cli 0.13.2`
  - `subwasm v0.21.3`
  - `zombienet 1.3.138`
  - `chopsticks 1.2.8`
- Updated `launch-gates/run-substrate-proof-pack.sh` to verify the installed tools.
- Re-ran the proof pack:
  - PASS: 8
  - WARN: 1
  - FAIL: 0
  - SKIP: 2

## Blocker

The benchmark-enabled node binary does not build cleanly yet, so pallet benchmarks cannot be executed and weight files cannot be replaced with real generated output.

Attempted command:

```bash
CARGO_BUILD_JOBS=1 cargo build -p x3-chain-node --features runtime-benchmarks
```

Observed failure after the long native dependency build:

```text
error: failed to write target/debug/.fingerprint/sp-rpc-*/invoked.timestamp
Caused by: No such file or directory (os error 2)
```

A retry with a separate target directory was attempted:

```bash
CARGO_TARGET_DIR=target/bench-node CARGO_BUILD_JOBS=1 cargo build -p x3-chain-node --features runtime-benchmarks
```

Observed failure:

```text
error: could not write output to target/bench-node/debug/deps/syn-*.rcgu.o: No such file or directory
```

## Contributing Factors

- The filesystem is nearly full: `/` was observed at 97% used with about 14 GiB available.
- A separate VS Code terminal was running `cargo check --all` in this workspace.
- A continuous verifier from `/home/lojak/Desktop/x3-chain-master/scripts/testnet/continuous-verify.sh` was also active.
- These concurrent jobs touched the workspace while the benchmark build was running, which makes Cargo target writes unreliable.

## Required Next Command Window

Run benchmarks only after freeing disk and stopping concurrent Cargo/testnet verifier jobs:

```bash
df -h .
ps -o pid,ppid,stat,etime,cmd -C cargo -C rustc -C make
CARGO_TARGET_DIR=target/bench-node CARGO_BUILD_JOBS=1 cargo build -p x3-chain-node --features runtime-benchmarks
```

If the binary builds, run pallet benchmarks and write generated weights only then.

## Launch-Pallet Weight Risk

The latest proof pack still reports manual/placeholder weights. See:

```text
launch-gates/evidence/substrate/placeholder-or-manual-weight-scan-20260426-121738.log
```
