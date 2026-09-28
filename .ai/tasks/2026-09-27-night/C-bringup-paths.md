# Workstream C — the bring-up path points at a directory that does not exist

You are a lane of the X3 engineering team. Repo: `/home/lojak/Desktop/xxxstar-main`. Read
`AGENTS.md` first, then this brief.

## The defect

Several scripts hardcode `/home/lojak/Desktop/X3_ATOMIC_STAR` — a directory that does not exist on
this box. Two of them were in the release path and are already fixed
(`scripts/mainnet/run_release_gates_rc6.sh`, `scripts/mainnet/rc2_mock_and_live_gate.sh`); the second
one had *created* that empty directory with `mkdir -p` before `cd`, then failed with
`manifest path tests/e2e/Cargo.toml does not exist` — a wrong-root bug that reads like a missing
crate. The operator is about to bring up **seven physical validator servers**, so every script on
that path has to work from any checkout.

Known offenders (`grep -rn 'X3_ATOMIC_STAR' scripts/ *.sh`):

- `scripts/bootstrap-validator.sh` (`WORKSPACE="${WORKSPACE:-/home/lojak/Desktop/X3_ATOMIC_STAR}"`)
- `scripts/start-validator-network.sh` (same default)
- `scripts/run-all-tests.sh` (`cd /home/lojak/Desktop/X3_ATOMIC_STAR`)
- `scripts/install-testing-tools.sh`, `scripts/run-substrate-tests.sh` (banner text only — check)
- `scripts/systemd/{x3fronend.service,x3-desktop.service,cloudflared-tunnel.service,start-x3fronend.sh}`
- `PHASE_5_COMPLETE_LAUNCHER.sh`, `QUICK_PHASE_5_START.sh`
- `scripts/mainnet/rc2_internal_settlement_smoke.sh` has a *fallback* `X3_ORIGINAL_WORKSPACE`
  default in its embedded JS `require` roots — harmless but worth naming honestly.

## What to do

1. For each script on the **bring-up / validator-install / release** path, derive the root from
   `BASH_SOURCE` (`ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"`) while keeping any
   documented `$WORKSPACE`/`$X3_*` override working. For systemd units, `%h`-relative or
   `Environment=` with an explicit path is fine, but the ExecStart must exist for a checkout at any
   path — say in the unit's comment what the operator must set if it cannot be inferred.
2. Do **not** rewrite the launcher scripts wholesale; fix the path and anything that is provably
   broken because of it. If a script is dead legacy that nothing references, say so in your report
   and leave it alone rather than deleting it.
3. Verify what you can *run*: `bash -n` on every script you touch, plus the repo's own gate
   `bash scripts/local-ci.sh --only script-syntax`. Where a script can be exercised safely (a
   `--help`, a dry-run mode, a check-only path), exercise it and paste the output in your report.
   Never start a real validator network on this box without asking me first.
4. Add a regression gate if it is cheap and honest: e.g. a check that fails when a script under
   `scripts/` hardcodes an absolute `/home/<user>/Desktop/` path outside a documented override. Wire
   it into `scripts/local-ci.sh` next to the other static checks. It must fail on the *old* text
   (show that) and pass on the tree.

## Rules

- Do not weaken tests. **Commit only your own paths, never `git add -A`, and do not push** — message
  me (`/root`) with hashes, evidence and what is left.
