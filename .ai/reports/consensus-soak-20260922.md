# Twenty minutes of consensus, and two networks that were sharing ports

Date: 2026-09-22. Base: `origin/master` = `81539d86fe`.

The consensus work so far was all measured in minutes: finality, agreement, failure,
recovery. `scripts/testnet/consensus-soak.sh` samples a running network on an interval
and fails on the first thing that a short run cannot see — a stall, a node that died, a
peer set that drifts away, or memory that only grows.

## Verified

```
$ COUNT=4 MINUTES=20 INTERVAL=20 bash scripts/testnet/consensus-soak.sh
[soak] PASS: all 4 validators agree on the same chain at heights 1273, 2546, 3819
[soak]   rpc 9944: height 289 -> 5091 (+4802), peers 3 (min 2), rss growth 479.6 MiB
[soak]   rpc 9945: height 289 -> 5091 (+4802), peers 3 (min 2), rss growth 481.6 MiB
[soak]   rpc 9946: height 292 -> 5091 (+4799), peers 3 (min 3), rss growth 498.7 MiB
[soak]   rpc 9947: height   0 -> 5093 (+5093), peers 3 (min 3), rss growth 515.1 MiB
[soak] PASS: 20 minute(s) with no stall beyond 60s, no node lost, agreement held
[soak] PASSED: 4 validators, 20 minutes, one chain throughout.

$ bash scripts/local-ci.sh --soak --only consensus-soak     # opt-in gate, MINUTES= to change
```

1,215 seconds, ~4,800 blocks per validator, one chain at three sampled heights, no
stall longer than a minute, no node lost.

Two readings are recorded rather than smoothed over: two validators dipped to 2 peers
during the run (back to 3), and each **debug** node's RSS grew ~490–515 MiB. That is
under the harness's 1 GiB bound, but twenty minutes cannot tell cache from leak — the
row's blocker says so, and a longer soak is the way to settle it.

## The collision this found by accident

While the soak was running, another network appeared on the same box:
`/tmp/x3-rotation-manual/node-1..3` — another agent's run — listening on **9944–9946**,
the same ports the soak's nodes hold. One of the two was not serving RPC, and neither
run could tell from its own logs.

The launchers hardcoded `30333`/`9944`/`9615` plus the validator index, so two networks
on one machine always collided. They now take `P2P_BASE`, `RPC_BASE` and `PROM_BASE`,
and the spec builder takes `P2P_BASE` too — because the spec's `bootNodes` must name
the p2p ports the validators will actually listen on, while the RPC base is
orthogonal:

```
$ OUT_DIR=/tmp/x3-p2p-check P2P_BASE=31400 python3 scripts/testnet/build-x3-testnet-spec.py 2
[spec] carries all 2 derived bootnodes
bootNodes: ['/ip4/127.0.0.1/tcp/31400/p2p/12D3KooWAdF…', '/ip4/127.0.0.1/tcp/31401/p2p/12D3KooWQ3W…']
```

So the second agent (or a soak beside a drill) runs with, say,
`RPC_BASE=10044 P2P_BASE=31400 PROM_BASE=19615`.

## Row

`X3-L1-001` — tested 85 → 88, mainnet-ready 55 → 60. Its blocker list no longer says
"nothing has run for longer than minutes"; it now says what twenty minutes cannot
settle: era rotation, and whether that RSS growth is cache or leak.
