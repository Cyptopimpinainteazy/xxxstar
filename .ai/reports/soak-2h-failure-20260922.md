# The two-hour soak failed, and the logs say why

Date: 2026-09-22
Run: 4 validators, isolated ports (RPC 12044–12047, p2p 32400–32403), logs `/tmp/x3-soak120/logs/`,
samples `/tmp/x3-soak120/samples.tsv` (one line per minute), launcher log `/tmp/x3-soak120b.log`.

## Verdict

```
[soak] FAIL: rpc 12046 has not finalized since 1790099493 (27677 at height)
```

The **twenty-minute** soak passed earlier the same day (1,215s, +4,802 blocks/validator, agreement at
three heights, no node lost). The **two-hour** run did not: at roughly the 100-minute mark one
validator lost its peers, fell behind, and the chain stopped finalizing at height 27,677.

This is recorded as a failure, not softened. The row `X3-L1-001` said "twenty minutes is not a
duration claim" — this is that claim being tested and not holding.

## What the log shows, in order

The mechanism is visible on every node, not just the one that failed. Counts over the two hours:

| node | bans issued | trie-cache lock timeouts | unknown-parent imports | state discarded | missed Aura slots |
| --- | --- | --- | --- | --- | --- |
| node-1 | 10 | 419 | 108 | 26 | 1 |
| node-2 | 14 | 316 | 31 | 29 | 0 |
| node-3 | 16 | 105 | 252 | 14 | 1 |
| node-4 | 12 | 355 | 146 | 30 | 1 |

1. **Lock contention from the start.** The first line is at 09:55:51, four minutes into the run:
   `Timeout while trying to acquire a write lock for the shared trie cache` — hundreds of times per
   node. Blocks are being imported faster than the node can service them.
2. **State falls out from under the import queue.** `State already discarded for 0x…` and
   `block has an unknown parent` — the node cannot keep the state for blocks it is still importing.
3. **Slots get missed.** `Creating inherent data took more time than we had left for slot …` — the
   node was still preparing a block for a 3-second slot when the slot ended.
4. **Then the peer set punishes the lagging node.** Node 3's first ban is at 10:53:49, one hour in:
   `Report 12D3KooW…: Reason: Same block request multiple times. Banned, disconnecting.` A validator
   that has fallen behind is *exactly* the node that requests the same block more than once. The
   network's response to a lagging peer is to disconnect it.
5. **So it falls further behind, and ends up on the wrong side of finality.** 11:12:08:
   `Potential long-range attack: block not in finalized chain` (its view had diverged from the
   finalised chain), and 11:46:28: `Re-finalized block #… (27111) in the canonical chain, current
   best finalized is #27136` — a finality *regression* in that node's local view.
6. **Finality stops.** All four nodes sit at the same height, node 3 with zero peers, and the soak's
   own check reports no finalisation since the stall.

## What was happening on the box

`load average: 55–62` with four **debug** nodes (1.3–1.6 GiB RSS each) plus other agents' networks
and builds on the same machine. That is the trigger, and it is not the network's fault — but the
*response* to being starved is the network's behaviour, and it is a feedback loop:

```
CPU starved → trie cache lock timeouts → imports dropped / slots missed
    → validator falls behind → repeats block requests → peers ban it (mutual)
    → fewer peers → further behind → finality stalls
```

## What is a defect and what is environment

* **Environment (certain):** the run competed with load ~55–60. Nothing here says a 4-validator
  network fails on an idle box.
* **Defect (probable, needs isolation):** the ban policy treats "same block request multiple times"
  as misbehaviour. For a behind validator that is the expected behaviour, and banning it converts a
  lag into a partition. Ten to sixteen bans per node over two hours is a peer set that is degrading
  itself, not healing.
* **Not a safety failure:** no node ever finalised two conflicting chains; the failure is liveness.
* **Still open from the 20-minute run:** RSS grew to 1.3–1.6 GiB per debug node. Two hours does not
  settle cache-vs-leak either, but it does rule out "stable at 500 MiB".

## Next actions

1. Re-run exactly this soak on an otherwise idle box, same duration, same sampling, so load is
   eliminated as the cause. That single run decides whether this is a robustness defect or an
   over-subscribed machine.
2. Investigate the peer-ban path for repeated block requests (`sc-network`/`sc-network-sync`
   reputation): a peer that asks for the same block twice while behind should be slowed, not
   banned.
3. Look at the trie-cache lock timeouts with block import: what is holding the write lock, and
   whether the node should throttle authoring when it is behind its own import queue.
4. Only then re-state a long-duration claim in `X3-L1-001`.
