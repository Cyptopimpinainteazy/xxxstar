'use client';

import React from 'react';
import Link from 'next/link';
import { useBlock, useBlockExtrinsics } from '@/hooks/useSubstrate';
import { useParams } from 'next/navigation';

export default function BlockDetailPage() {
  const params = useParams();
  const blockId = params.id as string;
  const blockNum = isNaN(Number(blockId)) ? blockId : Number(blockId);
  
  const { data: block, error: blockError, isLoading: blockLoading } = useBlock(blockNum);
  const { data: extrinsics, error: extError, isLoading: extLoading } = useBlockExtrinsics(blockNum);

  const formatHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 16)}...${hash.slice(-12)}`;
  };

  const formatTime = (timestamp: number | undefined) => {
    if (!timestamp) return 'Unknown';
    return new Date(timestamp).toLocaleString();
  };

  const getVmBadge = (section: string) => {
    if (section === 'atlasKernel') {
      return { label: 'Comit', color: 'bg-gradient-to-r from-blue-500 to-purple-500 text-white' };
    }
    if (section === 'evm' || section === 'ethereum') {
      return { label: 'EVM', color: 'bg-blue-100 text-blue-800' };
    }
    return { label: 'Native', color: 'bg-gray-100 text-gray-800' };
  };

  if (blockError) {
    return (
      <div className="min-h-screen bg-black py-8 pt-24">
        <div className="container-wide">
          <div className="glass-card p-8 text-center">
            <h1 className="text-2xl font-bold text-red-400 mb-4">Block Not Found</h1>
            <p className="text-gray-400 mb-4">
              Could not load block {blockId}. Make sure the node is running and the block exists.
            </p>
            <Link href="/explorer" className="text-cyan-400 hover:text-cyan-300">
              ← Back to Explorer
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black py-8 pt-24">
      <div className="container-wide">
        {/* Breadcrumb */}
        <nav className="mb-6">
          <ol className="flex items-center space-x-2 text-sm text-gray-500">
            <li><Link href="/explorer" className="hover:text-cyan-400">Explorer</Link></li>
            <li>/</li>
            <li><Link href="/explorer/blocks" className="hover:text-cyan-400">Blocks</Link></li>
            <li>/</li>
            <li className="text-gray-300 font-medium">Block #{blockId}</li>
          </ol>
        </nav>

        {/* Block Header */}
        <div className="glass-card mb-6">
          <div className="p-6 border-b border-[#1a1a1a]">
            <div className="flex items-center justify-between">
              <div className="flex items-center">
                <div className="w-16 h-16 bg-gradient-to-br from-blue-500 to-blue-600 rounded-lg flex items-center justify-center mr-4">
                  <span className="text-white text-2xl font-bold">#</span>
                </div>
                <div>
                  <h1 className="text-2xl font-bold text-white">
                    Block #{block?.number?.toLocaleString() || blockId}
                  </h1>
                  <p className="text-gray-500 text-sm">
                    {blockLoading ? 'Loading...' : formatTime(block?.timestamp)}
                  </p>
                </div>
              </div>
              
              {/* Navigation */}
              <div className="flex items-center space-x-2">
                {block && block.number > 0 && (
                  <Link
                    href={`/explorer/block/${block.number - 1}`}
                    className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#222222] rounded text-sm text-gray-200"
                  >
                    ← Prev
                  </Link>
                )}
                <Link
                  href={`/explorer/block/${(block?.number || Number(blockId)) + 1}`}
                  className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#222222] rounded text-sm text-gray-200"
                >
                  Next →
                </Link>
              </div>
            </div>
          </div>

          {/* Block Details */}
          {blockLoading ? (
            <div className="p-6 animate-pulse">
              <div className="space-y-4">
                {[...Array(6)].map((_, i) => (
                  <div key={i} className="flex">
                    <div className="w-32 h-4 bg-[#1a1a1a] rounded mr-4" />
                    <div className="flex-1 h-4 bg-[#1a1a1a] rounded" />
                  </div>
                ))}
              </div>
            </div>
          ) : block ? (
            <div className="p-6">
              <dl className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
                <div>
                  <dt className="text-sm text-gray-500">Block Number</dt>
                  <dd className="text-white font-medium">{block.number.toLocaleString()}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Timestamp</dt>
                  <dd className="text-gray-300">{formatTime(block.timestamp)}</dd>
                </div>
                <div className="md:col-span-2">
                  <dt className="text-sm text-gray-500">Block Hash</dt>
                  <dd className="text-gray-200 font-mono text-sm break-all bg-[#0a0a0a] p-2 rounded">
                    {block.hash}
                  </dd>
                </div>
                <div className="md:col-span-2">
                  <dt className="text-sm text-gray-500">Parent Hash</dt>
                  <dd className="text-gray-200 font-mono text-sm break-all bg-[#0a0a0a] p-2 rounded">
                    {block.parentHash}
                  </dd>
                </div>
                <div className="md:col-span-2">
                  <dt className="text-sm text-gray-500">State Root</dt>
                  <dd className="text-gray-200 font-mono text-sm break-all bg-[#0a0a0a] p-2 rounded">
                    {block.stateRoot}
                  </dd>
                </div>
                <div className="md:col-span-2">
                  <dt className="text-sm text-gray-500">Extrinsics Root</dt>
                  <dd className="text-gray-200 font-mono text-sm break-all bg-[#0a0a0a] p-2 rounded">
                    {block.extrinsicsRoot}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Extrinsics Count</dt>
                  <dd className="text-white font-medium">{block.extrinsicsCount}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Block Author</dt>
                  <dd className="text-gray-300 font-mono text-sm">
                    {block.author ? formatHash(block.author) : 'Unknown'}
                  </dd>
                </div>
              </dl>
            </div>
          ) : null}
        </div>

        {/* Extrinsics List */}
        <div className="glass-card">
          <div className="p-6 border-b border-[#1a1a1a]">
            <h2 className="text-lg font-semibold text-white">
              Extrinsics ({extrinsics?.length || 0})
            </h2>
          </div>

          {extLoading ? (
            <div className="p-6">
              <div className="animate-pulse space-y-4">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="h-20 bg-[#1a1a1a] rounded" />
                ))}
              </div>
            </div>
          ) : extrinsics && extrinsics.length > 0 ? (
            <ul className="divide-y divide-[#111111]">
              {extrinsics.map((ext, index) => {
                const vmBadge = getVmBadge(ext.section);
                return (
                  <li key={`${ext.hash}-${index}`} className="hover:bg-[#0a0a0a]">
                    <Link href={`/explorer/tx/${ext.hash}`} className="block p-4">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center">
                          <div className={`w-10 h-10 rounded-lg flex items-center justify-center mr-4 ${
                            ext.success ? 'bg-emerald-500/20' : 'bg-red-500/20'
                          }`}>
                            <span className="text-lg">
                              {ext.success ? '✓' : '✗'}
                            </span>
                          </div>
                          <div>
                            <div className="flex items-center gap-2">
                              <span className="font-medium text-white">
                                {ext.section}.{ext.method}
                              </span>
                              <span className={`px-2 py-0.5 text-xs rounded-full ${vmBadge.color}`}>
                                {vmBadge.label}
                              </span>
                            </div>
                            <div className="text-sm text-gray-500 font-mono">
                              {formatHash(ext.hash)}
                            </div>
                          </div>
                        </div>
                        <div className="text-right">
                          <div className={`text-sm ${ext.success ? 'text-emerald-400' : 'text-red-400'}`}>
                            {ext.success ? 'Success' : 'Failed'}
                          </div>
                          {ext.signer && (
                            <div className="text-xs text-gray-500">
                              {formatHash(ext.signer)}
                            </div>
                          )}
                        </div>
                      </div>
                    </Link>
                  </li>
                );
              })}
            </ul>
          ) : (
            <div className="p-8 text-center text-gray-500">
              No extrinsics in this block
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
