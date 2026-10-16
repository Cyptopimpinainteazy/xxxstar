'use client';

import React from 'react';
import Link from 'next/link';
import { useAccount, useIsAuthorized, useCanonicalBalance, useFormattedBalance, useRecentExtrinsics } from '@/hooks/useSubstrate';
import { useParams } from 'next/navigation';

export default function AccountDetailPage() {
  const params = useParams();
  const address = params.address as string;
  
  const { data: account, error, isLoading } = useAccount(address);
  const { data: isAuthorized } = useIsAuthorized(address);
  const { data: atlasBalance } = useCanonicalBalance(address, 0); // Asset ID 0 = ATLAS
  const { data: recentExtrinsics, error: extrinsicsError, isLoading: extrinsicsLoading } = useRecentExtrinsics(50);
  
  const formattedAtlasBalance = useFormattedBalance(atlasBalance || '0', 18);
  const formattedFreeBalance = useFormattedBalance(account?.free || '0', 18);
  const formattedReservedBalance = useFormattedBalance(account?.reserved || '0', 18);

  const formatHash = (hash: string) => {
    if (!hash) return '';
    return `${hash.slice(0, 16)}...${hash.slice(-12)}`;
  };

  const filteredActivity = (recentExtrinsics || [])
    .filter((ext) => {
      if (ext.signer === address) return true;
      const argValues = Object.values(ext.args || {});
      return argValues.some((v) =>
        typeof v === 'string' && v.includes(address)
      );
    })
    .slice(0, 10);

  if (error) {
    return (
      <div className="min-h-screen bg-black py-8 pt-24">
        <div className="container-wide">
          <div className="glass-card p-8 text-center">
            <h1 className="text-2xl font-bold text-red-400 mb-4">Account Not Found</h1>
            <p className="text-gray-400 mb-4">
              Could not load account {formatHash(address)}. Make sure the node is running.
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
            <li><Link href="/explorer/accounts" className="hover:text-cyan-400">Accounts</Link></li>
            <li>/</li>
            <li className="text-gray-300 font-medium truncate max-w-[200px]">{formatHash(address)}</li>
          </ol>
        </nav>

        {/* Account Header */}
        <div className="glass-card mb-6">
          <div className="p-6 border-b border-[#1a1a1a]">
            <div className="flex items-center justify-between flex-wrap gap-4">
              <div className="flex items-center">
                <div className="w-16 h-16 bg-gradient-to-br from-purple-500 to-purple-600 rounded-full flex items-center justify-center mr-4">
                  <span className="text-white text-2xl font-bold">
                    {address.slice(0, 2).toUpperCase()}
                  </span>
                </div>
                <div>
                  <h1 className="text-xl font-bold text-white">Account</h1>
                  <p className="text-gray-400 text-sm font-mono break-all max-w-md">
                    {address}
                  </p>
                </div>
              </div>
              
              {/* Status Badges */}
              <div className="flex items-center gap-2">
                {isAuthorized && (
                  <span className="px-3 py-1 bg-gradient-to-r from-blue-500 to-purple-500 text-white text-sm rounded-full">
                    ⚛ Comit Authorized
                  </span>
                )}
                {account?.nonce && account.nonce > 0 && (
                  <span className="px-3 py-1 bg-[#1a1a1a] text-gray-300 text-sm rounded-full">
                    Active
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Balance Cards */}
          {isLoading ? (
            <div className="p-6 animate-pulse">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="h-24 bg-[#1a1a1a] rounded-lg" />
                ))}
              </div>
            </div>
          ) : (
            <div className="p-6">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="glass-card p-4 border border-blue-500/40 bg-gradient-to-br from-blue-500/20 to-purple-500/10">
                  <div className="text-sm text-gray-400 mb-1">Canonical Balance (ATLAS)</div>
                  <div className="text-2xl font-bold text-white">{formattedAtlasBalance}</div>
                  <div className="text-xs text-gray-500 mt-1">Cross-VM unified balance</div>
                </div>

                <div className="glass-card p-4">
                  <div className="text-sm text-gray-400 mb-1">Free Balance</div>
                  <div className="text-2xl font-bold text-white">{formattedFreeBalance}</div>
                  <div className="text-xs text-gray-500 mt-1">Native Substrate balance</div>
                </div>

                <div className="glass-card p-4">
                  <div className="text-sm text-gray-400 mb-1">Reserved Balance</div>
                  <div className="text-2xl font-bold text-white">{formattedReservedBalance}</div>
                  <div className="text-xs text-gray-500 mt-1">Locked for staking/governance</div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Account Details */}
        <div className="glass-card mb-6">
          <div className="p-6 border-b border-[#1a1a1a]">
            <h2 className="text-lg font-semibold text-white">Account Details</h2>
          </div>
          
          {isLoading ? (
            <div className="p-6 animate-pulse">
              <div className="space-y-4">
                {[...Array(4)].map((_, i) => (
                  <div key={i} className="flex">
                    <div className="w-32 h-4 bg-gray-200 rounded mr-4"></div>
                    <div className="flex-1 h-4 bg-gray-200 rounded"></div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="p-6">
              <dl className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
                <div className="md:col-span-2">
                  <dt className="text-sm text-gray-500">Address</dt>
                  <dd className="text-gray-200 font-mono text-sm break-all bg-[#0a0a0a] p-2 rounded">
                    {address}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Nonce</dt>
                  <dd className="text-white font-medium">{account?.nonce || 0}</dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Comit Authorized</dt>
                  <dd className={`font-medium ${isAuthorized ? 'text-emerald-400' : 'text-gray-500'}`}>
                    {isAuthorized ? 'Yes ✓' : 'No'}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm text-gray-500">Sufficiency</dt>
                  <dd className="text-gray-300">
                    {account?.sufficients || 0} sufficient refs, {account?.consumers || 0} consumers, {account?.providers || 0} providers
                  </dd>
                </div>
              </dl>
            </div>
          )}
        </div>

        {/* Comit Authorization Info */}
        {isAuthorized && (
          <div className="bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg shadow p-6 text-white mb-6">
            <h2 className="text-lg font-semibold mb-2">⚛ Comit Authorized Account</h2>
            <p className="text-sm opacity-90 mb-4">
              This account is authorized to submit atomic cross-VM transactions (Comits) 
              that execute simultaneously on both EVM and SVM.
            </p>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
              <div>
                <span className="opacity-70">Capabilities:</span>
                <ul className="mt-1 space-y-1">
                  <li>• Submit Comit transactions</li>
                  <li>• Execute atomic cross-VM swaps</li>
                  <li>• Access canonical ledger</li>
                </ul>
              </div>
              <div>
                <span className="opacity-70">Limits:</span>
                <ul className="mt-1 space-y-1">
                  <li>• EVM payload: 16KB max</li>
                  <li>• SVM payload: 16KB max</li>
                  <li>• Combined: 32KB max</li>
                </ul>
              </div>
            </div>
          </div>
        )}

        {/* Recent Activity */}
        <div className="glass-card">
          <div className="p-6 border-b border-[#1a1a1a] flex items-center justify-between">
            <h2 className="text-lg font-semibold text-white">Recent Activity</h2>
            <span className="text-xs text-gray-500">
              Showing up to 10 recent extrinsics involving this account
            </span>
          </div>

          {extrinsicsError && (
            <div className="p-6 text-center text-red-400 text-sm">
              Unable to load recent activity. Please check your node connection.
            </div>
          )}

          {!extrinsicsError && extrinsicsLoading && (
            <div className="p-6 space-y-3">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="flex items-center justify-between bg-[#0a0a0a] border border-[#1a1a1a] rounded-lg px-4 py-3 animate-pulse">
                  <div className="h-4 w-40 bg-[#111111] rounded" />
                  <div className="h-4 w-24 bg-[#111111] rounded" />
                  <div className="h-4 w-32 bg-[#111111] rounded" />
                </div>
              ))}
            </div>
          )}

          {!extrinsicsError && !extrinsicsLoading && filteredActivity.length === 0 && (
            <div className="p-8 text-center text-gray-500 text-sm">
              <p>No recent extrinsics found for this account in the latest blocks.</p>
              <p className="mt-2">Try again later or explore network-wide activity from the Explorer home.</p>
            </div>
          )}

          {!extrinsicsError && !extrinsicsLoading && filteredActivity.length > 0 && (
            <div className="p-6 space-y-3">
              {filteredActivity.map((ext) => (
                <div
                  key={`${ext.blockNumber}-${ext.index}`}
                  className="flex flex-col md:flex-row md:items-center md:justify-between bg-[#0a0a0a] border border-[#1a1a1a] rounded-lg px-4 py-3 gap-2"
                >
                  <div className="flex items-center gap-3">
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                      ext.success ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30' : 'bg-red-500/10 text-red-400 border border-red-500/30'
                    }`}>
                      {ext.success ? 'Success' : 'Failed'}
                    </span>
                    <div className="flex flex-col">
                      <span className="text-sm text-white font-mono">
                        {ext.section}.{ext.method}
                      </span>
                      <span className="text-xs text-gray-500">
                        Block #{ext.blockNumber.toLocaleString()}
                      </span>
                    </div>
                  </div>
                  <div className="flex flex-col items-start md:items-end text-xs">
                    <span className="text-gray-500 mb-1">Extrinsic Hash</span>
                    <span className="font-mono text-gray-300 bg-[#050505] px-2 py-1 rounded break-all max-w-full md:max-w-md">
                      {formatHash(ext.hash)}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
