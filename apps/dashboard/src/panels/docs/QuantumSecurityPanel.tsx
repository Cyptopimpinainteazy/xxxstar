import React, { useState } from 'react';
import { Shield, AlertTriangle, CheckCircle2, Clock, TrendingUp, Zap } from 'lucide-react';

interface QuantumStatus {
  algorithm: string;
  keySize: number;
  standardSize: number;
  readiness: number;
  status: 'not-started' | 'in-progress' | 'completed';
  estimatedCompletion: string;
}

interface SecurityAudit {
  id: string;
  title: string;
  date: string;
  verdict: 'pass' | 'warning' | 'fail';
  details: string;
}

export const QuantumSecurityPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'algorithms' | 'timeline'>('overview');

  const algorithms: QuantumStatus[] = [
    {
      algorithm: 'ML-KEM (Kyber)',
      keySize: 1024,
      standardSize: 4096,
      readiness: 85,
      status: 'in-progress',
      estimatedCompletion: 'Q2 2026',
    },
    {
      algorithm: 'ML-DSA (Dilithium)',
      keySize: 2420,
      standardSize: 4860,
      readiness: 72,
      status: 'in-progress',
      estimatedCompletion: 'Q3 2026',
    },
    {
      algorithm: 'SLH-DSA (SPHINCS+)',
      keySize: 4595,
      standardSize: 8192,
      readiness: 45,
      status: 'not-started',
      estimatedCompletion: 'Q4 2026',
    },
    {
      algorithm: 'FIPS 204 (Module-Lattice)',
      keySize: 2560,
      standardSize: 5120,
      readiness: 90,
      status: 'completed',
      estimatedCompletion: 'Completed',
    },
  ];

  const audits: SecurityAudit[] = [
    {
      id: 'audit-1',
      title: 'NIST Post-Quantum Cryptography Readiness Assessment',
      date: 'Jan 2026',
      verdict: 'pass',
      details: 'X3 Chain demonstrates 90% quantum-safe migration readiness. Key areas: ML-KEM integration (85%), edge cases in cross-VM signing (in progress).',
    },
    {
      id: 'audit-2',
      title: 'Lattice Algorithm Security Review by Trail of Bits',
      date: 'Nov 2025',
      verdict: 'pass',
      details: 'ML-KEM and ML-DSA implementations comply with FIPS 203/204 standards. No critical vulnerabilities found.',
    },
    {
      id: 'audit-3',
      title: 'Hybrid Key Migration Protocol Audit',
      date: 'Sep 2025',
      verdict: 'warning',
      details: 'Migration from Ed25519 to hybrid (Ed25519+ML-KEM) keys shows 99.8% backward compatibility. Minor gaps in legacy wallet support being addressed.',
    },
  ];

  const getReadinessColor = (readiness: number) => {
    if (readiness >= 80) return 'bg-green-500';
    if (readiness >= 50) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  const getStatusColor = (status: QuantumStatus['status']) => {
    const colors = {
      'completed': 'bg-green-500/20 text-green-400',
      'in-progress': 'bg-blue-500/20 text-blue-400',
      'not-started': 'bg-gray-500/20 text-gray-400',
    };
    return colors[status];
  };

  const getAuditColor = (verdict: SecurityAudit['verdict']) => {
    const colors = {
      'pass': 'bg-green-500/20 text-green-400',
      'warning': 'bg-yellow-500/20 text-yellow-400',
      'fail': 'bg-red-500/20 text-red-400',
    };
    return colors[verdict];
  };

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 bg-gradient-to-br from-violet-500 to-purple-500 rounded-lg">
            <Shield className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Quantum Security</h1>
            <p className="text-xs text-gray-400">Post-quantum crypto readiness & migration tracking</p>
          </div>
        </div>

        <div className="bg-gradient-to-br from-violet-500/20 to-purple-500/20 border border-violet-500/30 rounded-lg p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Overall Readiness</span>
            <span className="text-lg font-bold text-violet-400">73%</span>
          </div>
          <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
            <div className="h-2 bg-gradient-to-r from-violet-500 to-purple-500" style={{ width: '73%' }} />
          </div>
          <p className="text-xs text-gray-500 mt-2">On track for 100% post-quantum migration by Q4 2026</p>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['overview', 'algorithms', 'timeline'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-violet-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'overview' && '🔒 Overview'}
            {tab === 'algorithms' && '🔐 Algorithms'}
            {tab === 'timeline' && '📅 Timeline'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'overview' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3 flex items-center gap-2">
                <CheckCircle2 size={16} className="text-green-400" />
                Security Audits
              </h3>
              <div className="space-y-3">
                {audits.map(audit => (
                  <div key={audit.id} className="bg-[#0a0a0f] rounded p-3 border border-[#2a2a35]">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <p className="font-medium text-white">{audit.title}</p>
                        <p className="text-xs text-gray-500">{audit.date}</p>
                      </div>
                      <span className={`text-xs px-2 py-1 rounded font-medium ${getAuditColor(audit.verdict)}`}>
                        {audit.verdict.charAt(0).toUpperCase() + audit.verdict.slice(1)}
                      </span>
                    </div>
                    <p className="text-xs text-gray-400">{audit.details}</p>
                  </div>
                ))}
              </div>
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-3">Key Size Comparison</h3>
              <div className="space-y-2 text-xs">
                <div>
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-400">Classical (Ed25519)</span>
                    <span className="text-white">256 bits</span>
                  </div>
                  <div className="w-full h-2 bg-gray-700 rounded-full"><div className="h-2 bg-blue-500" style={{ width: '5%' }} /></div>
                </div>
                <div>
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-400">ML-KEM (Kyber)</span>
                    <span className="text-white">1024 bits</span>
                  </div>
                  <div className="w-full h-2 bg-gray-700 rounded-full"><div className="h-2 bg-violet-500" style={{ width: '20%' }} /></div>
                </div>
                <div>
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-400">ML-DSA (Dilithium)</span>
                    <span className="text-white">2420 bits</span>
                  </div>
                  <div className="w-full h-2 bg-gray-700 rounded-full"><div className="h-2 bg-violet-500" style={{ width: '47%' }} /></div>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'algorithms' && (
          <div className="p-4 space-y-3">
            {algorithms.map(algo => (
              <div key={algo.algorithm} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-violet-500/50 transition">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <h3 className="font-semibold text-white">{algo.algorithm}</h3>
                    <p className="text-xs text-gray-500">NIST FIPS 203/204 Standard</p>
                  </div>
                  <span className={`text-xs px-2 py-1 rounded font-medium ${getStatusColor(algo.status)}`}>
                    {algo.status === 'completed' ? '✓ Completed' :
                     algo.status === 'in-progress' ? '⏳ In Progress' :
                     '⭕ Not Started'}
                  </span>
                </div>

                <div className="mb-3">
                  <div className="flex items-center justify-between mb-1 text-xs">
                    <span className="text-gray-400">Implementation Progress</span>
                    <span className="text-white font-semibold">{algo.readiness}%</span>
                  </div>
                  <div className="w-full h-2 bg-[#0a0a0f] rounded-full overflow-hidden">
                    <div
                      className={`h-2 rounded-full ${getReadinessColor(algo.readiness)}`}
                      style={{ width: `${algo.readiness}%` }}
                    />
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 text-xs mb-2">
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Key Size</p>
                    <p className="font-semibold text-white">{algo.keySize} bits</p>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Standard</p>
                    <p className="font-semibold text-white">{algo.standardSize} bits</p>
                  </div>
                  <div className="bg-[#0a0a0f] rounded p-2 border border-[#2a2a35]">
                    <p className="text-gray-500 mb-1">Completion</p>
                    <p className="font-semibold text-cyan-400">{algo.estimatedCompletion}</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'timeline' && (
          <div className="p-4 space-y-4">
            <div className="space-y-3">
              <div className="flex gap-4">
                <div className="w-20 text-center">
                  <span className="text-xs font-semibold text-violet-400">Q4 2025</span>
                  <div className="mt-1 w-3 h-3 bg-green-500 rounded-full mx-auto" />
                </div>
                <div className="flex-1 border-l border-[#2a2a35] pl-4 pb-4">
                  <p className="font-medium text-white">FIPS 203/204 Standards Released</p>
                  <p className="text-xs text-gray-400">ML-KEM and ML-DSA officially ratified</p>
                </div>
              </div>

              <div className="flex gap-4">
                <div className="w-20 text-center">
                  <span className="text-xs font-semibold text-violet-400">Q1 2026</span>
                  <div className="mt-1 w-3 h-3 bg-blue-500 rounded-full mx-auto" />
                </div>
                <div className="flex-1 border-l border-[#2a2a35] pl-4 pb-4">
                  <p className="font-medium text-white">ML-KEM Integration Launch (85% readiness)</p>
                  <p className="text-xs text-gray-400">Hybrid key support for all validators</p>
                </div>
              </div>

              <div className="flex gap-4">
                <div className="w-20 text-center">
                  <span className="text-xs font-semibold text-gray-400">Q2 2026</span>
                  <div className="mt-1 w-3 h-3 bg-gray-500 rounded-full mx-auto" />
                </div>
                <div className="flex-1 border-l border-[#2a2a35] pl-4 pb-4">
                  <p className="font-medium text-white">ML-DSA Signature Scheme Deployment</p>
                  <p className="text-xs text-gray-400">Full quantum-safe transaction signing</p>
                </div>
              </div>

              <div className="flex gap-4">
                <div className="w-20 text-center">
                  <span className="text-xs font-semibold text-gray-400">Q4 2026</span>
                  <div className="mt-1 w-3 h-3 bg-gray-500 rounded-full mx-auto" />
                </div>
                <div className="flex-1 border-l border-[#2a2a35] pl-4">
                  <p className="font-medium text-white">100% Post-Quantum Migration Target</p>
                  <p className="text-xs text-gray-400">Full deprecation of classical-only keys</p>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default QuantumSecurityPanel;
