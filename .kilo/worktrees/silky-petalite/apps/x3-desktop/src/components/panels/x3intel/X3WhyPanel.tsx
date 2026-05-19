interface Feature { icon: string; title: string; description: string }

const features: Feature[] = [
  {
    icon: '⚡',
    title: 'Deterministic Arbitrage',
    description:
      'Every intent is executed with a verifiable on-chain proof. No MEV extraction, no front-running — agents commit to bounded execution or face slashing. The result is guaranteed fills at predictable costs.',
  },
  {
    icon: '🤖',
    title: 'Agent Economy',
    description:
      'Autonomous agents put skin in the game through bond deposits. Reputation is earned through successful execution and lost through failure. The best agents rise, the worst get deregistered.',
  },
  {
    icon: '🔗',
    title: 'Cross-Chain Native',
    description:
      'X3 operates natively across EVM and SVM chains with atomic multi-leg execution. A single intent can route through Ethereum, Arbitrum, and Solana in one verifiable transaction bundle.',
  },
  {
    icon: '🛡️',
    title: 'Transparent Slashing',
    description:
      'Every penalty is recorded on-chain with full proof data. Agents can dispute slashes within 48 hours, and verifiers adjudicate with economic incentives for honest resolution.',
  },
];

export default function X3WhyPanel() {
  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-8 overflow-auto">
      <div className="text-center max-w-2xl mx-auto">
        <h1 className="text-2xl font-semibold text-white mb-3">Why X3 Intelligence?</h1>
        <p className="text-gray-400 text-sm leading-relaxed">
          A new execution layer for cross-chain arbitrage — deterministic, accountable, and verifiable from the first intent to the last proof.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4 max-w-3xl mx-auto">
        {features.map(f => (
          <div key={f.title} className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-6 space-y-3">
            <span className="text-3xl">{f.icon}</span>
            <h2 className="text-base font-medium text-white">{f.title}</h2>
            <p className="text-sm text-gray-400 leading-relaxed">{f.description}</p>
          </div>
        ))}
      </div>

      <div className="text-center max-w-xl mx-auto bg-[#111116] border border-[#1a1a1a] rounded-lg p-6">
        <p className="text-gray-400 text-sm mb-4">
          Ready to deploy an agent or submit your first intent?
        </p>
        <button className="px-6 py-2.5 bg-[#00d4aa] text-black text-sm font-medium rounded hover:brightness-110 transition">
          Get Started
        </button>
      </div>
    </div>
  );
}
