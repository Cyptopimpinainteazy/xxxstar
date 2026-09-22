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
- Key rotation is implemented twice (`node/src/authority.rs` rotation manager with tests and no caller;
`pallets/x3-custody` `ValidatorKeyRegistry` with `rotation_due_at`, read by nothing) and wired zero
times; nothing calls `session.setKeys`. That is the next bring-up gap, and it is independent of the
topology work.

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

**TICKET-097 — authorize bundle finalization.** Pick one: (a) require the assigned executor's
signature on the result (and adjust the off-chain worker's submission path); (b) give the runtime a
real finality source — a signed finalized-head tracker or a verified GRANDPA justification — so
`finality_cert` is not a value the caller chooses; or (c) both. Independently: enforce the documented
invariants in *dispatch*, not only in `ValidateUnsigned` (reject `Pending`, reject a second
finalization in one place), and add the missing tests — an anonymous caller must not be able to
finalize a bundle assigned to somebody else, and a certificate for an unfinalized block must not be
anchorable. Acceptance: a test proves each refusal, and the two comments quoted above are either
true or gone.

Related, smaller, and fixed by the same reading: `node/src/service.rs`'s `run_grandpa_finality_anchor`
logs `cert anchored for block N` even when the submit failed, and advances its cursor before the
submit, so a rejected anchor for one block is never retried. Its doc comment claims it writes
off-chain storage; it does not. TICKET-098.

## GAP-GPU-CLAIMS — a release note for a release that does not exist — 2026-09-22

`docs/testnet-config/RELEASE-NOTES.md` announced `solana-gpu-validator-v1.0.tar.gz` (269 MB, CUDA
kernels), "Achieved: 2.75M TPS in lab, 1-5M TPS on testnet", "Guarantee: Minimum 100k TPS on Solana
testnet" and "825k signatures/second per GPU". None of it is in this repository: no such artifact,
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
