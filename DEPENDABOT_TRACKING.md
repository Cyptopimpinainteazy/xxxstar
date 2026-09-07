# Dependabot Advisory Tracking

GitHub reports the following vulnerabilities on the default branch:

- Critical: 5
- High: 243
- Moderate: 278
- Low: 65
- Total: 591

## Status

- Recorded: yes
- Remediated: no
- Blocking mainnet merge: no, because these are dependency advisories rather than
  atomic-swap proof-path correctness issues.

## Remediation plan

1. Generate the complete advisory list from GitHub Security > Dependabot.
2. Group fixes by ecosystem: Cargo, npm, pnpm, Python.
3. Apply compatible `cargo update`/`npm audit fix`/`pnpm audit fix` upgrades.
4. Leave semver-major upgrades for separate PRs.
5. Run the full Rust, JS, and Python gates after each group.
6. Re-open or close this tracker when GitHub's reported counts change.
