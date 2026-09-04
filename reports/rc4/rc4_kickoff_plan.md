# RC4 Kickoff Plan

## Purpose

RC4 should convert the RC3 failure-drill proof into a current-runtime release candidate. RC3 proved validator failure, recovery, settlement, halt/refund, bridge-disabled safety, and bad-config rejection using the runtime-compatible RC2 node/spec fallback. RC4 should remove that fallback dependency.

## Current Reality

- RC3 passed: 37 checks, 0 failures.
- RC3 evidence is stored in `reports/rc3/`.
- RC3 ran with `target/rc2-current-node/debug/x3-chain-node` and `chain-specs/x3-local-rc2-raw.json` because current fresh node builds are still blocked by compiler/runtime build instability.
- The latest pinned debug build failed while compiling `sp-io` for `wasm32-unknown-unknown` with a Rust compiler `SIGSEGV` in `rustc_mir_build::check_unsafety`.

## Verified

- Three-validator local network boots, produces blocks, finalizes, degrades under validator loss, and recovers.
- One-validator state does not falsely finalize below GRANDPA quorum.
- Post-recovery settlement probes return pending supply to zero.
- Economic halt/refund safety checks pass.
- External bridge routes remain disabled/rejected.
- Bad genesis/config probes reject unsafe launch configurations.
- All RC3 JSON evidence files parse as valid JSON.

## RC4 Goal

Produce a current-runtime RC candidate that can run the same failure and safety drills without the RC2 fallback binary/spec pair.

## Required Work

1. Stabilize the current `x3-chain-node` build path.
2. Produce a compatible current raw local3 chain spec.
3. Run `scripts/mainnet/rc3_failure_drills.sh` against the current binary/spec pair.
4. Add an RC4 report under `reports/rc4/` with binary hash, spec hash, drill verdict, and any remaining release blockers.
5. Decide whether the rustix diagnostic-cfg workaround is required, and either document/commit it or remove it from the working tree.

## Known Blocker

The current build path is blocked by compiler instability, not by the RC3 drill script. The most recent failure was:

```text
error: rustc interrupted by SIGSEGV
could not compile `sp-io` for wasm32-unknown-unknown
rustc_mir_build::check_unsafety
```

## RC4 Pass Criteria

- `target/debug/x3-chain-node` or `target/release/x3-chain-node` builds from the current tree.
- Current binary can generate or consume a current local3 raw spec.
- Failure drill report passes with 0 failed checks.
- JSON evidence files validate.
- Release notes clearly distinguish verified behavior from remaining blockers.