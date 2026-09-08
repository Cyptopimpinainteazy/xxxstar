# Dependabot Advisory Tracking

GitHub reports the following vulnerabilities on the default branch:

- Critical: 3
- High: 41
- Moderate: 96
- Low: 30
- Total: 170

## Status

- Recorded: yes
- Remediated: no
- Blocking mainnet merge: no, because these are dependency advisories rather than
  atomic-swap proof-path correctness issues.

## Rust Advisory Evidence

- `cargo audit` exit code: 0
- Blocking vulnerabilities: 0
- Allowed warnings: 33

The 170 GitHub Dependabot findings are therefore predominantly JS/Python
ecosystem advisories and/or GitHub severity policy, not Rust security failures.

## Remediation plan

1. Generate the complete advisory list from GitHub Security > Dependabot.
2. Group fixes by ecosystem: Cargo, npm, pnpm, Python.
3. Apply compatible `cargo update`/`npm audit fix`/`pnpm audit fix` upgrades.
4. Leave semver-major upgrades for separate PRs.
5. Run the full Rust, JS, and Python gates after each group.
6. Re-open or close this tracker when GitHub's reported counts change.
