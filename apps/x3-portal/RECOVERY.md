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

## Repairs (separate commits) and verification, 2026-10-13
- 27 pages imported `components/frontend/ui/Logo`, a path that never existed in any commit;
  `HexagonCluster` lives in `src/components/ui/Logo.tsx`, now imported as `@/components/ui/Logo`
  (also `src/components/layout/{Footer,Navigation}.tsx`).
- A repo-wide find-and-replace had turned every "dash" into `apps/dash-legacy-2-legacy-2`
  (`dashArray`, `border-dashed`, "dashboard"); reversed in 10 places.
- `src/app/x3/swarm/api/**` (12 consent/claim/KYC/admin routes) and `src/lib/db.ts` are excluded
  from the build: Next serves the root `app/`, so these never routed, and their `@/lib/auth` never
  existed. They are kept, not wired: admin endpoints without real authentication would be a hole.
- `react` types pinned to the portal's copy (apps/shared installs a second version);
  `framer-motion` declared, since the shared components the portal renders need it.
- Result: `next build` succeeds (99 routes). Against a live `x3-chain-node --dev` (Oct 1 build)
  the explorer showed live block height, Aura block time, the validator count and the newest
  blocks updating; block detail and network status rendered with no page errors.
  Screenshots: `reports/recovery/portal-screenshots/`.
