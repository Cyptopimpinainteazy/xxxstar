import { useEffect, useState, useCallback } from 'react';
import { invoke } from '../../ipc/tauri';

interface WalletData {
  mnemonic: string;
  seed_hex: string;
  evm_address: string;
  evm_private_key: string;
  solana_address: string;
  solana_private_key: string;
  substrate_address: string;
  evm_chain_count: number;
  warning: string;
}

function WalletPanel() {
  const [wallet, setWallet] = useState<WalletData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showMnemonic, setShowMnemonic] = useState(false);
  const [balances, setBalances] = useState<{ chain: string; balance: string }[]>([]);

  const generateWallet = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<WalletData>('generate_universal_wallet');
      if (result) {
        setWallet(result);
        // Fetch balances for each chain
        const chains = [
          { id: 'evm', label: 'EVM' },
          { id: 'solana', label: 'Solana' },
          { id: 'substrate', label: 'Substrate' },
        ];
        const balanceResults = await Promise.allSettled(
          chains.map(async (c) => {
            const addr = c.id === 'evm' ? result.evm_address
              : c.id === 'solana' ? result.solana_address
              : result.substrate_address;
            const bal = await invoke<string>('get_wallet_balance', { chainId: c.id, address: addr });
            return { chain: c.label, balance: bal };
          })
        );
        setBalances(
          balanceResults
            .filter((r) => r.status === 'fulfilled')
            .map((r) => (r as PromiseFulfilledResult<{ chain: string; balance: string }>).value)
        );
      }
    } catch (err) {
      console.error('Failed to generate wallet:', err);
      setError('Failed to generate wallet. Is the Tauri backend running?');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    generateWallet();
  }, [generateWallet]);

  if (loading && !wallet) {
    return (
      <div className="view p-6">
        <h2 className="text-xl font-bold text-white mb-2">Wallet</h2>
        <div className="text-gray-400">Generating universal wallet...</div>
      </div>
    );
  }

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Universal Wallet</h2>
        <p className="text-gray-400 text-sm">Multi-chain wallet — EVM + Solana + Substrate</p>
      </div>

      {error && <div className="bg-red-900/30 border border-red-600/30 rounded-lg p-3 mb-4 text-red-300 text-sm">{error}</div>}

      {wallet && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
          {/* EVM */}
          <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
            <div className="text-gray-400 text-xs mb-1">EVM Address</div>
            <div className="text-cyan-400 font-mono font-bold text-sm break-all">{wallet.evm_address}</div>
            <div className="mt-2 flex items-center gap-1">
              <span className="text-gray-500 text-xs">Balance:</span>
              <span className="text-green-400 font-mono text-xs">
                {balances.find((b) => b.chain === 'EVM')?.balance ?? '...'}
              </span>
            </div>
          </div>

          {/* Solana */}
          <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
            <div className="text-gray-400 text-xs mb-1">Solana Address</div>
            <div className="text-cyan-400 font-mono font-bold text-sm break-all">{wallet.solana_address}</div>
            <div className="mt-2 flex items-center gap-1">
              <span className="text-gray-500 text-xs">Balance:</span>
              <span className="text-green-400 font-mono text-xs">
                {balances.find((b) => b.chain === 'Solana')?.balance ?? '...'}
              </span>
            </div>
          </div>

          {/* Substrate */}
          <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50">
            <div className="text-gray-400 text-xs mb-1">Substrate Address</div>
            <div className="text-cyan-400 font-mono font-bold text-sm break-all">{wallet.substrate_address}</div>
            <div className="mt-2 flex items-center gap-1">
              <span className="text-gray-500 text-xs">Balance:</span>
              <span className="text-green-400 font-mono text-xs">
                {balances.find((b) => b.chain === 'Substrate')?.balance ?? '...'}
              </span>
            </div>
          </div>
        </div>
      )}

      {wallet && (
        <div className="bg-gray-800/40 rounded-lg p-4 border border-gray-700/50 mb-4">
          <div className="flex items-center justify-between mb-2">
            <div className="text-gray-400 text-xs">Mnemonic (Secret — keep safe)</div>
            <button
              className="px-3 py-1 text-xs rounded bg-gray-700/50 text-gray-300 hover:bg-gray-600/50 transition-colors"
              onClick={() => setShowMnemonic(!showMnemonic)}
            >
              {showMnemonic ? 'Hide' : 'Reveal'}
            </button>
          </div>
          {showMnemonic && (
            <div className="bg-yellow-900/20 border border-yellow-600/30 rounded p-3">
              <div className="text-yellow-400 font-mono text-sm break-all">{wallet.mnemonic}</div>
              <div className="text-yellow-600 text-xs mt-2">{wallet.warning}</div>
            </div>
          )}
        </div>
      )}

      <div className="flex gap-3">
        <button
          className="px-4 py-2 text-sm bg-cyan-600/30 border border-cyan-500/40 text-cyan-300 rounded-lg hover:bg-cyan-600/50 transition-colors"
          onClick={generateWallet}
          disabled={loading}
        >
          {loading ? 'Generating...' : 'Generate New Wallet'}
        </button>
      </div>

      <div className="mt-3 text-xs text-gray-600">
        Query: invoke('generate_universal_wallet') → BIP39 mnemonic + keypairs for EVM, Solana, Substrate
      </div>
    </div>
  );
}

export default WalletPanel;
