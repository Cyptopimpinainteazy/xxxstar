import React, { useState } from 'react';
import { AlertTriangle, CheckCircle2, Shield, TrendingUp, Download } from 'lucide-react';

interface ComplianceItem {
  id: string;
  name: string;
  category: 'security' | 'audit' | 'legal' | 'governance';
  status: 'compliant' | 'in-progress' | 'pending' | 'failed';
  dueDate: string;
  owner: string;
  evidence?: string;
}

export const ComplianceChecklistPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'checklist' | 'audit-trail' | 'reports'>('checklist');
  const [filterCategory, setFilterCategory] = useState<string | null>(null);

  const complianceItems: ComplianceItem[] = [
    {
      id: '1',
      name: 'External Security Audit (CertiK)',
      category: 'audit',
      status: 'in-progress',
      dueDate: '2026-03-31',
      owner: 'Security Team',
      evidence: 'Initial scope review completed, penetration testing scheduled',
    },
    {
      id: '2',
      name: 'Smart Contract Bytecode Verification',
      category: 'security',
      status: 'compliant',
      dueDate: '2026-02-28',
      owner: 'Dev Team',
      evidence: 'All contracts verified on chain',
    },
    {
      id: '3',
      name: 'SOC 2 Type II Certification',
      category: 'governance',
      status: 'in-progress',
      dueDate: '2026-06-30',
      owner: 'Compliance Officer',
      evidence: 'Control documentation in progress',
    },
    {
      id: '4',
      name: 'KYC/AML Framework Implementation',
      category: 'legal',
      status: 'pending',
      dueDate: '2026-04-15',
      owner: 'Legal Team',
      evidence: 'Waiting for regulatory guidance',
    },
    {
      id: '5',
      name: 'Dependency Vulnerability Audit',
      category: 'security',
      status: 'in-progress',
      dueDate: '2026-03-15',
      owner: 'DevSecOps',
      evidence: '89 of 126 npm vulnerabilities resolved',
    },
    {
      id: '6',
      name: 'GDPR Data Processing Agreement',
      category: 'legal',
      status: 'compliant',
      dueDate: '2026-02-15',
      owner: 'Legal Team',
      evidence: 'Signed and filed',
    },
    {
      id: '7',
      name: 'Formal TLA+ Specification',
      category: 'audit',
      status: 'in-progress',
      dueDate: '2026-05-31',
      owner: 'Research Team',
      evidence: '60% complete - consensus protocol specified',
    },
    {
      id: '8',
      name: 'Jurisdiction Access Controls',
      category: 'governance',
      status: 'compliant',
      dueDate: '2026-02-28',
      owner: 'DevOps',
      evidence: 'Sanctioned countries blocked at frontend',
    },
  ];

  const auditTrail = [
    {
      date: '2026-02-28',
      action: 'GDPR DPA signed',
      status: '✓ Compliant',
    },
    {
      date: '2026-02-25',
      action: 'Vulnerability scan: 12 critical findings patched',
      status: '✓ Resolved',
    },
    {
      date: '2026-02-20',
      action: 'CertiK audit scoping call',
      status: '⏳ In Progress',
    },
    {
      date: '2026-02-15',
      action: 'SOC 2 documentation initiated',
      status: '⏳ In Progress',
    },
  ];

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'compliant':
        return 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30';
      case 'in-progress':
        return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
      case 'pending':
        return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      default:
        return 'bg-red-500/20 text-red-400 border-red-500/30';
    }
  };

  const getCategoryColor = (cat: string) => {
    switch (cat) {
      case 'security':
        return 'text-red-400';
      case 'audit':
        return 'text-blue-400';
      case 'legal':
        return 'text-purple-400';
      default:
        return 'text-cyan-400';
    }
  };

  const filteredItems = filterCategory
    ? complianceItems.filter((item) => item.category === filterCategory)
    : complianceItems;

  const completionRate = (complianceItems.filter((i) => i.status === 'compliant').length / complianceItems.length) * 100;

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-orange-500/20 to-red-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Shield className="w-5 h-5 text-orange-400" />
          <h1 className="text-lg font-bold text-white">Compliance Checklist</h1>
        </div>
        <p className="text-sm text-gray-400">8 compliance items tracked, {completionRate.toFixed(0)}% complete</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['checklist', 'audit-trail', 'reports'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-orange-400 border-b-2 border-orange-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'checklist' && 'Checklist'}
            {tab === 'audit-trail' && 'Audit Trail'}
            {tab === 'reports' && 'Reports'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'checklist' && (
          <div className="p-6">
            {/* Filter Buttons */}
            <div className="flex gap-2 mb-6 flex-wrap">
              <button
                onClick={() => setFilterCategory(null)}
                className={`px-3 py-2 text-sm rounded transition ${
                  filterCategory === null
                    ? 'bg-orange-600 text-white'
                    : 'bg-[#2a2a35] text-gray-400 hover:text-gray-300'
                }`}
              >
                All Items
              </button>
              {['security', 'audit', 'legal', 'governance'].map((cat) => (
                <button
                  key={cat}
                  onClick={() => setFilterCategory(cat)}
                  className={`px-3 py-2 text-sm rounded transition ${
                    filterCategory === cat
                      ? 'bg-orange-600 text-white'
                      : 'bg-[#2a2a35] text-gray-400 hover:text-gray-300'
                  }`}
                >
                  {cat.charAt(0).toUpperCase() + cat.slice(1)}
                </button>
              ))}
            </div>

            {/* Compliance Items */}
            <div className="space-y-3">
              {filteredItems.map((item) => (
                <div key={item.id} className={`p-4 border rounded-lg hover:border-orange-500/30 transition ${getStatusColor(item.status)}`}>
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <h3 className="font-semibold text-white">{item.name}</h3>
                      <p className="text-xs text-gray-500 mt-1">Owner: {item.owner}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`text-xs font-semibold ${getCategoryColor(item.category)}`}>
                        {item.category.toUpperCase()}
                      </span>
                      {item.status === 'compliant' && <CheckCircle2 className="w-4 h-4 text-emerald-400" />}
                    </div>
                  </div>
                  {item.evidence && (
                    <p className="text-xs text-gray-300 mb-2 px-2 py-1 bg-black/30 rounded">{item.evidence}</p>
                  )}
                  <div className="flex justify-between text-xs">
                    <span className="text-gray-600">Due: {item.dueDate}</span>
                    <span className="px-2 py-1 bg-[#2a2a35] rounded text-gray-400">{item.status}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'audit-trail' && (
          <div className="p-6">
            <div className="space-y-4">
              {auditTrail.map((entry, idx) => (
                <div key={idx} className="flex gap-4 relative">
                  <div className="flex flex-col items-center">
                    {entry.status.includes('✓') ? (
                      <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                    ) : (
                      <AlertTriangle className="w-4 h-4 text-yellow-400" />
                    )}
                    {idx < auditTrail.length - 1 && (
                      <div className="w-0.5 h-12 bg-gradient-to-b from-[#2a2a35] to-transparent" />
                    )}
                  </div>
                  <div className="flex-1 pb-4">
                    <p className="text-xs font-semibold text-gray-400">{entry.date}</p>
                    <p className="text-sm text-gray-300 mt-1">{entry.action}</p>
                    <p className="text-xs text-gray-600 mt-1">{entry.status}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'reports' && (
          <div className="p-6 space-y-4">
            {['External Audit Report', 'SOC 2 Type II Certificate', 'Compliance Summary'].map((report, idx) => (
              <div key={idx} className="p-4 border border-[#2a2a35] rounded-lg hover:border-orange-500/30 transition">
                <div className="flex justify-between items-center">
                  <div>
                    <h3 className="font-semibold text-white">{report}</h3>
                    <p className="text-xs text-gray-500 mt-1">Updated: Feb 28, 2026</p>
                  </div>
                  <button className="flex items-center gap-2 px-3 py-2 bg-orange-600 hover:bg-orange-700 rounded text-white text-sm font-semibold transition">
                    <Download className="w-4 h-4" />
                    Export
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default ComplianceChecklistPanel;
