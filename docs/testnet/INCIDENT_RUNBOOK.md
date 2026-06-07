# X3 Incident Runbook

## Purpose

Provide immediate and consistent response for public testnet incidents.

## Severity Levels

### SEV1

Chain halted, finality broken, invariant failure, key compromise, or critical consensus risk.

### SEV2

RPC outage, validator outage, faucet abuse, explorer outage, or major degraded service.

### SEV3

Documentation error, non-critical UI issue, low-impact operational defects.

## Immediate Actions

1. Preserve logs and relevant telemetry snapshots.
2. Stop unsafe services or endpoints when needed.
3. Do not delete chain data.
4. Capture incident timeline and affected components.
5. Notify validators/operators with scope and immediate guidance.
6. Create incident record with owner and next checkpoint time.

## Communication

- Primary status channel: `#engineering-updates` Slack
- Validator notification escalation: `#meta-blockers-questions` Slack
- SEV1/SEV2 security-critical escalation: `#security-critical` Slack
- Incident commander assignment: on-call operator

These channels are referenced in `stakeholder_comms/ENGINEERING_TEAM_ANNOUNCEMENT.md` and `stakeholder_comms/CTO_BRIEF_META_BLOCKERS.md`.

## Recovery Steps

1. Validate root cause and mitigation in staging/sandbox where possible.
2. Apply minimal safe remediation.
3. Confirm chain health, finality, and RPC recovery.
4. Publish post-incident update with residual risk.

## Postmortem Requirements

- Incident timeline
- Root cause
- Corrective action
- Preventive action and owner

## Scope Rule

External bridges are disabled for initial public testnet and are out of RC6 launch package scope.
