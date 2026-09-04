# Execution Playbook - Hub Fee Testnet Validation

## Mission

Get a binary answer in one session: is hub fee extraction safe and correct on testnet?

## Timebox

- 10 min deployment + health
- 30 min functional checks
- 30 min monitoring
- 5 min decision

## Commands

See `DEPLOYMENT_QUICK_REFERENCE.md`.

## Evidence to collect

- validator health response
- block production confirmation
- `HubFeeCollected` events for test values
- post-fee earnings/withdraw snapshots
- monitor script summary output

## Decision Rule

GO only if all checklist items pass.
NO-GO on any math, event, or stability inconsistency.
