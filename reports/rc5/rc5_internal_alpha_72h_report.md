# RC5 Internal Alpha 72h Report

## Verdict

RC5_SHORT_SHAKEDOWN: FAIL (RC5_72H: PENDING)

## Status Gates

- RC5_SHORT_SHAKEDOWN: FAIL
- RC5_72H: PENDING

## Scope

- validators: 3
- duration_seconds: 30
- snapshot_interval_seconds: 60
- settlement_interval_cycles: 10
- restart_drill_cycle: 3
- external bridges: disabled (required)

## Result Files

- health_snapshots.jsonl
- finality_snapshots.jsonl
- settlement_snapshots.jsonl
- invariant_snapshots.jsonl
- validator_restart_drill.json
- resource_usage.jsonl
- final_summary.json

## Blockers

- panic loop detected in validator logs
- database corruption indicators detected in logs
- one or more invariant snapshots failed
- external bridges not disabled
