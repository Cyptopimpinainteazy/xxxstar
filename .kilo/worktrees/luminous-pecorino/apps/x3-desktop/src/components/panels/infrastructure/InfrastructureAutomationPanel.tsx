import React, { useState } from "react";
import { Settings, CheckCircle, Zap, AlertTriangle, GitBranch, Cpu } from "lucide-react";
import clsx from "clsx";

interface ValidatorNode {
  id: string;
  name: string;
  status: "running" | "idle" | "deploying";
  geoLocation: string;
  cpu: number;
  memory: number;
  uptime: number;
}

interface AutomationTask {
  id: string;
  name: string;
  description: string;
  frequency: string;
  lastRun: string;
  nextRun: string;
  status: "active" | "paused" | "failed";
}

const MOCK_VALIDATORS: ValidatorNode[] = [
  { id: "1", name: "US-East-1", status: "running", geoLocation: "us-east-1", cpu: 42, memory: 58, uptime: 99.8 },
  { id: "2", name: "EU-West-1", status: "running", geoLocation: "eu-west-1", cpu: 35, memory: 45, uptime: 99.9 },
  { id: "3", name: "Asia-SG", status: "idle", geoLocation: "ap-southeast-1", cpu: 12, memory: 28, uptime: 98.5 },
];

const MOCK_TASKS: AutomationTask[] = [
  {
    id: "1",
    name: "Auto-Update Validator Software",
    description: "Check for updates every 6 hours, deploy if available",
    frequency: "Every 6 hours",
    lastRun: "2 hours ago",
    nextRun: "In 4 hours",
    status: "active",
  },
  {
    id: "2",
    name: "Database Backup",
    description: "Backup validator state to cloud storage",
    frequency: "Daily at 2 AM UTC",
    lastRun: "Yesterday at 2:00 AM",
    nextRun: "Tomorrow at 2:00 AM",
    status: "active",
  },
  {
    id: "3",
    name: "Performance Report Generation",
    description: "Generate and email performance metrics",
    frequency: "Weekly on Sunday",
    lastRun: "3 days ago",
    nextRun: "In 4 days",
    status: "active",
  },
];

export default function InfrastructureAutomationPanel() {
  const [validators, setValidators] = useState<ValidatorNode[]>(MOCK_VALIDATORS);
  const [tasks, setTasks] = useState<AutomationTask[]>(MOCK_TASKS);
  const [selectedValidator, setSelectedValidator] = useState<ValidatorNode | null>(MOCK_VALIDATORS[0]);
  const [selectedTask, setSelectedTask] = useState<AutomationTask | null>(MOCK_TASKS[0]);
  const [deploymentInProgress, setDeploymentInProgress] = useState(false);

  const runningCount = validators.filter((v) => v.status === "running").length;
  const avgUptime =
    validators.reduce((sum, v) => sum + v.uptime, 0) / validators.length;
  const activeTasksCount = tasks.filter((t) => t.status === "active").length;

  const handleDeployNode = () => {
    setDeploymentInProgress(true);
    setTimeout(() => {
      setValidators(
        validators.map((v) =>
          v.id === selectedValidator?.id ? { ...v, status: "running" } : v
        )
      );
      setDeploymentInProgress(false);
    }, 2000);
  };

  const handleToggleTask = (taskId: string) => {
    setTasks(
      tasks.map((t) => {
        if (t.id === taskId) {
          return { ...t, status: t.status === "active" ? "paused" : "active" };
        }
        return t;
      })
    );
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Settings size={20} className="text-orange-400" /> Infrastructure Automation
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Nodes Running</div>
            <div className="text-xl font-bold text-green-400">{runningCount}/{validators.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg Uptime</div>
            <div className="text-xl font-bold text-blue-400">{avgUptime.toFixed(1)}%</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Automation Tasks</div>
            <div className="text-xl font-bold text-purple-400">{activeTasksCount}/{tasks.length}</div>
          </div>
        </div>

        {/* Validator Nodes */}
        <div>
          <h3 className="font-semibold mb-2 text-sm flex items-center gap-2">
            <Cpu size={16} /> Validator Nodes
          </h3>
          <div className="space-y-2">
            {validators.map((validator) => (
              <button
                key={validator.id}
                onClick={() => setSelectedValidator(validator)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedValidator?.id === validator.id
                    ? "border-orange-600 bg-orange-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold">{validator.name}</div>
                    <div className="text-xs text-gray-400">{validator.geoLocation}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded border",
                      validator.status === "running"
                        ? "bg-green-600/30 border-green-600 text-green-400"
                        : validator.status === "deploying"
                        ? "bg-yellow-600/30 border-yellow-600 text-yellow-400"
                        : "bg-gray-600/30 border-gray-600 text-gray-400"
                    )}
                  >
                    {validator.status}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-2 text-xs mb-2">
                  <div className="flex justify-between">
                    <span className="text-gray-400">CPU</span>
                    <span className="font-semibold">{validator.cpu}%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Mem</span>
                    <span className="font-semibold">{validator.memory}%</span>
                  </div>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-orange-600 to-red-600"
                    style={{ width: `${validator.cpu}%` }}
                  />
                </div>

                <div className="text-xs text-gray-400 mt-2">Uptime: {validator.uptime}%</div>
              </button>
            ))}
          </div>
        </div>

        {/* Selected Validator Details */}
        {selectedValidator && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <h3 className="font-semibold mb-3 text-sm">{selectedValidator.name} Configuration</h3>

            <div className="space-y-3 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">CPU Usage</span>
                <span className="font-semibold">{selectedValidator.cpu}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Memory Usage</span>
                <span className="font-semibold">{selectedValidator.memory}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Uptime</span>
                <span className="font-semibold text-green-400">{selectedValidator.uptime}%</span>
              </div>
            </div>

            {selectedValidator.status === "idle" && (
              <button
                onClick={handleDeployNode}
                disabled={deploymentInProgress}
                className="w-full bg-green-600 hover:bg-green-700 disabled:bg-gray-600 py-2 rounded-lg font-semibold text-sm transition mt-4"
              >
                {deploymentInProgress ? "Deploying..." : "Deploy Node"}
              </button>
            )}

            {selectedValidator.status === "running" && (
              <div className="flex gap-2 mt-4">
                <button className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
                  Update Software
                </button>
                <button className="flex-1 bg-orange-600 hover:bg-orange-700 py-2 rounded-lg font-semibold text-sm transition">
                  Restart
                </button>
              </div>
            )}
          </div>
        )}

        {/* Automation Tasks */}
        <div>
          <h3 className="font-semibold mb-2 text-sm flex items-center gap-2">
            <Zap size={16} /> Automation Tasks
          </h3>
          <div className="space-y-2">
            {tasks.map((task) => (
              <button
                key={task.id}
                onClick={() => setSelectedTask(task)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedTask?.id === task.id
                    ? "border-purple-600 bg-purple-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex-1">
                    <div className="text-sm font-semibold">{task.name}</div>
                    <div className="text-xs text-gray-400">{task.description}</div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleToggleTask(task.id);
                    }}
                    className={clsx(
                      "px-2 py-1 rounded text-xs font-semibold transition",
                      task.status === "active"
                        ? "bg-green-600 text-white"
                        : "bg-gray-600 text-gray-200"
                    )}
                  >
                    {task.status}
                  </button>
                </div>

                <div className="text-xs text-gray-400 flex justify-between">
                  <span>{task.frequency}</span>
                  <span>Next: {task.nextRun}</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Task Details */}
        {selectedTask && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">{selectedTask.name}</h3>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Frequency</span>
                <span className="font-semibold">{selectedTask.frequency}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Last Run</span>
                <span className="font-semibold">{selectedTask.lastRun}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Next Run</span>
                <span className="font-semibold text-green-400">{selectedTask.nextRun}</span>
              </div>
            </div>

            <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
              Run Now
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Automated validator deployment, updates, and monitoring. Zero manual ops.
      </div>
    </div>
  );
}
