# Workflow Hook: Pre-Task

Run before any implementation starts.

This hook establishes the rules, scope, and success criteria for the task.

## Required Output

```md
## Pre-Task Checklist

### Task restatement
- <exact task being done>
- User intent: <why they want this>
- Success definition: <what done looks like>

### Scope Lock

**What is IN scope?**
- <file/module>
- <subsystem>
- <domain>

**What is OUT of scope?**
- <explicitly excluded>
- <why not included>

**What changes are FORBIDDEN?**
- Do not modify <subsystem>
- Do not change <interface>
- Do not touch <critical component>

### Acceptance Criteria

Success requires ALL of:
- [ ] <criterion 1>
- [ ] <criterion 2>
- [ ] <criterion 3>
- [ ] All tests pass
- [ ] No new warnings
- [ ] No FIXABLE_NOW items remain

### Blast Radius Map

```txt
Primary subsystem affected:
- <subsystem>

Upstream dependencies (will call this code):
- <subsystem 1>
- <subsystem 2>

Downstream dependencies (this code will call):
- <subsystem 1>
- <subsystem 2>

Cross-subsystem impacts:
- <potential impact>

Risk level:
- LOW / MEDIUM / HIGH / CRITICAL
```

### Agent Routing

**Primary agent:** <name>
**Supporting agents:** <names>
**Reason:** <why this routing>

### Canonical Path

Document the one true execution path:

```txt
input
    ↓
<step 1>
    ↓
<step 2>
    ↓
output/result
```

### Priority Stack

| Priority | Item | Reason | Status |
|----------|------|--------|--------|
| P0 | <blocker> | blocks everything | OPEN |
| P1 | <required> | required for task | OPEN |
| P2 | <nice> | hardening | DEFERRED |

Rules:
- Fix P0 before P1
- Fix P1 before P2
- Do not bury P0/P1 under P3/P4

### Stop Conditions

The task may stop only when:
- [ ] All acceptance criteria are satisfied
- [ ] No FIXABLE_NOW items remain
- [ ] All tests pass or failures are honestly blocked
- [ ] Score cap matrix has been applied
- [ ] Final handoff pack is complete

The task must stop if:
- [ ] Required secrets are unavailable
- [ ] Required hardware is unavailable
- [ ] Required external service is unavailable
- [ ] The task requires a user decision
- [ ] Continuing would require unrelated rewrite outside scope

### Cross-VM Impact

Does this task touch Cross-VM?
- YES / NO

If YES, run Cross-VM Trace skill before coding.

### Blockers / Known Issues

- <blocker 1>
- <blocker 2>
- <unknown unknowns>

### Next Steps

1. Route to agents
2. Run required skills
3. Implement smallest complete fix
4. Validate
5. Apply gates
6. Produce final output
```

## Approval

Before implementation starts:

- [ ] Task is restated clearly
- [ ] Scope is locked
- [ ] Acceptance criteria are defined
- [ ] Blast radius is mapped
- [ ] Agents are routed
- [ ] Canonical path is identified
- [ ] Priority stack is set
- [ ] Stop conditions are defined
- [ ] Blockers are documented

If any box is unchecked, pre-task is incomplete. Do not start implementation yet.

---

**Next:** Route to agents and run required skills.
