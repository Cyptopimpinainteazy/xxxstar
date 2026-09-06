#import "../style.typ": *
#import "../components.typ": *
#import "../data.typ": *

= Security Threat Model

This is a repository-specific threat model built from what this audit actually traced, organized by trust boundary rather than by generic attack taxonomy. It is not a substitute for a formal, independent security audit — see the Safety Disclaimer in the Front Matter.

== Protected Assets

- Validator stake and the ability to remain in the active validator set.
- The canonical-supply invariant underlying every asset in the system.
- Wrapped-asset mint authority at the EVM/SVM bridge boundary (currently disabled by default).
- Wallet keys, biometric templates, and multisig approval authority.
- Build/release provenance (the ability to trust that a released binary matches audited source).

== Privileged Roles and What They Can Do

#table(
  columns: (1.3fr, 2.7fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Role*], [*Power*],
  [Root / Sudo], [Can call `set_validators` to change the entire active validator set (`pallets/x3-consensus/src/lib.rs:229`). This is the top of the current trust hierarchy — there is no permissionless path around it (HIGH-07).],
  [Any signed account], [Can call `report_misbehavior` to slash any validator with zero evidence (CRIT-02). This is a privilege escalation bug: an unprivileged role has an effectively privileged capability.],
  [Holder of `X3_SUBMITTER_SEED`], [Can mint wrapped assets through the bridge RPC's "council" path with a threshold of 1 (HIGH-01) — currently only reachable when bridges are enabled.],
  [Holder of `X3_RELAY_PROOF_SIGNER` (defaults to `//Alice`)], [Can attest cross-chain deposit proofs for the relayer (HIGH-04) — currently only reachable when bridges are enabled.],
  [Treasury.sol owner (EOA, not deployed)], [Would control fee splits and routing destinations for any funds sent through the contract (MED-07) if it were ever deployed and funded.],
  [Anyone who cloned the repo before commit `fbd4613b`], [Has the plaintext validator seeds and Sepolia key described in CRIT-01.],
  [Any RPC caller, no authentication needed], [Receives a publicly-known compromised mnemonic and fabricated USD balances from the node's own `wallet_*` RPC methods (CRIT-03) — the lowest-privilege role in this table has access to the highest-severity finding.],
)

== Attack Surfaces Traced in This Audit

+ *Wallet/RPC-facing layer* — the node's own `wallet_*` RPC methods fabricate custodial wallet data (CRIT-03), requiring zero privilege and zero prior access to reach; this is the least-privileged attack surface in the entire codebase and the most concretely "fake" finding this audit made.
+ *Consensus/validator layer* — the `report_misbehavior` unauthenticated slash (CRIT-02) is the concrete, proven attack surface here; the GRANDPA equivocation path was checked and found correctly proof-gated.
+ *Bridge/RPC ingress* — `node/src/rpc.rs`'s cross-VM submission endpoint does real bounds-checking and proof-hash binding (VERIFIED), but the wrapped-asset mint path built on top of it (HIGH-01) and the relayer's proof-signing key (HIGH-04) are both single-key trust points disguised with governance-sounding names.
+ *Economic/DEX layer* — the AMM's floating-point pricing (HIGH-02) is an extraction surface via systematic rounding bias once any real liquidity is present, and a determinism hazard for validators running different execution backends.
+ *Supply-chain/dependency layer* — `cargo audit` found no blocking CVEs, but `solana_rbpf` 0.8.5's known unsoundness (RUSTSEC-2026-0191, out-of-bounds pointer arithmetic) sits directly in the SVM execution path this audit's cross-chain domain investigation reviewed.
+ *Secrets/key-management layer* — CRIT-01 is a textbook supply-chain/process failure: real key material committed to source control. The process gap (no history-aware secret scanning, MED-11) is as important as the specific leaked files.
+ *Deployment/operator-mistake layer* — cited srtool evidence (HIGH-05) and mislabeled migrations (MED-01) both represent the case where an operator trusts a status document instead of independently verifying, and gets misled.

== Risk Matrix (Likelihood × Impact, as evidenced by this audit)

#table(
  columns: (2.4fr, auto, auto, 2fr),
  fill: (x, y) => if y == 0 { c-brand } else if calc.even(y) { c-bg-panel } else { white },
  [*Finding*], [*Likelihood*], [*Impact*], [*Why*],
  [CRIT-03 fabricated wallet RPC], [Certain (already true)], [High], [No privilege needed at all — any RPC caller gets a compromised mnemonic and fake balances today, unconditionally, on every node.],
  [CRIT-02 unauthenticated slash], [High], [High], [Requires only a signed transaction — no special access. Reachable today on any network running this pallet.],
  [CRIT-01 leaked git-history secrets], [High], [Medium (testnet-scoped today)], [Secrets are already exposed to anyone who cloned; would be Critical impact if the same process recurred for mainnet keys.],
  [HIGH-02 DEX float pricing], [Medium], [High], [Requires real liquidity in the pool to be worth exploiting, but the defect is unconditional once any value is present.],
  [HIGH-01 / HIGH-04 bridge single-key trust], [Low today], [High if triggered], [Bridges are disabled by default; risk activates only if that gate is lifted without a corresponding code fix.],
  [HIGH-05 fabricated build evidence], [Certain (already true)], [Medium], [Does not enable an attack directly, but removes a control an operator would otherwise rely on.],
  [HIGH-06 fake health endpoints], [Certain (already true)], [Medium], [Operational blind spot, not a direct exploit path.],
)

== Unmitigated Risk Summary

The two largest unmitigated risks this audit identified are CRIT-03 and CRIT-02. CRIT-03 requires no signed transaction at all — it is reachable by an unauthenticated RPC call and is already actively returning fabricated data to any caller today. CRIT-02 requires only a signed transaction, which any account can produce, with no compromised key or social engineering needed. Every High-severity risk in this chapter, by contrast, is currently mitigated by a feature or governance gate that is honestly documented as disabled; neither Critical finding has such a gate.
