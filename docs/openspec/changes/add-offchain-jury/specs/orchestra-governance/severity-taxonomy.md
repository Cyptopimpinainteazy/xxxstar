# Severity Taxonomy for Task Classification

## Overview
This document defines the severity classification rules for routing tasks to jury review versus core approval.

## Classification Rules

### MAJOR Severity (→ Jury Review Required)
Tasks are classified as **MAJOR** if they involve any of:

#### Governance Domain
- Changes to governance rules or voting thresholds
- Changes to agent roles or role definitions
- Modifications to the Score (immutable core rules)
- Law proposals that affect protocol behavior
- Changes to jury composition rules or rotation logic

#### Financial/Treasury Domain
- Any transaction > threshold T (configurable, default: 100k X3)
- Agent rewards/slashing/staking modifications
- Treasury rebalancing or allocation changes
- Economic parameter adjustments (inflation, discount rates, etc.)
- Payment approvals or transfers

#### Security & Infrastructure
- Changes to security boundaries or isolation rules
- Cryptographic algorithm changes or key rotations
- Network topology changes with consensus implications
- Emergency overrides or privilege escalations
- Audit log modification or tampering

#### Schema & Storage
- Persistent storage schema migrations
- Runtime state layout changes
- On-chain state modifications
- Data retention or purge operations

#### Agent & Task Management
- First-wave agent onboarding or offboarding
- Agent privilege changes or tool access grants
- Task execution model changes
- Failure escalation rule modifications

### MINOR Severity (→ Core Approval Only)
Tasks are classified as **MINOR** if they:

- Update application configuration (non-security)
- Modify telemetry, logging, or observability settings
- Update documentation or comments
- Fix bugs (restoring intended behavior, no spec changes)
- Perform routine operations (status checks, caching, resets)
- Update monitoring thresholds or alert levels
- Clean up temporary files or logs
- Non-breaking dependency updates

## Severity Gating in Workflow

```
Task Submission
    ↓
[1] Parse task intent, extract severity
    ↓
    ├─→ MINOR? → Core Approval Gate → Execute if approved
    │
    └─→ MAJOR? → Jury Voting Gate → Execute only if jury approves
```

## Severity Override Rules

### Escalation (MINOR → MAJOR)
- **Automation**: No automatic escalation; must be explicit in task intent
- **Human Veto**: Humans (via file deletion) override any classification
- **Audit Trigger**: Operations flagged in audit logs may be escalated on demand

### De-escalation (MAJOR → MINOR) 
- **Prohibition**: MAJOR tasks cannot be de-escalated without explicit spec change
- **Reason**: Protects against malicious reclassification

## Examples

### Classified as MAJOR
- "Update inflation rate from 2% to 2.5%"
- "Approve 50k X3 transfer to new validator"
- "Rotate jury for epoch 5"
- "Update agent Role 7 permissions to include token approval"
- "Migrate user schema to v2 (incompatible with v1)"

### Classified as MINOR
- "Update log level from INFO to DEBUG"
- "Fix typo in error message"
- "Increase monitoring alert threshold from 80% to 85%"
- "Refresh metrics cache"
- "Update README documentation"

## Initial Subsystem Defaults

| Subsystem | MAJOR Threshold | Notes |
|-----------|-----------------|-------|
| Governance | Any rule change | Law proposals always routed to jury |
| Economics | > 100k X3 | Treasury safety threshold |
| Security | Any boundary change | Zero-trust principle |
| Agents | Any role change | Principle of least surprise |
| Operations | N/A (all MINOR) | Status & monitoring only |

## Future Extensibility

Subsystems may define their own severity rules by:
1. Creating `specs/[subsystem]/severity.md`
2. Referencing subsystem-specific thresholds or categories
3. Inheriting base rules from this taxonomy unless explicitly overridden

Example: `specs/oracle-network/severity.md` might define:
- MAJOR: Oracle price feed updates > 10% deviation
- MAJOR: Oracle node addition/removal
