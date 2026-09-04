# TESTNET_VERIFICATION.md

Evidence log for this audit run. Every completion claim must trace to a command+output
recorded here. Prior report files in the tree are untrusted until re-run.

Formats:
```
## <VERIFICATION-TOPIC> <date>
Command: <exact>
Result: PASS|FAIL
Evidence: <hash/output/link>
```

## Baseline

### Toolchain
- rustc 1.90.0 / cargo 1.90.0 (rust-toolchain.toml pinned 1.90.0). Confirmed 2026-09-03 via
  `rustc --version`, `cargo --version`.
- Hardware: 32 cores, 109 GiB RAM, 1.8 TB free on /home.

### Git/baseline snapshot
- Repo had NO .git (unversioned snapshot). Created git repo, committed full tree as
  `091dbe3 "Baseline snapshot ..."` on 2026-09-03. 21687 files staged. Working tree clean at baseline.
