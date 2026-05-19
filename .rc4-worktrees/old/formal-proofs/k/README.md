# K Framework specs (planned)

Executable K semantics for the X3 asset kernel and the bridge state
machine. Pairs with `../tla/` for model-checking and `../coq/` for
mechanized proof.

## Roadmap

- [ ] `asset-kernel.k` — operational semantics of mint / burn / transfer /
      bridge_lock / bridge_release / bridge_refund. Each rule must
      preserve `totalSupply == sum(balances) + sum(inflight)`.
- [ ] `bridge.k` — semantics of the cross-VM bridge state machine
      including replay-set monotonicity.

## Status

Initial harness is bootstrapped via `x3vm-spec.k` and is executed by
`x3-proof formal` through `kprove` in strict mode.
