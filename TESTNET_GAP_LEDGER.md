# TESTNET_GAP_LEDGER.md

Autonomous audit ledger. Live re-verified findings (2026-09-03/04). Prior report files in the
tree treated as untrusted until reproduced. Severity labels as head note.

## Open P0/P1 entries

| ID | Sev | Area | Description | Root cause | Evidence |
|----|-----|------|-------------|-----------|----------|
| GAP-CLI-1 | P1 | launch harness | `scripts/testnet/x3_testnet_up.sh` uses flags the current binary rejects (`--ws-port`, `--ws-external`, `--execution=NativeElseWasm`) and lacks forced node-key => first boot NetworkKeyNotFound. FIXED in place (edited script), re-verify under fresh review. | CLI surface drift vs doc'd harness. | bash -n clean after edit; single/7-node boots from the pattern now succeed. |
| GAP-SPEC-1 | P0 | chain-spec generation | Stale dev-seed raw spec (deployment/chain-specs/x3-testnet-raw.json older, id x3_testnet_v1) is invalid: runtime LoadSpec rejects Live raw (no runtimeGenesis.config) OR missing Aura authorities. Correct path is env-gated `--chain=testnet` build-spec with fresh keys (plain, not --raw, because node file-validator rejects raw Live) + per-validator X3_DEV_SEED. | Live raw genesis cannot satisfy node-side structural validator; Aura requires real authority keys not dev seeds for live. | see TESTNET_VERIFICATION.md § multi-validator + fresh-key. |
| GAP-AUTH-1 | P0 | node authoring | File-only keystore injection (standard substrate) does NOT drive Aura block authoring on this binary; needs programmatic insert via X3_DEV_SEED (service maybe_insert_dev_keys / insert_dev_keys_with_seed). Aura authorities ARE in block-0 storage. | Custom service; key discoverable only via maybe_insert_dev_keys path. | Single-node authored only after X3_DEV_SEED set (root-caused via A/B + storage read). |

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

## Still open / honest bound
- GAP-CLI-1, GAP-SPEC-1, GAP-AUTH-1 remain OPEN (table above) — bring-up/spec/author harness gaps,
separate from P2P topology; the mesh proof rides on the now-known-good plain-spec + X3_DEV_SEED
authoring + fixed node-key pattern.

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
