# Parallel workstreams, round 2 — 2026-09-26

Round 1 (see `2026-09-26-parallel-workstreams.md`) landed monitoring across the three
validators, snapshot/restore on a live chain, and the X3Lang receipt proof across the validator
network. Those are committed and pushed (`master` = `b82dfeb2c`).

This round takes three more bullets from the same objective.

## Ground rules (unchanged, and they have all been violated at least once here)

1. **Do not commit and do not push.** The primary agent commits.
2. **Do not edit `scripts/local-ci.sh`.** Write the gate line you want in your report as text.
   Three agents editing one gate array is how a gate list gets a duplicate entry.
3. **Own only your workstream's files.** Never touch `reports/rc6/*` (the operator's dirty set),
   `runtime/src/lib.rs`, or another workstream's paths.
4. **No fake green, and re-run your own proof.** Quote the command and its output.
5. A failing command is information: fix it, do not report it as a blocker.
6. Make the check load-bearing: show it FAILS when the thing it claims is not true (disable it,
   break it, or use a control), then restore and show it PASSES. That reversal is the evidence.

## Facts established this session that you should not re-derive

* `target/debug/x3-chain-node` is a *shared path*: another agent's `cargo build` relinks it under
  you. Freeze a copy (`cp target/debug/x3-chain-node /tmp/…`) before spawning nodes from it, and
  print the digest. Two builds of the same tree have two genesis hashes because the runtime blob
  is in genesis, and a multi-node gate will wedge rather than fail.
* `--dev` nodes pin **every** port (`--rpc-port`, `--port`, `--prometheus-port`); the defaults are
  held by whatever else is running and the node then never reaches RPC.
* A cold node AOT-compiles the runtime wasm with wasmtime. Under load, allow minutes, not seconds.
* `system_accountNextIndex` is the *pool's* view and advances when a transaction is queued. For
  "was this included", use the on-chain nonce (`state_call AccountNonceApi_account_nonce`) or the
  block's own `System.Events`.
* The chain has **no root path**: `Sudo` exists only in the `dev` runtime and its key is left
  unset (`development_config` sets `sudo: Default::default()`). Governance is the only privileged
  route, and `pallet_governance::enact_proposal` dispatches an approved proposal with
  `RawOrigin::Root`.

---

## Workstream D — a runtime upgrade through the path this chain actually has

**Deliverable:** the upgrade rehearsal (`scripts/mainnet/runtime_upgrade_rehearsal.sh`) performs a
runtime upgrade the way this chain permits, and proves the chain survives it.

Today it submits `subxt upgrade --suri //Alice`, i.e. it needs `subxt` (not installed) *and* a
`Sudo` key (this chain has neither). The bullet it serves is "Runtime upgrade: performed
successfully on the live 7-node testnet"; the seven nodes are not available, so the deliverable is
a rehearsal that really runs here, on the chain's own governance path, and is gated.

Read first: `pallets/governance/src/lib.rs` (`submit_proposal`, voting, `enact_proposal` →
`RawOrigin::Root`), the runtime's `impl pallet_governance::Config` (what `SubmitOrigin`,
`AuthorizedGovernanceAccounts`, `ProposalDeposit`, `VotingPeriod`, `EnactmentPeriod` are set to on
the dev chain), and `scripts/mainnet/runtime_upgrade_rehearsal.sh`.

What it has to show, at minimum:

* a `system.set_code` (or `code_substitutes`-equivalent on this runtime) proposal carried by the
  chain's own governance to approval and enacted with Root — **not** a sudo call;
* the runtime version actually changed (spec_version / the storage version of a pallet whose
  migration ran), read back over RPC from the node;
* blocks keep being authored and finalized after the upgrade;
* one operation that touches state after the upgrade (a balance transfer or an X3 comit) still
  works, so "the chain is alive" is not confused with "the chain is correct".

If part of that is impossible on this box, say precisely which call returned what and leave the
rehearsal failing rather than skipping it.

**Files you own:** `scripts/mainnet/runtime_upgrade_rehearsal.sh`, plus a new script/test under
`scripts/mainnet/` if you need one.

---

## Workstream E — EVM/SVM external paths: proven, or explicitly disabled

**Deliverable:** the goal says the EVM/SVM *external* paths must be "either proven on public
testnets or explicitly disabled". Make that true and machine-checked.

Start by finding what the claim covers: `feature-matrix/*` rows for the EVM/SVM external/bridge
paths, `TESTNET_FEATURE_FLAGS.toml` (`external_bridges_mainnet = "DISABLED_BLOCKED"` — the rc6 gate
already greps for it), the settlement engine's refusal of unattested external settlement proofs
(TICKET-063, mentioned in `runtime/src/lib.rs`), and `pallets/x3-settlement-engine`'s default
posture.

Then either (a) prove an external path against a real testnet — unlikely to be possible here and
do not fake it — or (b) make every external path that is *not* proven refuse to run, in the default
configuration, with a test that shows the refusal, and a gate that fails if any of them is
re-enabled without evidence.

Concretely, the evidence the primary agent wants from you:

* a list of the external paths (name → where it is gated → what happens if it is reached today);
* for each: a test proving the refusal (and, where a flag exists, that flipping the flag is what
  changes the answer — so the gate is load-bearing rather than a constant);
* a gate line, as text, for one command that checks all of them at once.

**Files you own:** `scripts/ci/**`, `TESTNET_FEATURE_FLAGS.toml`, new tests under
`pallets/x3-settlement-engine/tests/**` or `crates/external-chains/tests/**`. Do not touch
`crates/external-chains/src/settlement.rs` without saying so first — another workstream may.

---

## Workstream F — supply invariants under distributed traffic

**Deliverable:** the goal says "Supply invariants: proven under distributed traffic". Prove it on
the three-validator `local3` chain: drive concurrent traffic that moves value through the kernel
(comits / transfers on more than one validator), and require the supply invariant to hold on
**every** validator's own view at a finalized block.

Read first: `pallets/x3-kernel/src/invariant.rs` and `supply.rs` (what the invariant is and what
the pallet's own tests assert), `crates/x3-supply-ledger`, the existing `test x3-supply-ledger`
gate, and `node/tests/x3lang_network_receipt.rs` for how to boot the three validators from a frozen
binary.

The bar: not "a unit test with a mock runtime". Three real nodes, real finalized blocks, a
non-trivial number of operations that would break the invariant if accounting were wrong, and a
check read from each node. Include at least one negative control: show the same check DOES fail
when the ledger is deliberately corrupted in a scratch copy (then discard the copy).

**Files you own:** a new test under `node/tests/` (do not edit `x3lang_network_receipt.rs`) and any
new script it needs.

---

## Reporting back

```
FIXED:            (code changes, files)
TESTED:           (commands + the result they printed, including the negative control)
COMMIT:           (none — say what you would commit)
GATE LINE:        (as text, if one is needed)
REMAINING HARD BLOCKERS:
```

---

## Workstream G — a partition, not a crash

**Deliverable:** the goal's "Crash/restart/partition/recovery: all pass" has its crash
(`validator failure drill`, which kills and restarts validators) and its restart (the X3Lang
and lifecycle tests re-open a real database) but nothing exercises a **network partition**:
a validator that is alive, running and reachable to nobody.

Prove, on the built-in three-validator `local3` chain, in one test:

1. three validators connected and finalizing;
2. cut **one** validator off from the other two (it must keep running — do not kill it; the
   difference between "crashed" and "partitioned" is the whole point);
3. the remaining two keep authoring and finalizing — with three GRANDPA authorities, two is
   exactly the 2/3 threshold, so this is the case where a naive implementation stalls;
4. reconnect the isolated validator, and require it to converge: same finalized height and
   the same canonical hash as the other two, and it finalizes *new* blocks again rather
   than sitting at its stale height;
5. assert that while it was cut off it was genuinely behind (its best/finalized height was
   lower than the others') — otherwise step 3 proves nothing about partition behaviour.

How to cut it off: the chain is local, so the honest lever is the validator's own network —
stop forwarding its p2p traffic (`iptables`/`nft` on the loopback ports if permitted, or an
unshare/netns boundary), or drive `system_disconnect`/`system_removeReservedPeer` over its
RPC if the node exposes it. **Verify the cut actually happened** (peer counts drop on both
sides) before claiming the partition; a test that "partitions" without changing any peer
count is checking nothing. If no honest lever exists in this environment, say so with the
evidence and do not fake it.

**Files you own:** a NEW test under `node/tests/` (do not edit `x3lang_network_receipt.rs`)
and any NEW script it needs.

**Acceptance:** the command, its output, and the numbers — heights and hashes before,
during and after, plus the peer counts that show the cut and the heal.
