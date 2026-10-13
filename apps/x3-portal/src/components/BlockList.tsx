'use client';

import React from 'react';
import Link from 'next/link';
import { useRecentBlocks, useNewHeads } from '@/hooks/useSubstrate';
import type { BlockInfo } from '@/lib/substrate';

interface BlockListProps {
  limit?: number;
  showPagination?: boolean;
}

export default function BlockList({ limit = 10, showPagination = true }: BlockListProps) {
  const { data: blocks, error, isLoading, mutate } = useRecentBlocks(limit);
  const { data: newHead } = useNewHeads();

  // Auto-refresh when new block arrives
  React.useEffect(() => {
    if (newHead) {
      mutate();
    }
  }, [newHead, mutate]);

  const formatTime = (timestamp: number | undefined) => {
    if (!timestamp) return 'Just now';
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

  if (error) {
    return (
      <div className="bg-white shadow rounded-lg p-6">
        <div className="flex flex-col items-center justify-center h-64">
          <div className="text-red-500 mb-2">Failed to load blocks</div>
          <p className="text-gray-500 text-sm mb-4">
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
      <div className="bg-white shadow rounded-lg overflow-hidden">
        <div className="px-4 py-5 sm:px-6 border-b">
          <h3 className="text-lg leading-6 font-medium text-gray-900">Recent Blocks</h3>
        </div>
        <div className="divide-y divide-gray-200">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="px-4 py-4 sm:px-6 animate-pulse">
              <div className="flex items-center justify-between">
                <div className="flex items-center">
                  <div className="w-10 h-10 bg-gray-200 rounded-lg"></div>
                  <div className="ml-4">
                    <div className="h-4 w-24 bg-gray-200 rounded mb-2"></div>
                    <div className="h-3 w-32 bg-gray-200 rounded"></div>
                  </div>
                </div>
                <div className="flex flex-col items-end">
                  <div className="h-4 w-16 bg-gray-200 rounded mb-2"></div>
                  <div className="h-3 w-12 bg-gray-200 rounded"></div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-md">
      <div className="px-4 py-5 sm:px-6 border-b flex justify-between items-center">
        <div>
          <h3 className="text-lg leading-6 font-medium text-gray-900">Recent Blocks</h3>
          <p className="mt-1 max-w-2xl text-sm text-gray-500">
            Latest blocks on the Atlas Sphere network
          </p>
        </div>
        {newHead && (
          <div className="flex items-center text-sm text-green-600">
            <div className="w-2 h-2 bg-green-500 rounded-full mr-2 animate-pulse"></div>
            Live
          </div>
        )}
      </div>
      
      {blocks && blocks.length > 0 ? (
        <ul className="divide-y divide-gray-200">
          {blocks.map((block: BlockInfo) => (
            <li key={block.hash} className="hover:bg-gray-50 transition-colors">
              <Link href={`/explorer/block/${block.number}`} className="block">
                <div className="px-4 py-4 sm:px-6">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center">
                      <div className="flex-shrink-0">
                        <div className="w-10 h-10 bg-gradient-to-br from-blue-500 to-blue-600 rounded-lg flex items-center justify-center shadow-sm">
                          <span className="text-white text-xs font-bold">
                            #{block.number.toString().slice(-3)}
                          </span>
                        </div>
                      </div>
                      <div className="ml-4">
                        <div className="text-sm font-medium text-gray-900">
                          Block #{block.number.toLocaleString()}
                        </div>
                        <div className="text-xs text-gray-500 font-mono">
                          {formatHash(block.hash)}
                        </div>
                      </div>
                    </div>
                    <div className="flex flex-col items-end">
                      <div className="text-sm text-gray-900">
                        {block.extrinsicsCount} extrinsics
                      </div>
                      <div className="text-xs text-gray-500">
                        {formatTime(block.timestamp)}
                      </div>
                    </div>
                  </div>
                  
                  <div className="mt-2 flex flex-wrap gap-2 sm:justify-between">
                    <div className="flex items-center text-xs text-gray-500">
                      <span className="mr-2">Author:</span>
                      <span className="font-mono bg-gray-100 px-2 py-0.5 rounded">
                        {block.author ? formatHash(block.author) : 'Unknown'}
                      </span>
                    </div>
                    <div className="flex items-center text-xs">
                      <span className="text-blue-600 hover:text-blue-700 font-medium">
                        View Details →
                      </span>
                    </div>
                  </div>
                </div>
              </Link>
            </li>
          ))}
        </ul>
      ) : (
        <div className="px-4 py-12 text-center text-gray-500">
          No blocks found. The chain may be starting up.
        </div>
      )}
      
      {showPagination && blocks && blocks.length >= limit && (
        <div className="px-4 py-3 bg-gray-50 border-t text-center">
          <Link 
            href="/explorer/blocks"
            className="text-sm text-blue-600 hover:text-blue-700 font-medium"
          >
            View All Blocks →
          </Link>
        </div>
      )}
    </div>
  );
}