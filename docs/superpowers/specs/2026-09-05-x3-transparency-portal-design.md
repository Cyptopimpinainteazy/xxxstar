# X3 Transparency Portal Design

## Purpose

Build a standalone public website at `apps/x3-transparency` that shows the current completion evidence, an honest grant funnel, and a traceable explanation of where recorded money is allocated and spent. It must not process payments, connect wallets, claim live-chain treasury data, or invent financial activity.

## User-facing sections

The single-page site has five anchored sections:

1. Overview: current readiness, task completion, evidence freshness, and release decision.
2. Funding: a funnel that separates planned request, pledged, received, allocated, and spent amounts.
3. Work packages: each funding allocation tied to a bounded engineering objective and completion evidence.
4. Proof ledger: searchable proof records with an evidence class, date, reviewer, public reference, digest, and status.
5. Completion: live readiness score, open finding severity, task state, latest verification results, and links to the generated report/PDF.

## Sources and trust model

`scripts/transparency/generate.mjs` reads the existing local readiness pointer and snapshot JSON. It writes a static `public/data/portal.json` only after validating the required shape and preserving the current evidence fingerprint. A hand-maintained `data/treasury-ledger.json` contains only zero-valued, explicitly unverified initial funding states until real receipts or transaction references are supplied. The build rejects a record marked spent without at least one proof reference.

The website is display-only. It labels all source data, the generation time, live versus historical evidence, and unverified funding explicitly. It does not treat a PDF, a task checkbox, an unknown amount, or a user-entered note as proof of funds or protocol safety.

## Interface and accessibility

The app is a React/Vite static app using CSS only for charts and responsive layout. Semantic headings, tables, progress elements, visible status labels, keyboard-safe anchor navigation, and strong color contrast are required. Charts include text summaries so color is never the sole meaning. No stock assets, wallet controls, donation buttons, or investor-performance claims appear.

## Data flow

```text
readiness state + latest snapshot ─┐
                                  ├─ generate.mjs ─ public/data/portal.json ─ React portal
treasury-ledger.json ─────────────┘                                        ├─ charts
                                                                           ├─ proof ledger
                                                                           └─ external source links
```

## Acceptance criteria

- The generator rejects spending without proof references and stale/malformed readiness data.
- Generated portal data contains the current readiness score, task counts, open findings, checks, and explicit source paths.
- Funding totals are derived from ledger values; no amount is presented as received or spent without evidence status.
- The portal renders all five areas and exposes chart values in text.
- The app builds with `npm run build`, typechecks with `npm run typecheck`, and data contracts pass through `node --test`.
- Existing audit artifacts and readiness workflow remain unchanged except for consuming their generated data.
