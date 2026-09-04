'use client';

export default function WhyPage() {
  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-3xl font-bold mb-2">Why X3 Floor?</h1>
      <span className="text-gray-400 block mb-6">Purpose, design, and tradeoffs</span>

      <div className="space-y-4">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold text-white mb-2">Purpose</h2>
          <p className="text-gray-300">
            X3 Floor coordinates arbitrage execution across chains and protocols while
            enforcing economic guarantees via bonds and slashing.
          </p>
        </div>

        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold text-white mb-2">Key Concepts</h2>
          <ul className="space-y-1 text-gray-300">
            <li><strong>Intents</strong> — Execution plans submitted by agents</li>
            <li><strong>Bonds</strong> — Economic collateral to ensure honest behavior</li>
            <li><strong>Proofs</strong> — On-chain verification that execution occurred</li>
          </ul>
        </div>

        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold text-white mb-2">Design Tradeoffs</h2>
          <ul className="space-y-1 text-gray-300">
            <li>On-chain verification increases latency but guarantees auditable correctness</li>
            <li>Bonding creates friction for dishonest actors, improving market health</li>
            <li>Centralized optimization enables faster execution but requires monitoring</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
