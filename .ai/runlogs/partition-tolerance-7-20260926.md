# Round 3 — Workstream I: partition tolerance at seven authorities

File changed: `node/tests/partition_recovery.rs` (one new `#[ignore]`d test + reused helpers).

## Gate line (identity: host `x3star1`, branch `feat/x3-prelaunch-economics-x3lang-cutover`)

```bash
env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test partition_recovery \
  -- --ignored --nocapture --test-threads=1
```

Frozen artifact under test (one copy, printed by the test):
`x3-chain-node sha256 9ee22abc5de47eb4833a01e62d82b56938998e213180137e146e853865cd3987`

## PASS — the whole gate (both ignored tests) on the committed bytes, 546.13s

```
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 546.13s
```

Three-authority test (unchanged behaviour, shared helpers still good):

```
[x3-partition] cut confirmed: peers alice=1 bob=1 charlie=0 (charlie is alive but reachable to nobody)
[x3-partition] partition proven: two survivors authoring past 98 (alice=110 bob=110), charlie alive and behind (98), and no validator finalized past 90 — the 3-authority threshold held
[x3-partition] finality resumed: 90 (frozen) -> 123 (converged) -> 125 (still advancing) on all three
```

Seven-authority test:

```
[x3-partition7] every validator runs one frozen artifact: .../x3-chain-node (sha256 9ee22abc5de47eb4833a01e62d82b56938998e213180137e146e853865cd3987)
[x3-partition7] connected: 7 validators, peers [6, 6, 6, 6, 6, 6, 6]
[x3-partition7] consensus: all 7 agree on 110:0xf69114c4186d80212ec4686ab98610ae1aea6e2c0fdd557618fac5343c39bc13 (finalized [495, 495, 495, 495, 495, 495, 110])
[x3-partition7] restarting validator 4 inbound-only (--out-peers 0, unreachable bootnode) so the cut can be complete
[x3-partition7] validator 4 rejoined inbound-only: peers [6, 6, 6, 6, 6, 6, 6]
[x3-partition7] validator 4 caught up before the cut: finalized [661, 661, 661, 659, 661, 661, 661]
[x3-partition] cut installed, live sockets reset for p2p port 30423
[x3-partition7] cut confirmed: peer counts [5, 5, 5, 0, 5, 5, 5]
[x3-partition7] fork point ≈ 679; finality watermark 707; validator 4 frozen at 668
[x3-partition7] during: finalized [718, 718, 718, 668, 718, 718, 718]; best [723, 723, 723, 681, 723, 723, 723]; peers [5, 5, 5, 0, 5, 5, 5]
[x3-partition7] tolerance proven: 6 of 7 survivors finalized past 707 while validator 4 stayed alive at 668 and fell 42 behind
[x3-partition7] healed: peers [6, 6, 6, 6, 6, 6, 6]
[x3-partition7] converged: all 7 finalized 735:0xb28ab845e1830b34c64e588c7852b1ac5b8b9c81134cdb2842662def5f5a5a98 (heights [740, 740, 740, 735, 740, 740, 740]), past the freeze 707
[x3-partition7] finality resumed: 707 (frozen) -> 735 (converged) -> 738 (still advancing) on all 7
```

## Negative control — the cut check is load-bearing

```bash
X3_PARTITION_SKIP_RESET=1 env -u SKIP_WASM_BUILD cargo test -p x3-chain-node \
  --test partition_recovery seven_authority -- --ignored --nocapture --test-threads=1
```

With the live-socket reset disabled the DROP rules alone leave the sockets `ESTAB`, so the peer
counts never move and the test fails by name instead of pretending a partition happened:

```
[x3-partition] X3_PARTITION_SKIP_RESET set: leaving live sockets up (control)
[x3-partition7] cut confirmed: ...
thread '...seven_authority...' panicked:
timed out after 180s waiting for the cut to show as peer counts (victim 0, each survivor 5);
last observation: survivor 1 reports 6 peers, want 5; peer counts [6]
test result: FAILED. 0 passed; 1 failed; ... finished in 601.18s
```

## Two measured facts that corrected the workstream note

1. **The launcher's mesh is a full mesh only when the spec's bootnode ports match `P2P_BASE`.**
   `build-x3-testnet-spec.py` writes `bootNodes` as `/ip4/127.0.0.1/tcp/${P2P_BASE + i - 1}/…`, so
   the builder and the launcher must be given the same `P2P_BASE`. Given different ones, the spec
   advertises ports nobody listens on, only the CLI bootnode is reachable, and the network is a
   *star* (bootnode 6 peers, every other validator 1). The test passes `P2P_BASE` to both.
2. **A port-scoped cut is complete only if the isolated validator does not dial out.** With its
   normal outbound budget the cut validator re-dials its peers on a fresh *ephemeral* source port,
   which matches neither direction of a rule scoped to its P2P port, and it re-syncs — measured:
   peer counts fell to `[5,5,5,0,5,5,5]` at the cut and the victim was level with the survivors 180 s
   later. The test therefore restarts the victim the way the three-authority test boots Charlie
   (`--out-peers 0`, a spec whose only bootnode is unreachable), and only then cuts.
