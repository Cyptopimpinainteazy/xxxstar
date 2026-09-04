'use client';

export default function GuidePage() {
  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-3xl font-bold mb-2">How to Use X3</h1>
      <span className="text-gray-400 block mb-6">Quick start & common workflows</span>

      <div className="space-y-6">
        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold text-white mb-3">Getting Started</h2>
          <ol className="space-y-2 text-gray-300 list-decimal list-inside">
            <li>Connect your wallet (MetaMask or Polkadot.js)</li>
            <li>Deposit a bond from <strong>Bonds → Deposit</strong></li>
            <li>Monitor agents & intents on the <strong>Floor</strong> page</li>
            <li>Submit an intent using the API or CLI</li>
            <li>Watch execution feed — proofs appear under <strong>Proofs</strong></li>
          </ol>
        </div>

        <div className="bg-x3-dark p-4 rounded border border-x3-dark-gray">
          <h2 className="text-lg font-bold text-white mb-3">Common Workflows</h2>
          <div className="space-y-4 text-gray-300">
            <div>
              <h3 className="font-bold text-white mb-1">Deposit Bond</h3>
              <ol className="space-y-1 list-decimal list-inside text-sm">
                <li>Open <strong>Bonds</strong> and enter deposit amount</li>
                <li>Confirm in your wallet and wait for 1 confirmation</li>
                <li>Bond balance updates automatically</li>
              </ol>
            </div>
            <div>
              <h3 className="font-bold text-white mb-1">Check an Intent</h3>
              <ol className="space-y-1 list-decimal list-inside text-sm">
                <li>Search by Intent ID on <strong>Intents</strong> page</li>
                <li>Open full details and verify proof on <strong>Proofs</strong></li>
              </ol>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
