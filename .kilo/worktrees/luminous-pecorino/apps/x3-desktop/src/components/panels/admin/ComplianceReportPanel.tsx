import React, { useState } from "react";
import { Lock, Shield, FileText, Download, CheckCircle, AlertTriangle, Clock } from "lucide-react";
import clsx from "clsx";

interface ComplianceFrame {
  id: string;
  framework: string;
  status: "compliant" | "partial" | "non-compliant";
  checklist: number;
  total: number;
  dueDate: string;
  auditDate: string;
}

interface AuditLog {
  id: string;
  event: string;
  timestamp: string;
  category: string;
  severity: "info" | "warning" | "critical";
  evidence: string;
}

interface RegulatoryRequirement {
  id: string;
  requirement: string;
  region: string;
  status: "met" | "in-progress" | "pending";
  deadline: string;
}

const MOCK_FRAMEWORKS: ComplianceFrame[] = [
  {
    id: "1",
    framework: "SOC 2 Type II",
    status: "compliant",
    checklist: 45,
    total: 45,
    dueDate: "2024-12-31",
    auditDate: "2024-09-15",
  },
  {
    id: "2",
    framework: "GDPR Data Protection",
    status: "compliant",
    checklist: 28,
    total: 28,
    dueDate: "2024-12-31",
    auditDate: "2024-08-20",
  },
  {
    id: "3",
    framework: "ISO 27001",
    status: "partial",
    checklist: 78,
    total: 114,
    dueDate: "2025-06-30",
    auditDate: "2025-03-01",
  },
];

const MOCK_AUDIT_LOGS: AuditLog[] = [
  {
    id: "1",
    event: "SOC 2 Annual Audit Passed",
    timestamp: "2024-09-15",
    category: "Compliance",
    severity: "info",
    evidence: "Audit Report #AUD-2024-09-45",
  },
  {
    id: "2",
    event: "GDPR Data Processing Agreement Updated",
    timestamp: "2024-08-20",
    category: "Legal",
    severity: "info",
    evidence: "DPA v2.3 executed with all processors",
  },
  {
    id: "3",
    event: "Security Incident Log Review",
    timestamp: "2024-10-02",
    category: "Security",
    severity: "warning",
    evidence: "3 minor incidents, 0 material breaches",
  },
];

const MOCK_REQUIREMENTS: RegulatoryRequirement[] = [
  {
    id: "1",
    requirement: "Annual Penetration Testing",
    region: "Global",
    status: "met",
    deadline: "2024-12-31",
  },
  {
    id: "2",
    requirement: "GDPR Data Subject Rights Response (30d)",
    region: "EU",
    status: "met",
    deadline: "Continuous",
  },
  {
    id: "3",
    requirement: "ISO 27001 Gap Analysis",
    region: "Global",
    status: "in-progress",
    deadline: "2025-03-01",
  },
];

export default function ComplianceReportPanel() {
  const [frameworks, setFrameworks] = useState<ComplianceFrame[]>(MOCK_FRAMEWORKS);
  const [auditLogs, setAuditLogs] = useState<AuditLog[]>(MOCK_AUDIT_LOGS);
  const [requirements, setRequirements] = useState<RegulatoryRequirement[]>(MOCK_REQUIREMENTS);
  const [selectedFramework, setSelectedFramework] = useState<ComplianceFrame | null>(MOCK_FRAMEWORKS[0]);
  const [activeTab, setActiveTab] = useState<"frameworks" | "audit" | "requirements">("frameworks");

  const complianceCount = frameworks.filter((f) => f.status === "compliant").length;
  const overallScore = Math.round((complianceCount / frameworks.length) * 100);

  const getStatusColor = (status: string) => {
    switch (status) {
      case "compliant":
      case "met":
        return "text-green-400";
      case "partial":
      case "in-progress":
        return "text-yellow-400";
      case "non-compliant":
      case "pending":
        return "text-red-400";
      default:
        return "text-blue-400";
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "info":
        return "bg-blue-600/20 text-blue-300";
      case "warning":
        return "bg-yellow-600/20 text-yellow-300";
      case "critical":
        return "bg-red-600/20 text-red-300";
      default:
        return "bg-gray-600/20 text-gray-300";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} className="text-green-400" /> Compliance Dashboard
      </h2>

      {/* Overview */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Compliance Score</div>
          <div className="text-lg font-bold text-green-400">{overallScore}%</div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Frameworks</div>
          <div className="text-lg font-bold text-cyan-400">
            {complianceCount}/{frameworks.length}
          </div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Last Audit</div>
          <div className="text-lg font-bold text-blue-400">2024-09-15</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["frameworks", "audit", "requirements"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              "px-4 py-2 text-sm font-semibold transition border-b-2",
              activeTab === tab
                ? "border-cyan-600 text-cyan-400"
                : "border-transparent text-gray-400 hover:text-gray-300"
            )}
          >
            {tab === "frameworks" && "Frameworks"}
            {tab === "audit" && "Audit Logs"}
            {tab === "requirements" && "Requirements"}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {activeTab === "frameworks" && (
          <div className="space-y-3">
            {frameworks.map((framework) => (
              <button
                key={framework.id}
                onClick={() => setSelectedFramework(framework)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedFramework?.id === framework.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="font-semibold">{framework.framework}</div>
                    <div className="text-xs text-gray-400">Audit: {framework.auditDate}</div>
                  </div>
                  <span className={clsx("font-bold text-sm", getStatusColor(framework.status))}>
                    {framework.status.toUpperCase()}
                  </span>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden mb-1">
                  <div
                    className="h-full bg-gradient-to-r from-green-600 to-cyan-600"
                    style={{ width: `${(framework.checklist / framework.total) * 100}%` }}
                  />
                </div>
                <div className="text-xs text-gray-400">
                  {framework.checklist}/{framework.total} requirements met
                </div>
              </button>
            ))}
          </div>
        )}

        {activeTab === "audit" && (
          <div className="space-y-3">
            {auditLogs.map((log) => (
              <div
                key={log.id}
                className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2"
              >
                <div className="flex items-start justify-between">
                  <div className="font-semibold text-sm">{log.event}</div>
                  <span className={clsx("text-xs px-2 py-1 rounded-md", getSeverityColor(log.severity))}>
                    {log.severity.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-xs text-gray-400">
                  <div>
                    <span className="text-gray-500">Category:</span> {log.category}
                  </div>
                  <div>
                    <span className="text-gray-500">Date:</span> {log.timestamp}
                  </div>
                  <div>
                    <span className="text-gray-500">Evidence:</span> {log.evidence}
                  </div>
                </div>

                <button className="text-xs text-cyan-400 hover:text-cyan-300 flex items-center gap-1 mt-2">
                  <FileText size={12} /> View Full Report
                </button>
              </div>
            ))}
          </div>
        )}

        {activeTab === "requirements" && (
          <div className="space-y-3">
            {requirements.map((req) => (
              <div
                key={req.id}
                className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2"
              >
                <div className="flex items-start justify-between">
                  <div className="font-semibold text-sm">{req.requirement}</div>
                  <span className={clsx("font-bold text-sm", getStatusColor(req.status))}>
                    {req.status === "met" && <CheckCircle size={14} />}
                    {req.status === "in-progress" && <Clock size={14} />}
                    {req.status === "pending" && <AlertTriangle size={14} />}
                    {" " + req.status.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-xs text-gray-400">
                  <div>
                    <span className="text-gray-500">Region:</span> {req.region}
                  </div>
                  <div>
                    <span className="text-gray-500">Deadline:</span> {req.deadline}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Framework Details */}
        {selectedFramework && activeTab === "frameworks" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">{selectedFramework.framework} Details</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className={clsx("font-bold", getStatusColor(selectedFramework.status))}>
                  {selectedFramework.status.toUpperCase()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Completion</span>
                <span className="font-bold">{(selectedFramework.checklist / selectedFramework.total) * 100|0}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Last Audit</span>
                <span className="font-semibold">{selectedFramework.auditDate}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Audit Due</span>
                <span className="font-semibold">{selectedFramework.dueDate}</span>
              </div>
            </div>

            <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <Download size={14} /> Export Report
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Comprehensive compliance tracking and regulatory reporting.
      </div>
    </div>
  );
}
