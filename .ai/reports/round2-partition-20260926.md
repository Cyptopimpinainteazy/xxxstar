# Round 2 — Workstream G: a partition, not a crash (2026-09-26)

## Deliverable

`node/tests/partition_recovery.rs` — one integration test that boots the built-in three-validator
`local3` chain, cuts exactly one validator (Charlie) off from the other two at the kernel level
**while it stays alive**, and requires the chain to behave correctly and then heal.

Command (the gate line):

```bash
env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test partition_recovery \
  -- --ignored --nocapture --test-threads=1
```

Restore-to-green evidence for the round-2 rules ("show it fails when the thing it claims is not
true, then restore and show it passes") is the control run below.

## Finding: the workstream note's premise is wrong, and the honest test says so

The round-2 note said: *"the remaining two keep authoring and finalizing — with three GRANDPA
authorities, two is exactly the 2/3 threshold."* That is wrong for this chain.

`node/src/chain_spec.rs::local_three_validator_config` builds `local3` from exactly three
`initial_authorities` (Alice, Bob, Charlie). GRANDPA's finality threshold for a voter set of size
`n` is `n - (n - 1) / 3`; for `n = 3` that is `3 - 0 = 3`. Two of three is 66.7%, *below* GRANDPA's
strict-greater-than-2/3 threshold, so isolating one authority must **stall finality**. A chain that
finalized on two of three would be rounding the threshold the unsafe way.

The test therefore asserts the truthful behaviour: the two survivors keep **authoring** but finalize
**nothing** until the third returns. That "nothing was finalized" is exactly the safety property the
note's premise would have hidden.

## The lever, and why it is honest here

All three validators run on one host, so netfilter cannot tell two processes apart by port unless
each link is pinned to a known port. Two facts make that possible:

1. Substrate's libp2p reuses the listening port for outgoing connections, so every link between two
   validators uses exactly their two P2P ports (observed: `127.0.0.1:30410 <-> 127.0.0.1:30412`).
2. Charlie is booted with `--out-peers 0` and no bootnodes, so it never dials; every link it owns is
   an *inbound* connection to its one P2P port. Alice and Bob dial in.

The cut is installed without root through a privileged, host-network container that `chroot`s into
the host and runs the host's own binaries (`docker run --rm --privileged --network host -v /:/host
alpine chroot /host /usr/sbin/iptables …` and `… /usr/bin/ss …`).

Two tools are required, and that was found the hard way:

* `iptables` DROP rules on the isolated port keep the survivors from dialing Charlie back.
* `ss -K` forcibly resets the sockets that already exist. **A DROP alone does not partition this
  node**: packets are black-holed, but the node does not treat a black hole as a disconnect. Measured
  directly with a 3-node run: after a DROP the connections stayed in `ESTAB` with ~150 KB of unacked
  data queued and `system_health.peers` unchanged for 90 s. `ss -K` closes the sockets on both ends,
  which both nodes observe immediately. (A listening socket on that port survives `ss -K`.)

## Evidence

### PASS — three validators, one cut, recovery

```
[x3-partition] netfilter: /usr/sbin/iptables + /usr/bin/ss via privileged host-network container
[x3-partition] connected: alice=2 bob=2 charlie=2 peers
[x3-partition] consensus: finalized alice=5 bob=5 charlie=5, all agree on 5:0x1ddf7f48c340aa45...
[x3-partition] baseline at 5:0x1ddf... — best alice=96 bob=96 charlie=96; peers alice=2 bob=2 charlie=2
[x3-partition] cut installed, live sockets reset for p2p port 30412
[x3-partition] cut confirmed: peers alice=1 bob=1 charlie=0 (charlie is alive but reachable to nobody)
[x3-partition] fork point ≈ 105; finality watermark alice=98 bob=98 charlie=96
[x3-partition] during: best alice=119 bob=119 charlie=105; finalized alice=98 bob=98 charlie=96; peers alice=1 bob=1 charlie=0
[x3-partition] partition proven: two survivors authoring past 105 (alice=119 bob=119), charlie alive and behind (105), and no validator finalized past 98 — the 3-authority threshold held
[x3-partition] cut removed for p2p port 30412
[x3-partition] healed: peers alice=2 bob=2 charlie=2
[x3-partition] converged: all three finalized 127:0x7d0d26a1416a7ad57a929a853bd4a1202748dc44c86b27c8927694522f23e31c (alice=127 bob=127 charlie=127), past the freeze at 98
[x3-partition] finality resumed: 98 (frozen) -> 127 (converged) -> 164 (still advancing) on all three
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 218.04s
```

What this shows, step for step:

1. three validators connected (2 peers each) and finalizing;
2. one cut off and still alive (Charlie answers RPC; its peers drop to 0, the survivors' to 1);
3. the survivors keep authoring (best 105 → 119) while finality stalls at the watermark — the
   3-of-3 threshold is respected, so no premature finality;
4. Charlie was genuinely behind (best 105 vs 119), not merely disconnected at the same height;
5. on reconnect, all three converge on the **same** finalized hash and finality resumes and keeps
   advancing (98 → 127 → 164).

### CONTROL — disable the socket reset, and the gate must fail

```
[x3-partition] X3_PARTITION_SKIP_RESET set: leaving live sockets up (control)
[x3-partition] cut installed, live sockets reset for p2p port 30412
thread '...' panicked at node/tests/partition_recovery.rs:422:5:
timed out after 180s waiting for the cut to show as peer counts (charlie 0, others 1); last observation: alice=1 bob=1 charlie=2
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 323.84s
```

With the reset disabled the DROP rules alone leave Charlie reporting a stale peer, so the gate's own
cut-confirmation fails. The peer-count check is load-bearing: it is not a constant that always says
"partitioned".

## Files changed

* `node/tests/partition_recovery.rs` (new) — the test and its netfilter/`ss` helper.

No other file was touched. `scripts/local-ci.sh` was not edited (the gate line is given above as
text). Nothing was committed or pushed.

## Remaining risks / notes

* The gate needs a reachable netfilter plus `ss`; it fails loudly if neither a direct `iptables` nor
  the docker-chroot fallback works, rather than reporting a partition that did not happen.
* The cut installs four host `INPUT`/`OUTPUT` DROP rules scoped to one P2P port this gate owns, and
  removes them on heal and on drop (panic-safe). Leftover rules would only affect that port.
* The test proves the partition on `local3`; it says nothing new about the physical 7-node testnet,
  which does not exist on this box.
