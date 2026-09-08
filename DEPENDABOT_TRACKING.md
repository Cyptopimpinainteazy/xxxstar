# Dependabot Advisory Tracking

GitHub reports the following vulnerabilities on the default branch:

- Critical: 3
- High: 41
- Moderate: 96
- Low: 30
- Total: 170

## Status

- Recorded: yes
- Remediated: yes (critical advisories fixed; GitHub rescan pending on push)
- Blocking mainnet merge: no, because these are dependency advisories rather than
  atomic-swap proof-path correctness issues.

## Rust Advisory Evidence

- `cargo audit` exit code: 0
- Blocking vulnerabilities: 0
- Allowed warnings: 33

The 170 GitHub Dependabot findings are therefore predominantly JS/Python
ecosystem advisories and/or GitHub severity policy, not Rust security failures.

Latest applied: Cargo-compatible updates, tracked npm/pnpm lockfile fixes,
Next.js/postcss security bumps, and the Python requirements group. The three
remaining critical findings were then identified and remediated in the final
ecosystem-specific patch:

- #589 npm `vitest` in `apps/x3-studio/package.json`
  (`GHSA-5xrq-8626-4rwp`): manifest moved from `^1.6.0` to `^5.0.0`; the pnpm
  lockfile already resolved 5.0.0 and now matches the manifest.
- #393 Rust `wasmtime` in the root `Cargo.lock`
  (`GHSA-xx5w-cvp6-jv83`): patched `wasmtime` from 35.0.0 to the 36.0.7+
  fixed line (36.0.14).
- #390 Rust `wasmtime` in the root `Cargo.lock`
  (`GHSA-jhxm-h53p-jm7w`): cleared by the same 36.0.14 resolution.

The `wasmtime` move required local `sc-executor-wasmtime` and
`sp-wasm-interface` patches that depend on `wasmtime 36.0.7`, because the
stable2512 polkadot-sdk line still pins the vulnerable 35.x series.

## Remediation plan

1. Generate the complete advisory list from GitHub Security > Dependabot.
2. Group fixes by ecosystem: Cargo, npm, pnpm, Python.
3. Apply compatible `cargo update`/`npm audit fix`/`pnpm audit fix` upgrades.
4. Leave semver-major upgrades for separate PRs.
5. Run the full Rust, JS, and Python gates after each group.
6. Re-open or close this tracker when GitHub's reported counts change.
