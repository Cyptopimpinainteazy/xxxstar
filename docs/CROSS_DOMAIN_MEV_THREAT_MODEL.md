# Cross-domain MEV threat model — `crates/cross-vm-coordinator`

Scope: the coordinator that drives one cross-domain intent across an X3 leg and
one or more external legs, and the envelope it hands to the runtime to release
that intent. Written 2026-09-27 against the tree at `975e58eea` + the settlement
finality gate. It is specific to this crate; generic MEV essays do not apply.

## Assets an attacker wants

| Asset | Where it lives | Why it is worth attacking |
| --- | --- | --- |
| The preimage `S` of the intent's hashlock | `proof_vault`, `ValkeySecretRegistry`, the claim envelope | Whoever holds `S` first can claim the locked side on the *other* domain |
| The locked leg itself | external chain HTLC / X3 HTLC | A released claim against a leg that later reorgs leaves the intent settled on one side only |
| Release ordering | `settlement_outbox` | The attempt that lands first wins a time-sensitive leg (rate move, liquidation, arbitrage window) |
| The coordinator's own view of finality | `settlement_submission` envelope | It is the authority the runtime trusts when it releases |

## Attacker capabilities we assume

* **Relayer**: can deliver, delay, reorder or silently drop any message between
  the coordinator and the runtime, and can watch the mempool of every external
  domain. Cannot forge signatures and cannot alter the coordinator's own storage.
* **External-chain adversary**: can reorg an external chain up to some depth that
  the operator configures a confirmation requirement for; can include a
  transaction in a block and then rewrite it.
* **Observer**: sees everything in the public mempool of every domain, including
  preimages the moment they are revealed.

## Surfaces

### S1 — Release against a leg that is not yet reorg-safe — **closed**

`CrossDomainProofBundle::verify` binds the finality evidence to the bundle
(chain, vm, txid, block hash) and requires `finalized: true`, but `finalized` is
the bundle author's own field and no confirmation depth was compared with the
operator's policy anywhere. `CoordinatorConfig::confirmations`
(`evm: 12`, `svm: 50`, `x3: 1`) had no reader on the release path.

Attack: a relayer hands the coordinator a claim proof for an external leg that is
one block deep and self-declares `finalized`. The coordinator builds the release
envelope, the runtime releases, the external chain reorgs the leg away.

Mitigation: every release envelope is now built with a
`SettlementFinalityPolicy` (`crates/cross-vm-coordinator/src/settlement_finality.rs`)
derived from `CoordinatorConfig::confirmations`, and
`enforce_settlement_finality` refuses with `LegBelowFinalityDepth` when the
observed depth is below the configured one. There is no default policy: the
parameter is required, so a new caller cannot forget it.

Tests: `a_claim_at_the_configured_depth_is_accepted`,
`a_claim_one_confirmation_shallow_is_refused`,
`a_refund_below_the_configured_depth_is_refused`.

### S2 — Publish the preimage against a leg that is unsafe to reveal it on — **closed**

A claim envelope reveals `S`. `FinalityProof::safe_to_reveal_secret` expresses
the adapter's own judgement that revealing there cannot be front-run; nothing
read it, so a claim could be released while the external leg was still inside the
window where a watcher could use `S` first.

Mitigation: for `SettlementProofPurpose::Claim` the gate requires
`safe_to_reveal_secret` unless a caller explicitly opts out with
`without_secret_reveal_safety()` (no adapter in this workspace needs the opt-out;
it exists so a relaxation is visible at the call site). Refusal is
`SecretRevealNotSafe`.

Test: `a_claim_that_is_not_safe_to_reveal_the_secret_is_refused`.

### S3 — Choose your own depth on an unsized domain — **closed**

A domain the operator never sized inherited whatever depth the submitter's proof
asserted, i.e. no policy at all.

Mitigation: a domain the policy does not cover is refused
(`FinalityPolicyMissingForDomain`) rather than defaulted. Bitcoin is covered
explicitly at the documented reorg-safe depth
(`BITCOIN_REORG_SAFE_CONFIRMATIONS = 6`); every other VM family is
`FinalityDomain::Other` and must be opted in with
`with_domain(domain, depth)`. `SettlementFinalityPolicy::covers_all` lets an
operator check a config at startup instead of discovering the gap on the release
path.

Tests: `a_domain_the_policy_never_sized_is_refused_instead_of_defaulted`,
`the_policy_uses_the_depths_the_operator_configured`.

### S4 — Ordering: a relayer picks which settlement lands first — **NARROWED**

The settlement outbox now derives a canonical cross-domain release key from
`SHA-256("X3:CROSS_DOMAIN_RELEASE_ORDER:V1" || finalized X3 block hash ||
submission id)`. Prepared and signed submissions form the complete pending set;
the in-memory and Valkey stores sort that set and refuse any broadcast that is
not canonical next. Valkey persists an explicit submission index, so a relayer
cannot hide a pending settlement by racing a weak key scan, and the order exists
only after a block hash is fixed.

Still open: the X3 node's finality-notification watcher must authenticate the
finalized block hash it supplies to the coordinator. Until that producer is
wired, the ordering rule is deterministic but its beacon is an input the caller
must trust.

### S5 — Withholding — **OPEN, and unavoidable at this layer**

A relayer who simply never delivers a settlement cannot be detected by a release
gate: the intent ends in its timeout/refund path. The existing `recovery` and
`settlement_reconcile` modules bound the damage (the funds return), but the
attacker's gain is the delay itself, and no coordinator-side check removes that.

### S6 — The gate is only as good as the evidence handed to it — **PARTIAL**

`confirmations` and `safe_to_reveal_secret` are fields in the bundle. The gate
enforces them against the operator's policy, but a caller that fabricates a
*deep enough* bundle is not detected here; that is the adapter's job and the
runtime's binding checks (`verify_runtime_binding`, `verify_claim_set`). This
means the mitigation is "the coordinator cannot be talked into releasing too
early by evidence that says it is too early" — not "the coordinator can detect a
lying adapter". Real adapter evidence is covered by the matrix rows X3-XVM-* and
`tests/x3_adapter_route.rs`.

## What this does not claim

* No claim of MEV *elimination*. S4 is narrowed but still depends on an
  authenticated finality-notification source; S5–S6 remain open.
* Release ordering and restart behavior have run against a real local Valkey
  service, but not against an independently operated multi-node network.
* `crates/cross-vm-coordinator` is not in the runtime dependency graph: this
  changes what the coordinator will *ask* the runtime to release, not what the
  runtime enforces. The runtime's own checks are unchanged.
