/**
 * Enhanced TransactionList component with real blockchain data integration
 * Displays extrinsics across EVM, SVM, and Native execution
 */
'use client';

import React from 'react';
import Link from 'next/link';
import { useRecentExtrinsics, useNewHeads } from '@/hooks/useSubstrate';
import type { ExtrinsicInfo } from '@/lib/substrate';

interface TransactionListProps {
  limit?: number;
  showPagination?: boolean;
}

// VM badge styling based on extrinsic type
const getVmBadge = (section: string, _method: string) => {
  if (section === 'atlasKernel') {
    return {
      label: 'Comit',
      color: 'bg-gradient-to-r from-blue-500 to-purple-500 text-white',
      icon: '⚛',
    };
  }
  if (section === 'evm' || section === 'ethereum') {
    return {
      label: 'EVM',
      color: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      icon: '◆',
    };
  }
  if (section === 'svm' || section === 'solana') {
    return {
      label: 'SVM',
      color: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
      icon: '◎',
    };
  }
  return {
    label: 'Native',
    color: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200',
    icon: '○',
  };
};

export default function TransactionList({ limit = 20, showPagination = true }: TransactionListProps) {
  const { data: extrinsics, error, isLoading, mutate } = useRecentExtrinsics(limit);
  const { data: newHead } = useNewHeads();

  // Auto-refresh when new block arrives
  React.useEffect(() => {
    if (newHead) {
      mutate();
    }
  }, [newHead, mutate]);

  const formatTime = (timestamp: number | undefined) => {
    if (!timestamp) return 'Pending';
    const now = Date.now();
    const diff = now - timestamp;
    
    if (diff < 1000) return 'Just now';
    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return `${Math.floor(diff / 3600000)}h ago`;
  };

  const formatHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
  };

  const statusColor = (success: boolean) =>
    success ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400';
  const statusIcon = (success: boolean) => (success ? '✓' : '✗');

  if (error) {
    return (
      <div className="bg-white dark:bg-gray-800 shadow-sm rounded-lg p-6">
        <div className="flex flex-col items-center justify-center h-64">
          <p className="text-red-500 mb-2">Failed to load transactions</p>
          <p className="text-gray-500 dark:text-gray-400 text-sm mb-4">
            Make sure the Atlas Sphere node is running
          </p>
          <button 
            onClick={() => mutate()}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="bg-white dark:bg-gray-800 shadow-sm rounded-lg overflow-hidden">
        <header className="bg-gradient-to-r from-indigo-500 to-purple-600 text-white px-6 py-4">
          <h3 className="text-lg font-semibold">Recent Transactions</h3>
          <p className="text-sm opacity-80">Loading...</p>
        </header>
        <div className="divide-y divide-gray-200 dark:divide-gray-700">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="px-4 py-4 animate-pulse">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-16 h-6 bg-gray-200 dark:bg-gray-700 rounded"></div>
                  <div>
                    <div className="h-4 w-32 bg-gray-200 dark:bg-gray-700 rounded mb-2"></div>
                    <div className="h-3 w-40 bg-gray-200 dark:bg-gray-700 rounded"></div>
                  </div>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <div className="h-4 w-20 bg-gray-200 dark:bg-gray-700 rounded"></div>
                  <div className="h-3 w-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 shadow-sm rounded-lg overflow-hidden">
      <header className="bg-gradient-to-r from-indigo-500 to-purple-600 text-white px-6 py-4 flex justify-between items-center">
        <div>
          <h3 className="text-lg font-semibold">Recent Transactions</h3>
          <p className="text-sm opacity-80">EVM + SVM + Native activity</p>
        </div>
        {newHead && (
          <div className="flex items-center text-sm">
            <div className="w-2 h-2 bg-green-400 rounded-full mr-2 animate-pulse"></div>
            Live
          </div>
        )}
      </header>
      
      {extrinsics && extrinsics.length > 0 ? (
        <ul className="divide-y divide-gray-200 dark:divide-gray-700">
          {extrinsics.map((ext: ExtrinsicInfo, index: number) => {
            const vmBadge = getVmBadge(ext.section, ext.method);
            
            return (
              <li
                key={`${ext.hash}-${index}`}
                className="hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              >
                <Link href={`/explorer/tx/${ext.hash}`} className="block">
                  <div className="px-4 py-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                    <div className="flex items-center gap-4">
                      <span className={`inline-flex items-center px-2.5 py-1 rounded text-xs font-medium ${vmBadge.color}`}>
                        {vmBadge.icon} {vmBadge.label}
                      </span>
                      <div className="flex flex-col">
                        <span className="font-medium text-gray-900 dark:text-gray-100">
                          {ext.section}.{ext.method}
                        </span>
                        <span className="text-sm text-gray-500 dark:text-gray-400 font-mono truncate">
                          {formatHash(ext.hash)}
                        </span>
                        {ext.signer && (
                          <span className="text-sm text-gray-500 dark:text-gray-400 truncate">
                            From: {formatHash(ext.signer)}
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="flex flex-col items-end gap-1">
                      <span className="font-medium text-gray-900 dark:text-gray-100">
                        Block #{ext.blockNumber?.toLocaleString() || 'Pending'}
                      </span>
                      <span className={`text-sm ${statusColor(ext.success)}`}>
                        {statusIcon(ext.success)} {ext.success ? 'Success' : 'Failed'}
                      </span>
                      <span className="text-xs text-gray-500 dark:text-gray-400">
                        {formatTime(ext.timestamp)}
                      </span>
                    </div>
                  </div>
                </Link>
              </li>
            );
          })}
        </ul>
      ) : (
        <div className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
          No transactions found. Submit a transaction to see it here.
        </div>
      )}
      
      {showPagination && extrinsics && extrinsics.length >= limit && (
        <div className="px-4 py-3 bg-gray-50 dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 text-center">
          <Link 
            href="/explorer/transactions"
            className="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 font-medium"
          >
            View All Transactions →
          </Link>
        </div>
      )}
    </div>
  );
}
