# AI civilization inventory (Resurrection mission, Phase B)

Scan: every branch tip of the 11 mirrored repositories (`~/x3-recovery/mirrors`), the merge
train tip of xxxstar, and the unpushed working trees `~/.openclaw/workspace/x3triplethreat` and
`~/Desktop/x3-chain-master`; vendored code and lockfiles excluded. Raw map:
`~/x3-recovery/civ-map.json` (concept → source → files). "Code" means .rs/.py/.ts/.tsx/.js/.sol.

| Concept | Best implementation found | In xxxstar now? | Notes |
|---|---|---|---|
| Crown | `crates/gpu-swarm/src/crown/` (`auditor.rs`, `prophet.rs`, `scrapyard.rs`, `mod.rs`) | **yes** | also an older `gpu-swarm.backup` copy in x3-atomic-star/atlas-sphere history |
| Warden | same `gpu-swarm` crown/announcer modules | **yes** | |
| Scrap Yard | `crates/gpu-swarm/src/crown/scrapyard.rs` | **yes** | |
| X3 Court | `crates/x3-court` (court, docket, verdict, replay, vm) | **yes — byte-identical** to x3-atomic-star@eae83ab21 | jury/appeal/precedent vocabulary also in `FoundryDisputeResolver.sol`, `pallets/x3-agent-law` |
| Reaper | `the-reaper2` (154 code files: Python backend, `ReaperProxy.sol`, trading/blockchain routers) | partially (32 code files mention it) | a separate Python product; not ported |
| Goal Genome | `apps/x3-desktop/src/components/monitoring/GoalGenomeViewer.tsx` | **no** (viewer missing) | in x3-atomic-star; xxxstar's `apps/x3-desktop` lacks the monitoring viewers |
| Self-Model Ledger | `apps/x3-desktop/src/components/monitoring/SelfModelViewer.tsx` | **no** (viewer missing) | same |
| World Simulator | `apps/x3-desktop/src/components/monitoring/WorldSimViewer.tsx` (+ atlas-sphere copy) | **no** (viewer missing) | xxxstar has the simulator itself (`crates/x3-sim`, `x3-mc`) |
| Agent lifecycle | `crates/orchestra/src/agent/lifecycle.rs`, `crates/x3-swarm-core/tests/agent_lifecycle.rs` | **yes** | |
| Agent evolution | `pallets/evolution-core`, `crates/chronos-flash`, `crates/dream-mining` | **yes** (evolution-core in runtime genesis) | |
| Score Guardian | prose only (no code in any source) | no | |
| AGI Proximity Analyst | **not found in any repository or branch** | no | must be built new; the mission forbids an invented AGI percentage — it needs reproducible evals |

## Conclusions
1. The core civilization code already lives in xxxstar; nothing newer or more complete exists elsewhere
   for Crown/Warden/Scrap Yard/Court/lifecycle/evolution.
2. Recoverable and missing: the three monitoring viewers (Goal Genome, Self-Model, World Sim) from
   `x3-atomic-star:apps/x3-desktop/src/components/monitoring/`.
3. The Reaper as a product is the separate `the-reaper2` Python stack; integrating it is a
   post-testnet item (category C/D).
4. AGI Proximity Analyst does not exist; build it only with reproducible evaluations and stated
   uncertainty.
