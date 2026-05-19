import React from "react";

export function GuidePage() {
  return (
    <div className="page">
      <div className="page-header">
        <h1>How to Use X3</h1>
        <span className="subtitle">Quick start & common workflows</span>
      </div>

      <div className="card">
        <h2>Getting Started</h2>
        <ol>
          <li>Connect your wallet (MetaMask or Polkadot.js depending on chain).</li>
          <li>Deposit a bond from <strong>Bonds → Deposit Bond</strong>.</li>
          <li>Monitor active agents & intents on the <strong>Floor</strong> page.</li>
          <li>Submit an intent using the API or the CLI (see docs for examples).</li>
          <li>Watch the execution feed — proofs will appear under <strong>Proofs</strong>.</li>
        </ol>
      </div>

      <div className="card">
        <h2>Common Workflows</h2>
        <h3>Deposit Bond</h3>
        <ol>
          <li>Open <strong>Bonds</strong> and enter deposit amount.</li>
          <li>Confirm in your wallet and wait for 1 confirmation.</li>
          <li>Bond balance updates automatically.</li>
        </ol>

        <h3>Checking an Intent</h3>
        <ol>
          <li>Search by Intent ID on the <strong>Intents</strong> page.</li>
          <li>Open full details and verify the proof on <strong>Proofs</strong>.</li>
        </ol>
      </div>

      <div className="card">
        <h2>Troubleshooting</h2>
        <ul>
          <li>If live updates stop, check WebSocket connection in DevTools.</li>
          <li>Network errors: ensure API endpoint /api/v1 is reachable.</li>
          <li>Deployments: see <code>docs/PHASE5_DEPLOYMENT_RUNBOOK.md</code>.</li>
        </ul>
      </div>
    </div>
  );
}
