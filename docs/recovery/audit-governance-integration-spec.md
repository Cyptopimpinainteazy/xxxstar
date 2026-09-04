# Audit-Governance Integration Specification

Status: integration contract for recovered ProofGate evidence and future GPU-swarm receipts.

## Objective

Governance decisions that change runtime code, constitutional invariants, release policy, validator admission, bridge security, or GPU-swarm trust policy must reference independently verifiable evidence. Missing, stale, revoked, mismatched, or unauthorized evidence blocks enactment.

## Evidence envelope

Each receipt is canonical, schema-versioned data containing:

- claim ID and proof tier;
- repository, commit SHA, tree digest, build/toolchain identity, and policy digest;
- exact command or verifier identity, start/end timestamps, exit status, and artifact digests;
- issuer workload identity and signature, optional hardware attestation digest, and expiry;
- predecessor receipt when the claim is renewed or superseded;
- privacy classification and a content-addressed location for non-public evidence.

Canonical encoding and domain-separated signatures are mandatory. A human-authored status document is never evidence by itself.

## Governance binding

1. Proposal creation declares affected claim IDs and the exact code/policy digest to be enacted.
2. The ProofGate resolver expands transitive claim dependencies and determines required tiers from versioned policy.
3. Voting may proceed for review, but scheduling enactment requires all required receipts to verify against the proposal digest.
4. A challenge, revocation, expiry, or dependency-policy change freezes enactment and emits an auditable reason code.
5. At execution, the runtime rechecks the proposal digest, policy version, receipt-set Merkle root, validity window, and revocation root.
6. Emergency governance may pause or roll back, but may not bypass evidence requirements to introduce new privileged behavior.

## Separation of duties

- Evidence producers cannot approve their own policy exceptions.
- Policy authors cannot sign execution receipts.
- Governance origin authorization is separate from receipt validity; both must pass.
- Security council actions are time-bounded, publicly logged, and subject to ratification where the constitution requires it.
- External-audit receipts identify scope, auditor key, report digest, unresolved findings, and expiry; the label `audit_approved` is rejected without these fields.

## Storage and privacy

On-chain state stores claim identifiers, compact validity metadata, policy/version identifiers, revocation roots, and receipt-set roots. Full public receipts live in the repository or an immutable evidence store. Confidential material is encrypted off-chain; governance receives a redacted receipt and a verifier result, never secrets or personal data.

## Failure behavior

- Parser, network, signature, clock, schema, or dependency-resolution errors fail closed.
- Receipt replay across repositories, commits, networks, proposals, or policy versions is rejected by domain binding.
- Equivocating issuers are quarantined and their receipts suspended pending governance resolution.
- Reorg handling preserves a deterministic mapping between finalized proposal state and the accepted receipt-set root.

## Delivery phases

1. Publish canonical receipt schemas, policy schema, issuer registry, revocation format, and test vectors.
2. Add an offline verifier with positive and negative vectors; require deterministic output.
3. Integrate resolver output into proposal validation and enactment checks behind a disabled feature flag.
4. Replay existing proof receipts, classify unverifiable legacy evidence, and issue no automatic upgrades in tier.
5. Run shadow enforcement on testnet, compare decisions with human review, and resolve every divergence.
6. Activate through a proof-gated governance proposal with rollback metadata and monitoring.

## Acceptance evidence

- Unauthorized origin, missing receipt, wrong commit, expired receipt, revoked signer, altered artifact, stale policy, and replay tests all reject enactment.
- Valid multi-receipt dependency sets enact exactly once after finality.
- Receipt verification is deterministic across native and runtime implementations.
- Audit logs link proposal, vote, challenge, receipt-set root, execution result, and rollback without mutable gaps.
- Deployment workflows preserve verifier exit codes and cannot report success after a required gate fails.
