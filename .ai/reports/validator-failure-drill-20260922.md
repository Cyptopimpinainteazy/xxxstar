# What the network does when validators die

Date: 2026-09-22. Base: `origin/master` = `0c8ac1603`.

The multi-validator row said "no failure injection". A network that finalizes with
everything up proves liveness on the happy path and nothing about the properties
mainnet depends on, so `scripts/testnet/validator-failure-drill.sh` now kills
validators and requires both halves of consensus to hold:

1. **liveness under a minority down** — kill the largest number that still leaves
   more than two thirds, and finality must keep advancing;
2. **safety once that supermajority is gone** — kill one more, and finality must
   *stop*. Authoring continues (Aura needs one author), so a chain that keeps
   finalizing here has no safety at all. The stall is sampled after a full
   `STALL_WINDOW` (60 s), not at the first quiet poll, so the claim is "it cannot"
   rather than "it was slow";
3. **recovery** — restart the dead validators and require all of them to rejoin one
   chain;
4. **the harness leaves nothing behind** — a leaked validator holds the RPC and p2p
   ports, so the next run fails to bind and looks like the network's fault.

The kill count is derived (`max(1, (count-1)/3)`), so `--count` alone cannot produce
a drill whose first phase is out of range — and the guard refuses combinations that
would test half of what the script claims.

## Verified

Seven validators (two runs, one before phase 4 existed and one with the hardened
cleanup):

```
[drill] PASS: 7 validators up
[drill] PASS: all 7 finalizing; agreed at height 496 (0xb9a032ee…)
[drill] PASS: with 2 of 7 down, finality continued (570 → 583) and the survivors agree (0xa7bf7b85…)
[drill] PASS: with 3 of 7 down, finality stopped at 585 (still 585 after 60s) while authoring continued (head 589 → 760)
[drill] PASS: restarted; all 7 finalizing again at 1198; one chain (0x981aeae5…)
```

Four validators (complete, including the cleanup phase):

```
[drill] PASS: with 1 of 4 down, finality continued (303 → 305) and the survivors agree (0x51d19800…)
[drill] PASS: with 2 of 4 down, finality stopped at 307 (still 307 after 60s) while authoring continued (head 311 → 462)
[drill] PASS: all 4 finalizing again at 682; one chain (0x65281f32…)
[drill] PASS: no validator processes left under /tmp/x3-drill4
[drill] ALL PHASES PASSED: minority failure keeps finality, a missing supermajority stops it, recovery restores it.
```

And as a gate, so it is one command:

```
$ bash scripts/local-ci.sh --failure --only validator-failure-drill
PASS validator failure drill            358s
```

## Two harness bugs the drill found in itself

* **A restart rewrote a pid file.** `run-7-validators-local.sh --only <i>` (added
  for the restart phase) led to `node-1.pid` holding another node's pid, so the
  drill's pid-file kill missed the real node 1 — which stayed up holding ports 9944
  and 30333. Both the drill's cleanup and the launcher's `stop_nodes` now sweep by
  `--base-path <BASE_DIR>/node-` as well, and the drill asserts no process is left.
* **`pgrep` under `set -o pipefail`.** `running_nodes` was `pgrep … | wc -l`; when
  nothing matched, `pgrep` exited 1, pipefail propagated it, and the assignment
  under `set -e` killed the drill silently right after it had cleaned up correctly.
  Zero matches is the success case, so the status is swallowed explicitly.

Also fixed while wiring this up: `build-x3-testnet-spec.py` used to write over
`deployment/chain-specs/fresh/*.json`, a *tracked* fixture shared by the
`run-fresh-*`/mesh tooling. Every run left a dirty tree and, worse, a spec whose
authorities matched the new seeds while the committed fixture still looked
authoritative — which is exactly the mismatch the launcher's preflight then refused.
It now writes to the ignored `fresh/generated/` and prints the launch command.

## Where the row stands

`X3-L1-001` moves to implemented 88 / tested 85 / mainnet-ready 55. What remains is
what a single machine cannot show: independent operators, network partitions, clock
skew, a validator rejoining after long absence under real vote loss, slashing and
jailing evidence, and duration — nothing here has run for more than minutes.
