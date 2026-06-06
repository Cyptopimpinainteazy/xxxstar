import React, { useState } from 'react';
import { GitBranch, Lock, Send, TrendingUp, AlertTriangle, CheckCircle } from 'lucide-react';

interface BridgeEndpoint {
  id: string;
  name: string;
  chain: string;
  address: string;
  balance: number;
  status: 'active' | 'paused' | 'emergency';
  txsProcessed: number;
  securityAudits: number;
}

interface CrossChainMessage {
  id: string;
  source: string;
  destination: string;
  amount: number;
  status: 'pending' | 'relayed' | 'confirmed' | 'failed';
  timestamp: number;
  confirmations: number;
}

interface BridgeSecurityCouncil {
  id: string;
  entity: string;
  signer: string;
  signingPower: number;
  status: 'active' | 'inactive';
  lastSigned: number;
}

interface LiquidityPool {
  bridgeId: string;
  sourceChain: string;
  destChain: string;
  liquidityProvided: number;
  feesEarned: number;
  utilizationRate: number;
}

export const CrossChainBridgePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'endpoints' | 'messages' | 'council' | 'liquidity'>('endpoints');
  
  const [bridgeEndpoints] = useState<BridgeEndpoint[]>([
    {
      id: 'eth-canonical',
      name: 'Ethereum Canonical',
      chain: 'Ethereum',
      address: '0x3F5B7xF2a...',
      balance: 85420.5,
      status: 'active',
      txsProcessed: 45230,
      securityAudits: 4,
    },
    {
      id: 'solana-wormhole',
      name: 'Solana (Wormhole)',
      chain: 'Solana',
      address: '7xF2aQw34...',
      balance: 42150.25,
      status: 'active',
      txsProcessed: 28940,
      securityAudits: 3,
    },
    {
      id: 'cosmos-ibc',
      name: 'Cosmos (IBC)',
      chain: 'Cosmos Hub',
      address: 'cosmos1x3F5B7...',
      balance: 12840.75,
      status: 'active',
      txsProcessed: 5620,
      securityAudits: 2,
    },
    {
      id: 'bitcoin-htlc',
      name: 'Bitcoin (HTLC)',
      chain: 'Bitcoin',
      address: '3J98t1xF...',
      balance: 2.45,
      status: 'paused',
      txsProcessed: 840,
      securityAudits: 3,
    },
  ]);

  const [crossChainMessages] = useState<CrossChainMessage[]>([
    {
      id: 'msg-001',
      source: 'Ethereum',
      destination: 'Solana',
      amount: 150.5,
      status: 'confirmed',
      timestamp: Date.now() - 3600000,
      confirmations: 12,
    },
    {
      id: 'msg-002',
      source: 'Solana',
      destination: 'Cosmos',
      amount: 75.25,
      status: 'relayed',
      timestamp: Date.now() - 1800000,
      confirmations: 8,
    },
    {
      id: 'msg-003',
      source: 'Cosmos',
      destination: 'Ethereum',
      amount: 320.0,
      status: 'pending',
      timestamp: Date.now() - 600000,
      confirmations: 2,
    },
  ]);

  const [securityCouncil] = useState<BridgeSecurityCouncil[]>([
    {
      id: 'council-1',
      entity: 'Lido',
      signer: '0x742D35Cc...',
      signingPower: 16.67,
      status: 'active',
      lastSigned: Date.now() - 3600000,
    },
    {
      id: 'council-2',
      entity: 'Curve',
      signer: '0x8B5e2Fa...',
      signingPower: 16.67,
      status: 'active',
      lastSigned: Date.now() - 7200000,
    },
    {
      id: 'council-3',
      entity: 'Aave',
      signer: '0x9F2c3eA...',
      signingPower: 16.67,
      status: 'active',
      lastSigned: Date.now() - 1800000,
    },
    {
      id: 'council-4',
      entity: 'MakerDAO',
      signer: '0xA1b4dFc...',
      signingPower: 16.67,
      status: 'active',
      lastSigned: Date.now() - 10800000,
    },
    {
      id: 'council-5',
      entity: 'Balancer',
      signer: '0xB3c5eAe...',
      signingPower: 16.67,
      status: 'active',
      lastSigned: Date.now() - 5400000,
    },
  ]);

  const [liquidityPools] = useState<LiquidityPool[]>([
    {
      bridgeId: 'eth-sol',
      sourceChain: 'Ethereum',
      destChain: 'Solana',
      liquidityProvided: 5240000,
      feesEarned: 18500,
      utilizationRate: 65.3,
    },
    {
      bridgeId: 'sol-cosmos',
      sourceChain: 'Solana',
      destChain: 'Cosmos',
      liquidityProvided: 1850000,
      feesEarned: 4200,
      utilizationRate: 42.7,
    },
    {
      bridgeId: 'cosmos-eth',
      sourceChain: 'Cosmos',
      destChain: 'Ethereum',
      liquidityProvided: 2420000,
      feesEarned: 6850,
      utilizationRate: 38.5,
    },
  ]);

  const totalBridgedValue = bridgeEndpoints.reduce((sum, b) => sum + b.balance, 0);
  const activeBridges = bridgeEndpoints.filter((b) => b.status === 'active').length;
  const totalMessagesProcessed = bridgeEndpoints.reduce((sum, b) => sum + b.txsProcessed, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-500 mb-2">
              Cross-Chain Bridge Infrastructure
            </h1>
            <p className="text-gray-400">Ethereum • Solana • Cosmos • Bitcoin with Multi-Sig Security Council</p>
          </div>
          <GitBranch className="w-12 h-12 text-purple-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Bridged Value</div>
            <div className="text-2xl font-bold text-purple-400">${totalBridgedValue.toFixed(0)}</div>
            <div className="text-xs text-gray-500 mt-2">Across {activeBridges} chains</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Active Bridges</div>
            <div className="text-2xl font-bold text-green-400">{activeBridges}/4</div>
            <div className="text-xs text-gray-500 mt-2">All operational</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Messages Processed</div>
            <div className="text-2xl font-bold text-blue-400">{(totalMessagesProcessed / 1000).toFixed(0)}K</div>
            <div className="text-xs text-gray-500 mt-2">Total cross-chain txs</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Security Council</div>
            <div className="text-2xl font-bold text-yellow-400">5-of-5</div>
            <div className="text-xs text-gray-500 mt-2">Multi-sig active</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['endpoints', 'messages', 'council', 'liquidity'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-purple-400 border-b-2 border-purple-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'endpoints' && 'Bridge Endpoints'}
              {tab === 'messages' && 'Cross-Chain Messages'}
              {tab === 'council' && 'Security Council'}
              {tab === 'liquidity' && 'Liquidity Pools'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'endpoints' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Bridge Endpoints</h3>
              {bridgeEndpoints.map((bridge) => (
                <div key={bridge.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <h4 className="text-white font-semibold">{bridge.name}</h4>
                      <p className="text-xs text-gray-500 font-mono">{bridge.address}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        bridge.status === 'active'
                          ? 'bg-green-500/20 text-green-400'
                          : bridge.status === 'paused'
                            ? 'bg-yellow-500/20 text-yellow-400'
                            : 'bg-red-500/20 text-red-400'
                      }`}
                    >
                      {bridge.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400">Chain</div>
                      <div className="text-white font-semibold">{bridge.chain}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Balance</div>
                      <div className="text-white font-semibold">${bridge.balance.toFixed(2)}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Txs Processed</div>
                      <div className="text-white font-semibold">{bridge.txsProcessed.toLocaleString()}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Security Audits</div>
                      <div className="text-white font-semibold">{bridge.securityAudits}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Uptime</div>
                      <div className="text-green-400 font-semibold">99.7%</div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'messages' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Cross-Chain Message Relay</h3>
              <div className="space-y-4">
                {crossChainMessages.map((msg) => (
                  <div key={msg.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-3">
                        <div className="flex items-center text-sm text-gray-400">
                          <span className="text-white font-semibold">{msg.source}</span>
                          <Send className="w-4 h-4 mx-2 text-gray-500" />
                          <span className="text-white font-semibold">{msg.destination}</span>
                        </div>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          msg.status === 'confirmed'
                            ? 'bg-green-500/20 text-green-400'
                            : msg.status === 'relayed'
                              ? 'bg-blue-500/20 text-blue-400'
                              : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {msg.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Amount</div>
                        <div className="text-white font-semibold">{msg.amount} X3</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Message ID</div>
                        <div className="text-white font-mono text-xs">{msg.id}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Confirmations</div>
                        <div className="text-white font-semibold">{msg.confirmations}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Time</div>
                        <div className="text-white font-semibold">
                          {Math.round((Date.now() - msg.timestamp) / 60000)}m ago
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'council' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Bridge Security Council (5-of-5 Multi-Sig)</h3>
              <div className="space-y-3">
                {securityCouncil.map((member) => (
                  <div key={member.id} className="flex items-center gap-4 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex-shrink-0">
                      <Lock className="w-5 h-5 text-purple-400" />
                    </div>
                    <div className="flex-1">
                      <p className="text-white font-semibold">{member.entity}</p>
                      <p className="text-xs text-gray-500 font-mono">{member.signer}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-sm text-white font-semibold">{member.signingPower.toFixed(2)}%</p>
                      <p className="text-xs text-gray-400">Signing Power</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        member.status === 'active'
                          ? 'bg-green-500/20 text-green-400'
                          : 'bg-gray-500/20 text-gray-400'
                      }`}
                    >
                      {member.status.toUpperCase()}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-6 bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-lg p-4">
                <div className="flex items-start gap-3">
                  <Lock className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="text-sm font-semibold text-blue-400">Consensus Model</p>
                    <p className="text-xs text-gray-300 mt-2">
                      5-of-5 threshold required for bridge parameter changes. All signers have equal power. Quorum updates require 3-of-5 approval.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'liquidity' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Bridge Liquidity Pools</h3>
              <div className="space-y-4">
                {liquidityPools.map((pool) => (
                  <div key={pool.bridgeId} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-center justify-between mb-4">
                      <div>
                        <h4 className="text-white font-semibold">
                          {pool.sourceChain} ↔ {pool.destChain}
                        </h4>
                        <p className="text-sm text-gray-400">{pool.bridgeId}</p>
                      </div>
                    </div>
                    <div className="grid grid-cols-3 gap-4 md-3">
                      <div>
                        <div className="text-sm text-gray-400 mb-2">Liquidity Provided</div>
                        <div className="text-white font-semibold">${(pool.liquidityProvided / 1000000).toFixed(2)}M</div>
                      </div>
                      <div>
                        <div className="text-sm text-gray-400 mb-2">Fees Earned</div>
                        <div className="text-green-400 font-semibold">${(pool.feesEarned / 1000).toFixed(1)}K</div>
                      </div>
                      <div>
                        <div className="text-sm text-gray-400 mb-2">Utilization</div>
                        <div className="text-yellow-400 font-semibold">{pool.utilizationRate.toFixed(1)}%</div>
                      </div>
                    </div>
                    <div className="mt-3">
                      <div className="w-full bg-[#2a2a35] rounded-full h-2">
                        <div
                          className="bg-gradient-to-r from-cyan-500 to-blue-500 h-2 rounded-full"
                          style={{ width: `${pool.utilizationRate}%` }}
                        />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default CrossChainBridgePanel;
