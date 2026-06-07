/**
 * ConnectionStatus Component
 * 
 * Displays real-time connection status to the X3 Chain blockchain.
 * Shows chain info, block number, and connection state.
 */

'use client';

import React from 'react';
import { Wifi, WifiOff, RefreshCw, AlertCircle } from 'lucide-react';
import clsx from 'clsx';

interface ConnectionStatusProps {
  isConnected: boolean;
  isConnecting?: boolean;
  error?: string | null;
  chainName?: string | null;
  latestBlock?: number;
  finalizedBlock?: number;
  onReconnect?: () => void;
  compact?: boolean;
}

export function ConnectionStatus({
  isConnected,
  isConnecting,
  error,
  chainName,
  latestBlock,
  finalizedBlock,
  onReconnect,
  compact = false,
}: ConnectionStatusProps) {
  if (compact) {
    return (
      <div className="flex items-center gap-2 text-sm">
        <div
          className={clsx(
            'w-2 h-2 rounded-full',
            isConnected ? 'bg-green-500 animate-pulse' : isConnecting ? 'bg-yellow-500 animate-pulse' : 'bg-red-500'
          )}
        />
        <span className="text-gray-600 dark:text-gray-400">
          {isConnected
            ? `#${latestBlock?.toLocaleString() || '...'}`
            : isConnecting
            ? 'Connecting...'
            : 'Disconnected'}
        </span>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {isConnected ? (
            <div className="w-10 h-10 bg-green-100 dark:bg-green-900 rounded-full flex items-center justify-center">
              <Wifi className="w-5 h-5 text-green-600 dark:text-green-400" />
            </div>
          ) : isConnecting ? (
            <div className="w-10 h-10 bg-yellow-100 dark:bg-yellow-900 rounded-full flex items-center justify-center">
              <RefreshCw className="w-5 h-5 text-yellow-600 dark:text-yellow-400 animate-spin" />
            </div>
          ) : (
            <div className="w-10 h-10 bg-red-100 dark:bg-red-900 rounded-full flex items-center justify-center">
              <WifiOff className="w-5 h-5 text-red-600 dark:text-red-400" />
            </div>
          )}
          
          <div>
            <div className="flex items-center gap-2">
              <span
                className={clsx(
                  'font-medium',
                  isConnected
                    ? 'text-green-600 dark:text-green-400'
                    : isConnecting
                    ? 'text-yellow-600 dark:text-yellow-400'
                    : 'text-red-600 dark:text-red-400'
                )}
              >
                {isConnected ? 'Connected' : isConnecting ? 'Connecting...' : 'Disconnected'}
              </span>
              <div
                className={clsx(
                  'w-2 h-2 rounded-full',
                  isConnected ? 'bg-green-500 animate-pulse' : isConnecting ? 'bg-yellow-500 animate-pulse' : 'bg-red-500'
                )}
              />
            </div>
            {chainName && (
              <p className="text-sm text-gray-500 dark:text-gray-400">{chainName}</p>
            )}
          </div>
        </div>

        {!isConnected && !isConnecting && onReconnect && (
          <button
            onClick={onReconnect}
            className="px-3 py-1.5 text-sm font-medium text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition"
          >
            Reconnect
          </button>
        )}
      </div>

      {isConnected && latestBlock !== undefined && (
        <div className="mt-4 grid grid-cols-2 gap-4">
          <div>
            <span className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Latest Block</span>
            <p className="font-mono text-lg font-bold text-gray-900 dark:text-white">
              #{latestBlock.toLocaleString()}
            </p>
          </div>
          {finalizedBlock !== undefined && (
            <div>
              <span className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Finalized</span>
              <p className="font-mono text-lg font-bold text-gray-900 dark:text-white">
                #{finalizedBlock.toLocaleString()}
              </p>
            </div>
          )}
        </div>
      )}

      {error && (
        <div className="mt-4 flex items-start gap-2 p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
          <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
          <div>
            <p className="text-sm font-medium text-red-800 dark:text-red-200">Connection Error</p>
            <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
          </div>
        </div>
      )}
    </div>
  );
}

export default ConnectionStatus;
