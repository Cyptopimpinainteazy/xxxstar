import React, { useState } from 'react';
import { Shield, CheckCircle, AlertTriangle, TrendingUp, Calendar, DollarSign } from 'lucide-react';

interface AuditReport {
  id: string;
  auditor: string;
  date: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  score: number;
  vulnerabilities: number;
  status: 'completed' | 'in-progress' | 'pending';
  reportUrl?: string;
}

interface SecurityScore {
  category: string;
  score: number;
  trend: number;
  lastUpdated: string;
}

interface Remediation {
  id: string;
  issue: string;
  severity: string;
  remediated: boolean;
  completionDate?: string;
  auditId: string;
}

export const AuditDashboardPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'audits' | 'scores' | 'remediations' | 'timeline'>('audits');

  const [auditReports] = useState<AuditReport[]>([
    {
      id: 'audit-1',
      auditor: 'CertiK',
      date: '2024-02-15',
      severity: 'high',
      score: 92,
      vulnerabilities: 2,
      status: 'completed',
      reportUrl: 'certik-x3-audit-02-2024.pdf',
    },
    {
      id: 'audit-2',
      auditor: 'Trail of Bits',
      date: '2024-01-10',
      severity: 'medium',
      score: 88,
      vulnerabilities: 5,
      status: 'completed',
      reportUrl: 'tob-x3-audit-01-2024.pdf',
    },
    {
      id: 'audit-3',
      auditor: 'Halborn',
      date: '2024-03-01',
      severity: 'low',
      score: 95,
      vulnerabilities: 1,
      status: 'in-progress',
    },
  ]);

  const [securityScores] = useState<SecurityScore[]>([
    {
      category: 'Smart Contract Security',
      score: 92,
      trend: 3,
      lastUpdated: '2024-02-15',
    },
    {
      category: 'Infrastructure Security',
      score: 88,
      trend: 5,
      lastUpdated: '2024-02-20',
    },
    {
      category: 'Consensus Protocol',
      score: 95,
      trend: 2,
      lastUpdated: '2024-02-18',
    },
    {
      category: 'Key Management',
      score: 91,
      trend: 4,
      lastUpdated: '2024-02-19',
    },
    {
      category: 'Access Control',
      score: 87,
      trend: -1,
      lastUpdated: '2024-02-17',
    },
  ]);

  const [remediations] = useState<Remediation[]>([
    {
      id: 'rem-1',
      issue: 'Integer overflow in token supply calculation',
      severity: 'critical',
      remediated: true,
      completionDate: '2024-02-08',
      auditId: 'audit-1',
    },
    {
      id: 'rem-2',
      issue: 'Reentrancy guard missing on cross-chain bridge',
      severity: 'high',
      remediated: true,
      completionDate: '2024-01-25',
      auditId: 'audit-2',
    },
    {
      id: 'rem-3',
      issue: 'Insufficient input validation on RPC endpoints',
      severity: 'high',
      remediated: false,
      auditId: 'audit-2',
    },
    {
      id: 'rem-4',
      issue: 'Missing event logging on critical state changes',
      severity: 'medium',
      remediated: true,
      completionDate: '2024-02-12',
      auditId: 'audit-1',
    },
  ]);

  const avgScore = (auditReports.reduce((sum, a) => sum + a.score, 0) / auditReports.length).toFixed(1);
  const remediatedCount = remediations.filter((r) => r.remediated).length;
  const totalVulnerabilities = auditReports.reduce((sum, a) => sum + a.vulnerabilities, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-red-400 to-orange-500 mb-2">
              Audit Dashboard
            </h1>
            <p className="text-gray-400">CertiK • Trail of Bits • Halborn • Security Scores</p>
          </div>
          <Shield className="w-12 h-12 text-red-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Overall Security Score</div>
            <div className="text-2xl font-bold text-green-400">{avgScore}</div>
            <div className="text-xs text-gray-500 mt-2">Across all audits</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Vulnerabilities</div>
            <div className="text-2xl font-bold text-orange-400">{totalVulnerabilities}</div>
            <div className="text-xs text-gray-500 mt-2">Found across audits</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Remediations</div>
            <div className="text-2xl font-bold text-green-400">
              {remediatedCount}/{remediations.length}
            </div>
            <div className="text-xs text-gray-500 mt-2">Issues fixed</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Active Audits</div>
            <div className="text-2xl font-bold text-blue-400">1</div>
            <div className="text-xs text-gray-500 mt-2">Halborn in progress</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['audits', 'scores', 'remediations', 'timeline'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-red-400 border-b-2 border-red-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'audits' && 'Audit Reports'}
              {tab === 'scores' && 'Security Scores'}
              {tab === 'remediations' && 'Remediations'}
              {tab === 'timeline' && 'Timeline'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'audits' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-4">Audit Reports</h3>
              {auditReports.map((audit) => (
                <div key={audit.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{audit.auditor} Security Audit</h4>
                      <p className="text-sm text-gray-400">Completed: {audit.date}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        audit.status === 'completed'
                          ? 'bg-green-500/20 text-green-400'
                          : audit.status === 'in-progress'
                          ? 'bg-blue-500/20 text-blue-400'
                          : 'bg-gray-500/20 text-gray-400'
                      }`}
                    >
                      {audit.status.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400 text-xs">Security Score</div>
                      <div className="text-2xl font-bold text-green-400">{audit.score}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Vulnerabilities</div>
                      <div className="text-white font-semibold">{audit.vulnerabilities}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Max Severity</div>
                      <div
                        className={`font-semibold capitalize ${
                          audit.severity === 'critical'
                            ? 'text-red-400'
                            : audit.severity === 'high'
                            ? 'text-orange-400'
                            : audit.severity === 'medium'
                            ? 'text-yellow-400'
                            : 'text-green-400'
                        }`}
                      >
                        {audit.severity}
                      </div>
                    </div>
                    <div>
                      <CheckCircle className="w-5 h-5 text-green-400" />
                    </div>
                    {audit.reportUrl && (
                      <div>
                        <button className="px-3 py-1 text-xs bg-[#2a2a35] hover:bg-[#3a3a45] text-gray-400 rounded transition">
                          Download
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'scores' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Security Score Breakdown</h3>
              <div className="space-y-4">
                {securityScores.map((score) => (
                  <div key={score.category} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-baseline justify-between mb-3">
                      <h4 className="text-white font-semibold">{score.category}</h4>
                      <div className="flex items-center gap-2">
                        <div className="text-2xl font-bold text-green-400">{score.score}</div>
                        <div
                          className={`text-sm font-semibold flex items-center gap-1 ${
                            score.trend > 0 ? 'text-green-400' : score.trend < 0 ? 'text-red-400' : 'text-gray-400'
                          }`}
                        >
                          {score.trend > 0 ? '↑' : score.trend < 0 ? '↓' : '→'} {Math.abs(score.trend)} pts
                        </div>
                      </div>
                    </div>
                    <div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-3">
                        <div
                          className={`h-3 rounded-full ${
                            score.score >= 90
                              ? 'bg-green-500'
                              : score.score >= 80
                              ? 'bg-yellow-500'
                              : 'bg-red-500'
                          }`}
                          style={{ width: `${score.score}%` }}
                        />
                      </div>
                      <p className="text-gray-500 text-xs mt-2">Last updated: {score.lastUpdated}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'remediations' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Vulnerability Remediations</h3>
              <div className="space-y-3">
                {remediations.map((rem) => (
                  <div key={rem.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <h4 className="text-white font-semibold">{rem.issue}</h4>
                        <p className="text-sm text-gray-400">From audit: {rem.auditId}</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <span
                          className={`px-2 py-1 text-xs rounded font-semibold capitalize ${
                            rem.severity === 'critical'
                              ? 'bg-red-500/20 text-red-400'
                              : rem.severity === 'high'
                              ? 'bg-orange-500/20 text-orange-400'
                              : 'bg-yellow-500/20 text-yellow-400'
                          }`}
                        >
                          {rem.severity}
                        </span>
                        {rem.remediated ? (
                          <CheckCircle className="w-5 h-5 text-green-400" />
                        ) : (
                          <AlertTriangle className="w-5 h-5 text-orange-400" />
                        )}
                      </div>
                    </div>
                    {rem.remediated && rem.completionDate && (
                      <p className="text-sm text-gray-500">Fixed on: {rem.completionDate}</p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'timeline' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Audit Timeline</h3>
              <div className="space-y-4">
                {auditReports.map((audit, idx) => (
                  <div key={audit.id} className="flex gap-4">
                    <div className="flex flex-col items-center">
                      <Calendar className="w-5 h-5 text-blue-400 mb-2" />
                      {idx < auditReports.length - 1 && <div className="w-0.5 h-16 bg-[#2a2a35]" />}
                    </div>
                    <div className="pb-8">
                      <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4 w-full">
                        <div className="flex justify-between">
                          <div>
                            <h4 className="text-white font-semibold">{audit.auditor} Audit</h4>
                            <p className="text-sm text-gray-400">{audit.date}</p>
                          </div>
                          <div className="text-right">
                            <div className="text-2xl font-bold text-green-400">{audit.score}</div>
                            <p className="text-xs text-gray-500">{audit.vulnerabilities} vulns</p>
                          </div>
                        </div>
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

export default AuditDashboardPanel;
