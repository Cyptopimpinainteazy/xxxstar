'use client';

import { useState } from 'react';
import { SwapInterface } from './components/SwapInterface';
import { WalletConnector } from './components/WalletConnector';
import { LimitOrderInterface } from './components/LimitOrderInterface';

export default function HomePage() {
  const [walletConnected, setWalletConnected] = useState(false);
  const [walletId, setWalletId] = useState('');
  const [rpcEndpoint] = useState('ws://localhost:9944');
  const [activeTab, setActiveTab] = useState<'swap' | 'limit'>('swap');

  const handleConnect = (id: string) => {
    setWalletConnected(true);
    setWalletId(id);
  };

  const handleDisconnect = () => {
    setWalletConnected(false);
    setWalletId('');
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 text-white">
      {/* Header */}
      <header className="border-b border-gray-700 bg-gray-800/50 backdrop-blur-sm">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-gradient-to-br from-blue-500 to-purple-600 rounded-lg flex items-center justify-center font-bold text-xl">
                X3
              </div>
              <div>
                <h1 className="text-2xl font-bold">X3 DEX</h1>
                <p className="text-xs text-gray-400">Decentralized Exchange</p>
              </div>
            </div>
            <WalletConnector
              connected={walletConnected}
              walletId={walletId}
              onConnect={handleConnect}
              onDisconnect={handleDisconnect}
            />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="container mx-auto px-4 py-8">
        <div className="max-w-2xl mx-auto">
          {/* Tab Navigation */}
          <div className="flex gap-2 mb-6 bg-gray-800 p-1 rounded-lg border border-gray-700">
            <button
              onClick={() => setActiveTab('swap')}
              className={`flex-1 py-3 px-4 rounded-lg font-semibold transition ${
                activeTab === 'swap'
                  ? 'bg-gray-700 text-white'
                  : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
              }`}
            >
              Swap
            </button>
            <button
              onClick={() => setActiveTab('limit')}
              className={`flex-1 py-3 px-4 rounded-lg font-semibold transition ${
                activeTab === 'limit'
                  ? 'bg-gray-700 text-white'
                  : 'text-gray-400 hover:text-white hover:bg-gray-700/50'
              }`}
            >
              Limit Orders
            </button>
          </div>

          {/* Trading Interface */}
          {activeTab === 'swap' ? (
            <SwapInterface
              walletConnected={walletConnected}
              walletId={walletId}
              rpcEndpoint={rpcEndpoint}
            />
          ) : (
            <LimitOrderInterface
              walletConnected={walletConnected}
              walletId={walletId}
            />
          )}

          {/* Info Cards */}
          <div className="grid grid-cols-2 gap-4 mt-6">
            <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
              <div className="text-gray-400 text-sm mb-1">24h Volume</div>
              <div className="text-2xl font-bold">$2.4M</div>
            </div>
            <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
              <div className="text-gray-400 text-sm mb-1">Total Liquidity</div>
              <div className="text-2xl font-bold">$20.1M</div>
            </div>
          </div>

          {/* Footer */}
          <div className="mt-8 text-center text-sm text-gray-500">
            <p>Connected to: {rpcEndpoint}</p>
            <p className="mt-2">
              Powered by X3 Chain Settlement Engine
            </p>
          </div>
        </div>
      </div>
    </main>
  );
}

