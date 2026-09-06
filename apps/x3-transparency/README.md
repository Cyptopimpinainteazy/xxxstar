# X3 Transparency Portal

The X3 Transparency Portal is a static website for showing grant work packages, documented funding movement, proof records, and current protocol completion evidence.

It does not accept payments, connect wallets, provide accounting advice, or read an on-chain treasury. It displays only the data produced from the local repository ledger and readiness evidence.

## Data sources

- `data/treasury-ledger.json` is the funding source. Funding values must be non-negative and must satisfy `requested ≥ pledged ≥ received ≥ allocated ≥ spent` for every work package.
- Every non-zero `spent` value must contain a `proofIds` reference to a proof with an ID, status, and public reference.
- `../../audit-artifacts/mainnet-readiness/live/current.json` selects the current local readiness snapshot. The generator validates the snapshot path before loading its `summary.json`.
- `public/data/portal.json` and `public/data/evidence-assets/` are generated. The generator copies the readiness score, subsystem, findings, tasks, and checks charts from the selected snapshot. Do not edit generated data by hand.

Initial values are all zero because no reviewed funding receipts, treasury transactions, allocations, or spending records were supplied. Add only reviewed source records; do not use the portal to make funding or readiness claims without evidence.

## Run locally

From this application directory:

```bash
npm install
npm run generate:data
npm run dev
```

The development server uses port `1450`. The Vite base path is `/transparency/`.

To regenerate evidence when the readiness pointer or funding ledger changes, run this beside the site server or as a deployment worker:

```bash
npm run watch:data
```

The browser re-reads generated portal data every 30 seconds. A hosted deployment needs the same generator command in its CI or worker before it can show a new readiness snapshot.

## Validate and build

```bash
npm test
npm run typecheck
npm run build
```

`npm run build` regenerates the portal data before typechecking and producing `dist/`.

## Update funding data

1. Add a work package or proof record to `data/treasury-ledger.json`.
2. Link every recorded spend to a proof ID with a durable public reference.
3. Run `npm test` and `npm run generate:data`.
4. Review the generated `public/data/portal.json` and build output.

The generator fails on malformed snapshot paths, duplicate IDs, unknown proof references, non-finite values, broken funding ordering, and spending without proof.
