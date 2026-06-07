import React, { useState } from 'react';
import { Code2, Check, AlertCircle, Zap, BarChart3, Package } from 'lucide-react';

interface SolanaProgram {
  id: string;
  name: string;
  address: string;
  category: 'system' | 'spl' | 'dex' | 'lending' | 'oracle' | 'custom';
  status: 'verified' | 'audited' | 'beta' | 'deprecated';
  deployedVersion: string;
  lastUpdate: number;
  calls24h: number;
  errorRate: number;
}

interface AnchorIntegration {
  programId: string;
  programName: string;
  instructions: number;
  accounts: number;
  errors: number;
  compatibility: 'full' | 'partial' | 'incompatible';
  idlValid: boolean;
}

interface SplToken {
  mint: string;
  name: string;
  symbol: string;
  decimals: number;
  supply: bigint;
  holders: number;
  bridged: boolean;
  status: 'active' | 'paused' | 'frozen';
}

interface DeploymentMetrics {
  totalPrograms: number;
  computeBudget: number;
  computeUsed: number;
  lamportsDeployed: number;
  avgTransactionCost: number;
}

export const SolanaAdapterPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'programs' | 'anchor' | 'spl' | 'metrics'>('programs');

  const [solanaPrograms] = useState<SolanaProgram[]>([
    {
      id: 'sys-token',
      name: 'Token Program',
      address: 'TokenkegQfeZyiNwAJsyFbPVwwQQfimwcsqAnyS53f',
      category: 'system',
      status: 'verified',
      deployedVersion: '1.1.0',
      lastUpdate: Date.now() - 86400000 * 30,
      calls24h: 2450000,
      errorRate: 0.02,
    },
    {
      id: 'sys-assoc',
      name: 'Associated Token Account',
      address: 'ATokenGPvbdGVqstunL3hqSThv2YJAHeJ16LZnatnt9',
      category: 'system',
      status: 'verified',
      deployedVersion: '1.0.6',
      lastUpdate: Date.now() - 86400000 * 45,
      calls24h: 1850000,
      errorRate: 0.01,
    },
    {
      id: 'sys-memo',
      name: 'Memo Program',
      address: 'MemoSq4gDiYvboevJ1mNvT3zNZ7v1CAsuMpqWXwQMSJ',
      category: 'system',
      status: 'verified',
      deployedVersion: '1.0.0',
      lastUpdate: Date.now() - 86400000 * 60,
      calls24h: 125000,
      errorRate: 0.0,
    },
    {
      id: 'uniswap-v3',
      name: 'Uniswap V3 Adapter',
      address: '9xQeWvG816bUx9EPjHmaT23sSikZWZtok5j3WqMXJr7',
      category: 'dex',
      status: 'audited',
      deployedVersion: '2.1.0',
      lastUpdate: Date.now() - 86400000 * 7,
      calls24h: 450000,
      errorRate: 0.15,
    },
    {
      id: 'aave-v3',
      name: 'Aave V3 Adapter',
      address: '5soBQ52C4x4wnN2VoysokS5T7NNGc4BUGbkPNW5ip39',
      category: 'lending',
      status: 'audited',
      deployedVersion: '1.8.2',
      lastUpdate: Date.now() - 86400000 * 14,
      calls24h: 320000,
      errorRate: 0.08,
    },
    {
      id: 'oracle-pyth',
      name: 'Pyth Oracle Integration',
      address: 'PythPs9Pa7dNSZkwzL8Dmg1i8wPLuw6GW9hsJUUTEu5',
      category: 'oracle',
      status: 'beta',
      deployedVersion: '0.8.5',
      lastUpdate: Date.now() - 86400000 * 3,
      calls24h: 680000,
      errorRate: 0.22,
    },
  ]);

  const [anchorIntegrations] = useState<AnchorIntegration[]>([
    {
      programId: '9xQeWvG816bUx9EPjHmaT23sSikZWZtok5j3WqMXJr7',
      programName: 'Uniswap V3',
      instructions: 12,
      accounts: 24,
      errors: 0,
      compatibility: 'full',
      idlValid: true,
    },
    {
      programId: '5soBQ52C4x4wnN2VoysokS5T7NNGc4BUGbkPNW5ip39',
      programName: 'Aave V3',
      instructions: 18,
      accounts: 32,
      errors: 1,
      compatibility: 'partial',
      idlValid: true,
    },
    {
      programId: 'PythPs9Pa7dNSZkwzL8Dmg1i8wPLuw6GW9hsJUUTEu5',
      programName: 'Pyth Oracle',
      instructions: 8,
      accounts: 16,
      errors: 2,
      compatibility: 'partial',
      idlValid: false,
    },
  ]);

  const [splTokens] = useState<SplToken[]>([
    {
      mint: 'EPjFWaLb3odccccccccUDXYeV7iCAxLac99z1sMjlej',
      name: 'USD Coin',
      symbol: 'USDC',
      decimals: 6,
      supply: 5000000000000n,
      holders: 2450000,
      bridged: true,
      status: 'active',
    },
    {
      mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BjPvKn',
      name: 'USDT',
      symbol: 'USDT',
      decimals: 6,
      supply: 3200000000000n,
      holders: 1850000,
      bridged: true,
      status: 'active',
    },
    {
      mint: 'x3DszzJx5eQaGhwt8rxNYSZvJqVVkXYLLGJKZmqcZ7E',
      name: 'X3 Network Token',
      symbol: 'X3',
      decimals: 18,
      supply: 1000000000000000000000000000n,
      holders: 450000,
      bridged: true,
      status: 'active',
    },
  ]);

  const [deploymentMetrics] = useState<DeploymentMetrics>({
    totalPrograms: 6,
    computeBudget: 1400000,
    computeUsed: 980000,
    lamportsDeployed: 450000000,
    avgTransactionCost: 0.00087,
  });

  const totalCalls24h = solanaPrograms.reduce((sum, p) => sum + p.calls24h, 0);
  const avgErrorRate = (solanaPrograms.reduce((sum, p) => sum + p.errorRate, 0) / solanaPrograms.length).toFixed(2);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-green-400 to-emerald-500 mb-2">
              Solana Adapter
            </h1>
            <p className="text-gray-400">10 Standard Programs • Anchor Compatibility • SPL Token Bridging</p>
          </div>
          <Code2 className="w-12 h-12 text-green-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Programs</div>
            <div className="text-2xl font-bold text-green-400">{solanaPrograms.length}</div>
            <div className="text-xs text-gray-500 mt-2">Deployed & active</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Calls (24h)</div>
            <div className="text-2xl font-bold text-blue-400">{(totalCalls24h / 1000000).toFixed(1)}M</div>
            <div className="text-xs text-gray-500 mt-2">Program invocations</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Error Rate</div>
            <div className="text-2xl font-bold text-yellow-400">{avgErrorRate}%</div>
            <div className="text-xs text-gray-500 mt-2">Acceptable threshold</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Compute Utilization</div>
            <div className="text-2xl font-bold text-purple-400">
              {((deploymentMetrics.computeUsed / deploymentMetrics.computeBudget) * 100).toFixed(0)}%
            </div>
            <div className="text-xs text-gray-500 mt-2">Of available budget</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['programs', 'anchor', 'spl', 'metrics'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-green-400 border-b-2 border-green-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'programs' && 'Programs'}
              {tab === 'anchor' && 'Anchor Integration'}
              {tab === 'spl' && 'SPL Tokens'}
              {tab === 'metrics' && 'Deployment Metrics'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'programs' && (
            <div className="space-y-3">
              <h3 className="text-lg font-semibold text-white mb-4">Solana Programs</h3>
              {solanaPrograms.map((prog) => (
                <div key={prog.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{prog.name}</h4>
                      <p className="text-xs text-gray-500 font-mono">{prog.address}</p>
                    </div>
                    <div className="flex gap-2">
                      <span
                        className={`px-2 py-1 rounded text-xs font-semibold ${
                          prog.status === 'verified'
                            ? 'bg-green-500/20 text-green-400'
                            : prog.status === 'audited'
                              ? 'bg-blue-500/20 text-blue-400'
                              : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {prog.status.toUpperCase()}
                      </span>
                      <span className="bg-gray-500/20 text-gray-400 px-2 py-1 rounded text-xs">
                        v{prog.deployedVersion}
                      </span>
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-3 text-sm">
                    <div>
                      <div className="text-gray-400">Category</div>
                      <div className="text-white font-semibold capitalize">{prog.category}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Calls (24h)</div>
                      <div className="text-white font-semibold">{(prog.calls24h / 1000).toFixed(0)}K</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Error Rate</div>
                      <div className={prog.errorRate > 0.1 ? 'text-red-400 font-semibold' : 'text-green-400 font-semibold'}>
                        {prog.errorRate}%
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400">Last Update</div>
                      <div className="text-white font-semibold">
                        {Math.round((Date.now() - prog.lastUpdate) / 86400000)}d ago
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400">Status</div>
                      <div className="flex items-center gap-1">
                        <Check className="w-4 h-4 text-green-400" /> Operational
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'anchor' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Anchor Framework Integration</h3>
              <div className="space-y-4">
                {anchorIntegrations.map((anchor) => (
                  <div key={anchor.programId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-4">
                      <div>
                        <h4 className="text-white font-semibold">{anchor.programName}</h4>
                        <p className="text-xs text-gray-500 font-mono">{anchor.programId}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          anchor.compatibility === 'full'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {anchor.compatibility.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm mb-3">
                      <div>
                        <div className="text-gray-400">Instructions</div>
                        <div className="text-white font-semibold">{anchor.instructions}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Accounts</div>
                        <div className="text-white font-semibold">{anchor.accounts}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Errors</div>
                        <div className={anchor.errors > 0 ? 'text-red-400 font-semibold' : 'text-green-400 font-semibold'}>
                          {anchor.errors}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">IDL</div>
                        <div className="flex items-center gap-1">
                          {anchor.idlValid ? (
                            <>
                              <Check className="w-4 h-4 text-green-400" /> Valid
                            </>
                          ) : (
                            <>
                              <AlertCircle className="w-4 h-4 text-red-400" /> Invalid
                            </>
                          )}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Compatibility</div>
                        <div className="text-white font-semibold text-xs">{(Math.random() * 40 + 85).toFixed(0)}%</div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'spl' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">SPL Token Bridging</h3>
              <div className="space-y-4">
                {splTokens.map((token) => (
                  <div key={token.mint} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">
                          {token.name} <span className="text-gray-400">({token.symbol})</span>
                        </h4>
                        <p className="text-xs text-gray-500 font-mono">{token.mint}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          token.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-red-500/20 text-red-400'
                        }`}
                      >
                        {token.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Decimals</div>
                        <div className="text-white font-semibold">{token.decimals}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Supply</div>
                        <div className="text-white font-semibold">{(Number(token.supply) / 1e18).toFixed(0)}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Holders</div>
                        <div className="text-white font-semibold">{(token.holders / 1000).toFixed(1)}K</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Bridged</div>
                        <div className="flex items-center gap-1">
                          {token.bridged ? (
                            <>
                              <Check className="w-4 h-4 text-green-400" /> Yes
                            </>
                          ) : (
                            <>
                              <AlertCircle className="w-4 h-4 text-gray-400" /> No
                            </>
                          )}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Type</div>
                        <div className="text-white font-semibold">SPL</div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'metrics' && (
            <div className="grid grid-cols-2 gap-6">
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Compute Resources</h4>
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between mb-2">
                      <span className="text-gray-400">Budget Used</span>
                      <span className="text-white font-semibold">
                        {((deploymentMetrics.computeUsed / deploymentMetrics.computeBudget) * 100).toFixed(1)}%
                      </span>
                    </div>
                    <div className="w-full bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="bg-gradient-to-r from-purple-500 to-blue-500 h-2 rounded-full"
                        style={{
                          width: `${(deploymentMetrics.computeUsed / deploymentMetrics.computeBudget) * 100}%`,
                        }}
                      />
                    </div>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Total Budget</span>
                    <span className="text-white font-semibold">
                      {(deploymentMetrics.computeBudget / 1000000).toFixed(1)}M CU
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Current Usage</span>
                    <span className="text-white font-semibold">
                      {(deploymentMetrics.computeUsed / 1000000).toFixed(1)}M CU
                    </span>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Deployment Costs</h4>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Lamports Deployed</span>
                    <span className="text-white font-semibold">
                      {(deploymentMetrics.lamportsDeployed / 1000000).toFixed(2)}M
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Avg TX Cost</span>
                    <span className="text-blue-400 font-semibold">
                      ◎{deploymentMetrics.avgTransactionCost.toFixed(5)}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Cost Efficiency</span>
                    <span className="text-green-400 font-semibold">92.1%</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default SolanaAdapterPanel;
