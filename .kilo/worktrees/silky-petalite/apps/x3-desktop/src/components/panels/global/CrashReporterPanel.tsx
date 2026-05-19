import React, { useState } from "react";
import { AlertTriangle, Send, Check, Paperclip, MessageSquare } from "lucide-react";
import clsx from "clsx";

interface BugReport {
  id: string;
  title: string;
  description: string;
  severity: "critical" | "high" | "medium" | "low";
  status: "pending" | "received" | "resolved";
  timestamp: string;
}

export default function CrashReporterPanel() {
  const [page, setPage] = useState<"form" | "history">("form");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [reportData, setReportData] = useState({
    title: "",
    description: "",
    severity: "high" as const,
    stepsToReproduce: "",
    errorMessage: "",
    attachLogs: true,
  });

  const [reports, setReports] = useState<BugReport[]>([
    {
      id: "1",
      title: "Wallet balance not updating",
      description: "After swapping tokens, balance doesn't reflect immediately",
      severity: "high",
      status: "received",
      timestamp: "2 hours ago",
    },
    {
      id: "2",
      title: "Chart tooltip offset",
      description: "Price chart tooltip appears in wrong position on scroll",
      severity: "medium",
      status: "resolved",
      timestamp: "1 day ago",
    },
    {
      id: "3",
      title: "App crashes on mobile",
      description: "App crashes when opening wallet on iOS",
      severity: "critical",
      status: "received",
      timestamp: "3 days ago",
    },
  ]);

  const handleSubmitReport = () => {
    if (!reportData.title.trim() || !reportData.description.trim()) {
      alert("Please fill in title and description");
      return;
    }

    setIsSubmitting(true);
    setTimeout(() => {
      const newReport: BugReport = {
        id: String(Date.now()),
        title: reportData.title,
        description: reportData.description,
        severity: reportData.severity,
        status: "pending",
        timestamp: "Just now",
      };

      setReports([newReport, ...reports]);
      setReportData({
        title: "",
        description: "",
        severity: "high",
        stepsToReproduce: "",
        errorMessage: "",
        attachLogs: true,
      });
      setIsSubmitting(false);
      setPage("history");
    }, 1500);
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
      default:
        return "bg-gray-600/20 border-gray-600 text-gray-400";
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "pending":
        return "text-yellow-400";
      case "received":
        return "text-blue-400";
      case "resolved":
        return "text-green-400";
      default:
        return "text-gray-400";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold flex items-center gap-2">
          <AlertTriangle size={20} /> Crash Reporter
        </h2>
        <div className="flex gap-2">
          {(["form", "history"] as const).map((p) => (
            <button
              key={p}
              onClick={() => setPage(p)}
              className={clsx(
                "px-3 py-1 rounded text-sm font-medium transition",
                page === p
                  ? "bg-blue-600 text-white"
                  : "bg-[#15151b] text-gray-400 hover:bg-[#1a1a20]"
              )}
            >
              {p === "form" ? "Report Bug" : "History"}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto mb-4">
        {page === "form" ? (
          // Report Form
          <div className="space-y-4">
            <div className="bg-blue-600/10 border border-blue-600 rounded-lg p-3 flex items-start gap-2">
              <MessageSquare size={16} className="text-blue-400 flex-shrink-0 mt-0.5" />
              <div>
                <div className="text-sm font-semibold text-blue-400">Help Us Improve</div>
                <div className="text-xs text-blue-300">
                  Found a bug? Let us know. Your report helps us make X3 better.
                </div>
              </div>
            </div>

            {/* Title */}
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Bug Title</label>
              <input
                type="text"
                value={reportData.title}
                onChange={(e) => setReportData({ ...reportData, title: e.target.value })}
                placeholder="e.g., Wallet balance not updating"
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm placeholder-gray-500"
              />
            </div>

            {/* Severity */}
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Severity</label>
              <select
                value={reportData.severity}
                onChange={(e) => setReportData({ ...reportData, severity: e.target.value as any })}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
              >
                <option value="low">Low - Minor issue</option>
                <option value="medium">Medium - Feature broken</option>
                <option value="high">High - Major functionality broken</option>
                <option value="critical">Critical - App crashes/unusable</option>
              </select>
            </div>

            {/* Description */}
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Description</label>
              <textarea
                value={reportData.description}
                onChange={(e) => setReportData({ ...reportData, description: e.target.value })}
                placeholder="Describe what happened in detail"
                rows={4}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm placeholder-gray-500 resize-none"
              />
            </div>

            {/* Steps to Reproduce */}
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Steps to Reproduce (Optional)</label>
              <textarea
                value={reportData.stepsToReproduce}
                onChange={(e) => setReportData({ ...reportData, stepsToReproduce: e.target.value })}
                placeholder="1. Click X&#10;2. Do Y&#10;3. Bug appears"
                rows={3}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm placeholder-gray-500 resize-none"
              />
            </div>

            {/* Error Message */}
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Error Message (Optional)</label>
              <textarea
                value={reportData.errorMessage}
                onChange={(e) => setReportData({ ...reportData, errorMessage: e.target.value })}
                placeholder="Paste any error messages you see"
                rows={2}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm placeholder-gray-500 resize-none"
              />
            </div>

            {/* Attach Logs */}
            <div className="flex items-center gap-3 bg-[#15151b] border border-[#2a2a35] rounded p-3">
              <input
                type="checkbox"
                checked={reportData.attachLogs}
                onChange={(e) => setReportData({ ...reportData, attachLogs: e.target.checked })}
                className="w-4 h-4 rounded"
              />
              <label className="text-sm cursor-pointer flex-1">
                <div className="font-medium">Include Debug Logs</div>
                <div className="text-xs text-gray-400">Helps our team diagnose the issue faster</div>
              </label>
              <Paperclip size={16} className="text-gray-400" />
            </div>

            {/* Submit Button */}
            <button
              onClick={handleSubmitReport}
              disabled={isSubmitting || !reportData.title.trim()}
              className="w-full bg-blue-600 hover:bg-blue-700 disabled:opacity-50 py-3 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
            >
              {isSubmitting ? (
                <>
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  Submitting...
                </>
              ) : (
                <>
                  <Send size={16} /> Submit Report
                </>
              )}
            </button>
          </div>
        ) : (
          // Report History
          <div className="space-y-3">
            {reports.length === 0 ? (
              <div className="text-center py-8 text-gray-400">
                <AlertTriangle size={32} className="mx-auto mb-3 opacity-50" />
                <p>No bug reports yet</p>
              </div>
            ) : (
              reports.map((report) => (
                <button
                  key={report.id}
                  className="w-full text-left p-4 rounded-lg border border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45] transition"
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h3 className="font-semibold">{report.title}</h3>
                      <p className="text-xs text-gray-400 mt-1">{report.description}</p>
                    </div>
                    <div className="text-right">
                      <div className={clsx(
                        "inline-block px-2 py-1 rounded text-xs font-semibold border mb-1",
                        getSeverityColor(report.severity)
                      )}>
                        {report.severity.charAt(0).toUpperCase() + report.severity.slice(1)}
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center justify-between text-xs">
                    <span className="text-gray-500">{report.timestamp}</span>
                    {report.status === "resolved" ? (
                      <span className={clsx("flex items-center gap-1 font-semibold", getStatusColor(report.status))}>
                        <Check size={12} /> Resolved
                      </span>
                    ) : (
                      <span className={clsx("font-semibold capitalize", getStatusColor(report.status))}>
                        {report.status}
                      </span>
                    )}
                  </div>
                </button>
              ))
            )}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="text-xs text-gray-500 text-center mt-4 pt-4 border-t border-[#2a2a35]">
        Thank you for helping us improve X3! Our team reviews every report.
      </div>
    </div>
  );
}
