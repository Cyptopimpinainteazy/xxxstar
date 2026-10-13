# Recovered: X3 web portal (former atlas-sphere `apps/explorer`)

**Recovered historical code.** Source: `Cyptopimpinainteazy/atlas-sphere` at
`9282a07b0^` (`apps/explorer/`), deleted by `9282a07b0` (2026-02-11, "consolidate: all
stashes + branches merged into main") together with Quantum Kernel Voyager, whose planets
land on this portal's routes (`/explorer`, `/swap`, `/x3/swarm/gpu`, `/governance`, ...).

The first commit restores the tree as found, except media no page referenced:
144 `.mp4` files (458 MB, Grok-generated, several duplicated under the same name) and
395 unreferenced images (94 MB). All of them remain in atlas-sphere's history at that commit.
Every repair after the restore is a separate commit.

Live data: `src/lib/substrate/{client,queries}.ts` use `@polkadot/api` against
`NEXT_PUBLIC_SUBSTRATE_WS_ENDPOINT` (default `ws://127.0.0.1:9944`): headers, finality,
new-head subscriptions, `system.events`, `aura` and `atlasKernel` storage.

Imports `../shared` (`apps/shared`, `@x3-chain/shared`), which xxxstar already carries.
