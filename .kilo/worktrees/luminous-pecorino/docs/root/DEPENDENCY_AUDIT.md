# Dependency Audit

This document records the current cargo-deny exceptions that remain after the
workspace dependency review.

## Current State

- `cargo deny check advisories bans` is expected to pass.
- Duplicate-version allowances in `deny.toml` are restricted to the
  Substrate/Solana graph where upstream pins still force parallel versions.
- Advisory ignores are temporary exceptions, not silent acceptance.

## Advisory Exceptions

Source of truth: `deny.toml`.

- `RUSTSEC-2021-0139`, `RUSTSEC-2022-0093`: Substrate CLI transitive deps.
  Removal condition: upstream Substrate release without `ansi_term` / `atty`.
- `RUSTSEC-2023-0091`, `RUSTSEC-2024-0320`: Solana URL / serialization stack.
  Removal condition: compatible Solana upgrade lands in workspace.
- `RUSTSEC-2024-0336`, `RUSTSEC-2024-0344`: crypto advisories inherited from
  pinned Substrate crypto crates. Removal condition: Substrate rev refresh.
- `RUSTSEC-2024-0370`, `RUSTSEC-2024-0375`, `RUSTSEC-2024-0384`,
  `RUSTSEC-2024-0388`, `RUSTSEC-2024-0421`, `RUSTSEC-2024-0436`,
  `RUSTSEC-2024-0437`, `RUSTSEC-2024-0438`: unmaintained macro/runtime support
  crates transitively pinned by upstream SDKs. Removal condition: upstream
  dependency replacement.
- `RUSTSEC-2025-0009`, `RUSTSEC-2025-0010`, `RUSTSEC-2025-0055`,
  `RUSTSEC-2025-0057`, `RUSTSEC-2025-0118`, `RUSTSEC-2025-0119`,
  `RUSTSEC-2025-0134`, `RUSTSEC-2025-0141`: Hyper / h2 / RPC transitive issues
  inherited from pinned Substrate RPC components. Removal condition: upstream
  RPC stack refresh.
- `RUSTSEC-2026-0020`, `RUSTSEC-2026-0021`: Wasmtime executor issues inherited
  from the pinned Substrate executor. Removal condition: Substrate rev refresh.

## Ownership

- Runtime / node transitive exceptions: core runtime maintainers.
- Solana transitive exceptions: SVM / external chain maintainers.
- Any new ignore entry requires:
  - reason
  - owning subsystem
  - removal condition
