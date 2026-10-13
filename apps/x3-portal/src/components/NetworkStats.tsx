'use client';

import React, { useMemo } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar, PieChart, Pie, Cell } from 'recharts';
import {
  useNetworkStats,
  useRecentBlocks,
  useRecentExtrinsics,
  useAuthorities,
} from '@/hooks/useSubstrate';

// Chart colors for VM distribution
const VM_COLORS = {
  evm: '#3b82f6',  // blue
  svm: '#8b5cf6',  // purple
  native: '#10b981', // green
};

export default function NetworkStats() {
  const { data: stats, error: statsError, isLoading: statsLoading } = useNetworkStats();
  const { data: blocks, error: blocksError } = useRecentBlocks(10);
  const { data: extrinsics, error: extrinsicsError } = useRecentExtrinsics(50);
  const { data: authorities, error: authError } = useAuthorities();

  // Calculate VM distribution from recent extrinsics
  const vmDistribution = useMemo(() => {
    if (!extrinsics) return [];
    
    const counts = { evm: 0, svm: 0, native: 0 };
    extrinsics.forEach(ext => {
      if (ext.section === 'atlasKernel') {
        // Comits use both VMs
        counts.evm++;
        counts.svm++;
      } else if (ext.section === 'evm' || ext.section === 'ethereum') {
        counts.evm++;
      } else {
        counts.native++;
      }
    });

    return [
      { name: 'EVM', value: counts.evm, color: VM_COLORS.evm },
      { name: 'SVM', value: counts.svm, color: VM_COLORS.svm },
      { name: 'Native', value: counts.native, color: VM_COLORS.native },
    ].filter(item => item.value > 0);
  }, [extrinsics]);

  // Calculate block production chart data (last 10 blocks)
  const blockProductionData = useMemo(() => {
    if (!blocks) return [];
    
    return blocks.slice().reverse().map((block, index) => ({
      name: `#${block.number}`,
      extrinsics: block.extrinsicsCount,
      blockNum: block.number,
    }));
  }, [blocks]);

  // Calculate transactions per minute (approximation)
  const txPerMinute = useMemo(() => {
    if (!blocks || blocks.length < 2) return 0;
    const recentBlocks = blocks.slice(0, 5);
    const totalExtrinsics = recentBlocks.reduce((sum, b) => sum + b.extrinsicsCount, 0);
    // ~5 blocks = ~30 seconds (6s block time)
    return Math.round(totalExtrinsics * 2); // Scale to per minute
  }, [blocks]);

  if (statsError || blocksError) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="text-center">
          <div className="text-red-500 mb-2">Failed to load network stats</div>
          <p className="text-gray-500 text-sm">Make sure the Atlas Sphere node is running</p>
          <code className="text-xs bg-gray-100 px-2 py-1 rounded mt-2 inline-block">
            ./run-dev-node.sh
          </code>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Main Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="bg-white overflow-hidden shadow rounded-lg">
          <div className="p-5">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <div className="w-10 h-10 bg-blue-500 rounded-md flex items-center justify-center">
                  <span className="text-white text-lg font-bold">#</span>
                </div>
              </div>
              <div className="ml-5 w-0 flex-1">
                <dl>
                  <dt className="text-sm font-medium text-gray-500 truncate">Latest Block</dt>
                  <dd className="text-xl font-bold text-gray-900">
                    {statsLoading ? (
                      <span className="animate-pulse bg-gray-200 rounded h-6 w-24 inline-block" />
                    ) : (
                      stats?.blockNumber?.toLocaleString()
                    )}
                  </dd>
                </dl>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white overflow-hidden shadow rounded-lg">
          <div className="p-5">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <div className="w-10 h-10 bg-green-500 rounded-md flex items-center justify-center">
                  <span className="text-white text-lg font-bold">⚡</span>
                </div>
              </div>
              <div className="ml-5 w-0 flex-1">
                <dl>
                  <dt className="text-sm font-medium text-gray-500 truncate">Tx/min</dt>
                  <dd className="text-xl font-bold text-gray-900">
                    {txPerMinute}
                  </dd>
                </dl>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white overflow-hidden shadow rounded-lg">
          <div className="p-5">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <div className="w-10 h-10 bg-purple-500 rounded-md flex items-center justify-center">
                  <span className="text-white text-lg font-bold">V</span>
                </div>
              </div>
              <div className="ml-5 w-0 flex-1">
                <dl>
                  <dt className="text-sm font-medium text-gray-500 truncate">Validators</dt>
                  <dd className="text-xl font-bold text-gray-900">
                    {authorities?.length ?? '...'}
                  </dd>
                </dl>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white overflow-hidden shadow rounded-lg">
          <div className="p-5">
            <div className="flex items-center">
              <div className="flex-shrink-0">
                <div className="w-10 h-10 bg-orange-500 rounded-md flex items-center justify-center">
                  <span className="text-white text-lg font-bold">⏱</span>
                </div>
              </div>
              <div className="ml-5 w-0 flex-1">
                <dl>
                  <dt className="text-sm font-medium text-gray-500 truncate">Block Time</dt>
                  <dd className="text-xl font-bold text-gray-900">
                    6s
                  </dd>
                  <dd className="text-xs text-gray-500">Aura Consensus</dd>
                </dl>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Chain Info */}
      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Chain Information</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <span className="text-sm text-gray-500">Chain Name</span>
            <p className="font-medium">{stats?.chain || 'Atlas Sphere'}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Node</span>
            <p className="font-medium">{stats?.nodeName || 'atlas-sphere-node'}</p>
          </div>
          <div>
            <span className="text-sm text-gray-500">Version</span>
            <p className="font-medium">{stats?.nodeVersion || '...'}</p>
          </div>
          <div className="md:col-span-3">
            <span className="text-sm text-gray-500">Latest Block Hash</span>
            <p className="font-mono text-sm break-all">{stats?.blockHash || '...'}</p>
          </div>
        </div>
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Block Production</h3>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={blockProductionData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="name" tick={{ fontSize: 10 }} />
              <YAxis allowDecimals={false} />
              <Tooltip
                formatter={(value: number) => [value, 'Extrinsics']}
                labelFormatter={(label) => `Block ${label}`}
              />
              <Bar dataKey="extrinsics" fill="#3b82f6" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-medium text-gray-900 mb-4">VM Distribution</h3>
          {vmDistribution.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <PieChart>
                <Pie
                  data={vmDistribution}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={100}
                  paddingAngle={5}
                  dataKey="value"
                  label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                >
                  {vmDistribution.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip />
              </PieChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-[300px] flex items-center justify-center text-gray-500">
              No transaction data yet
            </div>
          )}
        </div>
      </div>

      {/* Dual VM Status */}
      <div className="bg-white p-6 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-4">Dual VM Status</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="flex items-center p-4 border rounded-lg hover:border-blue-300 transition-colors">
            <div className="w-3 h-3 bg-green-500 rounded-full mr-3 animate-pulse"></div>
            <div className="flex-1">
              <p className="font-medium">EVM Adapter</p>
              <p className="text-sm text-gray-500">Frontier-based Ethereum compatibility</p>
            </div>
            <div className="text-right">
              <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                Active
              </span>
            </div>
          </div>
          <div className="flex items-center p-4 border rounded-lg hover:border-purple-300 transition-colors">
            <div className="w-3 h-3 bg-green-500 rounded-full mr-3 animate-pulse"></div>
            <div className="flex-1">
              <p className="font-medium">SVM Adapter</p>
              <p className="text-sm text-gray-500">Solana program execution layer</p>
            </div>
            <div className="text-right">
              <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                Active
              </span>
            </div>
          </div>
        </div>
        
        {/* Atlas Kernel Status */}
        <div className="mt-4 p-4 bg-gradient-to-r from-blue-50 to-purple-50 rounded-lg border border-blue-200">
          <div className="flex items-center">
            <div className="w-10 h-10 bg-gradient-to-r from-blue-500 to-purple-500 rounded-lg flex items-center justify-center mr-4">
              <span className="text-white font-bold">⚛</span>
            </div>
            <div>
              <p className="font-medium">Atlas Kernel</p>
              <p className="text-sm text-gray-600">
                Orchestrating atomic cross-VM transactions (Comits)
              </p>
            </div>
            <div className="ml-auto">
              <span className="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-gradient-to-r from-blue-500 to-purple-500 text-white">
                ✓ Operational
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Validators */}
      {authorities && authorities.length > 0 && (
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Active Validators</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {authorities.map((validator, index) => (
              <div key={validator.address} className="flex items-center p-3 bg-gray-50 rounded-lg">
                <div className="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center mr-3">
                  <span className="text-white text-sm font-bold">{index + 1}</span>
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-gray-900 truncate">
                    {validator.address.slice(0, 8)}...{validator.address.slice(-6)}
                  </p>
                  <p className="text-xs text-gray-500">
                    {validator.isCurrentAuthor ? '🔨 Current Author' : 'Validator'}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}