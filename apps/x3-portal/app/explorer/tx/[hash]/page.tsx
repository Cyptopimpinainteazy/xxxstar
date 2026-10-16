'use client';

import React, { useEffect } from 'react';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import useSWR from 'swr';
import axios from 'axios';

// Fetch extrinsic by hash (this would need to be implemented in the API)
const fetcher = async (url: string) => {
  const res = await fetch(url);
  if (!res.ok) throw new Error('Failed to fetch');
  return res.json();
};

export default function TransactionDetailPage() {
  const params = useParams();
  const txHash = params.hash as string;

  // Analytics tracking
  useEffect(() => {
    const trackTransactionView = async () => {
      try {
        await axios.post('http://localhost:8080/event', {
          event_type: 'transaction_view',
          timestamp: new Date().toISOString(),
          user_id: null, // Placeholder - implement auth later
          metadata: { txHash }
        });
      } catch (error) {
        console.error('Failed to track transaction view:', error);
      }
    };

    trackTransactionView();
  }, [txHash]);

  const formatHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 16)}...${hash.slice(-12)}`;
  };

  return (
    <div className="min-h-screen bg-gray-100 py-8">
      <div className="container-wide">
        {/* Breadcrumb */}
        <nav className="mb-6">
          <ol className="flex items-center space-x-2 text-sm text-gray-500">
            <li><Link href="/explorer" className="hover:text-blue-600">Explorer</Link></li>
            <li>/</li>
            <li><Link href="/explorer/transactions" className="hover:text-blue-600">Transactions</Link></li>
            <li>/</li>
            <li className="text-gray-900 font-medium truncate max-w-[200px]">{formatHash(txHash)}</li>
          </ol>
        </nav>

        {/* Transaction Header */}
        <div className="bg-white rounded-lg shadow mb-6">
          <div className="p-6 border-b">
            <div className="flex items-center">
              <div className="w-16 h-16 bg-gradient-to-br from-cyan-500 to-cyan-600 rounded-lg flex items-center justify-center mr-4">
                <span className="text-white text-2xl">Tx</span>
              </div>
              <div>
                <h1 className="text-xl font-bold text-gray-900">Transaction Details</h1>
                <p className="text-gray-500 text-sm font-mono break-all max-w-2xl">
                  {txHash}
                </p>
              </div>
            </div>
          </div>

          {/* Transaction Details */}
          <div className="p-6">
            <dl className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
              <div className="md:col-span-2">
                <dt className="text-sm text-gray-500">Transaction Hash</dt>
                <dd className="text-gray-900 font-mono text-sm break-all bg-gray-50 p-2 rounded">
                  {txHash}
                </dd>
              </div>
              
              <div>
                <dt className="text-sm text-gray-500">Status</dt>
                <dd className="flex items-center">
                  <span className="w-2 h-2 bg-green-500 rounded-full mr-2"></span>
                  <span className="text-green-600 font-medium">Finalized</span>
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Type</dt>
                <dd className="text-gray-900">
                  <span className="px-2 py-1 bg-blue-100 text-blue-800 rounded text-sm">
                    Extrinsic
                  </span>
                </dd>
              </div>
            </dl>

            {/* Info Box */}
            <div className="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
              <h3 className="font-medium text-blue-900 mb-2">Transaction Lookup</h3>
              <p className="text-sm text-blue-700">
                Full transaction details require indexing. For development, use the block explorer 
                to view extrinsics by block, or implement a transaction indexer service.
              </p>
              <div className="mt-3">
                <Link 
                  href="/explorer"
                  className="text-sm text-blue-600 hover:text-blue-800 font-medium"
                >
                  ← Return to Explorer
                </Link>
              </div>
            </div>
          </div>
        </div>

        {/* Technical Details */}
        <div className="bg-white rounded-lg shadow">
          <div className="p-6 border-b">
            <h2 className="text-lg font-semibold text-gray-900">Technical Information</h2>
          </div>
          <div className="p-6">
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-medium text-gray-500 mb-2">Atlas Sphere Transaction Types</h3>
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                  <div className="p-4 bg-purple-50 rounded-lg">
                    <div className="font-medium text-purple-900">⚛ Comit</div>
                    <p className="text-sm text-purple-700 mt-1">
                      Atomic cross-VM transactions executing on both EVM and SVM
                    </p>
                  </div>
                  <div className="p-4 bg-blue-50 rounded-lg">
                    <div className="font-medium text-blue-900">◆ EVM</div>
                    <p className="text-sm text-blue-700 mt-1">
                      Ethereum-compatible smart contract calls
                    </p>
                  </div>
                  <div className="p-4 bg-green-50 rounded-lg">
                    <div className="font-medium text-green-900">◎ SVM</div>
                    <p className="text-sm text-green-700 mt-1">
                      Solana program invocations
                    </p>
                  </div>
                </div>
              </div>

              <div className="border-t pt-4">
                <h3 className="text-sm font-medium text-gray-500 mb-2">Extrinsic Format</h3>
                <p className="text-sm text-gray-600">
                  Atlas Sphere extrinsics follow the Substrate format with extensions for dual-VM payloads.
                  Each extrinsic contains section, method, arguments, and optional signatures.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
