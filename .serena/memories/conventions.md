# Project conventions

- Follow AGENTS.md and X3 instructions: production logic only; no fake adapters, fake proofs, no-op paths, TODO-only implementations, or silent security fallbacks.
- Preserve atomicity and supply invariants when changing cross-VM code; partial success without rollback is forbidden.
- Treat bridges and external integrations as feature-gated/governance-gated unless real support exists; fail safely rather than pretending support.
- Prefer targeted changes in the existing pallet/crate/app structure and reuse existing helpers/types. Do not weaken or delete failing tests.
- Keep generated artifacts, vendor trees, build outputs, secrets, and local credentials out of source changes; root .gitignore documents exclusions.
- Documentation may describe readiness, but implementation and test evidence decide completion.