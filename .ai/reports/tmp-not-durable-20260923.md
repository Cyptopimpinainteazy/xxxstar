# `/tmp` is not durable here — a cleanup cost a two-hour measurement and a build cache

Date: 2026-09-23, ~02:01–03:22 UTC

## What happened

Between one command and the next, everything this agent had under `/tmp` was gone:

| lost | what it was |
| --- | --- |
| `/tmp/x3-soak-run` | the git worktree all of yesterday's evening work ran from |
| `/tmp/x3-signer-target` | the cargo target directory (every build artifact, debug and release) |
| `/tmp/btc-core`, `/tmp/btc-regtest` | Bitcoin Core v28.1.0 and the live regtest chain (122 blocks, wallet `x3`) |
| `/tmp/x3-soak2h`, `/tmp/x3-soak-rel`, `/tmp/x3-soak-nocache2h` | three soak runs' samples, logs and reports — including the **in-flight** two-hour no-cache run (TICKET-100b), killed about ten minutes in |
| `packages/ts-sdk/node_modules` | the `@polkadot/api` install that finally made extrinsic signing possible on this host |

`df` went from ~1.4 TB used to 593 GB, so the cleanup freed far more than these: it was a
disk-space sweep of `/tmp`, not an accident aimed at this work.

## What survived, and why this is an inconvenience rather than a loss

* **Every commit.** All of yesterday's work was pushed and merged; `origin/master` is `770e13ab08`
  with PRs #466–#475 in it. The worktree was clean when it vanished.
* **The srtool images**, including the pinned `paritytech/srtool:1.93.0-0.18.4` at the digest the
  runtime record names (`sha256:8638a668…`), so runtime attestation still works.
* **The repository's own `target/`** (54 GB), which cut the rebuild to nine minutes.
* The captured Bitcoin block/transaction/merkle-path artifacts, which are committed under
  `.ai/reports/`.

## The rule this settles

This repository already ignores `/.wt-*/` — "git worktrees used by parallel fix agents"
(`.gitignore` line 169). That convention exists for exactly this reason, and this agent was putting
worktrees, build output and soak data in `/tmp` instead because it was convenient. So:

**Worktrees, soak base directories and build output belong inside the repository
(`<repo>/.wt-<name>/`) or another durable path — never `/tmp`.** A long measurement in particular
must write its samples somewhere that outlives the session: losing two hours of samples to a disk
sweep is a self-inflicted wound when a durable directory was one `cd` away.

Action taken: the worktree is now `<repo>/.wt-agent` (branch `agent/soak-100b`, at `770e13ab08`),
`CARGO_TARGET_DIR` points at the repository's own `target/`, and the two-hour no-cache soak was
restarted with `BASE_DIR` inside the worktree, so a repeat of the sweep costs nothing.

## Also worth knowing

After the cleanup, the Codex sandbox stopped working for non-escalated commands:

```
error building bubblewrap command: mountinfo path is not absolute
```

Every command in this session needed `require_escalated` afterwards. That is an environment
failure, not a repository one, but it changes the cost of a command, so the next agent should
expect it until the box's sandbox is restarted.

## Not claimed

Nothing here says the cleanup was wrong or malicious: freeing ~800 GB of `/tmp` is ordinary
housekeeping, and the only thing it actually destroyed was data that should not have been living
somewhere specified as temporary.
