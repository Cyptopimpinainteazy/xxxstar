import React, { useState } from "react";
import { CheckCircle, AlertCircle, Code, FileText, Download } from "lucide-react";
import clsx from "clsx";

interface AuditResult {
  id: string;
  name: string;
  severity: "critical" | "high" | "medium" | "low" | "info";
  description: string;
  status: "passed" | "warning";
}

const MOCK_AUDIT: AuditResult[] = [
  {
    id: "1",
    name: "Reentrancy Vulnerability",
    severity: "critical",
    description: "Contract properly uses checks-effects-interactions pattern.",
    status: "passed",
  },
  {
    id: "2",
    name: "Integer Overflow/Underflow",
    severity: "high",
    description: "Using Solidity 0.8+ which has built-in overflow protection.",
    status: "passed",
  },
  {
    id: "3",
    name: "Unchecked External Calls",
    severity: "high",
    description: "All external calls have proper error handling.",
    status: "passed",
  },
  {
    id: "4",
    name: "Access Control",
    severity: "medium",
    description: "Admin functions properly restricted to owner.",
    status: "passed",
  },
];

export default function TokenAuditPanel() {
  const [auditStatus, setAuditStatus] = useState<"idle" | "running" | "complete">("complete");
  const [selectedResult, setSelectedResult] = useState<AuditResult | null>(null);
  const [auditProvider, setAuditProvider] = useState("certik");

  const criticalIssues = MOCK_AUDIT.filter((a) => a.severity === "critical").length;
  const passedChecks = MOCK_AUDIT.filter((a) => a.status === "passed").length;
  const auditScore = Math.round((passedChecks / MOCK_AUDIT.length) * 100);

  const handleRunAudit = () => {
    setAuditStatus("running");
    setTimeout(() => setAuditStatus("complete"), 2000);
  };

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
      case "info":
        return "bg-gray-600/20 border-gray-600 text-gray-400";
      default:
        return "bg-gray-600/20 border-gray-600 text-gray-400";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <CheckCircle size={20} /> Smart Contract Audit
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Audit Score Card */}
        {auditStatus === "complete" && (
          <div className="bg-gradient-to-r from-green-600/20 to-blue-600/20 border border-green-600 rounded-lg p-4">
            <div className="flex items-center justify-between mb-3">
              <h3 className="font-semibold text-green-400">✓ Audit Complete</h3>
              <div className="text-3xl font-bold text-green-400">{auditScore}%</div>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="bg-[#15151b] p-2 rounded">
                <div className="text-gray-400">Checks Passed</div>
                <div className="font-bold text-green-400">{passedChecks}/{MOCK_AUDIT.length}</div>
              </div>
              <div className="bg-[#15151b] p-2 rounded">
                <div className="text-gray-400">Issues Found</div>
                <div className={clsx("font-bold", criticalIssues > 0 ? "text-red-400" : "text-green-400")}>
                  {criticalIssues}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Provider Selection */}
        {auditStatus === "idle" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <h3 className="font-semibold mb-3 flex items-center gap-2">
              <Code size={16} /> Select Audit Provider
            </h3>
            <div className="space-y-2">
              {[
                { id: "certik", name: "CertiK", desc: "Automated + manual review", time: "5-10 min" },
                { id: "hacken", name: "Hacken", desc: "AI-powered security scan", time: "2-5 min" },
                { id: "slowmist", name: "SlowMist", desc: "Community rating system", time: "1-3 min" },
              ].map((provider) => (
                <label
                  key={provider.id}
                  className="flex items-center gap-3 p-3 rounded cursor-pointer hover:bg-[#2a2a35] border border-transparent hover:border-[#3a3a45]"
                >
                  <input
                    type="radio"
                    checked={auditProvider === provider.id}
                    onChange={() => setAuditProvider(provider.id)}
                    className="w-4 h-4"
                  />
                  <div>
                    <div className="text-sm font-medium">{provider.name}</div>
                    <div className="text-xs text-gray-400">
                      {provider.desc} • ~{provider.time}
                    </div>
                  </div>
                </label>
              ))}
            </div>

            <button
              onClick={handleRunAudit}
              className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition mt-4 flex items-center justify-center gap-2"
            >
              <FileText size={14} /> Run Audit
            </button>
          </div>
        )}

        {/* Running State */}
        {auditStatus === "running" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-center space-y-3">
            <div className="inline-block w-6 h-6 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
            <div className="font-semibold">Running security analysis...</div>
            <div className="text-xs text-gray-400">This may take a few minutes</div>
          </div>
        )}

        {/* Audit Results */}
        {auditStatus === "complete" && (
          <>
            <div>
              <h3 className="font-semibold mb-3">Security Checks</h3>
              <div className="space-y-2">
                {MOCK_AUDIT.map((result) => (
                  <button
                    key={result.id}
                    onClick={() => setSelectedResult(result)}
                    className={clsx(
                      "w-full text-left p-3 rounded-lg border-2 transition",
                      selectedResult?.id === result.id
                        ? "border-blue-400 bg-blue-600/10"
                        : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                    )}
                  >
                    <div className="flex items-start justify-between">
                      <div>
                        <div className="text-sm font-semibold flex items-center gap-2">
                          {result.status === "passed" && (
                            <CheckCircle size={14} className="text-green-400" />
                          )}
                          {result.status === "warning" && (
                            <AlertCircle size={14} className="text-yellow-400" />
                          )}
                          {result.name}
                        </div>
                        <div className="text-xs text-gray-400 mt-1">{result.description}</div>
                      </div>
                      <span
                        className={clsx(
                          "text-xs font-semibold px-2 py-1 rounded border",
                          getSeverityColor(result.severity)
                        )}
                      >
                        {result.severity}
                      </span>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Audit Badge Display */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3">Audit Badge</h3>
              <div className="bg-[#2a2a35] p-4 rounded flex items-center justify-center">
                <div className="text-center">
                  <div className="text-4xl mb-2">✓</div>
                  <div className="text-sm font-bold">AUDIT PASSED</div>
                  <div className="text-xs text-gray-400 mt-1">{auditProvider.toUpperCase()} • Feb 28, 2026</div>
                </div>
              </div>

              <button className="w-full bg-[#2a2a35] hover:bg-[#3a3a45] py-2 rounded-lg text-sm font-semibold transition mt-3 flex items-center justify-center gap-2">
                <Download size={14} /> Download Badge
              </button>
            </div>

            {/* Report */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 flex items-center gap-2">
                <FileText size={16} /> Full Report
              </h3>
              <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg text-sm font-semibold transition">
                View Detailed Audit Report
              </button>
            </div>
          </>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Audit badges increase user trust and liquidity. Display on your token website.
      </div>
    </div>
  );
}
