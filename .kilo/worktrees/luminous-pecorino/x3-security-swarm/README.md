# X3 Security Swarm — Build Pack v1

This directory is the reference build pack for the X3 Security Swarm. It is designed to sit beside the existing GPU swarm and validator-swarm crates, then drive implementation into the runtime, orchestration layer, operator tools, and governance surfaces already present in the repository.

The pack is intentionally defensive. It does not include counter-hacking, device interference, off-chain punishment, or any retaliatory behavior. Its scope is protocol defense: detect, attribute, contain, record, challenge, and recover. That keeps the system lawful, auditable, and deployable.

## Integration targets

The nearest implementation anchors already exist in the repository. [crates/x3-gpu-validator-swarm/src/orchestrator.rs](../crates/x3-gpu-validator-swarm/src/orchestrator.rs) is the scheduling and lifecycle surface for deterministic GPU workloads. [crates/x3-gpu-validator-swarm/src/quarantine.rs](../crates/x3-gpu-validator-swarm/src/quarantine.rs) already contains validator isolation and audit primitives. [crates/gpu-swarm/src/warden/governance.rs](../crates/gpu-swarm/src/warden/governance.rs) already models threat levels, governance actions, and emergency overrides.

This build pack turns those building blocks into a coherent security program with four operational classes. `Sentinel-Watcher` performs detection and produces evidence-only findings. `Sentinel-Judge` correlates watcher outputs and assigns a bounded attribution score. `Sentinel-Warden` executes reversible containment only after quorum. `Sentinel-Scribe` records the incident bundle, evidence lineage, and action log.

## Directory layout

The `agents/templates/` directory contains machine-readable class definitions that can be consumed by a future spawner or orchestration service. The `agents/prompts/` directory contains the corresponding behavioral instructions. The `governance/` directory defines the security charter, appeals flow, and quorum rules. The `chaos/` directory defines live-fire drills and safety locks. The `registry/` directory defines the public threat registry schema and API contract. The `evidence/` directory defines retention obligations. The `postmortems/` directory contains one canonical fictional incident that sets the standard for real post-incident reporting.

## Operating model

No agent acts alone. No irreversible action is automatic. Every containment action must be reversible by default, time-bounded, and evidence-backed. Permanent sanctions require governance ratification or a separate due-process flow. Humans set policy and approve permanent consequences; machines execute bounded defensive controls.

## First implementation pass

Start by wiring watcher findings into the orchestrator event stream, then map judge outputs into the threat level and emergency override model already defined in the GPU swarm. After that, attach warden actions to reversible runtime or service controls such as rate limits, contract pause paths, validator quarantine, and circuit breakers. Finish by exporting scribe bundles into immutable off-chain storage with an on-chain anchor.