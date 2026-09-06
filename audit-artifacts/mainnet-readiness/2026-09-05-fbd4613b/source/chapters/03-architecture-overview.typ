#import "../style.typ": *
#import "../components.typ": *
#import "@preview/cetz:0.3.1": canvas, draw

#let node-box(pos, w, h, label, fill: white, stroke-color: rgb("#1a1a2e"), text-size: 7.6pt, text-color: rgb("#1a1a2e")) = {
  import draw: *
  rect(pos, (pos.at(0) + w, pos.at(1) - h), fill: fill, stroke: 0.9pt + stroke-color)
  content((pos.at(0) + w/2, pos.at(1) - h/2), text(size: text-size, fill: text-color)[#label], anchor: "center")
}

#let arrow(a, b, color: rgb("#5a5a6e")) = {
  import draw: *
  line(a, b, mark: (end: ">", fill: color, size: 0.16), stroke: 1pt + color)
}

= Architecture Overview

X3 Atomic Star is a Substrate node exposing three execution domains behind a single runtime state-transition function. This chapter maps what this audit directly traced in source, not an aspirational architecture diagram.

== Node & Runtime Topology

#figure(
  canvas(length: 1cm, {
    import draw: *
    node-box((0, 0), 5, 1.1, [x3-chain-node\ (node/src/service.rs)], fill: rgb("#e8eef5"), stroke-color: rgb("#1e3a5f"))
    node-box((-5.6, -2.4), 3.6, 1, [Aura\ block production], fill: white)
    node-box((-1.4, -2.4), 3.6, 1, [GRANDPA\ finality voter], fill: white)
    node-box((2.8, -2.4), 3.6, 1, [sc-transaction-pool\ (tx pool)], fill: white)
    arrow((-4, -1.1), (-3.8, -2.4))
    arrow((-1, -1.1), (-1.1, -2.4))
    arrow((2, -1.1), (2.8, -2.4))

    node-box((-7.2, -5), 3.4, 1, [pallet-x3-consensus\ (validator set, slash)], fill: rgb("#fbe9e7"), stroke-color: rgb("#8c1c13"))
    node-box((-3.4, -5), 3.4, 1, [pallet-x3-cross-vm-router\ (6-route matrix)], fill: white)
    node-box((0.4, -5), 3.4, 1, [pallet-x3-supply-ledger\ (canonical supply)], fill: white)
    node-box((4.2, -5), 3.4, 1, [pallet-x3-settlement-engine\ (atomic escrow)], fill: white)

    arrow((-3.8, -3.4), (-5.5, -5))
    arrow((-1.1, -3.4), (-1.8, -5))
    arrow((2, -3.4), (2, -5))
    arrow((2.8, -3.4), (5.8, -5))

    node-box((-3.4, -7.6), 3.4, 1, [pallet-x3-dex\ (AMM, f64 pricing — see HIGH-02)], fill: rgb("#faf3d9"), stroke-color: rgb("#8a6d00"))
    node-box((0.4, -7.6), 3.4, 1, [pallet-x3-atomic-kernel\ (economic-halt gate)], fill: white)
    node-box((4.2, -7.6), 3.4, 1, [pallet-x3-cross-vm-router:\ ExternalBridgesEnabled gate], fill: rgb("#e8f3e8"), stroke-color: rgb("#2e6b32"))

    arrow((-1.8, -6), (-1.8, -7.6))
    arrow((2, -6), (2, -7.6))
    arrow((5.8, -6), (5.8, -7.6))
  }),
  caption: [Node → runtime → pallet wiring as traced in `node/src/service.rs` and `runtime/src/lib.rs`. Red = a finding in this audit (CRIT-02); amber = a finding (HIGH-02); green = a verified, well-engineered control (ExternalBridgesEnabled circuit breaker).]
) <fig-node-topology>

== Consensus & Finality Flow

#figure(
  canvas(length: 1cm, {
    import draw: *
    node-box((0, 0), 4.4, 1, [Block proposed\ (Aura leader slot)], fill: white)
    node-box((5.2, 0), 4.4, 1, [GRANDPA vote\ round], fill: white)
    node-box((10.4, 0), 4.4, 1, [Finalized block], fill: rgb("#e8f3e8"), stroke-color: rgb("#2e6b32"))
    arrow((4.4, -0.5), (5.2, -0.5))
    arrow((9.6, -0.5), (10.4, -0.5))

    node-box((0, -2.6), 5.6, 1.3, [Equivocation observed\ → EquivocationProof\ → check_equivocation_proof()\ (real cryptographic check)], fill: rgb("#e8f3e8"), stroke-color: rgb("#2e6b32"), text-size: 7pt)
    node-box((6.6, -2.6), 5.6, 1.3, [report_misbehavior(origin, validator, reason)\ ensure_signed() only — NO PROOF\ → slash_validator() immediately], fill: rgb("#fbe9e7"), stroke-color: rgb("#8c1c13"), text-size: 7pt)

    arrow((2, -1.1), (2.5, -2.6))
    line((6, -2), (9, -1.1), stroke: (paint: rgb("#8c1c13"), thickness: 1pt, dash: "dashed"), mark: (end: ">", fill: rgb("#8c1c13"), size: 0.16))

    node-box((3.4, -5.2), 5.6, 1, [slash_validator() — stake reduced], fill: white)
    arrow((2.5, -3.9), (5, -5.2))
    arrow((9.2, -3.9), (7, -5.2))
  }),
  caption: [Two paths reach the same `slash_validator()` operation. The GRANDPA equivocation path (green) requires a verified cryptographic proof. The `report_misbehavior` path (red, CRIT-02) requires only a signed transaction from any account — no proof, no rate limit, no dispute window.]
) <fig-consensus-flow>

#callout(kind: "critical", title: "CRIT-02 — a shortcut around the codebase's own correct pattern")[
  The diagram above is the clearest way to see why CRIT-02 matters: the engineering team clearly *knows* the correct pattern (require a verified proof before slashing) because they built it correctly for GRANDPA equivocation. `report_misbehavior` reaches the exact same dangerous operation through a door with no lock.
]

== Cross-Chain Trust Boundary

#figure(
  canvas(length: 1cm, {
    import draw: *
    node-box((0, 0), 5, 1.3, [External chain (EVM / SVM)\ compile-time excluded from\ mainnet-rc1 by design], fill: white, stroke-color: rgb("#5a5a6e"))
    node-box((0, -2.4), 5, 1.3, [x3-relayer\ proof envelope signer:\ defaults to dev key //Alice], fill: rgb("#fdefe2"), stroke-color: rgb("#a8460a"), text-size: 7pt)
    node-box((0, -4.8), 5, 1, [node RPC:\ x3_submitCrossVmTransaction], fill: white)
    node-box((0, -7.2), 5, 1.3, [ExternalBridgesEnabled gate\ (real, dispatch-time check,\ auto-trips false on violation)], fill: rgb("#e8f3e8"), stroke-color: rgb("#2e6b32"), text-size: 7pt)
    node-box((6.5, -7.2), 5, 1.3, [pallet-x3-supply-ledger\ canonical_supply invariant], fill: white)

    arrow((2.5, -1.3), (2.5, -2.4))
    arrow((2.5, -3.7), (2.5, -4.8))
    arrow((2.5, -5.8), (2.5, -7.2))
    arrow((5, -6.55), (6.5, -6.55))

    line((-2.2, 1.5), (-2.2, -8), stroke: (paint: rgb("#c7c7d1"), thickness: 0.6pt, dash: "dashed"))
    content((-2.5, 0.6), rotate(90deg, text(size: 7pt, fill: rgb("#7a7a8a"))[TRUST BOUNDARY]), anchor: "center")
  }),
  caption: [The bridge relay's trust model when the gate is enabled: an external chain event is attested by a single dev-signed proof envelope (HIGH-04), not a threshold of independent validators. The `ExternalBridgesEnabled` gate itself is genuinely enforced (green) — the risk is entirely in what happens *if and when* it is turned on without first replacing the relayer's signing model.]
) <fig-trust-boundary>

== External Trust Assumptions Summary

#table(
  columns: (1.4fr, 2.6fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Trust assumption*], [*Current reality*],
  [Validator set is honest / correctly bonded], [Root-controlled admission list, no permissionless staking (HIGH-07). Any single signed account can slash any validator (CRIT-02).],
  [External bridge proofs are attested by a quorum], [Currently a single hardcoded dev key when the bridge path is exercised at all (HIGH-04); the feature is disabled by default.],
  [DEX price quotes are exact], [Computed in `f64`, not integer/fixed-point (HIGH-02) — not exact under precision loss.],
  [Health/readiness endpoints reflect real system state], [Several are hardcoded liveness-only responses (HIGH-06, MED-09, MED-10).],
  [The build is byte-for-byte reproducible], [Not currently demonstrated for the audited commit (HIGH-05).],
)
