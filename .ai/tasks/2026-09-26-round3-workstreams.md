# Round 3 — 2026-09-26

Round 2 is closed and pushed; see `.ai/reports/round2-integration-20260926.md`. The seven-validator
network, the soak under load, the drills, the public-testnet gate and the partition test all exist
now. What is left on this box is *scale*: the proofs that exist at three validators, run at seven.

## Ground rules (unchanged)

1. **Do not commit and do not push.** The primary agent commits. Leave your work in the tree.
2. **Do not edit `scripts/local-ci.sh`.** Hand me the gate line as text.
3. **Own only your workstream's files.**
4. **No fake green**, and re-run your own proof; quote the command and the output.
5. Make it load-bearing: show the check FAILS when the thing it claims is not true, then restore.

## Facts already established (do not re-derive)

* Bring up a seven-validator network with a *generated* spec (never dev keys):
  `X3_NODE_BIN=target/release/x3-chain-node OUT_DIR=<spec dir> python3 scripts/testnet/build-x3-testnet-spec.py 7`
  then
  `COUNT=7 RPC_BASE=<base> P2P_BASE=<p2p base> PROM_BASE=<prom base> BASE_DIR=<dir>
   CHAIN_SPEC=<spec dir>/x3-testnet-plain.json KEYS_DIR=<spec dir>/validator-keys
   LOG_DIR=<dir>/logs SKIP_BUILD=1 bash scripts/testnet/x3_testnet_up.sh --skip-build`.
  The launcher refuses a spec whose authority count does not match `COUNT` — it is right, heed it.
  It writes `pids/node-<n>.pid` and `logs/node-<n>.log` under `BASE_DIR`, and refuses to wipe a
  directory it does not recognise, so use a fresh one.
* GRANDPA's threshold is `n - (n-1)/3`: three authorities need all three, four need three, seven
  need five. **A three-validator set cannot lose a validator and keep finalizing** — measured. That
  is why the existing `node/tests/partition_recovery.rs` (three authorities) asserts that nothing
  finalizes during the cut, and why a *tolerance* claim needs four or more.
* Pin every port; a node started without `--no-mdns`/with default ports can silently never reach RPC.
* Freeze one copy of the node binary before spawning a set and print its digest: two builds of the
  same tree have different genesis hashes.
* `ss -ltnp` + `sed` gives the pid listening on a port; `/proc/<pid>/cmdline` gives its argv.

---

## Workstream I — partition *tolerance* at seven authorities

**Deliverable:** a test proving that a seven-validator network loses one validator to a network
partition and **keeps finalizing** — which is the property a public testnet needs, and which the
three-authority test cannot show (it asserts the opposite, correctly, for that set size).

Build it in `node/tests/partition_recovery.rs` as a **second `#[ignore]`d test** rather than a new
harness: that file already has the pieces — `NetTools::cut`/`heal` (netfilter DROP plus a socket
reset, because DROP alone leaves established connections up), `PartitionGuard` (panic-safe removal
of the rules), `view(port)`, `wait_until`, `Shell`, `freeze_node_binary`, and the port pre-flight.
Reuse them; do not copy them.

The test must:

1. build a generated `N`-authority spec (`N` from `X3_PARTITION_VALIDATORS`, default 7) and boot it
   with the launcher above, on ports in a free block (avoid 19954-19956/30389-30391, 19964-19967/
   30394-30397, 19974-19976/30410-30412, and 9944-9950);
2. wait until all `N` are connected (each with `N-1` peers) and *finalizing* past genesis;
3. cut exactly one validator while it keeps running;
4. **confirm the cut through peer counts** before asserting anything — if the cut does not show, the
   test fails and says so (the three-authority test proves this check can say no);
5. require the survivors to keep **finalizing** (assert on `chain_getFinalizedHead`, never on the
   best block) and the isolated validator to fall behind;
6. heal, then require all `N` to converge on the same finalized height and hash and to finalize new
   blocks again;
7. tear the network down on success *and* on panic (kill by the pid files the launcher wrote; do not
   `pkill -f x3-chain-node`, other gates' nodes are on this box).

**Acceptance:** the ignored test passes, with the output quoted: `N`, the confirmed cut (peer counts
before/during/after), finalized heights before/during/after on both sides, and the converged height
and hash. Plus the negative control: with the socket reset disabled the cut must fail to show (the
existing test's `X3_PARTITION_SKIP_RESET` control shows the shape) — or, if you find a better
control, say which and why it is load-bearing.

**Files you own:** `node/tests/partition_recovery.rs` only.

---

## Workstream J — monitoring and logging at seven validators

**Deliverable:** the monitoring and logging proofs currently cover the three-validator `local3`
chain. The goal's bullet is "live across all validators" and the testnet is seven. Make both checks
work against the generated seven-authority network, without losing what made them load-bearing.

`scripts/monitoring/local3-monitoring-check.sh` and `scripts/monitoring/local3-logging-check.sh`
(with `lib-local3.sh`) boot `local3` from a static scrape config with three targets and assert
per-validator identity (node name label matching the target label, chain label, finalized height,
peer count, authority role) plus, for logging, that each stream carries its own identity, imports
and finality, and that one aggregate query over the sink reaches all sources. Both have `--self-test`
negative controls; keep them working.

Add a seven-validator mode that uses the generated spec and the launcher (facts above), with a
`monitoring/testnet7/prometheus-*.yml` scrape config the check parses (so the config still drives
what is scraped) and a matching Grafana dashboard whose referenced metrics must exist in the live
scrape. Keep the `local3` path working — it is gated, and the goal says the check must fail, not
pass, when a validator is missing.

**Acceptance:** both checks run against seven validators on this box and pass, with the output
quoted: seven distinct identities scraped, the canonical-hash agreement check, and (for logging)
seven streams with their last imported/finalized heights. Plus each check's negative control still
failing when one validator is held out.

**Files you own:** `scripts/monitoring/**`, `monitoring/**`. Do not edit `scripts/local-ci.sh`
(hand me the gate lines as text) and do not edit the `local3` paths in a way that changes their
gated behaviour without saying so.

---

## Reporting back

```
FIXED:            (files)
TESTED:           (commands + the output they printed, including the negative control)
COMMIT:           (none — say what you would commit)
GATE LINE:        (as text)
REMAINING HARD BLOCKERS:
```
