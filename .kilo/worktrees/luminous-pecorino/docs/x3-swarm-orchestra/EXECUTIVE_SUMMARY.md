# X3 Swarm + Orchestra Platform — Executive Summary

This document captures the target operating model for the X3 swarm-orchestra platform in a way that matches the codebase as it exists today. The platform combines X3 Chain's dual-VM runtime, deterministic GPU validation, sidecar benchmark infrastructure, gateway APIs, and governance controls into a bounded execution fabric. It does not assume that the deprecated swarm path is still the production base, and it does not relabel the protocol court as a human approval board.

## Platform shape

The platform is split into four layers. The interface layer includes desktop, web, wallet, and CLI clients backed by the gateway. The protocol layer includes the X3 node, dual-VM execution, cross-VM settlement, verifier paths, and on-chain governance. The service layer includes `x3-gpu-validator-swarm`, `x3-sidecar`, gateway-backed workflow services, and future orchestra-control-plane services. The control layer includes human approval, CRM voting adapters, evidence capture, security enforcement, incident response, and audit.

```mermaid
graph LR
  subgraph Frontend
    UI[Desktop / Web / Mobile / CLI]
  end
  subgraph Gateway
    API[GraphQL / REST Gateway]
  end
  subgraph Core
    Chain[Blockchain Node (X3)]
    Validator[X3 GPU Validator Swarm]
    Sidecar[X3 Sidecar]
    Control[Orchestra Control Plane]
    CRM[CRM / Voting Adapter]
  end
  UI --> API
  API --> Chain
  API --> Sidecar
  API --> Control
  Validator --> Chain
  Sidecar --> Chain
  Control --> API
  CRM --> Control
```

The design goal is bounded autonomy under explicit routing, approval, and evidence rules. The conductor coordinates specialized services instead of replacing them, and every high-risk action stays attached to a deterministic policy boundary.

## Architecture and lifecycle

The blockchain layer is a Substrate-based node with EVM and SVM support, verifier and settlement paths, and governance-controlled runtime actions. The active validator path is `x3-gpu-validator-swarm`, which already implements deterministic execution, replay, quarantine, and CPU fallback. The sidecar already runs execution jobs, generates receipts, tracks status, and publishes benchmark reports. The gateway already exposes chain data and benchmark-report APIs. Orchestra-control-plane is the missing off-chain service boundary that should own intents, approvals, vote windows, evidence bundles, and reward orchestration.

Platform evolution follows proposal-first delivery for any subsystem addition, architecture shift, or material security and performance initiative. Operationally, new services do not become permanent parts of the platform until their interfaces, invariants, rollout plan, and safety prerequisites are written down.

## Compute model

GPU resources are reserved first for deterministic validation, replay, benchmarking, inference, graph analysis, and other bounded parallel workloads. CPU-oriented nodes run low-latency control services, blockchain nodes, orchestration paths, approval flows, and other tasks that depend on predictable coordination more than throughput.

The system should support hybrid execution. Local or colocated GPU resources provide the lowest-latency path for the most sensitive workloads. Specialized GPU cloud providers can absorb burst demand and exploratory workloads. General hyperscaler GPUs remain a fallback for availability and convenience, but their price profile makes them a worse steady-state option for heavy sustained inference. The routing layer should understand latency sensitivity, data sensitivity, cost ceilings, and determinism requirements before deciding where to run work.

## Service roles and workflow hierarchy

The platform should treat each execution surface as a role-typed worker. Validator services validate and benchmark. Gateway services expose approved APIs and persistence-backed reads. Orchestra-control-plane accepts intents, routes them to the correct workflow, and blocks execution until required approvals exist. CRM integration manages vote eligibility and tally import. Security services observe, contain, and preserve evidence.

Content and media workflows operate on a separate track from validation and financial execution. They should consume only pre-approved assets and templates, then route draft output back into human review. Compliance and security services inspect counterparties, workload profiles, and policy boundaries. Autonomic operations may restart services, scale infrastructure, and apply containment, but they do not publish, sanction, or spend without approval.

The workflow hierarchy is therefore intake first, policy classification second, approval or vote routing third, execution fourth, and evidence preservation last. Skipping stages should be possible only for explicitly low-risk internal tasks such as validation replay, benchmarking, or health checks.

## Governance court and approval flow

The platform includes a hybrid governance and dispute model. On-chain governance remains the place for runtime and treasury-affecting actions. `x3-court` remains the deterministic dispute-and-slashing system for replayable protocol disputes. Human approval boards and CRM voting windows are off-chain control-plane workflows for new strategies, exceptional spending, content publication, and policy escalation.

This is not a free-form social committee. The human board exists to approve policy and exceptional decisions, while automated services execute bounded rules and produce evidence. That lets the platform benefit from cryptoeconomic enforcement without losing operator accountability.

## Safety and reaper boundaries

Safety controls have to exist at multiple levels. Validator nodes need deterministic admission and quarantine controls. Critical services need hard kill paths, circuit breakers, and rate-limit controls. External-facing capabilities need sandboxing and egress restriction. Suspicious workloads should be diverted into quarantine or forensic environments rather than allowed to keep touching live systems.

The dedicated safety and security layer described in [x3-security-swarm/README.md](../../x3-security-swarm/README.md) should become the enforcement substrate for this platform. Two concrete preconditions now exist in code: authority startup must pass the determinism gate, and governance durations must match the live 200ms runtime cadence.

## Security, compliance, and operational controls

The platform has to assume that it will handle money, keys, regulated counterparties, sensitive business data, and privileged infrastructure. That requires HSM-backed or MPC-backed signing flows, strict service-to-service authentication, private networking around sensitive surfaces, immutable logs, and non-optional review around privileged changes. Compliance agents and reporting flows must stay downstream of the same event stream as the rest of the platform so audit and tax visibility are not reconstructed from partial data later.

Operationally, releases should move through development, staging, and production environments with automated checks for tests, invariant gates, spec compliance, and deployment safety. Observability should combine Prometheus, Grafana, log aggregation, and alert routing so the operator can see throughput, latency, failed approvals, agent health, trading losses, and security incidents in one place.

## Repository layout

The monorepo structure already supports this direction. Applications belong under `apps/`. Runtime modules belong under `pallets/`. Rust services and reusable libraries belong under `crates/`. Service-oriented adapters and workflow integrations belong under `services/`. Media assets remain isolated under a governed asset tree. Specifications, operator manuals, and execution rules belong under `docs/` and `docs/openspec/changes/`.

That structure matters because the platform is large enough to decay quickly without clear module boundaries. The point of the orchestra model is not just multi-agent automation. It is multi-agent automation that remains reviewable and maintainable once it stops fitting in one person’s head.

## Trading strategy and risk boundaries

Cross-chain and cross-venue arbitrage remain viable workloads for the platform, but they need to stay behind hard risk rules. Position caps, slippage ceilings, kill thresholds, circuit breakers, and route simulation are required before live capital is committed. Historical replay and Monte Carlo backtesting should calibrate profit thresholds and failure handling before a route is approved for real execution.

Compliance boundaries matter as much as strategy quality. Trading agents should be unable to move against unapproved counterparties, sanctioned assets, or unreviewed strategy classes. The platform is designed to make profitable automation possible without allowing invisible capital movement.

## Assumptions and next delivery step

This design assumes access to reliable market and chain data, GPU capacity, Kubernetes-based deployment, CRM approval infrastructure, pre-approved content assets, and a small trusted operator team. It also assumes that legal and compliance obligations remain the responsibility of the operating entity rather than the software itself.

The next delivery step is to treat this summary as a proposal-backed subsystem definition. That means the OpenSpec change, invariant entries, implementation tasks, and rollout phases should be recorded alongside this document before service scaffolding expands further.
