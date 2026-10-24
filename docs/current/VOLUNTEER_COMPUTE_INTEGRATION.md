# X3 Volunteer Compute — integration contract (draft)

Status: design only; not deployed, audited, or wired into runtime.

## Reuse existing modules
- `pallets/swarm`: inspect existing storage and dispatchables before extending.
- `pallets/depin-marketplace`: inspect provider/marketplace semantics; never assume free jobs have payment.
- `crates/x3-gpu-validator-swarm`: reuse compatible proof primitives after audit.
- `crates/x3-swarm-core`, `services/x3-swarm-api`, `services/x3-swarm-worker`: integrate volunteer enrollment and job execution without duplicating schedulers.

## Trust boundaries
- GPU computation runs off-chain, never in consensus.
- Enrollment requires explicit owner consent and per-node revocation.
- Workers initiate outbound connections; never accept arbitrary shell execution.
- Workloads must be allowlisted, signed, sandboxed and resource-bounded.
- Volunteer mode defaults to zero payment and zero cloud spend.
- Never promise proof of arbitrary GPU computation from a self-reported hash.

## Proposed lifecycle
1. Register worker identity, hardware capability commitment and consent policy.
2. Submit workload manifest with content hash, deterministic input reference, resource ceilings and verification policy.
3. Assign a time-limited lease off-chain with authenticated worker acknowledgment.
4. Execute within isolated runtime; return result hash, metrics and signed receipt.
5. Verify result using task-specific replay, redundant execution, or trusted proof before accepting on-chain commitment.
6. Expire, dispute, revoke or finalize without blocking chain consensus.

## Required tests before activation
- Consent/revocation and unauthorized lease rejection.
- Expired/replayed/duplicated receipts and forged worker signatures.
- GPU memory, wall-clock and workload allowlist enforcement.
- Malicious manifests, hostile artifacts and isolation escape attempts.
- Conflicting results and unverifiable jobs remain unfinalized.
- No paid provisioning and no hidden cryptocurrency mining.
- Chain runtime weights, bounds, migrations, benchmark and 7-validator testnet coverage.

## Implementation order
Audit existing X3 interfaces; add typed protocol and tests; add opt-in worker adapters; implement verification and runtime integration behind disabled-by-default feature flag; test on x3gpu1/x3gpu2; only then open public enrollment.
