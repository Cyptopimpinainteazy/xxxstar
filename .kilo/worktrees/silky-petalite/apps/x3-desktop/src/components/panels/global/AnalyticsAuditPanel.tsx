import React, { useState } from "react";
import { BarChart3, AlertTriangle, TrendingDown, FileCheck, Calendar, Shield } from "lucide-react";
import clsx from "clsx";

interface AuditHistory {
  id: string;
  date: string;
  auditFirm: string;
  contractVersion: string;
  scorePercentage: number;
  vulnerabilitiesFound: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  status: "completed" | "in-progress";
}

interface VulnerabilityTimeline {
  id: string;
  date: string;
  issue: string;
  severity: "critical" | "high" | "medium" | "low";
  resolved: boolean;
  fixedDate?: string;
}

const MOCK_AUDIT_HISTORY: AuditHistory[] = [
  {
    id: "1",
    date: "Feb 15, 2025",
    auditFirm: "CertiK",
    contractVersion: "v2.1.0",
    scorePercentage: 98,
    vulnerabilitiesFound: 0,
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
    status: "completed",
  },
  {
    id: "2",
    date: "Jan 20, 2025",
    auditFirm: "OpenZeppelin",
    contractVersion: "v2.0.0",
    scorePercentage: 96,
    vulnerabilitiesFound: 1,
    critical: 0,
    high: 0,
    medium: 1,
    low: 0,
    status: "completed",
  },
  {
    id: "3",
    date: "Dec 10, 2024",
    auditFirm: "Hacken",
    contractVersion: "v1.5.0",
    scorePercentage: 94,
    vulnerabilitiesFound: 2,
    critical: 0,
    high: 1,
    medium: 1,
    low: 0,
    status: "completed",
  },
];

const MOCK_VULNERABILITIES: VulnerabilityTimeline[] = [
  {
    id: "1",
    date: "Dec 8, 2024",
    issue: "Missing input validation in swap function",
    severity: "high",
    resolved: true,
    fixedDate: "Dec 12, 2024",
  },
  {
    id: "2",
    date: "Dec 10, 2024",
    issue: "Potential reentrancy in staking contract",
    severity: "medium",
    resolved: true,
    fixedDate: "Dec 15, 2024",
  },
  {
    id: "3",
    date: "Jan 18, 2025",
    issue: "Gas optimization opportunity identified",
    severity: "low",
    resolved: true,
    fixedDate: "Jan 25, 2025",
  },
];

export default function AnalyticsAuditPanel() {
  const [selectedAudit, setSelectedAudit] = useState<AuditHistory | null>(MOCK_AUDIT_HISTORY[0]);
  const [selectedVulnerability, setSelectedVulnerability] = useState<VulnerabilityTimeline | null>(null);
  const [filterSeverity, setFilterSeverity] = useState<"all" | "critical" | "high" | "medium" | "low">("all");

  const overallScore = Math.round(
    MOCK_AUDIT_HISTORY.reduce((sum, a) => sum + a.scorePercentage, 0) / MOCK_AUDIT_HISTORY.length
  );
  const resolvedVulnCount = MOCK_VULNERABILITIES.filter((v) => v.resolved).length;
  const criticalVulnCount = MOCK_VULNERABILITIES.filter((v) => v.severity === "critical").length;

  const filteredVulnerabilities = MOCK_VULNERABILITIES.filter(
    (v) => filterSeverity === "all" || v.severity === filterSeverity
  );

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "critical":
        return "bg-red-600/20 border-red-600 text-red-400";
      case "high":
        return "bg-orange-600/20 border-orange-600 text-orange-400";
      case "medium":
        return "bg-yellow-600/20 border-yellow-600 text-yellow-400";
      case "low":
        return "bg-blue-600/20 border-blue-600 text-blue-400";
      default:
        return "bg-gray-600/20 border-gray-600 text-gray-400";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <BarChart3 size={20} className="text-orange-400" /> Audit Analytics
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Security Score Overview */}
        <div className="bg-gradient-to-r from-orange-600/20 to-red-600/20 border border-orange-600 rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">Security Score</h3>
          <div className="flex items-center justify-between mb-3">
            <div>
              <div className="text-4xl font-bold text-orange-400">{overallScore}</div>
              <div className="text-xs text-gray-400">Overall Rating (from {MOCK_AUDIT_HISTORY.length} audits)</div>
            </div>
            <div className="text-right">
              <div className="text-2xl font-bold text-green-400">{resolvedVulnCount}/{MOCK_VULNERABILITIES.length}</div>
              <div className="text-xs text-gray-400">Vulnerabilities Resolved</div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-2">
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Latest Audit</div>
              <div className="font-bold">{MOCK_AUDIT_HISTORY[0].date}</div>
            </div>
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Current Version</div>
              <div className="font-bold">{MOCK_AUDIT_HISTORY[0].contractVersion}</div>
            </div>
          </div>

          {criticalVulnCount === 0 && (
            <div className="mt-3 p-2 bg-green-600/30 border border-green-600 rounded text-xs text-green-300 flex items-start gap-2">
              <Shield size={14} className="flex-shrink-0 mt-0.5" />
              <span>✓ No critical vulnerabilities found. Contract is secure.</span>
            </div>
          )}
        </div>

        {/* Audit History */}
        <div>
          <h3 className="font-semibold mb-3 text-sm flex items-center gap-2">
            <Calendar size={16} /> Audit History
          </h3>
          <div className="space-y-2">
            {MOCK_AUDIT_HISTORY.map((audit) => (
              <button
                key={audit.id}
                onClick={() => setSelectedAudit(audit)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedAudit?.id === audit.id
                    ? "border-orange-600 bg-orange-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold">{audit.auditFirm}</div>
                    <div className="text-xs text-gray-400">
                      {audit.date} • {audit.contractVersion}
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-lg font-bold text-orange-400">{audit.scorePercentage}%</div>
                    <span className="text-xs text-gray-400">score</span>
                  </div>
                </div>

                <div className="flex items-center gap-2 mb-2">
                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-orange-600 to-red-600"
                      style={{ width: `${audit.scorePercentage}%` }}
                    />
                  </div>
                </div>

                <div className="flex gap-2 text-xs">
                  {audit.critical > 0 && (
                    <span className="px-2 py-1 bg-red-600/30 border border-red-600 text-red-400 rounded">
                      {audit.critical} critical
                    </span>
                  )}
                  {audit.high > 0 && (
                    <span className="px-2 py-1 bg-orange-600/30 border border-orange-600 text-orange-400 rounded">
                      {audit.high} high
                    </span>
                  )}
                  {audit.medium > 0 && (
                    <span className="px-2 py-1 bg-yellow-600/30 border border-yellow-600 text-yellow-400 rounded">
                      {audit.medium} medium
                    </span>
                  )}
                  {audit.vulnerabilitiesFound === 0 && (
                    <span className="px-2 py-1 bg-green-600/30 border border-green-600 text-green-400 rounded">
                      No issues
                    </span>
                  )}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Selected Audit Details */}
        {selectedAudit && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <h3 className="font-semibold mb-3 text-sm">Audit Report</h3>

            <div className="space-y-3 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Audit Firm</span>
                <span className="font-semibold">{selectedAudit.auditFirm}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Date Completed</span>
                <span className="font-semibold">{selectedAudit.date}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Contract Version</span>
                <span className="font-mono text-xs">{selectedAudit.contractVersion}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Security Score</span>
                <span className={clsx("font-bold", selectedAudit.scorePercentage >= 95 ? "text-green-400" : "text-orange-400")}>
                  {selectedAudit.scorePercentage}%
                </span>
              </div>
            </div>

            <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg text-sm font-semibold transition mt-4 flex items-center justify-center gap-2">
              <FileCheck size={14} /> Download Full Report
            </button>
          </div>
        )}

        {/* Vulnerability Timeline */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-semibold text-sm flex items-center gap-2">
              <AlertTriangle size={16} className="text-red-400" /> Vulnerability Timeline
            </h3>
            <select
              value={filterSeverity}
              onChange={(e) => setFilterSeverity(e.target.value as any)}
              className="text-xs bg-[#15151b] border border-[#2a2a35] rounded px-2 py-1 focus:outline-none focus:border-blue-600"
            >
              <option value="all">All Severities</option>
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
          </div>

          <div className="space-y-2">
            {filteredVulnerabilities.map((vuln) => (
              <button
                key={vuln.id}
                onClick={() => setSelectedVulnerability(vuln)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedVulnerability?.id === vuln.id
                    ? "border-red-600 bg-red-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold">{vuln.issue}</div>
                    <div className="text-xs text-gray-400">Found: {vuln.date}</div>
                  </div>
                  <div className="text-right">
                    <span className={clsx("text-xs px-2 py-1 rounded border-2", getSeverityColor(vuln.severity))}>
                      {vuln.severity}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  {vuln.resolved ? (
                    <div className="flex items-center gap-1 text-xs text-green-400">
                      ✓ Resolved {vuln.fixedDate}
                    </div>
                  ) : (
                    <div className="flex items-center gap-1 text-xs text-orange-400">
                      ⏱ In progress
                    </div>
                  )}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Timeline Summary */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">Remediation Summary</h3>

          <div className="space-y-3">
            <div>
              <div className="flex justify-between mb-1">
                <span className="text-xs text-gray-400">Resolution Rate</span>
                <span className="text-xs font-bold">{((resolvedVulnCount / MOCK_VULNERABILITIES.length) * 100).toFixed(0)}%</span>
              </div>
              <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-green-600 to-blue-600"
                  style={{ width: `${(resolvedVulnCount / MOCK_VULNERABILITIES.length) * 100}%` }}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="bg-[#2a2a35] p-2 rounded">
                <div className="text-gray-400">Avg. Fix Time</div>
                <div className="font-bold">4 days</div>
              </div>
              <div className="bg-[#2a2a35] p-2 rounded">
                <div className="text-gray-400">Latest Fix</div>
                <div className="font-bold">Jan 25, 2025</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Regular audits and vulnerability tracking ensure long-term contract security.
      </div>
    </div>
  );
}
