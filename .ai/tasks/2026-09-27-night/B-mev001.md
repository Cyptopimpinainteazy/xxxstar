# Workstream B — X3-MEV-001: cross-domain MEV protection (row at composite 25, a STUB)

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` and `feature-matrix/mev-privacy.toml`'s `X3-MEV-001` row first, then this brief.

## The row

```
id = "X3-MEV-001"  name = "Cross-domain MEV protection"  paths = ["crates/cross-vm-coordinator"]
implemented = 35  tested = 18  mainnet_ready = 20  priority = "P0"  launch_scope = "guarded"
blockers = ["Needs threat model covering relayers, finality delays, and ordering"]
```

It is one of the two remaining `STUB` rows in the whole matrix. No lane owns it.

## What to do

1. **Threat model, written down and specific to this crate**: relayers (who can censor, reorder, or
   delay a settlement by withholding), finality delays (a settlement released against a not-yet-final
   or rewound external anchor), and ordering (who picks which settlement lands first). Name the
   assets, the attacker capabilities, and the attack per surface — not a generic MEV essay.
2. **Implement at least one concrete mitigation in `crates/cross-vm-coordinator` and prove it.**
   Candidates, in the order I would try them:
   - refuse to release/submit a settlement whose external leg has not reached the configured
     finality depth/anchor (the finality work in `crates/x3-atomic-swap` gives you `FinalityCertificate`
     and a persistent oracle you can mirror — do **not** add a dependency on that crate to the
     coordinator's own locked workspace; if you need the shape, define the minimal reader trait in the
     coordinator and keep the crate's `--locked --offline` gate working);
   - bind execution order to a published commitment (a hash of the ordered batch) so a relayer cannot
     reorder after seeing contents;
   - refuse a settlement whose observation is older than the configured window.
   Whatever you pick must fail closed: unknown/absent proof ⇒ refuse, never "proceed anyway".
3. **Tests**, including the negative cases: a reordered batch is refused, an anchor that rewinds is
   refused, a settlement without the required proof is refused — and the positive case lands. Prove
   each with break-it-first (remove the check, watch the test go red, restore byte-identically).

## Rules

- Do not weaken tests. Do not add `unwrap()`/`expect()` to production paths (the panic ratchet is
  live; see `scripts/audit/panic_unwrap_scan.py`).
- `crates/cross-vm-coordinator` is **outside** the root workspace with its own `Cargo.lock`; its gate
  is `cargo test --offline --locked --manifest-path crates/cross-vm-coordinator/Cargo.toml`. Keep it
  offline-and-locked green.
- Keep commits focused. **Commit only your own paths, never `git add -A`, and do not push** — message
  me (`/root`) with the hash, the row's new scores and why they are justified, and the commands run.
- Update the `X3-MEV-001` row in `feature-matrix/mev-privacy.toml` with named `required_tests` that
  actually exist, `test_evidence`, and honest scores; run `python3 scripts/feature_matrix.py check`,
  `python3 scripts/ci/check-matrix-tests-exist.py` and
  `python3 scripts/ci/check-matrix-test-evidence.py`.

## Deliverable

A threat model committed as a document beside the crate, the mitigation in code, tests proving both
directions, the row updated, and a report to me.
