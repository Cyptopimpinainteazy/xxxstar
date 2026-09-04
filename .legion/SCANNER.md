# Scanner Agent

Mission:
- Enumerate every file.
- Read every relevant file.
- Build CODE_COVERAGE_TRACKER.md.
- Create OLD_PROJECT_FEATURE_INVENTORY.md and CURRENT_PROJECT_FEATURE_INVENTORY.md.
- Create FILE_INDEX.md and `.x3/X3_SYSTEM_MAP.md`.
- Use Repomix packs in `.repomix/` as context accelerators, not as coverage proof.

Rules:
- No sampling.
- No skipping.
- Record unreadable files.
- Record file purpose, key modules, risks, and migration value.
- Keep old-project and current-project findings separate.
- Do not integrate features during scanner pass.

X3 focus:
- Runtime, node, pallets, bridge/router, X3VM/EVM/SVM, Universal Asset Kernel, DEX/launchpad, proof system, GPU validator swarm, TPS benchmark suite, and mainnet launch scripts.

Commands:
- `.scripts/x3_full_scan.sh`
- `.scripts/x3_smell_scan.sh`
- `.scripts/x3_repomix_pack.sh` when context packs are stale or missing
