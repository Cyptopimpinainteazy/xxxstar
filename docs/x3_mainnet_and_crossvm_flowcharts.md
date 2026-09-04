# X3 Mainnet + Atomic Cross-VM Flowcharts

This file includes diagrams in two forms:
- Mermaid diagrams for rich rendering
- Plain-text fallback for environments where Mermaid does not render

Exported image assets are available in [docs/diagrams](docs/diagrams):
- [atomic_crossvm_swimlane.svg](docs/diagrams/atomic_crossvm_swimlane.svg)
- [atomic_crossvm_swimlane.png](docs/diagrams/atomic_crossvm_swimlane.png)
- [full_features_launch_compact.svg](docs/diagrams/full_features_launch_compact.svg)
- [full_features_launch_compact.png](docs/diagrams/full_features_launch_compact.png)

---

## 1) Boardroom Swimlane Version (Atomic Cross-VM)

```mermaid
flowchart LR
    subgraph L1[Lane 1: User / Wallet]
      U1[Create transfer intent\nasset/amount/src/dst/nonce/ttl]
      U2[Submit xVM extrinsic]
      U3[Receive success/fail receipt]
      U1 --> U2
    end

    subgraph L2[Lane 2: Router + Kernel]
      R1[Pre-checks\nroute open, asset not paused, amount > 0]
      R2[Replay guards\nUsedMessages + UsedNonces]
      R3[Lock/debit source\nupdate represented + pending_supply]
      R4[Build packet commitment\nroute + expiry + proof context]
      R5{Internal or External route?}
      R6[Internal 6-route dispatch\nNative/EVM/SVM]
      R7[External gateway path\nonly if enabled + audited + governance]
      R8[Post-state invariant check\nrepresented_total <= canonical_supply]
      R9{Invariant broken?}
      R10[Economic halt + recovery-only mode]
      R11[Emit proof/event + finalize]
      R1 --> R2 --> R3 --> R4 --> R5
      R5 -->|Internal| R6
      R5 -->|External| R7
      R8 --> R9
      R9 -->|Yes| R10
      R9 -->|No| R11
    end

    subgraph L3[Lane 3: Destination VM]
      D1[Execute destination credit/mint/unlock]
      D2{Execution success?}
      D3[Mark completed]
      D4[Rollback/refund to source]
      D1 --> D2
      D2 -->|Yes| D3
      D2 -->|No| D4
    end

    subgraph L4[Lane 4: Settlement + Timeout]
      S1[MATCH]
      S2[ASSETS_LOCKED_X3]
      S3[EXTERNAL_EXECUTION]
      S4[PROOF_SUBMITTED]
      S5[FINALIZE_X3]
      S6[REFUND_X3]
      S7{TTL expired before completion?}
      S8[Cancel expired transfer\nrefund source\nclear pending_supply]
      S1 --> S2 --> S3 --> S4 --> S5
      S3 -->|failure/timeout| S6
      S7 -->|Yes| S8
    end

    subgraph L5[Lane 5: Governance / Audit]
      G1[Bridge toggle default OFF in RC1]
      G2[External audit required]
      G3[Governance supermajority approval]
      G4[Enable bridge + controlled rollout]
      G1 --> G2 --> G3 --> G4
    end

    U2 --> R1
    R6 --> D1
    R7 --> D1
    D3 --> R8
    D4 --> R8

    D1 -. external settlement path .-> S1
    R4 -. timeout watch .-> S7
    S8 --> R8

    G4 -. enables .-> R7

    style U3 fill:#ecfdf5,stroke:#166534
    style R10 fill:#fef2f2,stroke:#991b1b
    style G1 fill:#fff7ed,stroke:#9a3412
```

---

## 2) Full Features Launch Flow (Compact)

```mermaid
flowchart TD
    A([Start]) --> B[Prereqs + security hygiene]
    B --> C[Build real node]
    C --> D[Consensus health\nAura + GRANDPA + 3 validators]
    D --> E[Cross-VM + supply invariant test gates]
    E --> F[CI critical path all-pass]
    F --> G[RC1 scope freeze\ninternal features enabled, risky features off]
    G --> H[Public testnet gates\nexplorer + faucet + slashing test]
    H --> I[External security audit]
    I --> J[Launch hardening\nbug bounty + emergency runbooks + reproducible artifacts]
    J --> K[Genesis ceremony]
    K --> L([Mainnet Live])
    L --> M[Post-mainnet governance unlocks\nbridges, PQ, AI, GPU, advanced DEX, etc.]
```

---

## 3) Plain-Text Fallback (Always Visible)

### Atomic Cross-VM (swimlane equivalent)

1. User/Wallet
- Create transfer intent (asset, amount, source VM, destination VM, nonce, TTL)
- Submit transfer extrinsic

2. Router/Kernel
- Validate route/asset/amount/origin
- Enforce replay protection (message + nonce uniqueness)
- Lock or debit source-side value
- Increase pending supply accounting
- Build message packet with expiry and proof context
- Branch to internal route (Native/EVM/SVM) or external route (if enabled)

3. Destination VM
- Execute destination-side credit/mint/unlock
- If success: mark completed
- If failure: trigger rollback/refund path

4. Settlement/Timeout
- Settlement states: MATCH -> ASSETS_LOCKED_X3 -> EXTERNAL_EXECUTION -> PROOF_SUBMITTED -> FINALIZE_X3
- Failure/timeout goes to REFUND_X3
- If TTL expires before completion: cancel expired transfer and refund source

5. Global Safety
- Re-check supply invariants after completion/refund
- If invariant fails: economic halt, block risky ops, allow recovery-only flows
- If invariant passes: finalize and emit receipts/events

6. Governance/Audit for external features
- RC1 default: external bridges disabled
- To enable later: external audit + governance approval + controlled rollout
