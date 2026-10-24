# Call weights that cannot be measured yet

`scripts/swarm/x3_repo_scan.py` reports a dispatchable whose `#[pallet::weight(...)]` is a typed
literal rather than a generated `WeightInfo` call. That finding is right almost everywhere — it is
how the twelve-pallet weights burndown was found and closed — but a few calls cannot be reached by a
benchmark at all, and the honest answer for those is a recorded cost plus a reason, not a fabricated
measurement and not a benchmark that measures a path the chain never executes.

The decisions live in `UNMEASURABLE_CALL_WEIGHTS`, keyed `<pallet entry>::<extrinsic>`, each with an
`owner` (this document) and a `reason`. Two properties keep the list from becoming an excuse:

* the scanner still reports every **other** invented literal in the same pallet, so a new unmeasured
  call is a finding the moment it appears;
* an entry goes **stale** — and becomes a finding itself — the moment the call stops charging a
  literal, i.e. the moment somebody measures it. An exception cannot outlive its justification.

## Recorded entries

| Pallet | Call | Why no benchmark can reach it |
| --- | --- | --- |
| `pallet-x3-cross-vm-router` | `register_external_root` | The pallet benchmark mock still wires `RefuseExternalRoots`, so a benchmark cannot reach the stores the weight describes. The production runtime now binds EVM/SVM roots to attested headers and Bitcoin roots to admitted best-chain headers; the exception should be removed after the benchmark fixture provisions matching header state and a valid binding proof. |
| `pallet-x3-cross-vm-router` | `xvm_transfer` | Gated on `X3LangOrigin = pallet_x3_custody::EnsureAuthorizedGateway`, whose `try_successful_origin` returns `Err` deliberately — no account is *always* authorized, membership lives in genesis-configured storage. The router's `Config` requires no custody bound, so its benchmark has no way to authorize one. |
| `pallet-x3-cross-vm-router` | `xvm_transfer_from_vm` | Same custody-backed gateway origin, checked twice (VM-adapter gate and x3-lang gate). |
| `pallet-x3-cross-vm-router` | `complete_xvm_transfer` | Same custody-backed gateway origin. |
| `pallet-x3-cross-vm-router` | `cancel_expired_xvm_transfer` | Same custody-backed gateway origin. |

## What would retire them

The four origin entries share one cause, so they share one fix: give the router a benchmark-reachable
gateway origin that still performs the custody check the production origin performs. Two shapes are
possible and neither is free —

* a benchmark-only origin type in the runtime that authorizes the account it synthesizes, so the real
  `AuthorizedGateways` read is still inside the measured call (this is machinery that exists only in
  the benchmark build and must be argued for explicitly, because the production origin was
  deliberately moved off compiled-in accounts); or
* a runtime that names its benchmark gateway in genesis and a pallet-side way to be handed that
  account, which means a `Config` item that exists for benchmarking.

Until one of those exists, the alternative is a benchmark measuring an origin path the chain never
executes, which would report a cost *below* the real one for exactly the calls that gate the atomic
kernel. A stated cost plus this record is the truthful option.

The fifth entry is independent: wire a real per-chain light client as `ExternalRootVerifier` and the
call becomes both usable and measurable.
