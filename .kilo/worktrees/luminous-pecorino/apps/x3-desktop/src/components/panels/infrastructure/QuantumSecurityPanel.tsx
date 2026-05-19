import React, { useState } from "react";
import { Shield, TrendingUp, Zap, Eye, Download, AlertTriangle } from "lucide-react";
import clsx from "clsx";

interface QuantumAlgorithm {
  id: string;
  name: string;
  status: "ready" | "in-migration" | "legacy";
  classicalKeySize: number;
  quantumKeySize: number;
  migrationProgress: number;
  estimatedComplete: string;
  resilience: "high" | "medium" | "low";
}

interface SecurityAudit {
  id: string;
  algorithm: string;
  timestamp: string;
  status: "passed" | "warning" | "failed";
  vulnerabilities: number;
  lastUpdated: string;
}

interface MigrationTimeline {
  phase: string;
  startDate: string;
  endDate: string;
  completion: number;
  description: string;
}

const MOCK_ALGORITHMS: QuantumAlgorithm[] = [
  {
    id: "1",
    name: "ML-KEM (Kyber)",
    status: "in-migration",
    classicalKeySize: 256,
    quantumKeySize: 1024,
    migrationProgress: 65,
    estimatedComplete: "2024-06-15",
    resilience: "high",
  },
  {
    id: "2",
    name: "ML-DSA (Dilithium)",
    status: "ready",
    classicalKeySize: 256,
    quantumKeySize: 2336,
    migrationProgress: 100,
    estimatedComplete: "2024-03-01",
    resilience: "high",
  },
  {
    id: "3",
    name: "SLH-DSA (SPHINCS+)",
    status: "legacy",
    classicalKeySize: 256,
    quantumKeySize: 32,
    migrationProgress: 0,
    estimatedComplete: "2024-08-30",
    resilience: "medium",
  },
];

const MOCK_AUDITS: SecurityAudit[] = [
  { id: "1", algorithm: "ML-KEM Lattice", timestamp: "2024-04-10", status: "passed", vulnerabilities: 0, lastUpdated: "2024-04-10" },
  { id: "2", algorithm: "ML-DSA Signature", timestamp: "2024-04-08", status: "passed", vulnerabilities: 0, lastUpdated: "2024-04-08" },
  { id: "3", algorithm: "Hash-based Backup", timestamp: "2024-04-05", status: "warning", vulnerabilities: 1, lastUpdated: "2024-04-05" },
];

const MOCK_TIMELINE: MigrationTimeline[] = [
  { phase: "Phase 1: Assessment", startDate: "2024-01-01", endDate: "2024-03-31", completion: 100, description: "Quantum threat analysis and algorithm evaluation" },
  { phase: "Phase 2: Integration", startDate: "2024-04-01", endDate: "2024-06-30", completion: 65, description: "Hybrid classical-quantum key infrastructure" },
  { phase: "Phase 3: Migration", startDate: "2024-07-01", endDate: "2024-09-30", completion: 0, description: "User migration to post-quantum cryptography" },
  { phase: "Phase 4: Deprecation", startDate: "2024-10-01", endDate: "2024-12-31", completion: 0, description: "Legacy algorithm phase-out" },
];

export default function QuantumSecurityPanel() {
  const [algorithms] = useState<QuantumAlgorithm[]>(MOCK_ALGORITHMS);
  const [audits] = useState<SecurityAudit[]>(MOCK_AUDITS);
  const [timeline] = useState<MigrationTimeline[]>(MOCK_TIMELINE);
  const [activeTab, setActiveTab] = useState<"algorithms" | "audits" | "timeline">("algorithms");

  const totalProgress = algorithms.reduce((sum, a) => sum + a.migrationProgress, 0) / algorithms.length;
  const readyAlgorithms = algorithms.filter((a) => a.status === "ready").length;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} className="text-indigo-400" /> Quantum Security
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Overall Progress</div>
            <div className="text-lg font-bold text-indigo-400">{totalProgress.toFixed(0)}%</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Ready Algorithms</div>
            <div className="text-lg font-bold text-green-400">
              {readyAlgorithms}/{algorithms.length}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Audits Passed</div>
            <div className="text-lg font-bold text-cyan-400">{audits.filter((a) => a.status === "passed").length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Security Status</div>
            <div className="text-lg font-bold text-yellow-400">Hybrid Mode</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["algorithms", "audits", "timeline"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-indigo-600 text-indigo-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Algorithms Tab */}
        {activeTab === "algorithms" && (
          <div className="space-y-3">
            {algorithms.map((algo) => (
              <div key={algo.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <div className="font-semibold">{algo.name}</div>
                      <span
                        className={clsx(
                          "text-xs px-2 py-1 rounded font-bold",
                          algo.status === "ready" && "bg-green-600/20 text-green-400",
                          algo.status === "in-migration" && "bg-yellow-600/20 text-yellow-400",
                          algo.status === "legacy" && "bg-red-600/20 text-red-400"
                        )}
                      >
                        {algo.status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-400 mt-1">Resilience: <span className={algo.resilience === "high" ? "text-green-400" : algo.resilience === "medium" ? "text-yellow-400" : "text-red-400"}>{algo.resilience}</span></div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Classical Key Size</div>
                    <div className="font-bold text-cyan-400">{algo.classicalKeySize} bits</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Quantum Key Size</div>
                    <div className="font-bold text-purple-400">{algo.quantumKeySize} bits</div>
                  </div>
                </div>

                {algo.status === "in-migration" && (
                  <>
                    <div className="bg-[#0a0a0f] rounded p-2 mb-2">
                      <div className="text-xs text-gray-400 mb-1">Migration Progress</div>
                      <div className="bg-[#2a2a35] rounded-full h-2">
                        <div className="h-full bg-gradient-to-r from-yellow-600 to-orange-600 rounded-full" style={{ width: `${algo.migrationProgress}%` }} />
                      </div>
                      <div className="text-xs font-bold text-yellow-400 mt-1">{algo.migrationProgress}% complete — Est. {algo.estimatedComplete}</div>
                    </div>
                  </>
                )}

                {algo.status === "ready" && (
                  <div className="bg-[#0a0a0f] rounded p-2 mb-2">
                    <div className="text-xs text-green-400 font-semibold">✓ Post-Quantum Ready</div>
                  </div>
                )}

                {algo.status === "legacy" && (
                  <div className="bg-[#0a0a0f] rounded p-2 mb-2 flex items-center gap-2">
                    <AlertTriangle size={14} className="text-red-400" />
                    <span className="text-xs text-red-400 font-semibold">Legacy — Plan migration by {algo.estimatedComplete}</span>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Audits Tab */}
        {activeTab === "audits" && (
          <div className="space-y-2">
            {audits.map((audit) => (
              <div key={audit.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <div className="font-semibold text-sm">{audit.algorithm}</div>
                      <span
                        className={clsx(
                          "text-xs px-2 py-1 rounded font-bold",
                          audit.status === "passed" && "bg-green-600/20 text-green-400",
                          audit.status === "warning" && "bg-yellow-600/20 text-yellow-400",
                          audit.status === "failed" && "bg-red-600/20 text-red-400"
                        )}
                      >
                        {audit.status}
                      </span>
                    </div>
                    <div className="text-xs text-gray-400">{audit.timestamp}</div>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div>
                    <div className="text-gray-400">Vulnerabilities</div>
                    <div className={clsx("font-bold", audit.vulnerabilities === 0 ? "text-green-400" : "text-red-400")}>{audit.vulnerabilities}</div>
                  </div>
                  <div className="col-span-2">
                    <div className="text-gray-400">Last Updated</div>
                    <div className="font-semibold text-cyan-400">{audit.lastUpdated}</div>
                  </div>
                </div>

                {audit.status === "passed" && <div className="mt-2 bg-[#0a0a0f] rounded p-2 text-xs text-green-400 font-semibold">✓ All security checks passed</div>}
                {audit.status === "warning" && <div className="mt-2 bg-[#0a0a0f] rounded p-2 text-xs text-yellow-400 font-semibold">⚠ Minor issues detected — review recommended</div>}
              </div>
            ))}
          </div>
        )}

        {/* Timeline Tab */}
        {activeTab === "timeline" && (
          <div className="space-y-3">
            {timeline.map((phase) => (
              <div key={phase.phase} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="font-semibold text-sm">{phase.phase}</div>
                    <div className="text-xs text-gray-400 mt-1">{phase.startDate} → {phase.endDate}</div>
                  </div>
                  <div className="text-right text-xs">
                    <div className="font-bold text-indigo-400">{phase.completion}%</div>
                  </div>
                </div>

                <div className="text-xs text-gray-400 mb-2">{phase.description}</div>

                <div className="bg-[#0a0a0f] rounded p-2">
                  <div className="bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-gradient-to-r from-indigo-600 to-purple-600 rounded-full" style={{ width: `${phase.completion}%` }} />
                  </div>
                </div>
              </div>
            ))}

            {/* Security Roadmap */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 mt-4">
              <div className="text-xs text-gray-400 mb-2 font-semibold">Roadmap Status</div>
              <div className="space-y-2 text-xs">
                <div className="flex gap-2">
                  <span className="bg-green-600 w-2 h-2 rounded-full mt-1" />
                  <span className="text-gray-400">Cryptography research completed</span>
                </div>
                <div className="flex gap-2">
                  <span className="bg-yellow-600 w-2 h-2 rounded-full mt-1" />
                  <span className="text-gray-400">Hybrid key infrastructure in development</span>
                </div>
                <div className="flex gap-2">
                  <span className="bg-gray-600 w-2 h-2 rounded-full mt-1" />
                  <span className="text-gray-400">User migration tools pending</span>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Post-quantum cryptography, lattice algorithms, security audits, and migration planning.
      </div>
    </div>
  );
}
