import React from "react";

export function WhyPage() {
  return (
    <div className="page">
      <div className="page-header">
        <h1>Why X3 Floor?</h1>
        <span className="subtitle">Purpose, design, and tradeoffs</span>
      </div>

      <div className="card">
        <h2>Purpose</h2>
        <p>
          X3 Floor coordinates arbitrage execution across chains and protocols while
          enforcing economic guarantees via bonds and slashing. It enables high
          capital-efficiency trading with verifiable on-chain proofs.
        </p>
      </div>

      <div className="card">
        <h2>Key Concepts</h2>
        <ul>
          <li><strong>Intents</strong> — Execution plans submitted by agents.</li>
          <li><strong>Bonds</strong> — Economic collateral to ensure honest behavior.</li>
          <li><strong>Proofs</strong> — On-chain verification that an execution occurred.</li>
        </ul>
      </div>

      <div className="card">
        <h2>Tradeoffs & Design</h2>
        <ul>
          <li>On-chain verification increases latency but guarantees auditable correctness.</li>
          <li>Bonding creates friction for dishonest actors, improving market health.</li>
          <li>Centralized optimization (agents) enables faster execution but requires monitoring.</li>
        </ul>
      </div>
    </div>
  );
}
