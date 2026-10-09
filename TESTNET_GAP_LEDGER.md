# TESTNET_GAP_LEDGER.md

Autonomous audit ledger. Live re-verified findings (2026-09-03/04). Prior report files in the
tree treated as untrusted until reproduced. Severity labels as head note.

## Open P0/P1 entries

| ID | Sev | Area | Description | Root cause | Evidence |
|----|-----|------|-------------|-----------|----------|
| GAP-CLI-1 | P1 | launch harness | **CLOSED 2026-09-22.** `scripts/testnet/x3_testnet_up.sh` still required `subkey` (not installed, not part of this repo), defaulted to the storage-raw Live spec the node refuses, and started nodes with `--unsafe-force-node-key-generation` (so a spec's bootNodes could never name a stable peer id). It is now a thin wrapper: it resolves a *plain* spec (building one with `build-x3-testnet-spec.py` if needed), refuses a raw one with a clear message, and delegates to `run-7-validators-local.sh`, which owns key insertion, the authority/bootnode preflight and stable per-node `--node-key` files. | One launcher to keep correct instead of three copies drifting. | Booted 4 validators through the wrapper on a generated 4-authority Live spec: all four finalizing, agreeing on `0x5e85c483…` at height 1000 and `0x58687622…` at height 1050; the wrapper refuses `deployment/chain-specs/x3-testnet-raw.json` with "raw Live spec; the node refuses to load one". |
| GAP-SPEC-1 | P0 | chain-spec generation | **CLOSED 2026-09-22.** The default path is now a generated *plain* Live spec: `build-x3-testnet-spec.py` derives fresh authorities, writes per-validator seeds + node keys, derives `bootNodes` from those node keys (a Live spec with none cannot start a node), asserts the written spec carries every entry, and prints the launch command; `run-7-validators-local.sh` refuses to start unless every session key is an authority in the spec *and* every node's peer id is one of its bootNodes. | Live raw genesis cannot satisfy the node-side structural validator. | `x3-testnet-plain.json carries all N derived bootnodes`; the wrapper's raw-spec refusal; the 4-validator boot above. |
| GAP-AUTH-1 | P0 | node authoring | **CORRECTED 2026-09-22 — the premise no longer holds.** File-only keystore injection *does* drive Aura on this binary: a single node started with `--validator --force-authoring`, the keystore files written by `inject-keystore.sh` (which now goes through `keys insert`), and **no** `X3_DEV_SEED`, authored from the first slot (head reached 9 in ~45 s). `X3_DEV_SEED` remains a convenience for local runs, not the only mechanism, and nothing about it needs changing. | The 2026-09-04 measurement predates the current keystore/CLI path. | Re-measured 2026-09-22; `scripts/testnet/run-fresh-validators.sh`'s comment claiming the opposite is corrected in place. |

## Confirmed solid (for the record)
- Runtime GRANDPA consensus + tx finality correctness IS clean WHEN a connected majority forms: 7/7 identical finalized heads observed (net4), 2000/2000 remarks finalized at 110.6 finTPS / 0 lost, canonical head identical on all 7 under load.
- Substrate runtime mechanics (aura/grandpa/session/txpool/rpc/spec build) operate correctly; the port/runtime/spec/CLI bring-up path is fully mapped.
- Reserved full mesh keeps runtime consensus clean WITHOUT a fragile local P2P bootstrap: 8/8 cold-starts and 7/7 single-loss survival on one host (see Resolved below).

## Resolved — RESERVED FULL-MESH (deterministic node-keys) — 2026-09-04

Fixes GAP-P2P-1 / GAP-MESH-1 / GAP-CONSENSUS-REPRO-1 / GAP-BOOT-1 with `scripts/testnet/run-mesh.py`:
each validator gets a deterministic `--node-key` (ed25519 secret = 0x…000N); each PeerId is
derived from the node ITSELF (`system_localPeerId`, one throwaway sequential boot per key — ground
truth, reproducible because the key is fixed); then all 7 are cold-started with a RESERVED FULL
MESH (every node passes `--reserved-nodes /ip4/127.0.0.1/tcp/<P>/p2p/<PeerId>` for all OTHER nodes).
No sparse star, no bootnode race, no single point of failure. Spec/tables: TESTNET_VERIFICATION.md
§ RESERVED FULL-MESH.

Closed on loopback-host empirical proof (single 127.0.0.1 host):
- GAP-CONSENSUS-REPRO-1 (cold-start reliability): 8/8 fresh concurrent cold-starts each produced
exactly ONE GRANDPA-finalized head across all 7 — `run-mesh.py cycles --count 7 --cycles 8`.
- GAP-P2P-1 (single-loss partition): killing ANY one validator leaves the other 6 GRANDPA-finalizing
ONE chain (7/7 victims) — `run-mesh.py kills --count 7` (was: one leaf loss split a majority into
two finalized branches).
- GAP-MESH-1 (reserved wiring): derived `/p2p/<PeerId>` reserved addresses accepted (no more 'Peer
id is missing'); nodes reach peers=6/6.
- GAP-BOOT-1 (boot-order race): simultaneous reserved starts converge — no solo-lead ordering needed.

## Status update (2026-09-22)
- GAP-CLI-1 CLOSED, GAP-SPEC-1 CLOSED, GAP-AUTH-1 CORRECTED (table above) — the bring-up, spec and
authoring paths are all exercised end to end today: build a plain Live spec with fresh authorities and
derived bootnodes, launch it, get finality and agreement, kill and restart validators, rotate nothing
yet (see the note on key rotation below).
- **What is still open is not a harness gap: nothing is deployed.** `rpc.testnet.x3-chain.io`,
`faucet.testnet.x3-chain.io` and `bootnode.testnet.x3-chain.io` do not resolve (checked 2026-09-22 from
a host whose DNS reaches github.com), `testnet-deploy.yml` has never run (`gh run list --workflow` is
empty), every step of `docs/reports/TESTNET_DEPLOYMENT_CHECKLIST.md` is unchecked, and the only bootnode
list in the repository (`deployment/keys/bootnode-info.txt`) is three loopback addresses. All of the
local evidence in this ledger is loopback evidence.
- Key rotation is now wired end to end: the on-chain `pallet_x3_custody` registry
  (`ValidatorKeyRegistry` + `KeyRotationSchedule`) is the single source of truth, the node's
  `validator rotate` command reads it and submits `session.setKeys`, and the former in-memory
  `node/src/authority.rs` rotation manager (which had no caller) was deleted rather than left as a
  parallel schedule. What remains open is the *live* proof: a rotation on a running 3–4 validator
  network with finality held across it, which is independent of the topology work.

## GAP disposition (2026-09-04) — harness gaps are PATTERN-CLOSED, not source defects
Re-examined after SEC-v1 purge + memory-search fix. Honest re-classification of the three open
bring-up gaps:
- GAP-SPEC-1 (P0 "stale raw spec invalid"): RESOLVED in practice — the active launcher
  (scripts/testnet/run-mesh.py + run-7-validators-local.sh) uses the known-good PLAIN spec
deployment/chain-specs/fresh/x3-testnet-plain.json (Aura=7, Grandpa=7, matches DEV_SEEDS), never
the stale storage-raw Live file. The raw-file fallback path (GAP-SPEC-1's concern) is only hit if
an operator points at the stale spec, which the corrected launcher's preflight rejects. Not a
source defect; a harness/spec-selection fixed on the proven path.
- GAP-AUTH-1 (P0 "file-only keystore doesn't author"): WORKS-AS-DESIGNED, not a source bug to fix.
  Code trace confirms: sc_consensus_aura starts from in-process keystore; insert path is
  maybe_insert_dev_keys (node/src/service.rs:353) gated by X3_DEV_SEED (any chain) or --dev
  (Alice). X3_DEV_SEED programmatic sr25519(Aura)+ed25519(Grandpa) insert is the SECURE, explicit,
  proven authoring path (7/7 finality provenance). Adding auto-pickup of file keystores would be a
  silent-key-acquisition security change (forbidden per AGENTS: silent fallbacks in security code)
  with real regression risk and no reliability payoff — mesh convergence+survival is already
  proven. Deferred indefinitely as an optional stock-substrate ergonomic enhancement, NOT required.
- GAP-CLI-1 (P0 stale flags in x3_testnet_up.sh): FIXED in place pre-SEC-v1; x3_testnet_up.sh
  updated to current binary surface; the canonical launchers (run-mesh.py/run-fresh-mesh.py/
  run-7-validators-local.sh) all use `--validator --force-authoring --allow-private-ip` + X3_DEV_SEED
  and reject stale flags. Re-verified bash -n clean (2026-09-04).
NET: no remaining open source-code defect blocks multi-validator reliability on the proven path.
Only cross-host network re-proof (real hosts/NAT/latency) remains genuinely unproven on this box.

- Reserved-mesh convergence + single-loss survival is PROVEN on a SINGLE loopback host. Public
cross-host readiness should re-prove on real network paths (multi-host/NAT/latency) and stable
on-disk node keys; the P2P-structure root cause is deterministically addressed here, but loopback
does not exercise real-world network contention.

## NEW SEC-v1 (integrity) — testnet seeds leaked into git history — 2026-09-04

`build-x3-testnet-spec.py` writes each validator's plaintext Aura/Grandpa authoring seed to
BOTH the gitignored `validator-N.suri` (per-key, safe) AND aggregate `validator-keys/suris.txt`.
suris.txt — all 7 live authoring seeds — was committed in d594af8f despite fresh/.gitignore's
"NEVER commit" policy, that commit's own "Key SURIs excluded" note, and the file's own
"not committed" header.
ROOT CAUSE: fresh/.gitignore used root-relative paths (`deployment/chain-specs/fresh/...`)
but lives IN fresh/, so only the accidental `*.suri` glob matched; the dir and `.node-key-*`
rules never applied from that base, letting suris.txt through.
FIXED (non-destructive, commit cb1452e3): rewrote patterns relative to fresh/; untracked
suris.txt (working copy kept, mode 0600, now gitignored).
STILL OPEN: d594af8f remains in master history with the seeds — full purge needs a repo
history rewrite (filter-repo), deferred pending operator decision on audit-trail preservation.
Lesson: audit .gitignore path bases (nested dirs resolve relative to the .gitignore location),
and verify with `git check-ignore -v` + `git ls-files` not just gitignore presence.

## SEC-v1 RESOLVED — full history purge — 2026-09-04 (completed, operator "do what's best")
Committed the history rewrite with git filter-branch --index-filter removing
validator-keys/suris.txt from all commits, then deleted refs/original/*, the refs/codex
checkpoint that pinned the blob, orphaned stash ref, expired reflog, and gc --prune=now.
VERIFIED triple-clean: (1) no suris.txt path reachable from any ref; (2) secret blob
dab0e4e3... physically absent from object db (git cat-file -e fails); (3) deep scan of every
remaining object found no seed hex (0xe8e93d...). History intact at 37 commits; all files
present at HEAD; working tree clean. NOTE: cb1452e3/d594af8f hashes were REWRITTEN — use the
new hashes (purge commit ≈207fffeb, ledger commit a063faeb), not the pre-rewrite ones.
A pre-purge full-history bundle was written to /tmp/xxxstar-sec-histry-prebackup-*.bundle
(CONTAINS THE OLD SECRET — delete it; do not ship/share). Lesson: after any rewrite the
secret may survive in refs/* (refs/original, codex checkpoints) and reflog — must delete
all pinning refs + expire reflog + gc --prune=now, then cat-file --batch-all-objects verify.

Working notes: `.testnet-audit/`; evidence: `TESTNET_VERIFICATION.md`; mesh run logs /tmp/x3-mesh-*.

## GAP-BTC-SPV-ROOT — closed in code (spec_version 14), unset on every chain — 2026-09-22

The standing record said "the header chain the SPV check reads has no trusted bootstrap".
Measured against the code, it was worse than "no bootstrap": `submit_btc_header` accepted any
header whose `height` was 0 with **no parent at all**, and `nBits` is a field the submitter
writes — so a chain could be started anywhere, for the price of one hash. `submit_btc_proof`
was the second door: it inserted the header its argument carried into `BtcHeaders` with **no
proof-of-work check whatsoever**, so a party to an intent could choose the header whose merkle
root paid them out. Both are closed:

- **New call `anchor_btc_checkpoint` (root, call_index 34).** Commits this chain to
  "Bitcoin block H has hash X". `BtcCheckpoints` is write-once per height: a later call
  offering a different hash for the same height is refused (`BtcCheckpointConflict`), so an
  anchored height cannot be re-pointed at another branch. The event publishes the commitment
  so anyone can check it against a Bitcoin node.
- **`powLimit` is enforced** (`BtcPoWLimitBits`; mainnet/testnet `0x1d00ffff`, dev `0x207fffff`).
  A target easier than the network's limit is refused, which is what stops an anchor (or an
  extension) being mined in one hash.
- **One admission path** (`btc_admit_header`). Heights are derived from the parent link, not
  read from the header; the parent must be on an anchored chain; the target must be copied
  verbatim off a retarget boundary and move by at most 4x on one; the timestamp must postdate
  the median of up to 11 ancestors (Bitcoin's median-time-past).
- **SPV evidence must be anchored.** Both entry points (`submit_btc_proof` and the external
  `verify_proof` BTC path) require the header to be on an anchored chain, at the height the
  chain derived. `confirmations` is computed from that height rather than the caller's claim.

Eight negative controls pin this, including the one that matters most:
`the_raw_spv_verifier_accepts_the_fixture_the_pallet_refuses` shows the same bytes being
accepted by the raw SPV verifier (the pre-fix behaviour) and refused by the pallet, then
accepted once the header is genuinely anchored — so the change is the trust question, not the
merkle math.

**Still open, and it is not a code gap:** no chain in the repo has anchored a checkpoint, so
the BTC path is fail-closed until an operator does. `anchor_btc_checkpoint` takes a root
origin; the ceremony for choosing and publishing that hash is an operator runbook item, and
nothing pushes headers after the anchor (a bonded header relayer is not written).

## GAP-SOAK-2H — a two-hour run fails, and the peer set is what turns a lag into a partition — 2026-09-22

The twenty-minute soak passed. The two-hour run did not:

```
[soak] FAIL: rpc 12046 has not finalized since 1790099493 (27677 at height)
```

Every node's log shows the same sequence, and it is not subtle. Four minutes in, the first
`Timeout while trying to acquire a write lock for the shared trie cache` (105–419 per node over
two hours); then `State already discarded` and `block has an unknown parent` (14–252 per node);
then `Creating inherent data took more time than we had left for slot …` — a missed Aura slot.
An hour in, node 3 is banned for `Same block request multiple times`, which is what a validator
that has fallen behind does. It loses its peers, its view diverges from finality
(`Potential long-range attack: block not in finalized chain`), and it re-finalises *backwards*
(`Re-finalized block #… (27111) … current best finalized is #27136`). At the end all four nodes sit
at one height with node 3 at zero peers.

The trigger was an over-subscribed box (`load average 55–62`: four debug nodes at 1.3–1.6 GiB each,
plus other agents' networks and builds). The **defect candidate** is the response: 10–16 peer bans
per node means the peer set punishes slowness and degrades itself instead of healing, so a lag
becomes a partition. That is worth chasing regardless of what the machine was doing.

Two-hour run, 4 validators, one host — liveness, not safety: no node ever finalised two
conflicting chains. Evidence, with the log excerpts and counts: `.ai/reports/soak-2h-failure-20260922.md`.

**TICKET-094 — a lagging validator must not be banned out of the network.** Reproduce on an idle
box with the same launcher for two hours and confirm the run passes; if it does, re-run it under
deliberate CPU contention (`stress-ng --cpu $(nproc)`) to reproduce deterministically. Then look at
the repeated-block-request reputation path in the sync protocol, where a peer that asks for the same
block while behind should be slowed rather than disconnected, and at what holds the trie-cache write
lock during block import. Acceptance: on a contended box, a validator that falls behind keeps at
least one peer, catches up, and finality does not stall.

## GAP-BTC-ANCHOR-GENESIS — a chain can be born anchored, and `--chain dev` was not a dev runtime — 2026-09-22

The trust root landed in code earlier today (`anchor_btc_checkpoint`, spec_version 14) and left an
awkward bring-up story: the BTC path is fail-closed until a checkpoint exists, and creating one was
a root call — a manual step on a testnet, doable only by whoever holds the key first. Closed by
making the checkpoint part of the **spec**:

- `X3SettlementEngine::GenesisConfig` gained `btc_checkpoints`. Each entry is validated as genesis
  is built — proof of work under this network's `powLimit`, no two entries at one height — and then
  pins `(height, hash)` in `BtcCheckpoints` and **admits the header** (`BtcHeaders`,
  `BtcHeaderMetaStore { height, anchored: true }`, `BtcBestHeight`). A bad entry makes the node
  refuse to start rather than launch a chain whose root of trust is a lie.
- `X3_BTC_CHECKPOINTS="<80-byte header hex>@<height>[,<header>@<height>…]"` is how a spec gets one;
  `scripts/testnet/build-x3-testnet-spec.py` already forwards the environment, and the generated
  spec carries `btcCheckpoints` in plain JSON beside the rest of the genesis. Verified: a 3-validator
  testnet spec built with a captured Bitcoin header really does contain it.
- **Proof it works on a live chain:** `scripts/testnet/btc-checkpoint-genesis-drill.sh` (new
  `--testnet` gate) pins a real regtest header, boots a node, reads `BtcCheckpoints`,
  `BtcHeaderMetaStore` and `BtcBestHeight` back out of running storage over RPC, requires the chain
  to keep authoring blocks, and requires a spec whose checkpoint is not a mined header to be
  refused. 12/12 checks pass. The storage keys are computed by a reimplemented `twox` that is
  checked against Substrate's known `twox_128("System")` prefix before the drill uses it, because a
  wrong key looks exactly like an empty value.

**The discovery on the way:** the node had no `dev` feature. `--chain dev` built a spec called dev,
ran the **default** runtime — no `Sudo`, and `powLimit` at mainnet's `0x1d00ffff` — and refused
every regtest-difficulty header. So every local "dev" chain in this repository has been the default
runtime wearing a dev spec, and any dev-only behaviour (root calls, regtest proof of work) was
unavailable by construction. Fixed with `node` feature `dev = ["x3-chain-runtime/dev"]` and the
matching `sudo` genesis field; **build a dev chain with `cargo build -p x3-chain-node --features
dev`**. This is what the first version of the drill caught, by failing.

**Still open:** no public network has a checkpoint pinned, nothing pushes headers after the anchor
(a bonded header relayer does not exist), and the header source is still an operator copying a hash
by hand. Those are TICKET-095.

### TICKET-095 progress — the receiving end exists (2026-09-22)

`x3SettlementEngine.submitBtcHeaders` (call_index 35, spec_version 17) accepts up to 100 headers per
call from `Config::BtcHeaderOrigin` — `EnsureRoot` in this runtime, so root-only and unchanged, with
a network able to point it at a relayer account, a multisig or governance. The batch is atomic, and
every header still goes through the same admission rules as one-at-a-time submission. Four tests
cover the origin (an ordinary account refused, root and a designated relayer accepted), the
rollback of a batch that fails partway, and the size bound.

`scripts/btc/push-headers.mjs` is the sender: it reads headers from a local Bitcoin node, checks
the chain of them itself, and submits the batch (directly, or through `sudo` on a dev chain).


### TICKET-095 progress — a real header push, end to end (2026-09-23)

A live dev chain, **born anchored on real Bitcoin regtest header 119**, followed Bitcoin to **125**
through six headers pushed from a running Bitcoin Core v28.1.0 node:

```
[push] on-chain btcBestHeight before: 119
[push] block 120 … through … block 125, each included in a block
[push] on-chain btcBestHeight after:  125
[push] header meta at 125: {"height":"125","anchored":true}
```

and the negative control held — pushing only header 127, whose parent was never admitted, was refused
with `x3SettlementEngine.BtcParentMissing`.

**The drill found a real defect, and it was mine.** The first attempt refused header 120 with
`BtcTimestampTooOld`, and the header was legitimate: Bitcoin's median-time-past rule is the median of
the previous **eleven** blocks, and the pallet was padding that median with the one ancestor it had —
which is a *higher* number, so it required a header above an anchor to postdate the anchor. A rule
stricter than the chain it follows refuses real headers, which is a liveness bug in the relayer path.
`btc_median_time_past` now returns `Option<u32>` (`None` until eleven ancestors are stored; the check
is skipped, not made stricter) and two tests state both halves. `spec_version` 17 → 18, re-attested.

Two sender bugs were fixed with it: `--from-height`/`--to-height` parsing into keys the script never
read, and the fact that **`pallet_sudo` reports the inner call's failure in a `Sudid` event while the
outer extrinsic succeeds** — a refused header looked exactly like an accepted one until the script
read that event. Evidence: `.ai/reports/btc-header-push-drill-20260923.md`.

**The session is now a drill:** `scripts/testnet/btc-header-push-drill.sh` (gate entry
`btc header push`) starts a private regtest bitcoind, mines, pins the oldest captured header as the
checkpoint, gives the dev spec a sudo account, boots the chain, pushes six real headers in order,
requires `btcBestHeight` to reach the tip, and requires a gapped push to be refused with
`BtcParentMissing`. **5/5** — and it **skips loudly** without a Bitcoin Core install rather than
passing quietly.

**The sender runs unattended now** (2026-09-23): `push-headers.mjs --loop --cursor <file>` keeps the
chain at the local Bitcoin node's tip — batch pushes, a cursor written atomically only after a range
is included, catch-up after restarts, resume from the cursor, and a **stop** rather than a skip when a
range is refused (skipping a header would leave a permanent gap in the chain's view of Bitcoin). The
drill requires all of it: **6/6**, including "the relay loop follows new blocks unattended".

**What is still missing:** a relayer running against a *public* network; a checkpoint pinned on one;
a push origin that is not root (`BtcHeaderOrigin = EnsureRoot` here by design, and a dev spec ships
`sudo.key = null`, so an operator has to name it); and a bond, without which one relayer is a single
point of failure and can withhold.

**What is still missing, in order:** (1) one working signing path — nothing in this repository can
sign and submit an extrinsic on this host, because no `node_modules` is installed anywhere and the
repo's JS submission scripts all depend on `@polkadot/api` (`npm ci` in `packages/ts-sdk` is the
first step, and the same install makes `scripts/testnet/load-remarks-tps.js` runnable, which is the
TPS harness X3-OPS-005 claims); (2) an end-to-end run of the sender against a dev chain with headers
from the local regtest node; (3) a bond and slashing for withholding, without which one relayer is
a single point of failure; (4) pinning a real checkpoint on a public network.

## GAP-ATOMIC-AUTH — bundle finalization is unauthorized, and its finality gate is self-satisfying — 2026-09-22

Found by reading `pallets/x3-atomic-kernel` after the soak's log showed
`failed to anchor GRANDPA cert for block 149: Transaction pool error: Transaction temporarily Banned`
within two minutes of boot. The log line led to the anchor, and the anchor led to the authorization
model. Evidence and code references: `.ai/reports/atomic-kernel-finalization-authorization-20260922.md`.

Two unsigned calls, two false claims:

* `record_flash_finality_anchor` (unsigned) stores **the first non-zero cert for a height**, with no
  binding to the block, to a certificate, or to an authority. `do_finalize_bundle` then accepts a
  finalization when `finality_cert == FinalityCertAnchors[block]` — a comparison of the caller's
  input against the caller's earlier input. The comment saying this "prevent[s] submission of
  fabricated cert hashes" is the opposite of what the code does.
* `submit_finalization_result` (unsigned) reads only the bundle's status and that an executor is
  assigned — never who is calling, because an unsigned call has no caller. The `ValidateUnsigned`
  comment claiming this "prevents anonymous peers from finalizing bundles they never claimed" is
  true only of *assignment*. Any bundle in `Executing` can be finalized by anyone, which marks it
  `Finalized` with a proof nobody produced and permanently blocks the honest result
  (`ProofAlreadyExists`).

The dispatch path is also weaker than the validation path: `do_finalize_bundle` accepts
`BundleStatus::Pending`, which `ValidateUnsigned` rejects. A block author includes unsigned
extrinsics without the pool's validation, so the weaker check is the one that holds.

No fund path in this repository releases value on `BundleFinalized`, so the impact today is bundle
integrity and liveness rather than a drain — but it is a P0 core-pallet claim and `X3-RT-002` drops
from 40 to 25 on it. There is also no test for `submit_finalization_result` anywhere.

**Corrected 2026-09-23, and the tests now exist.** The claim that "the dispatch path is weaker than the
validation path, so a bundle that was never assigned can be finalized" was **wrong**: `executor` is set
only by `assign_bundle_executor` (which also sets `Executing`), and `verify_bundle_consistency` requires
it, so a `Pending` bundle was already refused — two checks later than the documented rule, which is
what made it confusing rather than exploitable. `do_finalize_bundle` now requires `Executing`
explicitly; rollback still accepts `Pending`, because cancelling an unclaimed bundle is the submitter's
right (a pre-existing test caught my first attempt at the edit, which hit the rollback check).

Five tests now cover the entry point, which had none: refusal of a bundle nobody was assigned to,
refusal of a certificate the chain never anchored, the receipt-root commitment (wrong root refused,
right root accepted, bundle `Finalized`, PoAE proof stored), finalization happening once, and
`an_unsigned_finalization_cannot_be_attributed_to_the_executor` — the hole asserted *as* today's
behaviour so that closing it must change the test.

**What remains for TICKET-097 is exactly the authorization model**, and it is the owner's call: the
anchor is unsigned and stores the first non-zero certificate for a height, the finalization is
unsigned, so a caller can plant the value it is about to be checked against. Either the result gains a
signature from the assigned executor (or a committee quorum), or the chain gets a real finality source
(a signed finalized-head tracker, or a verified GRANDPA justification). Both were judged too large to
guess at without that decision.

**TICKET-097 — CLOSED 2026-09-23.** The unsigned finalization extrinsic was removed rather than
signed. It had no producer: `sp_io::offchain::local_storage_set` appears once in this repository
(`node/src/service.rs`, writing `x3ff:`), so neither the pallet's `x3fin:` record nor the settlement
engine's `x3settle:` marker was ever written by anything, on any chain, in any feature configuration.
What was left was an `ensure_none` call whose certificate check compared the caller's input with the
caller's own earlier input. Finalization is now only `finalize_atomic_bundle` (`X3LangOrigin`, a
genesis-named account since spec 19) and `finalize_with_settlement` (`SettlementOrigin`) — both signed
— and the four behavioural tests that lived on the removed call now drive the signed one, plus a new
one showing `RuntimeOrigin::none()` cannot reach finalization at all. Evidence:
`.ai/reports/unsigned-finalization-removed-20260923.md`. `spec_version` 20.

Related, smaller, and fixed by the same reading: `node/src/service.rs`'s `run_grandpa_finality_anchor`
logs `cert anchored for block N` even when the submit failed, and advances its cursor before the
submit, so a rejected anchor for one block is never retried. Its doc comment claims it writes
off-chain storage; it does not. TICKET-098.

## GAP-GPU-CLAIMS — a release note for a release that does not exist — 2026-09-22

`docs/testnet-config/RELEASE-NOTES.md` announced `solana-gpu-validator-v1.0.tar.gz` (269 MB, CUDA
kernels), and claimed an unverified "2.75M TPS in lab, 1-5M TPS on testnet" with an unverified
"minimum 100k TPS on Solana testnet" and "825k signatures/second per GPU". None of it is in this
repository: no such artifact,
no `start-validator.sh`, no chain-level TPS measurement at all — and the one number that *is*
traceable ("PoH GPU acceleration: 1.55M hashes/second") is the repository's own **CPU** sha256 rate
from `tps_benchmark_results.json`, relabelled as GPU acceleration. The repository's own
`gpu_tps_benchmark_results.json` records `ed25519 = 113,759/s` and `secp256k1 = 89,659/s`, not 825k.

The kernels, the crates and the soak harness (`scripts/gpu/run_swarm_tps_soak_matrix.sh`) are real;
the numbers are not traceable, and the document's name gave them authority. Rewritten to say what
exists, what is measured and what is a target: `.ai/reports/gpu-claims-hygiene-20260922.md`. Row
`X3-CLAIM-001` moved 10/5/5 → 55/25/35.

**TICKET-099 — four result files with no producer.** `infra-structure/validator/benchmarks/tps_benchmark_results.json`,
`…/gpu_tps_benchmark_results.json`, `docs/testnet-config/day10-validation-results.json` and
`…/day10-hotfix-results.json` assert measurements (including CPU/GPU checksum parity and three
"issues fixed") that **no script, crate or test in this repository writes**, and none records the
host, date, command or hardware. Decide per file: re-run it on documented hardware and record the
command beside the number, move it under an "archive, unverified" path with a header saying so, or
delete it. Acceptance: no result file in the repository claims a measurement whose producer cannot
be run.

## GAP-CI-GATES — `make guard` is not the gate set, and a nested lockfile is stale — 2026-09-22

Running the repository's own CI of record (`scripts/local-ci.sh --testnet`) on merged master, after
a change that had passed `make guard`, readiness consistency and the feature matrix:

```
format check                       FAIL   8
clippy workspace                   FAIL   864
nested workspaces                  FAIL   19
... 26 other gates PASS, including the new "btc checkpoint genesis" (180s)
```

Two of the three were that change's own: `cargo fmt --check` (import ordering and two wraps) and one
clippy lint (`s.len() % 2 == 0` where this workspace uses `is_multiple_of`). Both are fixed. The
lesson is mechanical and worth stating plainly: **`make guard` runs three guards (agent, stub,
test-cheat); `scripts/local-ci.sh` is the set that includes `cargo fmt --check` and
`cargo clippy --workspace --all-targets -- -D warnings`.** A change is not verified because the
guards pass.

**TICKET-096 — CLOSED. `crates/x3-sidecar/Cargo.lock` was out of date.** `local-ci`'s
nested-workspaces gate failed with `error: the lock file crates/x3-sidecar/Cargo.lock needs to be
updated but --locked was passed`. Not new and not related to the change above: the nested
workspace's lockfile had drifted from its manifest. Regenerated with
`cargo update --offline --workspace` in `crates/x3-sidecar` (the registry cache had everything;
nothing was downloaded) and verified with the gate's own command:
`SKIP_WASM_BUILD=1 cargo check --locked --all-targets` → finished clean in 2m20s.

## GAP-SOAK-2H — RESOLVED (load), and a memory bound that was measuring a cache — 2026-09-22

The two-hour soak failed earlier today at load 55–62 (peers lost, finality stalled, 10–16 peer bans
per node). Re-run unchanged on a **quiet box** (load 5–12):

```
[soak] PASS: all 4 validators agree on the same chain at heights 9089, 18178, 27267
[soak]   rpc 12044: height 259 -> 36351 (+36092), peers 3 (min 3), rss growth 1714.3 MiB
[soak] FAIL: a node grew 1760.4 MiB, above the 1024.0 MiB bound
```

**Consensus held for two hours**: one chain, agreement at three heights, +36,092 blocks per
validator, peers 3 throughout, **zero peer bans** (against 10–16), 4–5 trie-cache lock timeouts
(against 105–419), no stall beyond 60s. The earlier failure was the machine, and TICKET-094's first
question is answered: a starved validator falls behind, repeats one block request, and the peer set
bans it out — a feedback loop that begins with CPU contention, not with consensus.

The memory bound is real and separate, and it survives the quiet box. A controlled run with the
state cache disabled (`NODE_TRIE_CACHE_BYTES=0`, a passthrough added to the launcher in this
change) plateaus at **+227 MiB and flat**, against **+432 MiB and still climbing** at the same
15 minutes with the default cache. So the growth is dominated by `--trie-cache-size` filling to its
default, i.e. by *configured* memory, not by an unbounded leak.

Evidence and the arithmetic that is still unaccounted for: `.ai/reports/soak-2h-idle-20260922.md`.

**TICKET-100 — make the soak's memory rule mean something.** It fails on growth above 1 GiB, and a
node with a ≥1 GiB state cache crosses that by filling a cache the operator configured. Two runs
settle it: (a) two hours with the cache disabled — the pipeline margin over the same window; (b) two
hours with the **release** binary and default caches — the production footprint. Then the rule
becomes `configured cache budget + measured margin`, the harness reports which cache size it ran
with, and the operator runbook states a validator's memory budget explicitly. Acceptance: a
two-hour run of a release validator passes or fails on a number that is about the node, not about
its cache configuration.

**TICKET-100 — CLOSED (2026-09-22).** Both runs were made, and the answer was simpler than the
plan: the rule was measuring the cache, so the harness no longer runs the cache.

| run | binary | state cache | 2h RSS growth |
| --- | --- | --- | --- |
| idle box | debug | default (1 GiB) | 1,714–1,760 MiB |
| idle box | **release** | default (1 GiB) | **1,736–1,748 MiB** |
| controlled, 15 min | debug | `--trie-cache-size 0` | **+227 MiB, flat** |

Release and debug grow the same, which rules out build overhead; with the cache off the growth
stops. So `scripts/testnet/consensus-soak.sh` now **disables the state cache by default**
(`NODE_TRIE_CACHE_BYTES=0`), keeps the 1 GiB bound as a real leak detector, and prints the cache
size it ran with. The production figure is recorded for operators instead of gating on it:
**~1.75 GiB of growth, ~2.4 GiB RSS per validator over two hours with default caches**.

Acceptance met in the way that matters — a two-hour run now passes or fails on a number about the
node. What replaces it as the next memory question is a *longer* one: two hours still cannot
distinguish "cache filling" from "slow leak" for a node whose growth is dominated by a cache, which
is why the harness measures with the cache off. TICKET-100b: a 2-hour no-cache run to confirm the
pipeline margin over the full window (15 minutes gave +227 MiB).

## GAP-MATRIX-PROVENANCE — twenty-eight rows cited five PRs, none of them open — 2026-09-22

Every row whose `open_prs` field pointed at PR #129, #135, #162, #163 or #166 was checked against
GitHub:

| PR | state | rows citing it |
| --- | --- | --- |
| #129 | merged 2026-09-18 | X3-LANG-004/008/009/010 |
| #135 | **closed, never merged** | X3-LANG-001/002/003/005/006/007, X3-MEV-002/003/004/005/006 |
| #162 | merged 2026-09-13 | X3-XCHAIN-013/018/019/020/021 |
| #163 | merged 2026-09-17 | same five cross-chain rows |
| #166 | merged 2026-09-17 | X3-XCHAIN-010/014/015/016/017, X3-SEC-001/002/004 |

`feature_matrix.py` emits *"feature includes open PR work and is not master capability"* for any row
with `open_prs` set, so the matrix was telling a reader that seventeen shipped capabilities were not
on master — and that eleven rows rested on a PR that was closed without merging. Those warnings are
gone; the field now carries provenance instead of a stale intent.

What changed: `source = "master"` for the merged-PR rows (the code is on master — verified by
provenance for the cross-chain and security rows, and by reading the tree for the language rows),
`open_prs` dropped, and each row carries a dated note in `evidence` naming the PR and the merge
date. The scores in those rows were recorded while the work was still in a PR, so they are
**conservative**, not wrong.

Two rows were re-measured rather than re-labelled, because there was new evidence: the compiler
carries a `max_price_impact` policy and the VM enforces price-impact and MEV-leakage ceilings,
failing closed when the host reports nothing (`x3-lang/vm/tests/trading_execution.rs`). X3-MEV-003
30 → 45 and X3-MEV-006 35 → 45, with blockers that say what the evidence does and does not cover.

X3-MEV-002 (private transaction submission controls) is the one row set to `source = "research"`:
the only on-master candidate is `crates/confidential-gpu`'s `execute_private_tx`, which is private
*execution*, not private *submission*, so the capability is not established.

**TICKET-101 — re-measure the seventeen rows whose scores predate their merge.** (This ledger first
said thirteen; recounting the citations gives seventeen — 4 language, 10 cross-chain, 3 security.)
Provenance is fixed; measurement is not. Each row's `evidence` now names its PR and merge date, so
this is a bounded audit: read the merged PR, re-derive `implemented`/`tested`/`mainnet_ready` from
what is on master today, and lower as readily as raise. Acceptance: no row cites a PR that is
closed, and every row's score is dated relative to the code it describes.

### First pass over TICKET-101 — 2026-09-22

**Five cross-chain rows now name their tests**, which is what their own `test_evidence` asked for
("exact named tests should be added before score increase"), and `tested` went 82 → 88 on each:

| row | tests named on master |
| --- | --- |
| X3-XCHAIN-010 semantic idempotency | `exact_rebind_is_idempotent`, `identical_fast_lock_replay_is_successful_noop_but_conflict_fails`, `duplicate_slow_claim_and_refund_completion_are_idempotent` |
| X3-XCHAIN-014 coordinator restart recovery | `distributed_fence_survives_authority_restart`, `fast_claim_retry_survives_coordinator_restart` |
| X3-XCHAIN-016 conflicting retry rejection | `conflicting_concurrent_lock_observations_yield_one_winner_one_conflict`, `terminal_phase_conflicting_with_canonical_evidence_halts`, `identical_fast_lock_replay_is_successful_noop_but_conflict_fails` |
| X3-XCHAIN-017 cross-session secret ownership | `distributed_secret_registry_allows_same_session_retry_only` |
| X3-XCHAIN-019 secret-release proof reuse | `release_does_not_reuse_fencing_epoch` |

(`crates/cross-vm-coordinator` holds 133 `#[test]` functions in all.)

**The three security rows were re-measured against reality, and one of their blockers was false.**
`X3-SEC-004` carried "srtool hardening is on #166 branch, not master yet" — #166 merged 2026-09-17
and the reproducible-build path is on master. What is actually missing is that the production gate
has never completed a run: the hosted `production-gate` workflow has **five runs, all cancelled,
longest 1h34m, none green**, and `scripts/mainnet_release_gate.py` has not been run end to end on a
release candidate either. `X3-SEC-004` mainnet_ready 72 → 62 on that; `X3-SEC-001`/`002` 60 → 65 with
the local evidence written down (two from-scratch srtool builds agreeing, five times over five
revisions this session, plus the freshness gate firing on `local-ci`).

**Second pass, 2026-09-23.** Checked the eight that were left: X3-XCHAIN-020 has the
refund-rejection family named for it (`test_double_refund_rejected`, `test_claim_after_refund_rejected`,
`test_stateful_*`, `test_evm_unauthorized_refund_rejected`) and now cites them; X3-XCHAIN-015 has
persistence round-trip tests and no fault injection, so its blocker stands and now names them. The
other six were checked and **nothing named for them exists on master**: no `test_*domain/binding/
firewall/permit/reuse/secret*` in `crates/x3-atomic-swap` (so X3-XCHAIN-013/018/021's `tested=82` is
not backed by tests of those behaviours), and no `test_*envelope/decode/route*` in
`crates/x3-integration` (whose code exists — `compiler_bridge.rs`, `executor.rs`, `hostcalls.rs` — so
X3-LANG-004/009/010's low scores are honest and their blockers accurate). Those scores stay until
someone measures them; the next pass should start from the test names, not from the numbers.

**Still unverified, deliberately untouched:** X3-LANG-004/009/010 and X3-XCHAIN-013/015/018/020/021
have no named test or file I could confirm on master this pass. Their scores stand until someone
does, which is the point of the ticket.

### TICKET-101 — CLOSED 2026-09-23, after three passes

**Every row that was left open has now been measured against master.** Pass 1: five cross-chain rows gained their tests (82 → 88) and the three security rows were re-measured against reality, one blocker being false. Pass 2: X3-XCHAIN-020 gained its refund-rejection family and X3-XCHAIN-015's blocker stood and named its tests. Pass 3 (below): the six remaining rows — and the second pass's verdict on them was wrong. Scores stay where measurement put them; what changed is that each one now names the tests that back it, which the matrix gate checks.

### Third pass over TICKET-101 — 2026-09-23 (the six that were left)

**The second pass was wrong about the tests, and the reason is worth keeping: it searched for names
beginning with `test_`.** The behaviours are covered, by sentence-named tests, and every name below is
now in its row's `required_tests` — which `feature_matrix.py check` resolves against the row's own
paths, so a later pass cannot claim absence without the gate contradicting it:

| row | tests on master |
| --- | --- |
| X3-XCHAIN-013 Secret-release firewall | `authorizes_only_when_all_required_domains_are_finalized`, `rejects_reused_proof_across_required_domains`, `rejects_refunded_destination`, `rejects_wrong_preimage`, `rejects_stale_intent_hash_and_terminal_status` |
| X3-XCHAIN-018 Secret-release domain binding | `rejects_wrong_chain_or_vm_binding`, `rejects_missing_destination_domain`, `rejects_refunded_destination`, `proof_set_rejects_duplicate_domain_operation` |
| X3-XCHAIN-021 Secret-release tx/block binding | `rejects_finality_for_different_transaction_or_block`, `rejects_wrong_finality_domain_transaction_and_block`, `rejects_included_but_not_finalized_transaction`, `rejects_insufficient_confirmations` |
| X3-LANG-004 Bytecode routing | `compile_source_emits_runtime_loadable_bytecode`, `compile_source_rejects_invalid_source`, `test_execute_returns_42`, `test_gas_exhausted` |
| X3-LANG-009 Authenticated decoder | `test_invalid_magic`, `test_parse_minimal_module` |
| X3-LANG-010 Versioned envelope | `test_invalid_magic`, `test_execute_returns_42`, `test_gas_exhausted` |

`crates/x3-atomic-swap/src/secret_release.rs` alone holds eleven tests, including the firewall and the
permit the row's old blocker said had to "land … together". Scores are unchanged: the evidence now
supports `tested`, and nothing here justifies raising `mainnet_ready` past the live-proof gap.

**Two blockers replaced, two sharpened, and the sharpening found something the numbers could not say.**

**TICKET-108 — CLOSED 2026-09-23.** The envelope is one format with one checksum now, and every
reader checks it. What measuring it found was worse than the ticket: there were **three**
implementations of the header's checksum — the writer's wrapping multiply-and-add
(`BytecodeModule::to_bytes`), a real CRC32 in `bc_format_helpers`' fixtures, and a second CRC32 in
`x3-vm`'s verifier — and the verifier's was the only one that compared anything. Because it compared
CRC32 against a value the writer had produced with a different algorithm, **every module a compiler
produced was refused by `Verifier::verify_module_bytes`**, and a module whose checksum field was zero
skipped the check entirely. Reproduced before the fix, with a test that is now the regression test:

```
a_written_module_passes_its_own_checksum ... FAILED   (ChecksumMismatch, compiler output vs verifier)
```

Fixed by defining the header once, in `x3-common::bytecode` — magic, header length, the packed version
bounds, `checksum`, and the two version predicates — and pointing the writer, `bc_format_helpers`,
`BytecodeModule::from_bytes`, `mini_x3` (the no-std decoder) and the verifier at it. The verifier's own
CRC32 block is gone: `from_bytes` verifies the checksum always, so one check remains. `mini_x3` now
reads the twenty header bytes it used to skip and refuses a future version, an unsatisfiable
`min_version` and a checksum mismatch; two hand-assembled fixtures in `x3-vm` and one in
`x3-integration` were writing a zero checksum and are stamped properly now. Tests:
`a_written_module_passes_its_own_checksum`, `a_corrupted_body_fails_the_checksum` (verifier),
`a_corrupted_body_fails_the_checksum` (backend), `the_shared_version_predicates_agree_with_version_info`,
`test_a_corrupted_body_is_rejected`, `test_a_future_format_version_is_rejected`,
`test_a_module_requiring_a_newer_loader_is_rejected`, plus the shared module's own two. Evidence:
`.ai/reports/bytecode-envelope-20260923.md`.

**TICKET-109 — crates declare `no_std` and cannot build without `std`, from one pre-existing root
cause.** Measured at `cc19883faf` — the revision *before* the envelope commit — so this is not that
change: `cargo check -p x3-common --no-default-features` fails there with

```
error[E0277]: the trait bound `String: serde::Serialize` is not satisfied
   --> crates/x3-common/src/lib.rs:44:35
error[E0277]: the trait bound `String: serde::Deserialize<'de>` is not satisfied
   --> crates/x3-common/src/lib.rs:48:12
```

— a `String`-carrying enum with serde derives, in a crate that turns `std` off: `serde`'s `alloc`
feature is not enabled on that path. Every crate that depends on `x3-common` with
`default-features = false` inherits it, which is why `bash scripts/check-no-default-features.sh` is
red (its `KNOWN_UNBUILDABLE` list is deliberately empty), and `x3-chain-runtime` is one of them.
Note what this is *not*: the runtime's **WASM** build works, because crates in that graph enable
`serde` with `alloc`; it is the isolated no-default-features configuration that cannot build, which is
exactly what the gate exists to catch and what nothing in the default suite runs.

**A correction to this ticket's first filing.** It attributed the failures to `x3-common::signing`
needing `std`/`alloc`/`full_crypto`. That was true for about twenty minutes on 2026-09-24 and it was
**this agent's bug**, not the repository's: inserting the shareable `bytecode` module above
`#[cfg(feature = "std")] pub mod signing;` moved the attribute onto the inserted module, un-gating
`signing` for no-std builds — which also broke the runtime's WASM build in srtool, the check that
caught it. The fix restores the attribute and leaves `bytecode` ungated (it is `no_std`-safe by
construction: constants and a loop). The lesson is in the memory file: a scripted insertion anchored on
a bare `pub mod X;` moves whatever attribute precedes it.

What was left after the fix, and is the actual ticket:

```
no-default-features: 7 of 87 cannot build without default features and is **not** on the known list:
  pallet-atomic-trade-engine  pallet-x3-coin  pallet-x3-kernel  pallet-x3-settlement-engine
  x3-chain-runtime            x3-common       x3-x3-integration
```

The failing set, with the signing errors removed by the fix and the serde errors remaining, is what
`check-no-default-features.sh` reports; the earlier seven-crate reading included the signing errors my
bug introduced. Acceptance: `x3-common` builds without `std` (enable `serde/alloc` on that path, and
make any other `std`-only dependency explicit), or the crates that cannot honestly be `no_std` stop
claiming it — and `check-no-default-features.sh` is green with the empty known-list intact.

**TICKET-108 (as filed) — the bytecode envelope's version and checksum are written and never checked.**
`crates/x3-backend/src/bc_format.rs` is the canonical format: magic, semantic version, `min_version`,
feature flags, and a checksum over the body (`compute_checksum`, written at offset 12). For `no_std`
builds `crates/x3-integration/src/mini_x3.rs` re-implements it — and that is the decoder
`executor::execute` uses when the `std` feature is off, which is the runtime's case. Its
`parse_module` reads the magic and then `r.skip(20)`: version, flags, checksum, min-version and feature
flags are all skipped, and `x3-backend`'s own reader reads the checksum into `_checksum` without
verifying it. A module with a wrong version, an unsatisfiable `min_version`, or a corrupted body is
accepted as long as it parses structurally. So: two decoders for one format, the weaker one in the
runtime, and X3-LANG-009's own name ("Authenticated bytecode decoder") describes something with no
signature or digest behind it. Acceptance: one decoder for the format, or the no-std one verifying the
magic, version, `min_version` and checksum the writer already emits, with a test each for a corrupted
body and a future version. Evidence: `.ai/reports/feature-matrix-third-pass-20260923.md`.

## GAP-SECRETS-COMMITTED — four live credentials in tracked source — 2026-09-23

Found while looking for somewhere to put a DRPC key the operator had just bought. Tracked source
carried an Alchemy key, a paid DRPC key, an Ankr key **and a wallet private key** (the provider keys
twice, in `crates/external-chains/src/{env_config,rpc}.rs`), plus a live Infura key in two
`mcp-config.json` copies. The private key was `EnvConfig::from_env()`'s *default* wallet, so any build
of that crate could sign from an account whose key anyone reading the repository knows. That address
holds 0 ETH on Arbitrum and 0 on Base as of today (checked against public RPCs), so nothing has been
taken — but it is burned.

Fixed: credentials come only from `ALCHEMY_API_KEY` / `DRPC_API_KEY` / `ANKR_API_KEY`, the wallet only
from `X3_BOT_PRIVATE_KEY` + `X3_BOT_ADDRESS` (absent means absent — no default key, no default
wallet), symmetric with the private key never being defaulted; keyless public endpoints are the
built-in default (`EnvConfig::new`) and paid endpoints are promoted ahead of them when configured
(`EnvConfig::from_env`); and `scripts/check-no-provider-secrets.sh` — wired into `make guard` and
`local-ci`, with a reasoned allow-list at `scripts/allowed-provider-secrets.txt` — refuses keyed
provider URLs or 64-hex signing keys in tracked files, printing file, line and pattern but never the
value. Evidence: `.ai/reports/provider-secrets-20260923.md`.

**TICKET-102 — rotate, and decide about history.** Removing a value from HEAD does not un-expose it:
rotate all four keys, treat the wallet as burned, and decide whether to rewrite history the way SEC-v1
did for the validator seeds. Acceptance: every key that was ever in this repository is revoked, and no
build anywhere defaults to a credential.

**TICKET-103 — stop tracking build output and crawler state.**
`packages/blockchain-connector/dist/` and `infra-structure/services/rpc-crawler/crawler_state.json` are
output and state, currently allowed by the secret guard only because the keyed values in them are
other people's recorded endpoints rather than our credentials. Untracking them removes those values as
a side effect and stops a build artifact from being edited by hand. Acceptance: `git ls-files`
contains no `dist/` and no crawler state.

## GAP-GATEWAY-ORIGIN — the atomic gateway was a compiled-in dev key — 2026-09-23

The runtime gated the atomic kernel, the cross-VM router and the settlement finalization path with
`EnsureSignedBy<X3LangGatewayAccount, AccountId>` and
`EnsureSignedBy<SettlementGatewayAccount, AccountId>`. Those constants are the sr25519 public keys of
`//x3-atomic-gateway` and `//x3-settlement-gateway`, which `node/src/atomic_gateway.rs` asserts in a
test (`gateway_account_matches_runtime_constant`), and which the node's own comment calls "overridable
via CLI/env". The node service can override them; **the runtime could not** — the account was compiled
into the WASM. Every chain built from this runtime, `mainnet-rc1` included, handed
`assign_bundle_executor`, `finalize_atomic_bundle`, `rollback_bundle`, the router's
`X3LangOrigin`/`VmAdapterOrigin` and `finalize_with_settlement` to an account whose seed is in this
repository. Anyone who can read the source can sign as it, and can fund it themselves to pay fees.

This is the credential class from GAP-SECRETS-COMMITTED one level up: not a provider key but the
authorization root, and not in a file that can be rotated but in the runtime every chain shares.

Fixed: the privileged gates read `pallet_x3_custody`'s genesis-configured `AuthorizedGateways`
(`GatewayRole::X3Lang` / `GatewayRole::Settlement`) through a new `EnsureAuthorizedGateway` origin,
which requires a signature **and** membership. The dev accounts stay as the accounts the dev, local
and testnet specs name in genesis; staging, testnet and production take
`X3_{STAGING,TESTNET,PRODUCTION}_ATOMIC_GATEWAYS` and `..._SETTLEMENT_GATEWAYS`, and
`assert_no_dev_gateway_accounts` refuses the published dev seeds at spec-build time — the guard the
specs already applied to endowed accounts and authorities, now applied to privilege.
`spec_version` 19. Evidence: `.ai/reports/gateway-origin-registry-20260923.md`.

**TICKET-105 — CLOSED 2026-09-23.** There is no default any more, in the code or in the help text:
the CLI's doc comment claimed `//x3-atomic-gateway` was the dev default, which the spawn path never
implemented. `--x3-gateway-uri` (or `X3_ATOMIC_GATEWAY_URI`) is required for the service, and a
**live** chain refuses to start the service with a published development seed
(`atomic_gateway::published_dev_seed`, exact match after trimming, so `//x3-atomic-gateway-prod` is a
different account) — with one error line naming the chain, the seed and the flag to pass instead.
Dev and local chains still accept them, which is what they are for. Four unit tests
(`service::published_seed_tests`, `atomic_gateway::tests::published_dev_seeds_are_recognised_and_nothing_else_is`).
Evidence: `.ai/reports/gateway-uri-default-removed-20260923.md`.

**TICKET-105 (as filed) — the node's default gateway URI was a public seed.** `AtomicGatewayKey` defaults to
`//x3-atomic-gateway`, so a live chain that names an operator account in genesis and runs its service
with the default URI gets a service whose extrinsics are rejected — fail-closed, but silent until an
operator reads the log. Acceptance: the node refuses to start the atomic service with a published dev
seed against a live chain id, or says so on one line at startup, and the runbook names
`--x3-gateway-uri` as required for a live chain.

**TICKET-106 — CLOSED 2026-09-23.** Both files are deleted, and the runtime tests now cover what
they claimed. They were not merely undeclared test targets: they were written against an API that
never existed — `submit_atomic_bundle(origin, legs, deadline)` is missing the chain id and nonce the
call takes, `BundleLeg::Lock { amount, asset }` is not a variant of the leg type,
`RuntimeOrigin::signed(1)` is not an account this runtime ever authorized for the atomic gate, and
`assert_err!(result, "NonceAlreadyUsed")` compares a `DispatchError` with a string. Nothing could have
compiled them; "declare the targets and fix them" would have meant writing new tests under an old file
name. Four real ones live in `runtime/src/tests.rs` against the actual API:
`the_atomic_kernel_refuses_an_account_the_chain_did_not_authorize`,
`the_genesis_authorized_gateway_reaches_the_atomic_kernel`,
`a_bundle_finalizes_once_with_the_receipt_root_the_chain_requires`, and
`a_bundle_nonce_cannot_be_replayed`. Evidence:
`.ai/reports/e2e-safety-tests-replaced-20260923.md`.

**TICKET-106 (as filed) — two e2e files were not test targets.** `tests/e2e/safety_tests.rs` and
`tests/e2e/real_finality_proofs.rs` sit in the `e2e_tests` workspace member but are not declared in
`tests/e2e/Cargo.toml`, and `safety_tests.rs` declares `mod mock;` for a file that does not exist, so
neither can compile. Both drive `finalize_atomic_bundle` with `RuntimeOrigin::signed(1)` — an origin
this runtime has never accepted — and both assert `Ok`. They read as coverage of the atomic lifecycle
and provide none. Acceptance: declare them as targets and repair them against the current origin model,
or delete them and say so in the commit.

**TICKET-097 was closed the same day, by deleting the unsigned path** rather than signing it — its
off-chain marker had no writer anywhere in the repository, so there was nothing to preserve. See the
section above and `.ai/reports/unsigned-finalization-removed-20260923.md`. This change removed the
*key* that made the authorized paths anybody's; that one removed the *unauthenticated* path.

**TICKET-107 — CLOSED 2026-09-23 (node-side).** `AtomicGatewayService::finalize_bundle` no longer
finalizes with whatever certificate the chain's anchor holds. The node keeps the certificates *it*
observed, per block (`node/src/finality_certs.rs`; written by the flash-finality voter and, when Flash
is off, by the GRANDPA anchor task, which is the same value each of them anchors), and
`decide_finalization_cert` finalizes only when the chain's anchor agrees with the observed value. A
disagreement is refused with both hashes in the error, so a planted anchor can no longer be signed —
it stalls finalization for *that height* and nothing else, because the service keeps polling and the
next finalized height has a fresh anchor. Five unit tests, including
`a_planted_anchor_is_refused_rather_than_signed`. Evidence:
`.ai/reports/finality-certificate-trust-20260923.md`. What is *not* done: the anchor call itself is
still unsigned and first-write-wins, so a peer can still consume a height's anchor; making it
authenticated is a chain change and is not needed for safety now that no node signs an anchor it did
not observe.

**TICKET-107 (as filed) — the signed path trusted an unsigned anchor.** `node/src/atomic_service.rs`'s
`finalize_bundle` waits for `FinalityCertAnchors[block]` and finalizes with whatever certificate it
finds there. Anyone can write that anchor first (the call is still `ensure_none`), so an attacker can
make the honest service sign a fabricated certificate: the bundle ends `Finalized` with a proof no
voter produced. Client-side and small — the service should finalize with the certificate its own
finality voter observed (the value it writes under `x3ff:`) and treat the chain's anchor as a
cross-check, which turns the attack into a bounded liveness failure. Acceptance: a test where a
planted anchor does not change the certificate the service finalizes with.

## GAP-DEAD-ENDPOINT-CONFIG — the RPC endpoint tables no code reads — 2026-09-23

`config/rpc-endpoints.toml` and `infra/mainnet-rpc-endpoints.toml` list a chain, a chain id, a public
`rpc_url` and an `env_var` for every supported chain — the shape of an operator surface. Nothing
reads either file. `git grep` for the names finds a memory note, an audit inventory and one
marketing blurb in `infra/x3star-subdomains/server.js`; the only runtime `toml::from_str` in the
workspace parses the feature registry and the feature flags in `x3-readiness`. The header's
instruction and the file's own rows also disagree: it says to set `X3_CHAIN_RPC_<NAME>`, the rows
declare `X3_RPC_<NAME>`, and neither name is consulted anywhere.

This mattered the moment the operator bought paid endpoints, because that file is where an operator
would put one. Dropping a URL into it, or exporting `X3_RPC_ETHEREUM`, changes nothing today. The
paid-endpoint path that does exist is `ProviderCredentials::from_env()` in `crates/external-chains`
(`DRPC_API_KEY` → `https://lb.drpc.org/<network>/<key>`), promoted ahead of the keyless list by
`EnvConfig::from_env()` for the five networks it names — `X3_NETWORK` picks one — while the table in
this file lists seven, Ethereum and the test networks among them.

**TICKET-104 — make one endpoint surface real, or say it is documentation.** Either wire a single
loader and delete the other table (and make the env var the file names the one that is read), or
keep the tables as reference and put that at the top instead of an instruction that does nothing.
Acceptance: one canonical way to point a chain at an endpoint, and `git grep` for the chosen
mechanism finds the code that consumes it.

## GAP-TMP-NOT-DURABLE — a `/tmp` sweep cost a two-hour measurement — 2026-09-23

Everything this agent had under `/tmp` was removed between 02:01 and 03:22 UTC: the git worktree the
work ran from, the cargo target directory, Bitcoin Core and its live regtest chain, three soak
directories — including the **in-flight** two-hour no-cache run (TICKET-100b) — and the
`@polkadot/api` install that had just made extrinsic signing possible here. `df` fell from ~1.4 TB
to 593 GB, so it was a disk-space sweep, not an accident aimed at this work.

**Nothing of value was lost**, because everything was pushed: `origin/master` is `770e13ab08` with
#466–#475 in it, the srtool images survive at the digest the runtime record names, and the
repository's own `target/` cut the rebuild to nine minutes. The captured Bitcoin artifacts are
committed.

The repository already ignores `/.wt-*/` — "git worktrees used by parallel fix agents" — for
exactly this reason, and this agent was using `/tmp` because it was convenient. **Worktrees, soak
base directories and build output belong inside the repository (`<repo>/.wt-<name>/`) or another
durable path, never `/tmp`.** A long measurement must write its samples somewhere that outlives the
session. Fixed as described: `.wt-agent`, `CARGO_TARGET_DIR=<repo>/target`, soak `BASE_DIR` inside
the worktree. Evidence: `.ai/reports/tmp-not-durable-20260923.md`.

Separately: the Codex sandbox stopped working for non-escalated commands after the sweep
(`error building bubblewrap command: mountinfo path is not absolute`), so every command now needs
escalation. Environment, not repository.

## GAP-CLAIM-SURFACES — the retracted claim had moved somewhere nothing checked — 2026-09-26

`X3-CLAIM-002` ("MEV-proof marketing claim", 20/10/10) instructed: rename it to "MEV-resistant
architecture under development". Measured today, `CURRENT_MAINNET_STATUS.md` does not mention MEV at
all — the instruction had been carried out, which is why the row read as half-closed. The claim had
not gone away; it had moved into the desktop CRM's outbound templates and grown there:

| where | what it asserted |
| --- | --- |
| `apps/x3-desktop/src-tauri/src/crm/outreach_system.rs` | "Deterministic execution guarantees (no MEV/reorg risk)", "Sub-300ms cross-chain settlement (vs 12+ seconds standard)", "300ms cross-chain finality (vs 12s on Solana)", "GPU utilization improvement from 65% → 92%", "Compute revenue per GPU: $1200/month (current pilot)", "5,000 TPS baseline", "300ms P99 latency", "Live in 3 production networks", "We have 3 regional partners already live", and a PQC roadmap whose first item ("Q2 2026: Dilithium signature integration complete") was already in the past and untrue |
| `apps/x3-desktop/src-tauri/src/crm/hardware_acquisition_commands.rs` | unverified claims, quoted here only so they are not written again: a 100K TPS / 300ms finality figure, "real-world performance data from a production network", "we'll deploy 500+ GPUs", "committed minimum $500K/quarter purchase", "VC-backed ... projected $50M+ HW spend", "X3 provides certified e-waste and IT surplus management services", "NIST SP 800-88 compliant", "Payment within 48 hours" |
| `apps/x3-desktop/src-tauri/src/crm/outreach_system.rs` (seeded contacts) | real people at real organisations — DeepMind, Anthropic, MIT CSAIL, the Institute for Quantum Computing, CoreWeave — with invented personal addresses (`demis@deepmind.com`, `dario@anthropic.com`, `shor@mit.edu`, `mmosca@uwaterloo.ca`) that `crm/smtp.rs` could mail |

The distinction that matters is not "documentation vs code": these are strings a human pastes into
an email to a company, which makes them the most expensive place in the repository to be wrong. The
package's own manifest is evidence of which way it was pointing — `crates/quantum-crypto/Cargo.toml`
says "Research/simulated ... NOT audited post-quantum security" while the outreach template sold
"Production-grade quantum-resistant consensus".

Fixed by rewriting the surfaces to what the project can show and replacing the seeded contacts with
reserved-`example.com` placeholders, and then by making the rule a gate rather than a one-time edit:
`scripts/ci/check-claims-hygiene.py` (gate `claims hygiene`, default set) reads 78 declared claim
surfaces, tree-wide for phrases that are false in every context and surface-only for figures, and
skips lines that qualify themselves. Measured load-bearing: appending
`> MEV-proof ordering with 4,200 TPS finality.` to `CURRENT_MAINNET_STATUS.md` fails it on both
rules; removing the line returns `OK - 6016 file(s) scanned, 78 claim surface(s), no unqualified
claim`.

**TICKET-138 — the desktop CRM is twelve modules, five of which are compiled.** `crm/mod.rs`
declares `db`, `models`, `commands`, `smtp` and `agents`; `outreach_system.rs`,
`hardware_acquisition_commands.rs`, `funding.rs`, `funding_war_plan*.rs`, `dorks*.rs`,
`audit_tournament.rs`, `hardware_sources_db.rs` are neither compiled nor called by anything
(`apps/x3-desktop/src-tauri` is also excluded from the cargo workspace, so nothing type-checks them).
Honest text in an unreachable file is not a working feature. Acceptance: either declare the modules
and give each a test that exercises what it claims, or delete them and the seeding functions with
them; either way `cargo check` (in a workspace that includes the crate, or an explicit gate) must
cover whatever remains. Do not "fix" this by re-enabling the modules without wiring their callers:
an operator UI that seeds invented contacts is worse than one that ships none.

**TICKET-139 — widen the claim scanner past the surfaces it declares.** The gate reads root
markdown, `docs/testnet-config/` and the CRM. Roughly 150 further unqualified figures live outside
that list and are, for now, only reported here: `docs/openspec/changes/p4-solana-gpu-acceleration/
P4_IMPLEMENTATION_GUIDE.md` (15 hits, including unverified "100,000+ TPS" and unverified "250x faster settlement"),
`benchmarks/tps-archive-2026-02/README.md`, `.planning/README.md` ("Public testnet running 100+
nodes, 1000 TPS" - unverified), `.audit/CLAIMS_INVENTORY_AND_TRACKER.md` (an unverified "4,200 TPS" figure), `docs/runbooks/`,
`infra-structure/`, `tests_phase4/`. Acceptance: decide per surface whether it is a claim surface
or an evidence record; add the claim surfaces to the scanner and fix what it finds, and add the
evidence records to a skip list with the reason written down. A surface list nobody revisits is how
this gap happened the first time.

**TICKET-140 — the explorer criterion should check chain data, not a page title. CLOSED
2026-09-26.** `X3-OPS-009`'s page (`apps/explorer/app/page.tsx`) was eight lines: a heading and
the sentence "Block explorer for X3 Chain". It satisfied the launch gate's criterion 14, because
that criterion only required the served body to identify itself as the explorer — so a stub
passed, and an operator could open a "testnet" on it. Closed in two halves:

* the page reads the chain. It calls `chain_getFinalizedHead` and `chain_getHeader` over JSON-RPC
  (`X3_EXPLORER_RPC`, falling back to `X3_RPC_URL`), renders the finalized number, hash, endpoint
  and read time, and — when the endpoint does not answer or answers with something that is not a
  head — renders no height at all and says the chain is unreachable. The markers the gate keys on
  are `data-x3-explorer="chain-head"`, `data-x3-explorer-height="<n>"` and
  `data-x3-explorer-error="rpc-unreachable"`.
* the criterion cross-checks. `scripts/mainnet/public_testnet_gate.sh` reads the finalized head
  itself and requires the explorer's number to equal it; when it cannot read a chain it requires
  the page's explicit unreachable marker instead, so a page that says nothing fails either way.
  The report row names the URL, the number it checked, and on a mismatch both numbers.

`scripts/testnet/explorer-gate-drill.sh` proves it in four directions against a stub JSON-RPC and
the real app: decoy page → FAIL; explorer and gate reading the same chain (#4242) → PASS, named,
head recorded; explorer on #4242 while the gate reads #9999 → FAIL naming both; explorer with an
unreachable endpoint → PASS only because it renders no height and says so. Two of those four
phases failed when first written, and both were real defects rather than test noise: the pinned
`X3_EXPLORER_URL` was not a pin (a leftover `next start -p 3010` satisfied the criterion), and the
gate's height extractor read the `3` in `x3-explorer` as a height, failing a correct explorer.

Not closed by this ticket, and not claimed: there is still no block view, no transaction view, no
account view, no search, no indexer for history and no hosted deployment. `X3-OPS-009` is 55/70/25
for that reason.

**TICKET-142 — a test named for fair ordering was asserting `50 > 0`. CLOSED 2026-09-26.**
`crates/x3-dex/src/tests/attack_liquidation_frontrun.rs` held one test,
`liquidation_frontrun_eliminated_by_fair_ordering`, whose entire check was
`assert!(first_bonus > second_bonus)` over two hardcoded locals (`50u64` and `0u64`). It built a
two-swap batch, executed it, and then compared the literals — so it would have passed with the
router ordering by arrival, by reverse arrival, or by nothing, which is what it did: measured
2026-09-26, `BatchSwapRouter::execute_batch_swap` summed the caller's `actual_outputs` in vector
order and never read `SwapInstruction::sequence`. The test's name was the claim, and the name was
unearned.

Closed in two parts, both narrower than the old name and honest about it:

* the router now enforces the order it declares. `swaps_are_in_sequence_order` is called by
  `create_batch_swap` *and* by `execute_batch_swap` — the latter because `BatchSwap`'s fields are
  public and the type is `Decode`, so a batch can arrive without going through `create`. A batch
  whose vector disagrees with its own `sequence` fields is refused with
  `"Batch swaps are not in sequence order"` before any output is accepted, and a refused batch
  keeps `status = 0` and `total_output = 0`.
* the file now asserts three things instead: a batch in the order it declares executes
  (`a_batch_in_the_order_it_declares_executes`); a hand-reordered batch is refused
  (`a_front_runner_cannot_reorder_the_batch_by_handing_it_over_differently`); and the refusal
  happens before the output checks, so nobody buys execution by paying every minimum
  (`the_refusal_happens_before_any_output_is_accepted`).

Measured load-bearing: deleting the `execute_batch_swap`-side call makes
`the_refusal_happens_before_any_output_is_accepted` fail with
`assertion left == right failed: a batch carrying one order while declaring another must be
refused`; restoring it returns the suite to `216 passed; 0 failed` (`cargo test -p x3-dex`).

Not closed and not claimed: this router still has no *fair* ordering. Ordering by a secret nobody
can grind is the commit-reveal window in `crates/x3-swap-router/src/mev_protection/fair_ordering.rs`,
and nothing in this crate calls it.

**TICKET-143 — the DEX settlement bridge has six tests and no caller.** Found while working the
ordering rows, not fixed: `crates/x3-dex/src/settlement_bridge.rs` (520 lines, 6 unit tests) is
declared and re-exported (`pub use settlement_bridge::{LimitOrderSettlementBridge,
OrderSettlementIntent, SettlementStatus}`) but nothing outside the crate names
`LimitOrderSettlementBridge` — `rg` over the tree outside that file returns no other hit. Its own
doc comment is honest about the gap ("Actual on-chain submission requires runtime integration via
extrinsic... documents the mapping for implementation in node/src/rpc.rs"), and five of its doc
examples are marked ```ignore```, so nothing here is passing as a test that is not one. What is
missing is the caller: the mapping is written, the extrusion to
`pallet-x3-settlement-engine` is not. Acceptance: either a real submission path with a test that
fails when the mapping is wrong, or delete the bridge and the claim that the DEX settles on chain.

## TICKET-139 — the claim scanner now reads the surfaces people actually read — 2026-09-26

**CLOSED.** `scripts/ci/check-claims-hygiene.py` scans `docs/**`, `production/public/**`,
`.planning/**`, root markdown and the CRM. Every path this ticket named is *decided in the file*:

| path | decision | reason (written in the scanner) |
| --- | --- | --- |
| `docs/**` | claim surface | the documentation a person reads as a statement of what X3 does |
| `production/public/**` | claim surface | the published site — a stranger reads it before deciding we are real |
| `.planning/**` | claim surface | roadmap/sprint plans carry dated `Complete`/`Live` rows |
| `benchmarks/`, `.audit/`, `infra-structure/`, `tests_phase4/`, `tests_core/`, `tools/` | evidence record | each is declared in `EVIDENCE_RECORDS` with its reason and still listed as a surface, so deleting an entry re-enables scanning |

It found 41 real claims. The worst were a `P4_IMPLEMENTATION_GUIDE.md` whose every success
criterion was ticked `[x]` for a GPU system with no `.cu`/`.ptx`, no artifact and no benchmark; the
public site's `zero-fee flashloans` / `sub-200ms finality` hero copy; and `.planning/README.md`'s
dated `Testnet Live | Public testnet running 100+ nodes, 1000 TPS` row for a testnet the same ledger
records as undeployed. All 41 now state the honest state, a target, or a measurement.

Two new rule sets came with it, both because the widened surfaces turned up lines that look like
claims and are not: an unchecked `- [ ]` box is a plan item (a checked `[x]` stays a claim), and
`acceptance`/`requirement`/`criteria`/`threshold`/`goal`/`objective`/`must`/`versus`/`expected`
mark a bound or a projection rather than a result.

**Measured, all four controls on this box:**

```
OK   - 5482 file(s) scanned, 460 claim surface(s), 520 declared evidence record(s) excluded, no unqualified claim
FAIL - production/public probe `<p>Sub-200ms finality at 100,000 TPS.</p>`  (new surface IS scanned)
OK   - the same file with `- [ ] 1,000 TPS demonstrated`                    (a plan item is not a claim)
FAIL - the same file with `- [x] 1,000 TPS demonstrated`                    (a ticked box is a claim)
FAIL - EVIDENCE_RECORDS with `benchmarks/` removed reddens the TPS archive   (the skip list is load-bearing)
```

**Found while widening it:** `SKIP_DIRS` was matched one path component at a time, so its
`docs/audit` and `docs/superpowers` entries — the two that contain a slash — did nothing, and those
files were scanned while the docstring said the registry is out of scope. `--list` named
`docs/audit/X3_AGENT_QUEUE.md` and `docs/audit/X3_FEATURE_COMPLETION_MATRIX.md` as claim surfaces
before the fix and names none after. The gate was green either way, so this is the stated scope
becoming true rather than a way to obtain green.

**TICKET-140 — decide the `apps/**` surfaces the scanner still does not read.** Measured
2026-09-26: `apps/inferstructor-dashboard/src/components/RegisterPage.tsx` carries 5 unqualified
figures, `apps/x3-studio/electron/main.ts` 4, `apps/dashboard/src/panels/docs/AnalyticsReportingPanel.tsx`
2. Only `apps/*/src-tauri/src/crm` is a surface today. Acceptance: for each app, either declare the
path a claim surface and fix what it finds, or record it in `EVIDENCE_RECORDS` with the reason.

## TICKET-141 — deployment-state copy, and the rule that now reads it — 2026-09-26

**CLOSED.** This ticket opened because the pages under `production/public/` carried the hero tag
`Now live on testnet` — a liveness claim with no deployment behind it, the same class as the
retracted MEV one, and no scanner rule matched it. Measured on this box, four sites stated a
deployment state that does not exist:

| file | said | now says |
| --- | --- | --- |
| `production/public/x3-ecosystem.html` (hero) | `Now live on testnet` | `In development — public testnet not yet deployed` |
| `production/public/x3-ecosystem(1).html` (hero) | the byte-identical twin of the line above | same fix, or the two files diverge |
| `production/public/x3-ecosystem*.html` (final CTA) | "Testnet is live. Contracts are deployed." | "The public testnet is not deployed yet. …in development…" |
| `docs/root/README.md` | "X3 Chain Testnet v1 is NOW LIVE!" | "…is in development — the public testnet is not deployed yet." |

The `README.md` case is the sharpest: the same file says ten lines later that nothing resolves,
so the page contradicted its own evidence. The hero's pulsing dot went with the claim — a
blinking green "live" indicator is the claim in a different alphabet.

`LIVENESS` is a third rule set in `scripts/ci/check-claims-hygiene.py`, surface-scoped like
`NUMERIC` and with the same self-qualification escape, so `not live`, `planned`, `will be live`
and `- [ ] go live on testnet` are not claims. The rule is load-bearing — measured, and measured
missing first:

```
OK   - the committed scanner at b80604eac reads this page with the hero tag: no claim found
FAIL - the scanner with LIVENESS, same page, same line: "unqualified deployment-state claim"
OK   - the fixed page, same line: no hit
OK   - a probe line that says it is not live yet, and a "- [ ]" plan box: qualified, no hit
```

**Still open, adjacent, not this ticket:** the pages' `Launch on Testnet` button links to `#`,
and the built bundle `production/public/assets/index-*.js` falls back to fabricated dashboard
numbers (42 validators, "99.8%" uptime) when its API call fails. Both point at a deployment
state the ledger says does not exist, but a dead CTA and a baked bundle are TICKET-140-shaped
(a surface the scanner still does not read), not rule-shaped.

## TICKET-144 — the MEV/privacy family has three rows of code and no path to a chain — 2026-09-26

**OPEN.** Round 4 measured the three shallowest rows of `feature-matrix/mev-privacy.toml` and moved
all three on real code and real tests: `X3-MEV-002` 45/20/25 → 55/40/25, `X3-MEV-007` 5/0/0 →
45/40/10, `X3-MEV-008` 15/5/5 → 45/40/10. What none of them gained is a caller. Measured
2026-09-26:

```
$ cargo tree -i private-mempool --workspace      -> crates/confidential-gpu, and nothing depends on it
$ cargo tree -i x3-swap-router --workspace       -> the crate itself
$ rg submission|private|ModeCheck over crates/{x3-compiler,x3-vm,x3-backend,x3-common}  -> zero hits
```

So a private transaction cannot be submitted (no ingress), a decryption share cannot be produced by
a running validator (no committee is wired to a node), and an order cannot be placed fairly (the
commit-reveal lane in `crates/x3-swap-router/src/mev_protection/fair_ordering.rs` has no caller) —
even though each of the three is implemented and tested in isolation, and each row's blockers now
say so. This ticket is the single acceptance target that closes all three at once, because they
fail for the same reason.

Acceptance: one node-level test that starts a node, submits an encrypted transaction through a real
ingress, has a threshold of committee members decrypt it under the DKG epoch the ciphertext names,
executes it and checks the receipt — with the ordering lane deciding the order inside that same
path, or an explicit record of which link is still missing and why. A unit test cannot satisfy this;
the path is the point.
**TICKET-145 — the production rollback dropped pre-window writes, and the journal had no writer.
CLOSED (both halves, 2026-09-26).**

`X3-MEV-004` ("Transactional host rollback", P0, 50/25/30) cites `x3-lang/compiler`, but the chain
does not run that compiler: `pallet-x3-kernel` executes X3BC through `crates/x3-vm`, and that is
where the atomic window lives (`AtomicBegin` / `AtomicCommit` / `AtomicRollback`, with
`AtomicAborted` as the aborted-scope error). Measured on 2026-09-26, its rollback was not
transactional:

```rust
pub fn rollback(&mut self) -> Result<(), StorageError> {
    let snap = self.snapshots.pop()...;
    self.data = snap;
    self.journal.clear();   // <- drops every write recorded BEFORE the window too
    Ok(())
}
```

Write `A`, snapshot, write `B`, roll back: `data = {A}` and an **empty** journal — while the journal
is documented as "all writes since last flush", used for cross-VM delta sync. Whoever applies that
delta loses `A`. Nested windows make it worse: an inner rollback wipes the outer window's writes as
well.

**Closed:** the snapshot now records `journal.len()` and the rollback truncates to it, so a reverted
window abandons only its own writes. Four tests, and the control is measured: restoring
`journal.clear()` makes
`test_rollback_keeps_the_journal_of_writes_that_predate_the_window`,
`test_nested_rollback_truncates_the_journal_to_the_inner_window` and
`test_rollback_of_the_outer_window_abandons_a_committed_inner_window` fail; truncating returns the
storage suite to 15/15. `cargo test -p x3-vm` is 154 passed.

**Second half, closed 2026-09-26:** the journal could not be populated from a program.
`crates/x3-vm/src/vm.rs` touched `self.storage` only through `snapshot`/`commit`/`rollback` — no
instruction wrote a key. `EvmSstore`/`EvmSload` were in the ISA
(`x3-backend/src/opcode.rs`, 0xB4/0xB3), emitted by the backend
(`emit_evm_sstore`/`emit_evm_sload`), decoded and priced by the verifier (200/5000 gas), and priced
by the interpreter's own table — and both hit the interpreter's `_` arm and returned
`UnimplementedOpcode`, so a deployed X3VM program could not carry one value between calls and the
atomic rollback over storage was provable only at the storage unit level.

Both opcodes are now interpreted (`crates/x3-vm/src/vm.rs`):

* slots live in their own keyspace, disjoint from globals (`evm_slot_key`); a global index is read as
  a `u32`, so no global can address a slot key, and `evm_load_of_an_unwritten_slot_reads_zero`
  depends on that disjointness;
* slot payloads carry a tag and a length (`value_to_storage_value`'s untagged layout cannot tell
  `Bytes([1, 2])` from `Bytes([1, 2, 0])`, or either from `I64(197_121)`), so a store/load round trip
  is exact for every `Value` kind;
* a store refuses by name rather than writing something else: a negative or non-integer slot, a
  payload over 30 bytes (the old helper silently truncated at 32), and `Unit`;
* a load fails closed on a payload this ISA did not write (unknown tag, or a length that disagrees
  with the fixed-width kind);
* the interpreter charges the verifier's figures (200/5000), and a store lands in the journal — the
  delta `drain_storage_journal` documents — while a store inside a reverted atomic window does not.

Measured: `cargo test -p x3-vm` is 165 passed, 0 failed (154 before); clippy `-D warnings` and
`cargo fmt --check` are clean. The control is measured: guarding both arms off (`if false`, so they
fall through to the `_` arm as before) reddens 10 of the 11 new tests with
`UnimplementedOpcode(180)`; restored, `cargo test -p x3-vm --lib evm_` is 15 passed.

Still open, and now its own ticket (TICKET-147): the writes do not reach chain state. Also still
open from this ticket: `TradingHost::begin_transaction`/`commit_transaction`/`rollback_transaction`
(`x3-lang/vm/src/trading.rs`) have no caller outside that crate's tests, and the language VM's
`ATOMIC_ROLLBACK` restores VM state without invoking any host hook — safe only under an unwritten
contract that hosts never apply effects during execution.

## TICKET-147 — X3VM contract storage has no channel into chain state — 2026-09-26

Found while closing TICKET-145. A program can now write a slot, but nothing carries the write out of
the VM:

* `crates/x3-integration/src/executor.rs` builds the receipt with `state_changes: vec![]` and the
  comment "Hostcall state change collection deferred to runtime integration";
* `crates/x3-vm/src/vm.rs::drain_storage_journal` has no caller outside `crates/x3-vm`'s own tests,
  so the documented cross-VM delta is never applied;
* the receipt's `state_changes` channel is *balance-shaped*, not storage-shaped:
  `pallet-x3-kernel`'s `apply_canonical_ledger_update_v2` decodes every entry as (address -> account,
  key -> asset id, value -> balance) and counts anything else in `DecodeFailureCount`. Routing slot
  writes through it would increment a monitoring counter and persist nothing, so the fix is not
  "fill the field in".

Acceptance: a typed X3VM-storage channel (its own receipt field or its own pallet storage map, with
a versioned encoding), `X3VmAdapter` mapping the drained journal into it, a `DecodeFailureCount`
that stays at zero for a storage-writing comit, and a test that a slot written by a program is
readable back after the receipt is stored — plus the same for a reverted window (nothing applied).
Until then, `X3-MEV-004`'s `mainnet_ready` stays at 40 with this as the reason.

**TICKET-146 — wallet registration accepted anything, and recovery was an announcement. CLOSED
2026-09-26.** Both rows are `P0`/`core`, and both blockers ("biometric template handling
unaudited", "recovery logic security review pending") turned out to understate what was there.

*Biometric (`X3-ECO-001`).* `register_biometric` built the profile by hand: any `biometric_type`,
an all-zero template hash, an all-zero PIN hash, and `owner: [0u8; 32]`, so the stored record did
not name the account that registered it. `crates/x3-wallet`'s `BiometricManager::create_profile`
had rejected the first three the whole time and the pallet already depended on that crate. It now
calls the library and maps the refusals onto `InvalidBiometricType` / `EmptyTemplateHash` /
`EmptyPinHash`, with `InvalidBiometricProfile` for anything unnamed. Delegation is measured, not
asserted: deleting the library's `template_hash == [0u8; 32]` check makes the pallet's
`register_biometric_refuses_an_empty_template_hash` fail.

*Recovery (`X3-ECO-004`).* `initiate_recovery(origin, new_owner)` needed only that
`RecoveryAccounts[caller]` existed, then emitted `RecoveryInitiated { account, new_owner }` — no
guardian check, no quorum, no delay, no state change. And nothing in the pallet ever wrote
`RecoveryAccounts`, so the `ensure!(recovery.is_some())` could never pass on a real chain; the
pallet's own test made it pass by inserting the record into storage directly. Recovery is now the
library's model, wired: `register_recovery_guardians` (owner-only, validated by
`SocialRecoveryManager::create_recovery_account`) → `initiate_recovery(account_id, new_owner)` by
a guardian (with a delay) → `approve_recovery` per guardian, duplicates refused → `finalize_recovery`
only once the threshold is met *and* the delay elapsed, which is the one place the stored recovery
owner changes → `cancel_recovery` by the recovery owner.

Two controls were measured on the recovery path: with the library's delay check *and* the pallet's
`ensure!` removed, an early finalize succeeds and the test fails; and the first version of the fix
reported `previous_owner == new_owner` in the executed event (the test caught it) until the event
captured the owner before the move. `cargo test -p pallet-x3-wallet` is 23 passed, up from 13, and
`cargo check -p x3-chain-runtime` compiles the pallet into all four runtime variants.

Still open, and recorded on the rows rather than hidden: nothing consumes the biometric
`attempts_remaining` / `locked_until_block` (no verify-unlock extrinsic exists, so the lockout is
stored and never spent, and a re-registration resets it); the recovery owner is a `[u8; 32]`
address in the recovery record, so what is recovered is the right to manage a guardian set — no
`T::AccountId`, balance or `HardwareWallets` entry follows it; a pending request never expires and
only the recovery owner can cancel it; and neither path has had an independent security review.

**TICKET-148 — the on-chain interpreter answered 44 opcodes it could not execute. CLOSED
2026-09-26.** `crates/x3-integration::mini_x3` is the interpreter a block runs: `node/src/service.rs`
builds `sc_service::new_wasm_executor`, so the runtime's `x3-x3-integration/std` feature is off in
the wasm build and `X3VmAdapter` reaches the `no_std` arm. It held arms that decoded their operands,
wrote a placeholder and continued — `LoadIndex`/`StoreIndex`/`LoadField`/`StoreField` and the whole
array/tuple family answered `I64(0)`/`Unit`, every context read returned a zero (`CtxChainId`
returned a hard-coded `3375`), `Emit` dropped the event, the atomic family was skipped so a failed
window still committed, and every EVM/SVM/GPU intrinsic returned `I64(0)` after skipping six bytes —
the wrong width for most of them, which could desynchronise the instruction stream. That is
`AGENTS.md` §5, §4 and §18 at once. All 44 now refuse with `X3Error::UnsupportedOpcode(byte)`;
the atomic window is implemented for real over the module's globals (the only state this interpreter
has) and bounded by `MAX_ATOMIC_DEPTH = 32`. Proof:
`.ai/runlogs/2026-09-26-onchain-interpreter-fail-closed.md`; tests
`crates/x3-integration/tests/mini_x3_fail_closed.rs` (5) and
`crates/x3-integration/tests/interpreter_agreement.rs` (3). Break-it-first measured on both halves
of the table: 18 of 44 red with the aggregate/context/agent arms restored, 26 of 44 red with the
intrinsic arm restored.

## TICKET-149 — the two interpreters still disagree about aggregates and slot storage — 2026-09-26

Found while closing TICKET-148. The fail-open class is gone, but the engines are not the same
engine:

* `crates/x3-vm` has no arm for the aggregate family, `inc`/`dec`, `mod_f`, the numeric conversions,
  `ctx_gas` or `atomic_check`, all of which the runtime interpreter now executes. A program using
  one runs on chain and fails off-chain.
* `crates/x3-vm` implements `evm_sstore`/`evm_sload` against its journaled storage; the runtime
  interpreter refuses both, so a contract can carry a slot between calls off chain and cannot carry
  one on chain at all. `interpreter_agreement.rs::slot_storage_is_implemented_off_chain_and_refused_on_chain`
  asserts both halves on one artifact.
* The verifier admits all of these at intake, so the refusal happens at execution rather than at
  validation.

Closing this means one implementation, or a generated conformance table the two are checked against
in both directions: every opcode the compiler can emit, in both engines, with the verdict recorded.

## TICKET-150 — the receipt replay was described but not run, and the floors and ceilings were the receipt's own — 2026-09-26

Found by reading what `x3c replay` actually calls. Both `x3c replay` and `x3c receipt verify` printed
that they had checked the receipt's economic replay; neither called `verify_receipt_economics`.
`verify_receipt` re-derives the hash and the accounting invariants and deliberately stops there, so a
receipt whose reported figures contradicted the policy it carried was reported as verified — in
`receipt verify`'s case under the words "hash + economic invariants". Four holes sat behind that,
each reachable with a receipt that is internally consistent and re-hashed, so the hash check has
nothing to say about any of them:

* `AssertMinNetProfit { settlement_asset, minimum }` was recorded as *present* (`saw_profit_guard`)
  and never read. A receipt netting 2,000,000 USDC against its own stated floor of 5,000,000
  replayed cleanly, and a floor stated in an asset the receipt never touched was not read at all.
  The policy-level `minimum_net_profit` / `minimum_net_profit_asset` was not read either.
* `max_slippage_bps` bounds a quantity no receipt carried. The swap path records each leg's realized
  slippage in its quote window now (receipt format version 3), computed by the same helper that
  enforces the ceiling, and replay re-derives it. A window without the figure — a format-2 receipt —
  is refused by name rather than read as "no slippage".
* `max_flash_fee_bps` was enforced on repay and never re-checked at replay. The principal and fee are
  already in the receipt, so the ratio is re-derived from them.
* Replay never compared the receipt's operation sequence to the artifact. The artifact hash proves
  which artifact a receipt *claims*, not that the sequence inside it is what that artifact compiles,
  and the economics read the ceilings out of the receipt's own operations. A receipt re-hashed with
  a 900 bps ceiling against an artifact compiling 30 passed every check the command made.

`verify_receipt_economics` gained typed refusals (`ProfitBelowCompiledFloor`,
`ProfitFloorWithoutAsset`, `MissingRealizedSlippage`, `SlippageExceeded`, `FeeCeilingExceeded`) and
`basis_points_of` became the one place the fee/slippage ratio is computed, so the figure execution
enforces and the figure a receipt carries cannot drift. The dead `deltas` accumulator in the replay
is gone: `accrue_cost` already nets costs into `net_deltas`, and the accumulator was never read,
inviting a reader to believe costs were counted twice.

Break-it-first, measured on both halves. With the floor check removed,
`receipt_below_its_compiled_profit_floor_fails_economic_replay` and
`profit_floor_in_an_asset_the_receipt_never_touched_fails_economic_replay` fail. With the
artifact-binding loop removed, `cli_replay_refuses_a_receipt_whose_policy_is_not_the_artifacts` fails
and the CLI prints `x3c replay: ok — CrossDexArb replays against /tmp/cli_forge_weaker_policy.x3b` for
the forged receipt. Both controls were restored before the commit.

Verified: `cargo test --workspace` in `x3-lang` (vm 165 — receipts 20, execution 68, properties 14;
x3-tools cli 71 + cli_integration 9), `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets`.

Still open on the row, and named there rather than hidden: the finality references and the host
inputs the phase asks about are not carried by either artifact or receipt, and the state the
`state_commitment` commits to is not part of a receipt, so a replay cannot check the balances behind
it. The three fees the execution path also enforces — `max_gas`, the oracle-deviation ceiling and
`max_cumulative_loss` — are re-derived for the ceilings that appear in a receipt and not for the
per-run cost the host reports; that is the remaining half of "correct risk checks".

## TICKET-151 — the capability checks had no caller that could disagree — 2026-09-26

Found while reading what `x3c receipt execute` hands the VM. The rule is implemented:
`TradingVm::execute_atomic` calls `validate_compiled_policy` before the first host call, and it is
the real check the row describes — the policy's chain against the host's, its policy version against
the host's, `require_private_submission` against the host's capabilities, and then a per-operation
capability check (a borrow's provider, a swap's venue, a bridge's adapter). What the command did was
make every one of those clauses unfalsifiable: it built the host manifest **out of the artifact it
was checking**.

```
chain: policy.chain.clone(),
version: format!("trading-policy-v{}", policy.policy_version),
private_submission: policy.require_private_submission,
providers / venues / bridges: collected from the operations
```

Measured on that version: `x3c receipt execute /tmp/trade.x3 --chain base --provider aave_v3 --venue
uniswap_v3 --venue sushiswap` printed `trade 'CrossDexArb' committed, receipt verified` — a program
compiled for `ethereum` and requiring private submission, run by an operator who declared a different
chain and no private lane. Nothing failed, because the host agreed with the program by construction.
`CapabilityChainMismatch`, `CapabilityVersionMismatch`, `PrivateSubmissionRequired` and
`UnknownCapability` could not fire through this command at all.

The host is now declared by the caller: `--chain` (required — a default would answer the question the
flag exists to ask), `--private-submission`, and repeatable `--provider` / `--venue` / `--bridge`.
The fixture host's policy version is a constant of the host (`FIXTURE_HOST_POLICY_VERSION`), not a
copy of the artifact's, so the version clause has a referent too. Four refusals are exercised by name
through the CLI (chain, venue, provider, private lane), plus one test that the declaration is required
and one that a host declaring what it offers still runs the program. Break-it-first: with the two
manifest lines reverted to the artifact's values the chain test's forgery runs green as shown above;
restored, it refuses with `compiled policy chain 'ethereum' does not match host chain 'base'`.

Verified: `cargo test -p x3-tools` (cli 77, cli_integration 9), `cargo fmt --all -- --check`, `cargo
clippy --workspace --all-targets`.

Still open on the row: a **live upgrade** — an old artifact executed against a newer VM on a running
chain — is what the envelope's version bounds exist for and is not exercised by any test here. That
needs the seven-node network and a runtime upgrade, not a unit test.

**TICKET-148 — the relayer's CLI monolith signs its proofs with a dev key. CLOSED 2026-09-26 (the
quorum half), and TICKET-148 for what is left.** `crates/x3-relayer/src/submitter.rs` carried
`svm_required_signatures: 1` with a comment that quorum enforcement "belongs at the aggregator
layer" — and no aggregator exists here, so the production path was one signature. That is gone:
`x3-validator-attestation` owns `supermajority_threshold(n) = floor(2n/3)+1` (never zero, monotonic
in n), `crates/x3-relayer/src/quorum.rs` decodes an `AuthorizedValidatorSet` (refusing malformed,
wrong-length and repeated keys at startup rather than trimming them) and counts *distinct
authorized* signers, and the submitter refuses with typed `SvmQuorumUnreachable` /
`NoAuthorizedValidatorSet` / `UnauthorizedSubmitter` rather than emitting a proof it cannot back.
Measured: `cargo test -p x3-relayer -p x3-validator-attestation` → 58 + 5 + 15 passed; removing the
policy check makes `safety_pipeline_refuses_a_one_of_three_proof_that_declares_itself_satisfied`
accept a one-of-three proof, and restoring it returns 9/9.

Still open, and the reason `X3-XCHAIN-003`'s mainnet readiness moves *down* (35 → 25) while its
implementation moves up: `RelayerService` has no production constructor in this workspace (`node`
and `x3-gateway` consume only its types), signature aggregation across validators has no host, the
authority path is still undecided (the settlement engine accepts a proof only from the intent's
maker or taker), and `crates/x3-relayer/src/main.rs` is a separate monolith whose proof signer
defaults to `//Alice` (`X3_RELAY_PROOF_SIGNER`) and does not use `RpcSubmitter`/`RelayerService` at
all. TICKET-148: give the CLI one relayer path, with no dev-key default.

## TICKET-152 — the finality certificate is a shape, not yet a fact about a chain — 2026-09-26

`X3-XCHAIN-005` closed its headline gap: `crates/x3-atomic-swap` no longer decides finality from
numbers a caller supplies. `FinalityCertificate` carries `{ chain, block_height, block_hash, tx_id,
confirmations, observed_at }` with private fields and two constructors, `confirmations` is derived
(`observed_at - block_height + 1`), and `FinalityOracle` refuses a wrong chain, a tip below one it
already accepted, a tip outside the configured staleness window, and a depth its anchor does not
imply. `Relayer::verify_finality` takes the certificate instead of `(required, current, chain)`.

What that does **not** yet buy, and what this ticket is for:

1. **No producer.** Nothing builds a certificate from a real chain. `block_hash` has to be bound to
   `chain` by a reader — an RPC quorum, a light client, a receipts-trie proof — and no such reader
   is wired on this row. The certificate is therefore a checked *shape*: it cannot invent depth for
   an anchor, but nothing here proves the anchor is the chain's block.
2. **No persistence.** The oracle's accepted and witnessed tips live in the oracle's memory. A
   rewind that spans a process restart is not caught. They belong with the proof ledger.
3. **No live reorg evidence.** `CertificateRewindsAcceptedAnchor` is proven to be refused once such
   a certificate is presented; no test drives a real Ethereum or Bitcoin reorg into one.

Acceptance criteria:

* a reader that constructs certificates from chain data, with the `block_hash`-to-`chain` binding
  proven (reuse the `x3-verification-router` receipt path already exercised against anvil); a
  certificate whose hash does not match the block at that height is refused, not repaired;
* the accepted/witnessed tips persisted and reloaded, with a restart test: accept tip `T`, restart,
  present a certificate at tip `< T`, and require `CertificateRewindsAcceptedAnchor`;
* one live drill per chain family (EVM at minimum) that produces a certificate, settles on it, then
  produces a certificate from a rewound fork and requires the refusal from real data.

Validation: `cargo test -p x3-atomic-swap` stays green, the restart test fails with the persistence
removed, and the reorg drill fails when `CertificateRewindsAcceptedAnchor` is removed.

## TICKET-153 — the halt exemption list is a policy, and nothing checks it against the runtime — 2026-09-26

Found while closing X3-RT-001 (`6d7bfc540`). A halt that refuses *everything* also refuses its own
remedy, so `RuntimeInvariantCheck`-style gates now consult `pallet_x3_invariants::Config::HaltExemptCalls`
(`RuntimeHaltExemptCalls` in the runtime): `clear_halted`, `set_halt_on_violation`, `resume_transfers`,
`rollback_atomic_bundle`, `emergency_unpause`, and the council motion that reaches them. Everything
else is still refused while `Halted` is set.

What that does **not** prove, and what this ticket is for:

1. ~~**The list can go stale.**~~ **CLOSED 2026-09-27.** The completeness gap is now a gate.
   `scripts/ci/check-halt-fund-holding.py` parses every pallet the runtime wires (`construct_runtime!`)
   for dispatchables that can reach `reserve` / `reserve_named` / `hold` / `hold_named` / `set_lock`
   (through same-file helpers) and compares them with the reviewed inventory in
   `security/halt-fund-holding.toml`; it is wired into `scripts/local-ci.sh` (gate
   `halt fund holding`). It fails on a fund-holding call with no entry, an entry that outlives its
   call, a `transient` claim whose call reaches no release primitive, a `permanent_charge` whose
   amount is not a `*Fee` constant, and a `recoverable_by`/`while_halted` claim the code does not
   support. Measured inventory: **26 fund-holding dispatchables**; **1 releasable while halted**
   (`X3AtomicKernel::rollback_atomic_bundle`), **2 transient** (`AtlasKernel::submit_cross_vm_operation`
   / `prepare_cross_vm_operation`, which free what they hold in-call), **23 exceptions** whose funds
   the halt keeps locked until `Council::close` clears it. Four break-it-first controls
   (delete the atomic-bundle entry; relabel `governance::submit_proposal` transient; point
   `x3-slash`'s `recoverable_by` at a non-release call) each go red, then restore green
   byte-identically (`.ai/runlogs/halt-fund-holding-check-20260927T055706Z/`). The two genuine
   traps it found — a reserve that is never released at all — are TICKET-154.
   `emergency_unpause` is still on the list on the reasoning that a paused kernel is the routine
   case, not because a test drives a halt with a paused kernel and requires recovery; that stays open
   under item 3.
2. ~~**Nothing measures the halt/unhalt cycle on a node.**~~ **CLOSED 2026-09-26.**
   `scripts/drills/halt_recovery_live.sh` (gate `halt recovery on a live chain`, PASS in ~25 s) starts
   three validators, trips the halt through a council motion, requires two different validators'
   pools to refuse a transfer with the halt code while the balance stays put, requires the
   bond-releasing rollback to be *included* and refused by the pallet instead, clears both flags
   through council motions submitted while the chain is still halted, and requires traffic to resume.
   With the exemption check removed the same drill fails at the remedy, which is the one-way door.
3. **`clear_halted` has no benchmark.** Its weight is hand-copied from `set_halt_on_violation`'s
   shape (one storage write) with a comment saying so.
4. ~~**There is no automatic unhalt.**~~ **CLOSED 2026-09-27.** The re-raise sequence is now tested
   (`d62fecf6b`): `pallets/x3-invariants/src/tests.rs::the_halt_re_raises_while_the_violation_persists_and_sticks_after_remediation`
   arms the policy, violates `MaxSupply`, requires `Halted`; clears it while the violation persists,
   requires the next `enforce_all` to re-raise it, then remediates the bound and requires the clear
   to stick (and `ViolationCount` to have counted exactly the two violating blocks). The control —
   making `clear_halted` also set `HaltOnViolation(false)` — fails the test at the re-raise
   assertion, then restores green byte-identically
   (`.ai/runlogs/halt-sequence-test-20260927T060113Z/`). The halt branch ends in `defensive!`, which
   panics under `debug_assertions` while a release runtime only logs, so the test catches that one
   panic and requires it to be the defensive failure (the `x3-supply-ledger` idiom). "Automatic
   unhalt" remains *deliberately absent* — remediation still needs governance — which is the
   intended design and is what the sequence proves.

Acceptance criteria:

* a checker that lists the runtime's fund-holding calls (those that reserve, escrow, lock or bond)
  and fails when one of them is neither on `RuntimeHaltExemptCalls` nor accompanied by an exempt
  rollback/refund path, with the list of exceptions written down in the checker;
* a live drill: three-validator network, halt through the kernel, a pending bundle's bond released
  while halted, both flags cleared through a council motion, and an ordinary `balances.transfer` that
  was refused during the halt landing afterwards;
* `clear_halted` re-benchmarked rather than copied;
* a halt-and-remediate-then-clear sequence test: while the violation persists the gate re-raises the
  flag, and once the bound is back in range `clear_halted` sticks.

Validation: the checker fails on a deliberately-removed exemption; the drill's post-clear transfer is
`InBlock`; the re-benchmarked weight is within the copied value's order of magnitude or the copied
value is replaced.

**Items 1 and 4 status: CLOSED 2026-09-27** (`a91cf63a0`, `d62fecf6b`). Item 2's live drill is
closed under item 2 above (the pending-bundle-bond half) and item 3 (`clear_halted` re-benchmark,
hand-copied weight) remains open.

## TICKET-154 — two pallets reserve an anti-spam fee and never release it — 2026-09-27

Found by the TICKET-153 completeness gate on its first run. `pallets/x3-da` and `pallets/x3-sequencer`
both charge their per-byte anti-spam fee by calling `T::Currency::reserve(&submitter, fee)` — and
neither pallet contains a single `unreserve`, `slash_reserved` or `repatriate_reserved`, so the
reserved balance is never touched again.

* `pallets/x3-da/src/lib.rs::submit_blob_commitment` reserves `PerByteFee * size_bytes`
  (`x3-da` has no other dispatchable that releases funds; `submit_shard_proof` only writes storage).
* `pallets/x3-sequencer/src/lib.rs::submit_transaction` reserves `BaseFee + PerByteFee * payload_size`
  (`x3-sequencer` has one dispatchable, this one).

A `reserve` is a bond: the funds stay in the account and are unspendable until one of the release
primitives frees them. With none, the submitter's balance is locked forever — indistinguishable from
a fund trap, and exactly the class TICKET-153 was written to surface. The comment in both pallets
says the amount is a *fee*, where a charge should leave the account (a transfer to the treasury, or
`withdraw(.., WithdrawReasons::FEE, ..)`), not sit in `reserved`.

The gate records this honestly as `disposition = "permanent_charge"` (the amount is a named `*Fee`
constant) rather than pretending a release path exists; that disposition is a *finding*, not a
pass. What is not decided here — and is the owner's call because it moves funds and changes an
economic path — is which charge primitive replaces the reserve: burn the fee (`withdraw` and drop
the imbalance), forward it to the treasury, or turn it into a refundable bond with a real release
call. Until that is chosen, both calls are listed as exceptions in the halt gate.

Validation for the fix: a `cargo test -p pallet-x3-da` / `-p pallet-x3-sequencer` test that a
submission's `reserved_balance` returns to zero (or that the fee lands where the chosen policy says),
and the halt gate's entry for each call changes from `permanent_charge` to `recoverable`/`exempt`.

**TICKET-154 — CLOSED 2026-10-07 (operator chose: forward to the treasury).** Both pallets now
charge the fee with `Currency::transfer(.., T::ProtocolTreasury::get(), .., KeepAlive)` and emit
`DaFeeCollected` / `SequencingFeeCollected`; neither reserves anything, so both entries were removed
from `security/halt-fund-holding.toml` (the call no longer holds funds, and the checker passes).
Unlike the router (GAP-ROUTER-FEE-DEPOSIT) the fee is **not waived** when the treasury refuses a
deposit below the existential deposit — a waived anti-spam fee makes spam free — so the call fails
with the new `FeeDestinationRefused`, distinct from the payer's `InsufficientFee`. That makes a
funded treasury a launch requirement for any chain running these pallets (genesis must endow
`TreasuryAccountId` with at least the existential deposit). The benchmarks fund the treasury in
setup; the weights were measured with a reserve and should be re-run. Tests:
`cargo test -p pallet-x3-da -p pallet-x3-sequencer --features runtime-benchmarks` (20 + 19), each
pallet with a fee-reaches-treasury, a dead-treasury refusal, and an insufficient-funds case.

## GAP-TOOLCHAIN-WIPE — `~/.cargo/bin` loses everything that was installed after the base image — 2026-09-26

Not a repository defect, but it stopped work twice in one day, so it is recorded where the next
agent will look. **Measured, twice (once at ~11:56 local, once at ~18:43):**

* **gone** — every *regular file* in `/home/lojak/.cargo/bin`: the `rustup` binary itself, the
  `subkey` the launcher needs, and everything installed with `cargo install` (`cargo-audit`,
  `cargo-deny`, `srtool`). Those are exactly the files that did not exist in the base image.
* **survived** — the rustup *shim symlinks* (dated Sep 2), the toolchains under `~/.rustup`
  (Sep 2, hundreds of MB), `~/.cargo/registry` and `advisory-db`, `~/.local/bin` (Sep 20), and the
  Homebrew tools (`promtool`, `prometheus`, `grafana`, `fluent-bit`). So it is neither a HOME reset
  nor a disk-space sweep: it is a targeted removal of the base image's *absence*.
* **what was ruled out** — no repository script writes or deletes there (`rg '\.cargo/bin'` finds
  only scripts that *copy tools in*); no cron entry or user timer; the operator's `~/.bash_history`
  (last written Sep 24) has no destructive command; and today's Codex session transcripts contain no
  `rm`/`rustup self uninstall` against that path — they contain three sessions *discovering* the
  damage within two minutes of each other.
* **who wrote it back** — at 18:43 `~/.cargo/env` was rewritten and by 18:45 `~/.cargo/bin` was
  **root-owned** (my own shells run as `lojak`, uid 1000, so this was another actor on the box), then
  I reinstalled it at 18:56 as `lojak`. At least three agent sessions run concurrently on this
  machine, and each reacted to the missing toolchain by reinstalling rustup into the same HOME. An
  interrupted install leaves exactly the observed state — shims present, binaries gone — so the
  repairs are themselves a source of churn.

**Honest limit:** the first deleter is not identifiable from what is on disk. What is proven is the
*scope* (only post-image files under `~/.cargo/bin`), the *not-repo* part, and that at least one
concurrent session reinstalled into that directory as root while repairing the same damage.
OpenClaw, the second agent framework on this box, keeps its state in SQLite and its records stop at
03:36 today, so it does not describe the later events either.

**ROOT CAUSE FOUND (same day, 2026-09-27 01:15 UTC) — it is `Swatinem/rust-cache`, running on the
self-hosted runner.** The "honest limit" above is now closed; the earlier note that "no repository
script deletes there" was true and incomplete — the repository *calls an action* that deletes there.
The evidence, in the order it was found:

* **The actor.** `actions.runner.Cyptopimpinainteazy-xxxstar.x3star1.service` is an enabled systemd
  unit. The runner executes as `lojak`, so it shares `$HOME` with the developer shells. This is the
  only non-interactive actor on the box with write access to `/home/lojak/.cargo/bin`.
* **The timestamps.** `_diag/Worker_*.log` records a `Post Cache cargo` step (`dist/save.js`) at
  **17:55:20 UTC** and again at **00:45:48 UTC**. The two observed wipes were reported at ~11:56
  local (17:56 UTC) and ~18:43-18:45 local (00:43-00:45 UTC). The job ending at 00:45:52 UTC is
  `svm-live / SVM HTLC live validator lifecycle`, whose `x3vm-svm-live-lifecycle.yml` has a
  `Cache cargo` step; the job ending 17:55:23 UTC is the same workflow.
* **The mechanism.** `Swatinem/rust-cache`'s save step (`src/save.ts`, pinned rev
  `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` for `@v2`) calls `cleanBin(config.cargoBins)` when the
  `cache-bin` input is true — and `action.yml` declares `default: "true"` (the runner's own manifest
  expansion in `Worker_20260927-003230-utc.log` line 7739 confirms the effective value `true`).
  `config.cargoBins` is snapshotted as **every regular file in `$CARGO_HOME/bin`** when the action's
  restore step runs; `cleanBin` then walks that directory and unlinks each one
  (`src/cleanup.ts`: `if (dirent.isFile() && binsToRemove.has(dirent.name)) await rm(...)`). It is
  deterministic, not a race, and it is not an interrupted anything.
* **The signature matches exactly.** `cleanBin` filters on `dirent.isFile()`, and `Dirent.isFile()`
  is false for a symlink, so it removes the regular files (`rustup`, `subkey`, `cargo-audit`,
  `cargo-deny`, `srtool`) and leaves the `-> rustup` shims behind. That is precisely what was
  observed twice, and it is why `~/.rustup` and `~/.local/bin` were never touched.
* **Other things the same save step removes**, for the record: `$CARGO_HOME/credentials.toml` is
  unlinked (`cleanRegistry`; it is absent on this box right now), and the registry
  index `.cache`/src/cache and `target/` dirs are pruned to what the job's dependency graph needs.
  That is the real source of the repeated "the registry cache/target dir vanished" churn, not just
  the bin wipe.
* **Why it stopped looking like a mystery.** The deletion is done by an action the workflows
  *invite*, so `rg 'rm -rf' scripts/ .github/` finds nothing, no cron exists, and the only trace is
  a runner step name. The root-owned state seen at 18:45 was the *repair*, not the deletion: two
  runner workers plus agent sessions all reinstalled `rustup` into the same HOME.
* **Why it is safe upstream and not here.** `rust-cache` assumes a disposable runner whose
  `$CARGO_HOME` is job-scoped. On a hosted runner `cleanBin` is a no-op nobody notices. On this
  machine `$HOME` is the developer's, and the prune is against the shared toolchain.

**FIXED — repo side.** All **24** `Swatinem/rust-cache` steps across **17** workflow files now set
`cache-bin: false` (which disables `cleanBin` and nothing else), and
`scripts/check-cargo-home-safety.py` refuses any workflow that calls that action without it. It is
wired into the default gate set as `cargo home safety`:

```bash
bash scripts/local-ci.sh --only cargo-home-safety      # PASS, 24 steps / 17 files verified
python3 scripts/check-cargo-home-safety.py --workflows-dir <dir-with-a-reverted-copy>  # FAIL, exit 1
```

Break-it-first was run in both directions: removing one `cache-bin: false` from a copy of
`x3vm-svm-live-lifecycle.yml` makes the gate report that exact file, job and step and exit 1.

**Residual risk (not fixed):** `cache-bin: false` stops the toolchain deletion, but the same save
step still prunes the registry index/src/cache and `target/` and still unlinks
`$CARGO_HOME/credentials.toml`. Stopping that means either running these jobs with a job-scoped
`CARGO_HOME` (and paying a toolchain install per job) or dropping `rust-cache` from the self-hosted
jobs. Neither is needed to keep the toolchain alive, so both are recorded rather than done. Also
still missing and needed by `four-validator-mesh.yml` and
`scripts/testnet/x3-testnet-verify.service` (`Environment=SUBKEY_BIN=.../subkey`):

```bash
cargo install subkey --locked --git https://github.com/paritytech/polkadot-sdk --branch stable2512
```

## RUNTIME-ATTESTATION — 64 runtime-graph files moved since `335a27d8c`, and the rebuild is the last release-gate step — 2026-09-27

The release gate rebuilds the runtime and fails when any hash differs from
`docs/reports/runtime-wasm-hashes.json`, so every runtime-affecting change has to re-attest that
record. It is the one named requirement of the public-testnet goal ("every release gate green")
that is step-wise complete but **not yet performed**, and the reason is scheduling, not code.

**Measured state (2026-09-27 01:49 UTC, at `e9c4b481e`):**

```bash
bash scripts/local-ci.sh --only runtime-hash-freshness   # FAIL, 15s
```

> `[runtime-hash] 64 changed file(s) since the record was taken at 335a27d8c can alter the runtime
> that mainnet governance attests to, but docs/reports/runtime-wasm-hashes.json did not move`

The 64 include every lane's landings today: `crates/x3-atomic-swap/{scoreboard,adapter,lib}.rs`
(the internal/external split), `crates/x3-integration/**`, `crates/x3-dex/**`,
`crates/x3-order-window/**`, `pallets/x3-kernel/src/**`, `pallets/x3-invariants/**`,
`pallet-x3-control`, `x3-wallet`, `x3-backend`, `runtime/src/lib.rs` and more. The gate names all
of them, so this is the complete list, not a guess.

**Why it is still open.** `./scripts/update-runtime-hashes.sh` builds the runtime **twice** (it
removes srtool's target dir between builds and refuses to write anything unless the two agree) and
takes ~30 minutes. It has to run alone: this box is also the self-hosted CI runner, and an attempt
at 01:45 UTC was aborted after six minutes because the runner was *already* executing
`./scripts/run-srtool.sh build` in its own checkout
(`/home/lojak/actions-runner-2/_work/xxxstar/xxxstar`, container up since ~01:35 UTC). Aborting is
safe and wrote nothing — the script only touches the record after both builds agree — and the
partial `runtime/target/srtool` it leaves is cleared by the next run's first step.

**Do this when the box is quiet** (no `docker ps` srtool container, no `run-srtool.sh` in `ps`):

```bash
./scripts/update-runtime-hashes.sh                     # ~30 min, two builds, writes on agreement
bash scripts/local-ci.sh --release --only 'release-gate-(mainnet-check)'
python3 scripts/feature_matrix.py check                 # the record's revision line moves nothing else
```

If the WASM turns out to be byte-identical (a path the runtime never instantiates), the hashes stay
and only `recorded_revision` moves — that is a legitimate outcome, not a failure to attest.

**Also waiting on that quiet tree:** `reports/rc6/*` are dirty in the working tree and read FAIL
from the earlier wasm-builder collision with a concurrent build; regenerate them on the quiet tree
after the attestation rather than committing the collision's output. Those files, and
`reports/panic_unwrap_audit.md`, are another lane's leftovers — nobody owns them right now, and they
should be either regenerated or reverted by whoever takes the attestation.

## PANIC-RATCHET — the instrument was broken in both directions, and the real number is 520, not 516 — 2026-09-27

Found while chasing the release gate's stage 4b, which failed on the **panic ratchet** during the
first green-looking run after the WASM re-attestation:

> `panic_unwrap_audit: FAIL — production panics/unwraps grew: 516 -> 519`

The message was true and the comparison behind it was meaningless, because the measuring instrument
had changed underneath the baseline and nothing noticed.

**The bug.** `scripts/audit/panic_unwrap_scan.py`'s brace matcher stripped `"…"` strings and
comments before counting braces but did not understand **raw strings** (`r"…"`, `r#"…"#`,
`br##"…"##`). A `#[cfg(test)]` fixture written as a multi-line raw string therefore pushed its own
`{`/`}` into the depth counter, the test module appeared to close early, and everything after it in
the file was classified as production code. Measured on the two trees with the same fixed scanner
versus the buggy one:

| tree | buggy scanner | fixed scanner |
| --- | --- | --- |
| `b6520e7d35` (the commit the baseline was recorded at) | 475 | **477** |
| `HEAD` (`400985f9e`) | 519 | **520** |

So the bug did both things at once: it invented **11** test-code false positives (ten in
`crates/external-chains/src/evm_rpc.rs`, one of them mine in
`crates/x3-atomic-swap/src/scoreboard.rs`) and it *hid* **12 real production sites** in
`crates/external-chains/src/chains/{base,universal}.rs` and `crates/x3-lsp/src/diagnostics.rs` —
finding for finding, the net looked like small growth.

**The real movement, same scanner on both sides: 477 → 520 = +43 production panic/unwrap sites**
since 2026-09-21. Grown in 19 files, led by `crates/gpu-swarm/src/admin.rs` (+10),
`node/src/chain_spec.rs` (+8), `crates/gpu-swarm/src/crown/scrapyard.rs` (+7),
`crates/x3-order-window/src/lib.rs` (+3), `crates/x3-accel/src/lib.rs` (+3). Block-hook panics and
pallet-call panics are both still **0**, so nothing here sits in `on_initialize`/`on_finalize` or in
a `#[pallet::call]` body; the growth is on ordinary production paths that a release node build
includes.

**Fixed in the instrument, not in the number:**

* raw strings are stripped like other literals, with two self-test cases in
  `scripts/audit/panic_unwrap_self_test.py` (a multi-line raw-string fixture inside `#[cfg(test)]`
  followed by a real production panic — both halves asserted);
* the baseline now carries `scanner_sha256`, and the audit **refuses to compare** a count taken by a
  different scanner instead of silently reporting motion. That is the part that makes the ratchet a
  ratchet: before this, any scanner change re-based the metric without anyone deciding to.

**Re-baselined deliberately** to `520` at `400985f9e` (the ratchet's own instruction is to refresh
and say why). The growth is recorded here rather than absorbed, and the burn-down is the next task:
the 19 files above, starting with the four that account for 31 of the 43. Do not raise this baseline
again without a reason in the commit message — the instrument now makes that a deliberate act, which
is the property that was missing.

## RELEASE-GATE — where it stands after the re-attestation, and the hang that bounded it — 2026-09-27

The gate is `make mainnet-check` (one local-ci gate, `--release --only 'release-gate-(mainnet-check)'`,
~30 minutes on a quiet box). Its state at `52165c585`, from three runs:

* **Stage 6b — the runtime hash rebuild — PASSED and matched the new record.** Both the compact and
  compressed hashes rebuilt in `paritytech/srtool:1.93.0-0.18.4` equal
  `docs/reports/runtime-wasm-hashes.json` (`0xce698445…` / `0x4971c1fc…`). That is the proof the
  re-attestation of `400985f9e` needed, and the reason this section exists rather than a claim.
* **Stage 4b — the panic ratchet — was the first run's only failure** (516 -> 519). Fixed under
  `52165c585`: the instrument bug, the pinned scanner, and a deliberate re-baseline to the corrected
  520. `bash scripts/mainnet/panic_unwrap_audit.sh` now reports `0/0/520` and PASSes.
* **The second run's failure reason was lost** — local-ci pruned the per-gate log before it could be
  read, and its `.reason` file was empty. Do not trust "it failed" as a diagnosis; re-run with the
  output captured (`make mainnet-check > <file> 2>&1`, not through local-ci) when the reason matters.
* **The third run hung for 52 minutes in stage 6b** with the srtool container at 0% CPU, on
  `Updating git repository https://github.com/paritytech/polkadot-sdk` — while a CI job ran its own
  srtool build from the actions-runner checkout and the host could `git ls-remote` that repo in four
  seconds. Two builds sharing the docker cargo volume is enough to stall the fetch, and nothing in
  `run-srtool.sh` bounded it, so the gate never finished.

**Fixed in `run-srtool.sh`:** every build now runs under `timeout --kill-after=30`
(`SRTOOL_BUILD_TIMEOUT`, default 2700s). The first attempt at this did **not** work and the test
showed why, twice: `set -e` aborts on the failing pipeline before the check runs (`|| rc=$?` fixes
it), and `timeout` alone never returns because the docker CLI forwards SIGTERM to a container whose
build ignores it (the container is now removed by name on the way out). Measured after the fix:
`SRTOOL_BUILD_TIMEOUT=5 ./scripts/run-srtool.sh build` exits 1 with the reason and leaves no
container behind.

**Next step, and the only thing between this and "every release gate green":** re-run
`make mainnet-check` with the box quiet — no `docker ps` srtool container, no `run-srtool.sh` in
`ps`, and preferably with `SRTOOL_CARGO_GIT_CACHE` pointing at a world-readable copy of a warm
`~/.cargo/git`, which takes the polkadot-sdk fetch off the critical path entirely (the script's own
header documents the mount; measured 2026-09-25: 30+ minutes in that fetch versus seconds with the
cache). Capture the output to a file this time.

## RC2/RC6 SEQUENCE — seven defects between a red sequence and a truthful one — 2026-09-27

`reports/rc6/*` said **FAIL** and the sequence behind it had never run on this box. Fixing it turned
up seven distinct defects, each measured rather than guessed. They are listed here because six of
them are shapes that will recur (a stale absolute path, a launcher that does not produce blocks, a
cargo target that needs a feature, a spec that pins an old runtime, an authorized-but-unfunded
account, and a client library that cannot decode the chain).

| # | defect | evidence | fix |
| --- | --- | --- | --- |
| 1 | `run_release_gates_rc6.sh` hardcoded `ROOT=/home/lojak/Desktop/X3_ATOMIC_STAR` | that path does not exist; every step `cd`'d into nothing and "failed" instantly — the FAIL in `reports/rc6/*` was the script's own path | derive `ROOT` from `BASH_SOURCE` |
| 2 | `rc2_mock_and_live_gate.sh` hardcoded the same path | worse: its `mkdir -p "$ROOT/reports/rc2"` **created** the empty directory, then it failed with `manifest path tests/e2e/Cargo.toml does not exist`, which reads like a missing crate | derive `ROOT`; the stray tree (0 files) was removed |
| 3 | the live suite needs `--features real-chain` | `error: target live_internal_mainnet_e2e in package e2e_tests requires the features: real-chain` | pass the feature |
| 4 | gates that need finality booted `scripts/start-x3-chain.sh` (`--chain dev`, no session keys) | node log: `Failed to trigger bootstrap: No known peers`; the smoke died with `block height/finality did not advance` | new `scripts/mainnet/local3_lib.sh` boots the three-validator `local3` network and waits for a **finalized** height; both rc2 gates use it and stop only what they started |
| 5 | the committed `chain-specs/x3-local3-raw.json` is from Sep 25 | it handed the smoke `spec_version 11` while the tree's runtime is 20 — a gate reporting on code it is not running | `local3_lib.sh` builds a raw spec from the **current binary** into its temp dir unless a caller names one |
| 6 | `local3` genesis *authorized* the gateway accounts but never *endowed* them | `EnsureX3LangGateway` lets `//x3-atomic-gateway` submit `xvmTransfer`, and the call failed with `1010: Invalid Transaction: Inability to pay some fees` | `local3` now extends `atomic_gateway_endowed_accounts()`, as `development_config`, `staging_config` and `testnet_config` already did |
| 7 | the rc2 smoke's JavaScript driver used `HttpProvider` | `HttpProvider` cannot subscribe, so `signAndSend` callbacks arrive with no `status` → `TypeError: Cannot read properties of undefined (reading 'isInBlock')`; the driver computed a `wsRpc` and never used it | the driver talks WS |

**Retired, not silenced:** the same driver still cannot decode this chain — `createType(ExtrinsicUnknown)::
Unsupported unsigned extrinsic version 5` for every block, because the runtime's extrinsics are
version 5 and the pinned `@polkadot/api` in `packages/blockchain-connector` knows version 4. The rc6
sequence's step 2 is therefore recorded as retired, with that reason, instead of failing every run.

**The ticket this creates.** The JS smoke swept all six internal routes (X3Native/X3Evm/X3Svm pairs)
plus nine negative cases — external route, wrong recipient per domain, wrong sender type, duplicate
message, duplicate nonce, refund-after-finalize, refund-before-expiry, completion-after-refund — with
a supply-invariant check at the end. The Rust live suite that replaces it as evidence
(`tests/e2e --features real-chain --test live_internal_mainnet_e2e`) has four tests: node progress and
required RPC methods, bridge-proof crypto and full accounting paths, timeout expiry, and reordered
delivery / duplicate ack rejection. **Port the route sweep and the negative matrix into the Rust
suite** (or raise the JS client to a polkadot-js that understands extrinsic v5), then delete the JS
driver. Until that lands, the sequence's green is not the same breadth of green it used to claim.

Measured after the fixes: `rc2_internal_settlement_smoke.sh` reaches the chain and submits
transactions; `rc2_mock_and_live_gate.sh` passes both halves (`PASS: mock suite and live suite both
passed`, 16 + 4 tests).

## X3-ECO-002 — the distributed proof passes, and the driver is what was broken — 2026-09-27

The row's open blocker was *"the invariant is proven for one ledger view in one process — not under
concurrent cross-domain traffic on a multi-validator network"*. The test for that already existed
(`node/tests/supply_invariant_distributed.rs`) and was failing — for a reason in the driver, not the
chain:

> `complete_xvm_transfer 5: :19966 refused the submission: 1014: Priority is too low: (419 vs 419)`

The SDK's own description of `POOL_TOO_LOW_PRIORITY` is *"the transaction has too low priority to
replace another transaction already in the pool"* — a **nonce collision, not pool backpressure**. The
completion loop signed every `complete_xvm_transfer` up front and then submitted them; because
`sign_complete_xvm_transfer` reads the gateway account's nonce from the node, every signature carried
the *same* nonce, so each validator's pool accepted the first and refused the rest. The transfer loop
twenty lines above documents exactly this ("each submission has to follow the previous one's
inclusion or the nonces collide") and follows it; the completion loop did not. Fixed by waiting for
each completion's effect — the pending counter falling by one transfer's amount — before signing the
next.

**After the fix the proof runs green (exit 0, 271s, one frozen node binary for every node):**

```text
:19964/:19965/:19966 at 119 — 10 accounts, accounted 9999999998338644985,
                              TotalIssuance 9999999998338644985 — conserved
pending phase at 159 — native 999,994,000,000 (was 1,000,000,000,000), pending 6,000,000
                       on all three validators, agreeing
pending phase at 191 — native 999,994,000,000, evm 6,000,000, pending 0 —
                       every leg resolved, on all three validators
```

with the corrupted-chain control still refusing (a scratch chain carrying one extra unit in
`Balances::TotalIssuance` is reported as a violation with the delta named). X3-ECO-002's second
blocker is closed on that evidence; what remains open there is a public-network run and the
per-bridge observation path (X3-XVM-002).

Two process notes worth keeping. A first attempt failed with a *different* error — `chain_getFinalizedHead
on :19964 failed: Connection refused` — while the other two validators were conserved at the same
block: another lane's live gate was running concurrently and its cleanup killed my nodes. The test
itself refuses to start unless its ports are free (`assert_ports_free`), so the only fix is to
serialize; a clean run needs the box quiet, and a failure of that shape is environmental, not a
conservation violation. Second, a stray commit-then-push window meant one agent's commit
(`feat(snapshot)`) rode along with a push of mine before I had verified it; it was verified
afterwards (53 crate tests, three snapshot gates) and one defect in it was found and fixed
(`snapshot-zero-downtime-proof.sh` was missing the `--regenesis` its own comment requires).

**Mitigation taken:** `cargo-audit`, `cargo-deny` and `srtool` are now also installed in
`/home/lojak/.local/bin` (on PATH, and untouched by every event so far), so the `dependency audit`
gate and the release gate keep working if `~/.cargo/bin` is emptied again. `rustup` itself cannot be
relocated, but `rustup-init.sh` restores it in seconds and the toolchains under `~/.rustup` survive
every time.

**If it happens again:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
--no-modify-path --default-toolchain 1.90.0-x86_64-unknown-linux-gnu`, then copy the three tools
back into `~/.cargo/bin` from `~/.local/bin`. Do it as one actor: concurrent installers into the
same directory are what makes this look worse than it is.

---

## GAP-SNAPSHOT-REGENESIS — a restored state snapshot was a chain that panicked on its first block — 2026-09-27

**How it was found.** The snapshot row's last open item was that the archive is cut with the node
stopped, so `scripts/snapshot-zero-downtime-proof.sh` was written to take it the other way: export a
*running* node's state over RPC at a finalized, GRANDPA-justified block and build the snapshot from
that. The export half worked on the first try — `x3-state-snapshot root --from-raw-spec` recomputed
the trie root from the exported entries and it equalled the `stateRoot` the chain published in that
block's header — and the drill then booted the restored spec, which is the step nothing had ever
done. The node answered RPC and then failed every block it tried to author:

```
assertion `left == right` failed: Block number must be strictly increasing.
  left: 513
 right: 1
```

**The cause.** `x3-state-snapshot restore` wrote the state verbatim into `genesis.raw.top`. A raw
spec is the state itself, so it carried the *producing* chain's `frame_system` bookkeeping — notably
`System::Number` = 513 — and `frame_system::initialize` opens every block with
`assert_eq!(Self::block_number() + 1, *number)`. A genesis is defined by those keys being absent:
`x3-chain-node build-spec --dev --raw` carries no `System::Number` at all, so the runtime's storage
default of zero applies. Reproduced on its own in seconds by patching that one key to `0x05000000`
in an otherwise untouched raw genesis and booting it (124 panics,
`.ai/runlogs/snapshot-zero-downtime-20260927T0013Z/reproduction-number-not-zero.txt`). This was a
real defect in the restore path, not a drill artifact: the CLI's own help said "boot a node with
`--chain <spec>` to build a database from this state", and no node could.

**The fix.** `restore --regenesis` drops the six `frame_system` entries that record where the chain
*was* (`CHAIN_BOOKKEEPING_ENTRIES`; keys derived with `sp_core::twox_128`, and the `System::Number`
key is pinned in a test against the well-known `:number` constant every client uses). It is opt-in,
because the honest default for an existing restore is "verbatim", and it reports *both* roots rather
than presenting one as the other: the snapshot's declared root and the regenerated genesis root (they
differ by exactly those keys), plus the list of dropped keys, in the spec's `properties` so the
provenance travels with the file. With it the restored chain boots as an authority, finalizes, and
its non-bookkeeping entries are byte-identical to the export.

**What is still open on this row, and why it is not mainnet-ready.** The export reads one RPC call
per key (1881 keys in ~2.4 s on loopback); no batched or range reader has been written, so the
multi-GB size class and the wall-clock an operator would spend exporting are both unexercised. The
anchor must be inside the node's pruning window — the gate demonstrates the *refusal* (a bounded node
refuses an anchor it pruned, names pruning, and writes no spec) but the operator path that follows
from it is documented guidance, not automated. `--regenesis` drops a fixed list of six entries;
nothing enumerates every chain-local key a third-party pallet might keep. And a restore produces a
*new* chain, not a continuation: state-sync for an existing chain still needs the base-path path or
warp sync.

**Gate:** `snapshot zero downtime export` (serial, ~4 min; `bash scripts/local-ci.sh --only
snapshot-zero-downtime-export`). Red evidence for the fix: removing `--regenesis` from the drill's
restore makes the gate fail
(`.ai/runlogs/snapshot-zero-downtime-20260927T0013Z/red-without-regenesis.log`).

## GAP-ROUTER-FEE-DEPOSIT — the signed cross-VM transfer was unusable below `ED * 10_000 / bps`

**Found 2026-09-27** while driving X3-ECO-002's pending-supply invariant through the router on three
validators. `X3CrossVmRouter::xvm_transfer` charges `amount * RoutingFeeBps / 10_000` to the signing
account and sends it to the protocol treasury. The currency refuses any deposit that would leave the
*destination* below the existential deposit (`pallet-balances` `can_deposit` → `BelowMinimum`; SDK
checkout `substrate/frame/balances/src/impl_fungible.rs`), and the treasury account has never existed
on a dev/local chain — the runtime's transaction fees do not go to it. So every transfer whose fee is
below `ExistentialDeposit = 100 * MICRO_ATLAS` was refused with `RoutingFeeNotAffordable`.

Measured by post-mortem on the kept chain (block
`0x6523cca5195dcbac4d421943ae8286b81f8ed78ed5d8992a4102057f3f99e3c5`; the test keeps its data dir
on failure): the refused extrinsic decoded as pallet `0x1a` = `X3CrossVmRouter`, call `0x00` =
`xvm_transfer`, signed by the X3Lang gateway `4c81d416…`; the router's error code was variant **36**
`RoutingFeeNotAffordable`; and the payer's `System::Account` at that block read
`free = 999_999_999_900_043_000`, `frozen = 0`, `flags = NEW_LOGIC`, i.e. nothing locked and nothing
spent except the extrinsic fee. The treasury's `System::Account` key **did not exist**. The amount
was 1_000_000 and the fee 2_000.

Two reasons the existing evidence could not see it. The router's own mock wires `RoutingFeeBps = 0`,
so the branch never ran in `pallets/x3-cross-vm-router/src/tests.rs`. And the rc2 smoke drives every
route at `amount = 10`, whose fee is 0, so it skipped the branch too — its six-route results
(`reports/rc2/six_route_results.json`, committed in `681e2e260`) show the routes failing *later*, at
completion, while never exercising the fee.

**Fixed** in `ded7e558d`: `do_initiate_transfer` now asks the payer's balance which side refused.
Funds present ⇒ the refusal is the destination's, the fee is waived and a distinct
`XvmRoutingFeeWaived` event records it (a waived fee is uncollected revenue and must not be silent).
Funds absent ⇒ `RoutingFeeNotAffordable` stands unchanged. Three pallet tests cover all three
branches, measured break-it-first. Residual: waived fees are not accrued, and nothing yet requires an
operator to keep the treasury funded — a policy decision, recorded on row X3-XVM-014.

**Re-attestation ordering:** `crates/x3-cross-vm-router` is in the runtime graph, so this change
invalidates any runtime attestation taken before it. An srtool run was in progress on this box while
this landed, and several lanes were still editing runtime files at 02:15 local. The release
attestation has to be taken after the last runtime-graph change.

## GAP-RC2-DRIVER-FORMAT — the six-route live gate cannot read the chain, and reported a driver bug as a chain bug

`scripts/mainnet/rc2_internal_settlement_smoke.sh` is the only thing that drives all six internal
routes plus the router's negative cases on a live chain. Step 2 of the rc6 sequence is **retired**,
with the reason recorded in `681e2e260`: this driver cannot decode the chain at all. Its pinned
`@polkadot/api` understands transaction format v4 and the chain emits v5 (`Unsupported unsigned
extrinsic version 5` on every block). So the sequence no longer runs it, and nothing else exercises
the six routes end to end.

Before it was retired it also reported a **driver** bug as a chain bug. Every route's completion was
signed by `alice`:

```js
await submit(api, alice, api.tx.x3CrossVmRouter.completeXvmTransfer(messageId), …)
```

But `complete_xvm_transfer` is gated on the same `EnsureX3LangGateway` origin as `xvm_transfer`, and
on a dev/local chain the only account authorized for `GatewayRole::X3Lang` is `//x3-atomic-gateway`.
Alice is not it, so the call could only ever be refused with `BadOrigin`. The committed
`reports/rc2/six_route_results.json` shows exactly that shape: every route with `source_delta -10`,
`pending_after_transfer 10`, `destination_delta 0` and `pending_zero false` — the debit moved and the
completion never did. A reader would file that as a broken router. The same commit fixed the same
mistake in the refund cleanup call and missed this one.

**Fixed 2026-09-27:** the completion is submitted by the gateway, and the file now asserts *first*
that a non-gateway completion of a `SourceDebited` transfer is refused, so the origin requirement is
pinned while it is real rather than after the fact. **Unproven:** the fix cannot be run until the
driver can decode the chain, so it is verified only by `node --check` and by the runtime's own origin
wiring. The real repair for the coverage is a Rust live suite for the six routes plus the nine
negative cases (the port `681e2e260` names); the pending-supply phase in
`node/tests/supply_invariant_distributed.rs` is one route's worth of that port and the rest is open.

## GAP-RC1-VARIANT-NOT-BUILT — the runtime variant mainnet is meant to run did not compile

**Found 2026-09-27** by running the variants gate on the `mainnet-rc1` feature set — the one mainnet
is meant to run, and the one that has the scope lock excluding unaudited pallets.

```
=== runtime variant: mainnet-rc1 (features: std,mainnet-rc1) ===
error[E0599]: the function or associated item `get` exists for struct
              StorageValue<_GeneratedPrefixForStorageEnabled<Runtime>, bool, ...>, but its trait
              bounds were not satisfied
    --> runtime/src/lib.rs:1404:55
     |     pallet_private_execution::Enabled::<Runtime>::get()
     |     doesn't satisfy `Runtime: pallet_private_execution::Config`
FAIL (36s)
```

`ed798764d` (tonight's private-submission wiring) added `RuntimePrivateSubmissionChannel`, whose
`get()` reads `pallet_private_execution::Enabled::<Runtime>`. `pallet-private-execution` is **not** in
the `mainnet-rc1` `construct_runtime!` block — the scope lock leaves it out, as it leaves out the
other unaudited surfaces — so `Runtime: pallet_private_execution::Config` is unimplemented there and
the storage item does not resolve. The variant simply did not build.

**The default gate set was red and nobody ran it.** `clippy runtime rc1` is in `GATES_FAST` — it is
`cargo clippy -p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1 -- -D
warnings`, the same feature set this error comes from — so a plain `bash scripts/local-ci.sh` failed
from `ed798764d` until this fix, and the lane that landed that commit did not see it. The
`--variants` group's `runtime variant dry-runs` (`scripts/check-runtime-variants.sh`, auto-selected by
`scripts/local-ci.sh` when a `runtime/*` path changes) and the rc6 sequence's stage 5 both claim a
migration dry-run for *every* variant, and neither had been re-run either. The breakage is invisible
to `cargo check --workspace`, which compiles the runtime once, with default features, and never sees
the other five `construct_runtime!` blocks.

**Fixed 2026-09-27:** the reference is now cfg-aware. On `mainnet-rc1` the answer is a constant
`false`, which is not a workaround — it is the fact the derived version would have reported on a
chain with no private channel, and it keeps the posture fail-closed: a program whose compiled policy
demands private submission is refused at intake. Measured after: `bash
scripts/check-runtime-variants.sh` — full PASS 35s, dev PASS 47s, dev+frontier PASS 95s, frontier
PASS 63s, mainnet-rc1 PASS 7s, testnet PASS 45s.

**Still open:** a build of the variant is not a chain of it. No rc1-featured runtime has been booted
and no rc1 genesis exists in `chain-specs/`, so this closes "the variant compiles and its
`OnRuntimeUpgrade` work fits in a block", not "an rc1 network runs". That is recorded on row
X3-RT-003, whose scores were resting on a variant that did not build.

## GAP-WORKSPACE-RED — the whole workspace stopped building, and two gates said so

**Found 2026-09-27** by running the *entire* default gate set for the first time tonight instead of
the gates around whatever was being edited. 128 gates: 124 passed, 4 failed. Three of the four were
real, and all three came from the same night's private-submission work (`ed798764d`), which is what a
`CompilationOptions` field added for one caller does to every other caller.

| gate | failure | cause |
| --- | --- | --- |
| `workspace check` | `crates/x3-cli` failed with `E0063: missing field \`require_private_submission\` in initializer of \`CompilationOptions\`` | five struct literals in `crates/x3-cli/src/commands/{compile,build,repl}.rs` |
| `clippy workspace` | same E0063 | same |
| `test x3-sidecar` | `the lock file crates/x3-sidecar/Cargo.lock needs to be updated but --locked was passed` | the sidecar has its own lockfile, and `pallet-x3-kernel` gained an `x3-common` dependency that never reached it |

**`cargo check --workspace` failing is the loudest thing a repository can say, and it was not run.**
`crates/x3-sidecar` is outside the workspace (its own manifest and lockfile), so the workspace check
cannot cover it either — which is why it needs its own gate, and why its lockfile has to be
regenerated whenever a path dependency's own dependency list grows. The earlier `a434ea852` fixed the
same class for a different nested lockfile.

**Fixed 2026-09-27.** The CLI's five initializers now name the field, and the artifact-producing
commands declare the capability properly rather than defaulting it: `x3 compile
--require-private-submission` and `x3 build --require-private-submission` compile the demand into the
artifact, which is where it belongs — a chain with no private channel refuses the program at intake,
so the demand is a property of what you ship, not of the invocation that runs it. The REPL, which
compiles a snippet in-process for immediate execution, passes `false` with that reason written at the
site. Verified: `x3 compile --help` and `x3 build --help` both list the flag; `workspace check` PASS
50s; `clippy workspace` PASS 120s; `test x3-sidecar` PASS 120s after `cargo metadata` added the one
missing lockfile line (`x3-common` under `pallet-x3-kernel`).

**Still open:** the fourth failure is `runtime hash freshness`, which is the stale release attestation
and is being re-taken separately. The lesson that produced all three is worth keeping: a gate set is
only as good as how often the whole of it runs, and three lanes editing one tree means "green around
my change" is not "green".

## GAP-SILENT-FEE-WAIVER — two more fees can be waived with nothing recording it

**Found 2026-09-27** by hunting the class of `GAP-ROUTER-FEE-DEPOSIT` across every
`Currency::transfer` whose destination is a configured account. Two more pay into
`T::ProtocolTreasury::get()` and both already tolerate a refusal — they are *best-effort*, so unlike
the router they do not fail the operation:

* `pallets/atomic-trade-engine/src/lib.rs` — protocol trade fee on a completed batch
  (`if <T as Config>::Currency::transfer(…).is_ok() { deposit ProtocolFeeCollected }`)
* `pallets/x3-settlement-engine/src/lib.rs` — protocol settlement fee on finalization
  (`if …transfer(…).is_ok() { deposit SettlementFeeCollected }`)

That is the right behaviour and it is not a blocker. What is wrong is that the refusal is **silent**:
when the treasury is dead and the fee is below the existential deposit — which is every fee below
`ED`, not an edge case — the protocol collects nothing and no event, counter or log says so. A
treasury that can never accept dust fees looks exactly like a treasury with no fees to collect, and
the difference is revenue.

**Ticket (not fixed tonight, deliberately):** give both sites the same treatment the router got —
waive, and emit a named event (`ProtocolFeeWaived` / `SettlementFeeWaived`) — so waived revenue is
auditable in one place across all three fee paths. Held back because both pallets are in the runtime
graph, a release attestation was being taken while this was found, and editing a runtime-graph file
underneath a running `make mainnet-check` invalidates it. The alternative fix — keep the treasury
funded so the fee can always be credited — is a policy decision for the operator, and it is the same
one recorded on row X3-XVM-014.

**CLOSED 2026-10-07 (operator chose both halves).** (1) Both sites now emit a named event when the
transfer fails — `ProtocolFeeWaived { who, fee }` and `SettlementFeeWaived { intent_id, fee }`,
appended at the end of each `Event` enum so no existing variant index moves. The operation still
completes, as before; only the silence is gone. Both mocks wired the fee rate to `0`, so neither
branch had ever run in a test — the rates are now settable statics, and each pallet has a
collected-fee and a waived-fee test (`cargo test -p pallet-atomic-trade-engine -p
pallet-x3-settlement-engine fee`). (2) Every genesis built by `x3_chain_genesis` (dev, local,
local3, staging, testnet, production) now creates `TreasuryAccountId` with the existential deposit,
unless the spec already endows it — so a fresh chain's treasury can take a dust fee from block 0,
which the DA and sequencer fees (TICKET-154) now require. Specs generated before this change still
lack the account and must be regenerated.

## NIGHT SHIFT — the state at 05:00 local on 2026-09-27, for whoever starts next

Written for the operator's return. Nothing below is a plan; it is what is true on disk and on
`origin/master`.

**Green and verified at `8e23c9cbe`:** `make mainnet-check` passes end to end, including stage 6b
rebuilding the runtime in srtool and matching `docs/reports/runtime-wasm-hashes.json`
(`0x1ea62909…` / `0x50c5a499…`, `recorded_revision` `248435935`); the full default gate set is
**102/102**; the rc6 sequence is 5/5 (step 2 retired with a ticket); `runtime hash freshness` PASSes.

**Scoreboard:** composite **67.94%**, P0 mean **69.38%**, **66 of 82 P0 rows below 80**, 0 broken
rows, 2 stubs (`X3-GPU-001` hardware-blocked at 7, `X3-MEV-001` at 25 — a lane is on it).

**What moved overnight, worst-first:**

| area | change |
| --- | --- |
| X3-MEV-002 private submission | 39 → **75**: the compiled artifact records the demand, `pallet-x3-kernel` refuses it at intake, both engines enforce it, the runtime binds the posture to `pallet_private_execution::Enabled` |
| runtime variant / CLI / sidecar | the **mainnet-rc1 variant did not compile** and `x3-cli` did not build; both fixed, with the gates that missed them named |
| router fee | every small `xvm_transfer` was refused because the treasury could not accept the 20 bps fee; fixed with an explicit waiver + event |
| X3-ECO-002 distributed supply | proven on three validators (pending 6,000,000 → 0, every leg resolved, per-validator conservation at one finalized block); the failure was a nonce collision in the driver |
| X3-OPS-002 zero-downtime snapshot | the proof script was missing the `--regenesis` its own comment requires; with it, a running chain is exported, rebuilt and restored, and the restored chain finalizes |
| rc2/rc6 gates | seven defects between a red sequence and a green one (hardcoded repo path ×2, a non-starting dev chain, a missing cargo feature, a stale raw spec, an authorized-but-unfunded gateway, an HttpProvider that cannot subscribe) |
| panic ratchet | **520 → 450** sites, with the baseline re-baselined downward at each step |

**The morning's real blockers**, in the order I would take them:

1. **The 7 physical servers.** The 72-hour soak, public testnet hosting, the live 7-node runtime
   upgrade and the per-validator monitoring all need them; nothing local substitutes.
2. **No rc1-featured network exists yet** — the `mainnet-rc1` variant compiles now, but there is no
   rc1 genesis, so the feature mode has never run on a chain.
3. **The six-route live sweep is still one route in Rust.** The legacy JS driver cannot decode
   extrinsic v5, so the route/negative matrix it used to cover is recorded as GAP-RC2-DRIVER-FORMAT,
   half-ported (`node/tests/supply_invariant_distributed.rs` covers X3Native → X3Evm).
4. **Two decisions are the operator's, not an agent's:** whether a validator must keep its treasury
   funded (rather than the router fee being waived), and what charge primitive replaces
   `pallet-x3-da`/`pallet-x3-sequencer`'s `reserve`-as-anti-spam-fee, which holds funds nothing can
   release (TICKET-154).
5. **Remaining panic sites are now concentrated** in `pallets/*/benchmarking.rs` (101 of the 450,
   compiled only under `runtime-benchmarks`), `crates/x3-dex` (19), `crates/x3-gpu-validator-swarm`
   (18) and `crates/x3-bridge-adapters` (18) — all runtime-graph, so each batch costs a re-attest
   (~10 min) and a `mainnet-check` (~30 min). The free (non-runtime-graph) ones left are
   `crates/x3-bot` (15), `crates/x3-mobile-sdk` (14), `crates/quantum-swarm`'s `CircuitBuilder`
   (11) and `crates/x3-cli` (10).

**Process notes worth keeping.** Five actors share this box; load average sat at 19–27 and the
default `target/` is a queue everyone waits in — `CARGO_TARGET_DIR=/tmp/<name>` cuts a build from
minutes to under a minute, but it breaks the nested WASM build for a node check, so pair it with
`SKIP_WASM_BUILD=1`. Two `make mainnet-check` runs at once fight over ports 9944/9945; two
`supply_invariant_distributed` runs cannot coexist at all (the test asserts its ports free).
`~/.cargo/bin` is still the one directory CI deletes, and the fix for that is `ab18567f5`.
