# Scan Bootstrap Summary

Date: 2026-05-02

## Phase 1 Status

Full file enumeration was executed with `.scripts/full_scan.sh`.

Evidence:
- `.cache/file_list.txt`: 114395 files
- `CODE_COVERAGE_TRACKER.md`: populated from `.cache/file_list.txt`
- Current deep-scan coverage: 0%

The repo is now bootstrapped for autonomous scanning, but the file-by-file deep
scan is not complete. Do not claim 100% coverage until every path in
`CODE_COVERAGE_TRACKER.md` is processed and moved out of Remaining.

## Smell Scan Status

`.scripts/smell_scan.sh` completed and wrote `.reports/smells.txt`.

Evidence:
- `.reports/smells.txt`: 32089 lines

Keyword counts from the initial smell report:
- `TODO`: 1596
- `FIXME`: 116
- `unwrap(`: 14137
- `expect(`: 6587
- `panic!`: 1108
- `unimplemented!`: 131
- `todo!`: 42
- `mock`: 6167
- `hardcoded`: 243
- `unsafe`: 1633
- `localhost`: 4270

Top noisy paths:
- `reports/panic_unwrap_audit.md`: 3789 matches
- `proof/reports/todo_gate_mainnet_20260426_194331.txt`: 532 matches
- `launch-gates/evidence/embarrassment-raw-findings.txt`: 298 matches
- `crates/cross-vm-bridge/src/lib.rs`: 113 matches

## Inventory Notes

Top-level file distribution from `.cache/file_list.txt`:
- `launch-gates/`: 54361 files
- `target_strict/`: 33683 files
- `apps/`: 13395 files
- `.kilo/`: 6105 files
- `crates/`: 1133 files
- `docs/`: 1110 files
- `.venv/`: 974 files
- `pallets/`: 555 files

`target_strict/` and `.venv/` are currently included by the requested scanner.
That may be intentional for full inventory, but they are generated or environment
artifacts and should be classified separately before using coverage percentage
as a source-readiness signal.

## Validation

Executed:
- `.scripts/full_scan.sh`
- `.scripts/smell_scan.sh`
- `bash -n .scripts/full_scan.sh .scripts/smell_scan.sh`

Result:
- Shell syntax check passed.
