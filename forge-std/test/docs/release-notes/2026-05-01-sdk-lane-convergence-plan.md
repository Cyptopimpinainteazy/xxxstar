# SDK Lane Convergence Plan - 2026-05-01

## Why this exists

`cargo report future-incompatibilities --id 1` reports `trie-db v0.30.0` (never-type fallback warning).  
`cargo tree -p x3-chain-node` shows this enters via mixed dependency lanes:

- `polkadot-sdk` `stable2509-7`
- `polkadot-sdk` `stable2506` (via Frontier branch)
- partial overrides to `polkadot-sdk` `stable2603`

Current launch status remains GO; this is a forward toolchain-hardening task.

---

## Current root cause

1. Workspace baseline in root `Cargo.toml` is still primarily pinned to `polkadot-stable2509-7`.
2. Frontier dependencies are pinned to `branch = "stable2506"`.
3. Local `[patch]` overrides pin a subset of `sp-*` / `sc-*` crates to `polkadot-stable2603`, creating lane split rather than full convergence.

This split keeps `sp-trie`/`sp-state-machine` paths that still resolve `trie-db v0.30.0`.

---

## Execution strategy (safe order)

### Phase 0 - Freeze proof baseline

Run and keep artifacts:

```bash
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.proofforge.receipt_integrity
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.funding.milestone_receipts
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.evolution.no_regression
```

### Phase 1 - Single-lane substrate core (root workspace deps)

In root `Cargo.toml`, migrate `[workspace.dependencies]` Substrate/FRAME/SC/SP entries from `polkadot-stable2509-7` to `polkadot-stable2603` **as one atomic change**.

Scope includes (not exhaustive): `frame-*`, `pallet-*` (Substrate-native), `sp-*`, `sc-*`, `substrate-*`.

### Phase 2 - Frontier lane alignment

Replace Frontier `branch = "stable2506"` entries with the Frontier line matching the chosen substrate lane (preferred: matching `stable2603` compatibility line).

Affected root deps include:

- `pallet-evm`, `pallet-ethereum`, `pallet-base-fee`
- `pallet-evm-precompile-*`
- `fp-*`, `fc-*`

### Phase 3 - Reduce patch surface

After phases 1–2 compile:

1. Remove no-longer-needed `[patch.crates-io]` / `[patch."https://github.com/paritytech/polkadot-sdk"]` entries that only papered over lane drift.
2. Keep only truly required local patches (prometheus, executor, etc.).

### Phase 4 - Verify trie-db eviction

```bash
cargo tree -p x3-chain-node | rg "trie-db v0.30.0|trie-db v0.31.0"
cargo report future-incompatibilities --id 1
```

Success target:

- `trie-db v0.30.0` absent from node/runtime graph
- future-incompat report clear (or reduced with explicit remaining upstream blockers)

### Phase 5 - Re-lock launch evidence

```bash
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.proofforge.receipt_integrity
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.funding.milestone_receipts
cargo run --manifest-path proof-forge/Cargo.toml --bin x3-proof -- verify x3.evolution.no_regression
launch-gates/mainnet-go-no-go-template.sh
```

---

## Guardrails

- Do **not** mix multiple substrate tags and Frontier branches in final state.
- Prefer workspace-level pinning over scattered per-crate pin overrides.
- Keep changes phased; validate compile and proof gates between phases.
- If a phase fails, stop and record exact blocking crate pair (do not apply broad extra patches blindly).

---

## Recommended first implementation slice

Start with Phase 1 on a dedicated branch and run:

```bash
cargo check -p x3-chain-runtime
cargo check -p x3-chain-node
```

Then proceed to Phase 2 only if both pass.
