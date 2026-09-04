'use client';

import { useState } from 'react';

interface WalletConnectorProps {
  connected: boolean;
  walletId: string;
  onConnect: (walletId: string) => void;
  onDisconnect: () => void;
}

export function WalletConnector({ connected, walletId, onConnect, onDisconnect }: WalletConnectorProps) {
  const [connecting, setConnecting] = useState(false);

  const handleConnect = async () => {
    setConnecting(true);
    try {
      // Simulate wallet connection (in production, use Polkadot.js extension)
      await new Promise(resolve => setTimeout(resolve, 1000));
      const mockWalletId = '0x' + '1'.repeat(64);
      onConnect(mockWalletId);
    } catch (error) {
      console.error('Failed to connect wallet:', error);
    } finally {
      setConnecting(false);
    }
  };

  if (connected) {
    return (
      <div className="flex items-center gap-3">
        <div className="bg-gray-800 px-4 py-2 rounded-lg border border-gray-700">
          <div className="text-xs text-gray-400">Connected</div>
          <div className="text-sm font-mono">
            {walletId.slice(0, 6)}...{walletId.slice(-4)}
          </div>
        </div>
        <button
          onClick={onDisconnect}
          className="px-4 py-2 bg-red-600/20 text-red-400 rounded-lg hover:bg-red-600/30 transition border border-red-600/50"
        >
          Disconnect
        </button>
      </div>
    );
  }

  return (
    <button
      onClick={handleConnect}
      disabled={connecting}
      className="px-6 py-2 bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg hover:from-blue-700 hover:to-purple-700 transition font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {connecting ? 'Connecting...' : 'Connect Wallet'}
    </button>
  );
}
