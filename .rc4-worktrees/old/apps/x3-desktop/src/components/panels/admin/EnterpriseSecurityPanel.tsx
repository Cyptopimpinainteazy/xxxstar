import React, { useState } from "react";
import { Lock, Key, Users, Shield, LogOut, Activity } from "lucide-react";
import clsx from "clsx";

interface AccessLog {
  id: string;
  user: string;
  action: string;
  resource: string;
  timestamp: string;
  status: "success" | "denied";
}

interface AccessControl {
  id: string;
  user: string;
  role: "admin" | "operator" | "viewer";
  permissions: string[];
  lastAccess: string;
  status: "active" | "inactive";
}

const MOCK_ACCESS_LOGS: AccessLog[] = [
  { id: "1", user: "0x1234...5678", action: "Login", resource: "Admin Panel", timestamp: "2 mins ago", status: "success" },
  { id: "2", user: "0x8765...4321", action: "Configure Validator", resource: "Validator A", timestamp: "15 mins ago", status: "success" },
  { id: "3", user: "0xabcd...efgh", action: "Attempt Bridge Access", resource: "Bridge", timestamp: "32 mins ago", status: "denied" },
];

const MOCK_USERS: AccessControl[] = [
  { id: "1", user: "0x1234...5678", role: "admin", permissions: ["read", "write", "delete", "configure"], lastAccess: "2 mins ago", status: "active" },
  { id: "2", user: "0x8765...4321", role: "operator", permissions: ["read", "write"], lastAccess: "15 mins ago", status: "active" },
  { id: "3", user: "0xabcd...efgh", role: "viewer", permissions: ["read"], lastAccess: "1 day ago", status: "inactive" },
];

export default function EnterpriseSecurityPanel() {
  const [accessLogs, setAccessLogs] = useState<AccessLog[]>(MOCK_ACCESS_LOGS);
  const [users, setUsers] = useState<AccessControl[]>(MOCK_USERS);
  const [selectedLog, setSelectedLog] = useState<AccessLog | null>(MOCK_ACCESS_LOGS[0]);
  const [selectedUser, setSelectedUser] = useState<AccessControl | null>(MOCK_USERS[0]);
  const [activeTab, setActiveTab] = useState<"logs" | "users" | "keys">("logs");

  const deniedCount = accessLogs.filter((log) => log.status === "denied").length;
  const activeUserCount = users.filter((u) => u.status === "active").length;

  const handleRevokeAccess = (userId: string) => {
    setUsers(users.map((u) => (u.id === userId ? { ...u, status: "inactive" } : u)));
  };

  const handleRestoreAccess = (userId: string) => {
    setUsers(users.map((u) => (u.id === userId ? { ...u, status: "active" } : u)));
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} className="text-red-400" /> Enterprise Security
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Users</div>
            <div className="text-xl font-bold text-green-400">{activeUserCount}/{users.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Access Attempts</div>
            <div className="text-xl font-bold text-blue-400">{accessLogs.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Denied</div>
            <div className={clsx("text-xl font-bold", deniedCount > 0 ? "text-red-400" : "text-green-400")}>
              {deniedCount}
            </div>
          </div>
        </div>

        {/* Tab Navigation */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["logs", "users", "keys"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 border-b-2 transition font-semibold text-sm",
                activeTab === tab
                  ? "border-red-600 text-red-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "logs" && "Access Logs"}
              {tab === "users" && "User Access"}
              {tab === "keys" && "Encryption Keys"}
            </button>
          ))}
        </div>

        {/* Access Logs Tab */}
        {activeTab === "logs" && (
          <div className="space-y-2">
            {accessLogs.map((log) => (
              <button
                key={log.id}
                onClick={() => setSelectedLog(log)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedLog?.id === log.id
                    ? "border-red-600 bg-red-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold flex items-center gap-2">
                      <Activity size={14} />
                      {log.action} • {log.resource}
                    </div>
                    <div className="text-xs font-mono text-gray-400">{log.user}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded border",
                      log.status === "success"
                        ? "bg-green-600/30 border-green-600 text-green-400"
                        : "bg-red-600/30 border-red-600 text-red-400"
                    )}
                  >
                    {log.status}
                  </span>
                </div>
                <div className="text-xs text-gray-400">{log.timestamp}</div>
              </button>
            ))}
          </div>
        )}

        {/* User Access Tab */}
        {activeTab === "users" && (
          <div className="space-y-2">
            {users.map((user) => (
              <button
                key={user.id}
                onClick={() => setSelectedUser(user)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedUser?.id === user.id
                    ? "border-red-600 bg-red-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="text-sm font-semibold flex items-center gap-2">
                      <Users size={14} /> {user.user}
                    </div>
                    <div className="text-xs text-gray-400">{user.role.toUpperCase()}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded border",
                      user.status === "active"
                        ? "bg-green-600/30 border-green-600 text-green-400"
                        : "bg-gray-600/30 border-gray-600 text-gray-400"
                    )}
                  >
                    {user.status}
                  </span>
                </div>
                <div className="text-xs text-gray-400">Last access: {user.lastAccess}</div>
              </button>
            ))}
          </div>
        )}

        {/* Encryption Keys Tab */}
        {activeTab === "keys" && (
          <div className="space-y-2">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm flex items-center gap-2">
                <Key size={16} /> Key Management
              </h3>

              <div className="space-y-2">
                <div className="bg-[#2a2a35] p-3 rounded">
                  <div className="text-xs text-gray-400 mb-1">Master Key Status</div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                    <span className="font-semibold text-green-400">Secure (Hardware-backed)</span>
                  </div>
                  <div className="text-xs text-gray-400 mt-2">Last rotated: 15 days ago</div>
                </div>

                <div className="bg-[#2a2a35] p-3 rounded">
                  <div className="text-xs text-gray-400 mb-1">Encryption Algorithm</div>
                  <div className="font-semibold">AES-256-GCM</div>
                </div>

                <div className="bg-[#2a2a35] p-3 rounded">
                  <div className="text-xs text-gray-400 mb-1">Key Storage</div>
                  <div className="font-semibold">HSM (Hardware Security Module)</div>
                </div>
              </div>

              <div className="flex gap-2">
                <button className="flex-1 bg-orange-600 hover:bg-orange-700 py-2 rounded-lg font-semibold text-sm transition">
                  Rotate Keys
                </button>
                <button className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition">
                  Export Key Backup
                </button>
              </div>

              <div className="bg-yellow-600/20 border border-yellow-600 rounded p-3 text-xs text-yellow-300">
                ⚠️ Key rotation recommended every 30 days. Last rotation was 15 days ago.
              </div>
            </div>
          </div>
        )}

        {/* Details Panel */}
        {activeTab === "logs" && selectedLog && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">Log Details</h3>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">User</span>
                <span className="font-mono text-xs">{selectedLog.user}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Action</span>
                <span className="font-semibold">{selectedLog.action}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Resource</span>
                <span className="font-semibold">{selectedLog.resource}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Timestamp</span>
                <span className="font-mono text-xs">{selectedLog.timestamp}</span>
              </div>
            </div>
          </div>
        )}

        {activeTab === "users" && selectedUser && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-sm space-y-3">
            <h3 className="font-semibold">User Details</h3>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Address</span>
                <span className="font-mono text-xs">{selectedUser.user}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Role</span>
                <span className="font-semibold capitalize">{selectedUser.role}</span>
              </div>
              <div>
                <span className="text-gray-400 block mb-2">Permissions</span>
                <div className="flex gap-1 flex-wrap">
                  {selectedUser.permissions.map((perm) => (
                    <span key={perm} className="bg-green-600/30 text-green-400 px-2 py-1 rounded text-xs">
                      {perm}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex gap-2">
              {selectedUser.status === "active" ? (
                <button
                  onClick={() => handleRevokeAccess(selectedUser.id)}
                  className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition"
                >
                  <LogOut size={14} className="inline mr-1" /> Revoke Access
                </button>
              ) : (
                <button
                  onClick={() => handleRestoreAccess(selectedUser.id)}
                  className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition"
                >
                  Restore Access
                </button>
              )}
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Enterprise-grade access control, audit logs, and key management.
      </div>
    </div>
  );
}
