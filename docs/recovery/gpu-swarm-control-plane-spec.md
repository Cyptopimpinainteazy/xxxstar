# Secure GPU-Swarm Recovery Specification

Status: recovery design; no production enablement is authorized by this document.

## Recovery boundary

Recover the useful scheduling, attestation, failover, and monitoring concepts from the historical GPU repositories. Do not restore the historical watchdog's string-based signatures, embedded credentials, trust-on-first-use enrollment, or any path that treats a health response as proof of computation.

## Trust model

- Every coordinator, worker, and operator has a short-lived workload identity issued by the deployment trust domain.
- Control-plane traffic uses mutually authenticated TLS. Authorization is deny-by-default and scoped by role, swarm, operation, and expiry.
- Worker admission requires a hardware-backed attestation whose nonce binds the quote to the current enrollment challenge and worker public key.
- Accepted measurements, firmware baselines, driver versions, and revocations are versioned policy inputs. Unknown or stale evidence fails closed.
- Job artifacts are content-addressed and signed. Workers verify digest, signer, policy version, and resource limits before execution.
- A completion receipt binds job ID, input/output digests, image digest, worker identity, attestation digest, timestamps, and monotonic attempt number.

## Control-plane state machine

`Discovered -> Quarantined -> Attested -> Ready -> Assigned -> Running -> Draining`

Any failed identity, attestation, heartbeat-integrity, or receipt check moves the worker to `Quarantined`. Only a new admission challenge may return it to `Attested`; operators cannot manually mark it `Ready` without evidence.

## Scheduler invariants

1. A job has at most one active lease; leases have bounded duration and monotonically increasing epochs.
2. Reassignment requires expiry or cryptographically acknowledged relinquishment of the prior lease.
3. Results from an old lease epoch are rejected, even if otherwise valid.
4. Confidential jobs run only on an attestation profile explicitly allowed by their policy.
5. Coordinator quorum authorizes policy changes and emergency drains; a single coordinator may not weaken admission policy.
6. GPU and CPU reference execution must agree for deterministic kernels before a result can satisfy a ProofGate claim.

## Secrets and network controls

- Secrets arrive through a workload-identity secret broker, remain memory-only where possible, and are never placed in images, repository files, command lines, or receipts.
- Workers have no inbound public listener. Egress is allowlisted to control-plane, artifact, time, and telemetry endpoints.
- Administrative APIs are isolated from job traffic and require phishing-resistant operator authentication plus recorded change approval.

## Recovery sequence

1. Inventory historical components and classify each as reusable algorithm, unsafe implementation, test fixture, or documentation.
2. Define signed schemas for enrollment, lease, heartbeat, revocation, and completion receipts.
3. Implement identity and attestation verification with negative tests before scheduler integration.
4. Implement lease fencing and idempotent result submission; prove stale-epoch rejection under failover.
5. Add deterministic GPU/CPU vectors and fault injection for coordinator loss, worker loss, replay, clock skew, and corrupted artifacts.
6. Deploy a network-isolated canary swarm with synthetic data. Promote only after receipt verification and recovery objectives pass.

## Required evidence before enablement

- Threat model and data-flow review approved by security owners.
- Reproducible images with SBOMs, provenance attestations, signature verification, and vulnerability policy results.
- Tests for forged/stale quotes, replayed leases, duplicate results, revoked workers, split brain, and secret leakage.
- Measured recovery time and job-loss bounds under coordinator and worker failure.
- Proof receipts stored through the audit-governance integration, with raw secrets and tenant data excluded.

Rollback is a signed policy change that drains new scheduling, preserves immutable receipts, revokes worker credentials, and leaves submitted governance evidence verifiable.
