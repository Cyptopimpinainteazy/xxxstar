# X3 Critical Ops Flowcharts

This pack contains the top 3 high-value operational flows:
- Runtime Upgrade Safety
- Incident Response and Chain Halt
- Validator Lifecycle

Generated image assets:
- [runtime_upgrade_safety.svg](docs/diagrams/runtime_upgrade_safety.svg)
- [runtime_upgrade_safety.png](docs/diagrams/runtime_upgrade_safety.png)
- [incident_response_chain_halt.svg](docs/diagrams/incident_response_chain_halt.svg)
- [incident_response_chain_halt.png](docs/diagrams/incident_response_chain_halt.png)
- [validator_lifecycle.svg](docs/diagrams/validator_lifecycle.svg)
- [validator_lifecycle.png](docs/diagrams/validator_lifecycle.png)

---

## 1) Runtime Upgrade Safety Flow

```mermaid
flowchart TD
    A([Upgrade proposal drafted]) --> B[Pre-upgrade checks\nweight, storage version, migration plan, compatibility]
    B --> C{Simulation and rehearsal pass?}
    C -- No --> C1[Fix migration/runtime issues\nupdate tests and proofs]
    C1 --> B
    C -- Yes --> D[Governance vote + timelock]
    D --> E{Approval threshold reached?}
    E -- No --> E1[Proposal rejected or revised]
    E1 --> A
    E -- Yes --> F[Canary deploy\nsmall validator subset]
    F --> G{Health signals stable?\nblock production, finality, RPC, invariants}
    G -- No --> H[Rollback or emergency pause\ninvoke incident runbook]
    H --> A
    G -- Yes --> I[Full rollout]
    I --> J[Post-upgrade verification\nstate checks, event checks, performance checks]
    J --> K{Post checks pass?}
    K -- No --> H
    K -- Yes --> L([Upgrade accepted])
```

---

## 2) Incident Response and Chain Halt Flow

```mermaid
flowchart TD
    A([Alert triggered]) --> B[Triage severity\nSEV1/SEV2/SEV3]
    B --> C{Consensus or funds at risk?}
    C -- No --> D[Standard incident workflow\nassign owner and monitor]
    D --> Z([Resolved with postmortem])

    C -- Yes --> E[Activate emergency response team]
    E --> F[Collect evidence\nlogs, hashes, block ranges, affected modules]
    F --> G{Need immediate containment?}
    G -- Yes --> H[Emergency governance/multisig action\npause risky paths or halt]
    G -- No --> I[Apply guarded mitigations\nrate limits, route pause, feature disable]

    H --> J[Publish operator + community notice]
    I --> J
    J --> K[Root-cause analysis + patch]
    K --> L[Test fix on isolated environment]
    L --> M{Fix verified?}
    M -- No --> K
    M -- Yes --> N[Controlled recovery\nresume services in stages]
    N --> O{Health and invariants stable?}
    O -- No --> H
    O -- Yes --> P[Close incident\npublish postmortem and action items]
    P --> Z
```

---

## 3) Validator Lifecycle Flow

```mermaid
flowchart TD
    A([Prospective validator]) --> B[Operator readiness\nhardware, network, monitoring, keys policy]
    B --> C[Generate keys securely\nseparate stash/controller]
    C --> D[Register and stake/bond]
    D --> E{Accepted into active set?}
    E -- No --> F[Stay in waiting set\nimprove stake/reputation]
    F --> E
    E -- Yes --> G[Active duties\nblock production, finality votes, telemetry]

    G --> H{Liveness and performance healthy?}
    H -- Yes --> I[Continue operations\nupgrade on schedule]
    I --> G

    H -- No --> J[Alert + operator remediation window]
    J --> K{Recovered within policy window?}
    K -- Yes --> G
    K -- No --> L[Apply penalties/slash per rules]

    L --> M{Severe or repeated offense?}
    M -- No --> N[Return to waiting set or reduced role]
    N --> F
    M -- Yes --> O[Forced exit and rotation]

    O --> P[Key rotation and replacement onboarding]
    P --> Q([Validator lifecycle completed])
```
