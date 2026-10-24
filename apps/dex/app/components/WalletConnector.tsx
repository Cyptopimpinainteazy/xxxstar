'use client';

import { useState } from 'react';

interface WalletConnectorProps {
  connected: boolean;
  walletId: string;
  onConnect: (walletId: string) => void;
  onDisconnect: () => void;
}

/// The subset of the injected-extension surface this component uses. Declared
/// locally so the app needs no dependency to talk to a standard Polkadot wallet.
interface InjectedAccount {
  address: string;
  name?: string;
}

interface InjectedExtension {
  accounts: { get: () => Promise<InjectedAccount[]> };
}

interface InjectedSource {
  enable: (appName: string) => Promise<InjectedExtension>;
}

export function WalletConnector({ connected, walletId, onConnect, onDisconnect }: WalletConnectorProps) {
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /// Connect to an injected wallet, or say why we cannot.
  ///
  /// This used to wait a second and hand back `'0x' + '1'.repeat(64)` — a
  /// wallet id no account owns. The UI then showed "Connected" and every request
  /// carried that fabricated identity. Nothing is connected unless an extension
  /// returns a real account.
  const handleConnect = async () => {
    setConnecting(true);
    setError(null);
    try {
      const injected = (window as unknown as {
        injectedWeb3?: Record<string, InjectedSource>;
      }).injectedWeb3;
      const source =
        injected?.['polkadot-js'] ??
        (injected ? Object.values(injected)[0] : undefined);
      if (!source) {
        setError(
          'No wallet extension found. Install Polkadot.js or SubWallet to trade.'
        );
        return;
      }

      const extension = await source.enable('X3 DEX');
      const accounts = await extension.accounts.get();
      const account = accounts[0];
      if (!account) {
        setError('The wallet extension has no authorized account for X3 DEX.');
        return;
      }
      onConnect(account.address);
    } catch (cause) {
      console.error('Failed to connect wallet:', cause);
      setError(
        cause instanceof Error
          ? `Wallet connection refused: ${cause.message}`
          : 'Wallet connection refused.'
      );
    } finally {
      setConnecting(false);
    }
  };

  if (!connected && error) {
    return (
      <div className="flex flex-col gap-2">
        <button
          onClick={handleConnect}
          disabled={connecting}
          className="px-4 py-2 bg-blue-600 rounded-lg hover:bg-blue-700 transition disabled:opacity-50"
        >
          {connecting ? 'Connecting...' : 'Connect Wallet'}
        </button>
        <div className="text-xs text-red-400" role="alert">
          {error}
        </div>
      </div>
    );
  }

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
