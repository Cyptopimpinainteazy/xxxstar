import React, { useState } from "react";
import { HardDrive, RotateCcw, AlertTriangle, CheckCircle, Clock, Zap, Download, FileText } from "lucide-react";
import clsx from "clsx";

interface BackupSnapshot {
  id: string;
  name: string;
  timestamp: string;
  size: number;
  status: "healthy" | "degraded" | "corrupt";
  type: "full" | "incremental";
  retentionDays: number;
}

interface RestoreTest {
  id: string;
  snapshotId: string;
  timestamp: string;
  status: "passed" | "failed" | "in-progress";
  duration: number;
  dataIntegrity: number;
}

interface DisasterScenario {
  id: string;
  name: string;
  description: string;
  rto: number;  // Recovery Time Objective in minutes
  rpo: number;  // Recovery Point Objective in minutes
  lastTest: string;
  status: "ready" | "testing" | "pending";
}

const MOCK_SNAPSHOTS: BackupSnapshot[] = [
  {
    id: "1",
    name: "Daily Backup 2024-10-05",
    timestamp: "2024-10-05T02:00:00Z",
    size: 45.2,
    status: "healthy",
    type: "full",
    retentionDays: 30,
  },
  {
    id: "2",
    name: "Hourly Backup 2024-10-05T14:00",
    timestamp: "2024-10-05T14:00:00Z",
    size: 12.8,
    status: "healthy",
    type: "incremental",
    retentionDays: 7,
  },
  {
    id: "3",
    name: "Weekly Full Backup 2024-10-01",
    timestamp: "2024-10-01T00:00:00Z",
    size: 47.5,
    status: "healthy",
    type: "full",
    retentionDays: 90,
  },
];

const MOCK_RESTORE_TESTS: RestoreTest[] = [
  {
    id: "1",
    snapshotId: "1",
    timestamp: "2024-10-03T08:30:00Z",
    status: "passed",
    duration: 45,
    dataIntegrity: 100,
  },
  {
    id: "2",
    snapshotId: "2",
    timestamp: "2024-10-02T15:45:00Z",
    status: "passed",
    duration: 28,
    dataIntegrity: 100,
  },
  {
    id: "3",
    snapshotId: "3",
    timestamp: "2024-09-28T12:00:00Z",
    status: "passed",
    duration: 67,
    dataIntegrity: 100,
  },
];

const MOCK_SCENARIOS: DisasterScenario[] = [
  {
    id: "1",
    name: "Data Center Failure",
    description: "Complete data center outage - switch to backup region",
    rto: 30,
    rpo: 5,
    lastTest: "2024-09-25",
    status: "ready",
  },
  {
    id: "2",
    name: "Database Corruption",
    description: "Restore from clean backup snapshot",
    rto: 15,
    rpo: 1,
    lastTest: "2024-09-20",
    status: "ready",
  },
  {
    id: "3",
    name: "Ransomware Attack",
    description: "Full system rollback with integrity verification",
    rto: 60,
    rpo: 10,
    lastTest: "2024-10-01",
    status: "testing",
  },
];

export default function DisasterRecoveryPanel() {
  const [snapshots, setSnapshots] = useState<BackupSnapshot[]>(MOCK_SNAPSHOTS);
  const [tests, setTests] = useState<RestoreTest[]>(MOCK_RESTORE_TESTS);
  const [scenarios, setScenarios] = useState<DisasterScenario[]>(MOCK_SCENARIOS);
  const [selectedSnapshot, setSelectedSnapshot] = useState<BackupSnapshot | null>(MOCK_SNAPSHOTS[0]);
  const [selectedScenario, setSelectedScenario] = useState<DisasterScenario | null>(MOCK_SCENARIOS[0]);
  const [activeTab, setActiveTab] = useState<"backups" | "tests" | "scenarios">("backups");

  const healthySnapshots = snapshots.filter((s) => s.status === "healthy").length;
  const lastBackup = snapshots[0];
  const passedTests = tests.filter((t) => t.status === "passed").length;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <HardDrive size={20} className="text-red-400" /> Disaster Recovery
      </h2>

      {/* Overview */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Healthy Backups</div>
          <div className="text-lg font-bold text-green-400">{healthySnapshots}/{snapshots.length}</div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Last Backup</div>
          <div className="text-lg font-bold text-cyan-400">
            {lastBackup.timestamp.split("T")[0]}
          </div>
          <div className="text-xs text-gray-500 mt-1">{lastBackup.size}GB</div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Test Success Rate</div>
          <div className="text-lg font-bold text-blue-400">{((passedTests / tests.length) * 100).toFixed(0)}%</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["backups", "tests", "scenarios"] as const).map((tab) => (
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
            {tab === "backups" && "Snapshots"}
            {tab === "tests" && "Tests"}
            {tab === "scenarios" && "Scenarios"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {activeTab === "backups" && (
          <div className="space-y-3">
            <button className="w-full bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <Zap size={14} /> Create New Backup
            </button>

            <div className="space-y-2">
              {snapshots.map((snapshot) => (
                <button
                  key={snapshot.id}
                  onClick={() => setSelectedSnapshot(snapshot)}
                  className={clsx(
                    "w-full text-left p-3 rounded-lg border-2 transition",
                    selectedSnapshot?.id === snapshot.id
                      ? "border-cyan-600 bg-cyan-600/10"
                      : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                  )}
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <div className="font-semibold text-sm">{snapshot.name}</div>
                      <div className="text-xs text-gray-400">{snapshot.timestamp}</div>
                    </div>
                    <div className="text-right">
                      <div className={clsx("text-xs px-2 py-1 rounded-md font-bold", snapshot.status === "healthy" ? "bg-green-600/20 text-green-400" : snapshot.status === "degraded" ? "bg-yellow-600/20 text-yellow-400" : "bg-red-600/20 text-red-400")}>
                        {snapshot.status.toUpperCase()}
                      </div>
                      <div className="text-sm font-bold text-cyan-400 mt-1">{snapshot.size}GB</div>
                    </div>
                  </div>

                  <div className="flex justify-between text-xs text-gray-400">
                    <span className="capitalize">{snapshot.type} backup</span>
                    <span>Retain {snapshot.retentionDays} days</span>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {activeTab === "tests" && (
          <div className="space-y-2">
            <button className="w-full bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <RotateCcw size={14} /> Run Restore Test
            </button>

            <div className="space-y-2 mt-3">
              {tests.map((test) => (
                <div key={test.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-semibold text-sm">Test #{test.id}</div>
                      <div className="text-xs text-gray-400">{test.timestamp}</div>
                    </div>
                    <div className="text-right">
                      {test.status === "passed" ? (
                        <CheckCircle size={16} className="text-green-400" />
                      ) : test.status === "failed" ? (
                        <AlertTriangle size={16} className="text-red-400" />
                      ) : (
                        <Clock size={16} className="text-yellow-400 animate-spin" />
                      )}
                    </div>
                  </div>

                  <div className="space-y-1 text-xs text-gray-400">
                    <div className="flex justify-between">
                      <span>Duration</span>
                      <span className="font-semibold">{test.duration} minutes</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Data Integrity</span>
                      <span className={clsx("font-bold", test.dataIntegrity === 100 ? "text-green-400" : "text-yellow-400")}>
                        {test.dataIntegrity}%
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === "scenarios" && (
          <div className="space-y-2">
            {scenarios.map((scenario) => (
              <button
                key={scenario.id}
                onClick={() => setSelectedScenario(scenario)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedScenario?.id === scenario.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="font-semibold text-sm">{scenario.name}</div>
                    <div className="text-xs text-gray-400">{scenario.description}</div>
                  </div>
                  <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", scenario.status === "ready" ? "bg-green-600/20 text-green-400" : "bg-yellow-600/20 text-yellow-400")}>
                    {scenario.status.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-xs text-gray-400">
                  <div className="flex justify-between">
                    <span>RTO</span>
                    <span className="font-semibold">{scenario.rto} min</span>
                  </div>
                  <div className="flex justify-between">
                    <span>RPO</span>
                    <span className="font-semibold">{scenario.rpo} min</span>
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}

        {/* Snapshot Details */}
        {selectedSnapshot && activeTab === "backups" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 text-sm">
            <h3 className="font-semibold">Backup Details</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Type</span>
                <span className="font-semibold capitalize">{selectedSnapshot.type}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Size</span>
                <span className="font-bold text-cyan-400">{selectedSnapshot.size}GB</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className="font-bold text-green-400">{selectedSnapshot.status.toUpperCase()}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Retention</span>
                <span className="font-semibold">{selectedSnapshot.retentionDays} days</span>
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              <button className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
                <RotateCcw size={14} /> Restore
              </button>
              <button className="flex-1 bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
                <Download size={14} /> Export
              </button>
            </div>
          </div>
        )}

        {/* Scenario Details */}
        {selectedScenario && activeTab === "scenarios" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 text-sm">
            <h3 className="font-semibold">{selectedScenario.name} Playbook</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">RTO (Recovery Time)</span>
                <span className="font-bold text-cyan-400">{selectedScenario.rto} minutes</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">RPO (Recovery Point)</span>
                <span className="font-bold text-cyan-400">{selectedScenario.rpo} minutes</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className={clsx("font-bold", selectedScenario.status === "ready" ? "text-green-400" : "text-yellow-400")}>
                  {selectedScenario.status.toUpperCase()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Last Tested</span>
                <span className="font-semibold">{selectedScenario.lastTest}</span>
              </div>
            </div>

            <button className="w-full bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <RotateCcw size={14} /> Execute Drill
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Automated backup snapshots with restoration verification and RTO/RPO testing.
      </div>
    </div>
  );
}
