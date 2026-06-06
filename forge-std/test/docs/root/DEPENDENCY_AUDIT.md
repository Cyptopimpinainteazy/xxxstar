# Dependency Audit

This document tracks the current Rust advisory exceptions used by
`cargo-deny` for the X3 workspace.

## Current Reality

- Source of truth is [deny.toml](../../deny.toml).
- Ignore entries are temporary risk-acceptance records, not permanent fixes.
- Most remaining advisories are transitive from pinned Substrate and Solana
  dependency trees.

## Verified

- Local patching is active for selected crates (for example `quinn-proto`).
- The workspace still depends on pinned SDK revisions where not all
  advisories can be removed independently.

## Advisory Exceptions By Family

### Substrate And Runtime Pinning

- `RUSTSEC-2022-0093`
- `RUSTSEC-2024-0336`
- `RUSTSEC-2024-0344`
- `RUSTSEC-2024-0370`
- `RUSTSEC-2024-0384`
- `RUSTSEC-2024-0388`
- `RUSTSEC-2024-0421`
- `RUSTSEC-2024-0436`
- `RUSTSEC-2025-0009`
- `RUSTSEC-2025-0010`
- `RUSTSEC-2025-0055`
- `RUSTSEC-2025-0057`
- `RUSTSEC-2025-0118`
- `RUSTSEC-2025-0119`
- `RUSTSEC-2025-0134`
- `RUSTSEC-2025-0141`
- `RUSTSEC-2025-0161`
- `RUSTSEC-2026-0049`
- `RUSTSEC-2026-0098`
- `RUSTSEC-2026-0099`
- `RUSTSEC-2026-0104`

Removal condition: update to an upstream Substrate or Polkadot SDK revision
that resolves the advisory chain without breaking runtime compatibility.

### Wasmtime Executor Pinning

- `RUSTSEC-2026-0006`
- `RUSTSEC-2026-0020`
- `RUSTSEC-2026-0021`
- `RUSTSEC-2026-0085`
- `RUSTSEC-2026-0086`
- `RUSTSEC-2026-0087`
- `RUSTSEC-2026-0088`
- `RUSTSEC-2026-0089`
- `RUSTSEC-2026-0091`
- `RUSTSEC-2026-0092`
- `RUSTSEC-2026-0093`
- `RUSTSEC-2026-0094`
- `RUSTSEC-2026-0095`
- `RUSTSEC-2026-0096`

Removal condition: move executor stack to an updated upstream wasmtime chain
through supported Substrate updates.

### Solana And Tooling Transitives

- `RUSTSEC-2024-0320`
- `RUSTSEC-2024-0437`
- `RUSTSEC-2025-0052`

Removal condition: update Solana and related tooling dependencies to compatible
releases that drop the vulnerable transitives.

## Ownership

- Runtime and node advisory chain: core runtime maintainers.
- Cross-chain and Solana advisory chain: SVM and external chain maintainers.
- Build and tooling advisory chain: platform and release engineering.

## Gaps And Risks

- A full remediation to zero ignored advisories is currently blocked on SDK
  dependency upgrades, not only leaf crate bumps.
- Risk acceptance must be reviewed at each release gate.

## Next Required Work

- Run `cargo deny check advisories bans` in CI on every PR.
- Keep advisory IDs in this file synchronized with [deny.toml](../../deny.toml).
- Split remediation into two tracks:
  - leaf dependency fixes that can land immediately
  - SDK upgrade work for pinned transitive chains
