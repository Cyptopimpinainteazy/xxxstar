# TESTNET_GAP_LEDGER.md

Autonomous audit ledger. Live re-verified findings (2026-09-03/04). Prior report files in the
tree treated as untrusted until reproduced. Severity labels as head note.

## Open P0/P1 entries

| ID | Sev | Area | Description | Root cause | Evidence |
|----|-----|------|-------------|-----------|----------|
| GAP-P2P-1 | P1 | testnet/network bring-up | **Default libp2p graph is sparse/star-ish; single-node loss can partition a >2/3-majority net into forking groups.** Reproduced: 7-validator net (identical finalized heads) then killed sj3 (leaf). Survivors kept finalizing but split into two finalized branches {sj1,sj2,sj4}=0xb3334567… and {sj5,sj6,sj7}=0xf93d5110…; sj1 peers=5 yet did not bridge the two sides. | Local nodes on 127.0.0.1 with --no-mdns form a star to the bootnode + few edges; libp2p does not auto-heal into a full mesh; public-addr/kademlia gossip not wired on this host. | run: `scripts/testnet/run-solo-join.py 7 40` (converged 7/7) then `kill` node-3 (see logs /tmp/x3-solo-join/logs). |
| GAP-CLI-1 | P1 | launch harness | `scripts/testnet/x3_testnet_up.sh` uses flags the current binary rejects (`--ws-port`, `--ws-external`, `--execution=NativeElseWasm`) and lacks forced node-key => first boot NetworkKeyNotFound. FIXED in place (edited script), re-verify under fresh review. | CLI surface drift vs doc'd harness. | bash -n clean after edit; single/7-node boots from the pattern now succeed. |
| GAP-SPEC-1 | P0 | chain-spec generation | Stale dev-seed raw spec (deployment/chain-specs/x3-testnet-raw.json older, id x3_testnet_v1) is invalid: runtime LoadSpec rejects Live raw (no runtimeGenesis.config) OR missing Aura authorities. Correct path is env-gated `--chain=testnet` build-spec with fresh keys (plain, not --raw, because node file-validator rejects raw Live) + per-validator X3_DEV_SEED. | Live raw genesis cannot satisfy node-side structural validator; Aura requires real authority keys not dev seeds for live. | see TESTNET_VERIFICATION.md § multi-validator + fresh-key. |
| GAP-AUTH-1 | P0 | node authoring | File-only keystore injection (standard substrate) does NOT drive Aura block authoring on this binary; needs programmatic insert via X3_DEV_SEED (service maybe_insert_dev_keys / insert_dev_keys_with_seed). Aura authorities ARE in block-0 storage. | Custom service; key discoverable only via maybe_insert_dev_keys path. | Single-node authored only after X3_DEV_SEED set (root-caused via A/B + storage read). |
| GAP-BOOT-1 | P1 | local cold start | Concurrently starting all 7 validators from empty genesis can produce racing light forks on one host (deterministic ONLY via solo-lead then join). | Aura/GRANDPA race during connect window. | solo-join (node1 lead then join) converges 7/7 deterministically; concurrent starts intermittent. |

## Confirmed solid (for the record)
- Runtime GRANDPA consensus correct when a sufficient majority is connected: 7/7 identical finalized heads; 2000/2000 remarks finalized at 110.6 finTPS, 0 lost; canonical head identical on all 7 under load. Single/grandpa minority also finalize correctly.
- Deterministic clean cold-start recipe: solo-lead node1 to a canonical head, then join others (run-solo-join.py).

## Closed
(empty; fixes above remain to be finally reviewed/committed as canonical ops.)

Working notes: `.testnet-audit/`; evidence: `TESTNET_VERIFICATION.md`, run logs under /tmp/x3-*.
