import React, { useState } from "react";
import { Key, BarChart3, AlertTriangle, CheckCircle, Zap, TrendingUp } from "lucide-react";
import clsx from "clsx";

interface APIKey {
  id: string;
  name: string;
  key: string;
  status: "active" | "disabled" | "revoked";
  rateLimit: number;
  requestsToday: number;
  createdDate: string;
  lastUsed: string;
  permissions: string[];
}

interface QuotaUsage {
  date: string;
  requests: number;
  rateLimit: number;
  percentage: number;
}

interface RateLimitRule {
  id: string;
  name: string;
  requestsPerMinute: number;
  burstLimit: number;
  status: "active" | "paused";
}

const MOCK_KEYS: APIKey[] = [
  {
    id: "1",
    name: "Production Dashboard",
    key: "sk_prod_abc123...xyz",
    status: "active",
    rateLimit: 10000,
    requestsToday: 7234,
    createdDate: "2024-01-15",
    lastUsed: "2 mins ago",
    permissions: ["read", "write", "stream"],
  },
  {
    id: "2",
    name: "Mobile App",
    key: "sk_mobile_def456...uvw",
    status: "active",
    rateLimit: 5000,
    requestsToday: 2891,
    createdDate: "2024-03-20",
    lastUsed: "5 mins ago",
    permissions: ["read", "stream"],
  },
  {
    id: "3",
    name: "Legacy Integration",
    key: "sk_legacy_ghi789...tst",
    status: "disabled",
    rateLimit: 1000,
    requestsToday: 0,
    createdDate: "2023-06-10",
    lastUsed: "185 days ago",
    permissions: ["read"],
  },
];

const MOCK_QUOTA: QuotaUsage[] = [
  { date: "2024-10-01", requests: 3500, rateLimit: 10000, percentage: 35 },
  { date: "2024-10-02", requests: 4200, rateLimit: 10000, percentage: 42 },
  { date: "2024-10-03", requests: 5800, rateLimit: 10000, percentage: 58 },
  { date: "2024-10-04", requests: 4500, rateLimit: 10000, percentage: 45 },
  { date: "2024-10-05", requests: 7234, rateLimit: 10000, percentage: 72 },
];

const MOCK_RULES: RateLimitRule[] = [
  { id: "1", name: "Default Rate Limit", requestsPerMinute: 1000, burstLimit: 2000, status: "active" },
  { id: "2", name: "Premium Tier", requestsPerMinute: 5000, burstLimit: 10000, status: "active" },
  { id: "3", name: "Trial Tier", requestsPerMinute: 100, burstLimit: 200, status: "active" },
];

export default function APIGatewayPanel() {
  const [keys, setKeys] = useState<APIKey[]>(MOCK_KEYS);
  const [quota, setQuota] = useState<QuotaUsage[]>(MOCK_QUOTA);
  const [rules, setRules] = useState<RateLimitRule[]>(MOCK_RULES);
  const [selectedKey, setSelectedKey] = useState<APIKey | null>(MOCK_KEYS[0]);
  const [activeTab, setActiveTab] = useState<"keys" | "quota" | "rules">("keys");

  const activeKeys = keys.filter((k) => k.status === "active").length;
  const totalRequests = MOCK_QUOTA.reduce((sum, q) => sum + q.requests, 0);
  const todayUsage = MOCK_QUOTA[MOCK_QUOTA.length - 1];

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Key size={20} className="text-orange-400" /> API Gateway
      </h2>

      {/* Overview */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Active Keys</div>
          <div className="text-lg font-bold text-orange-400">{activeKeys}/{keys.length}</div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Today Usage</div>
          <div className="text-lg font-bold text-cyan-400">{todayUsage.percentage}%</div>
          <div className="text-xs text-gray-500 mt-1">{todayUsage.requests.toLocaleString()} requests</div>
        </div>
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
          <div className="text-xs text-gray-400 mb-1">Monthly Total</div>
          <div className="text-lg font-bold text-blue-400">{(totalRequests / 1000).toFixed(0)}K</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35]">
        {(["keys", "quota", "rules"] as const).map((tab) => (
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
            {tab === "keys" && "API Keys"}
            {tab === "quota" && "Quota"}
            {tab === "rules" && "Rate Limits"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {activeTab === "keys" && (
          <div className="space-y-3">
            <button className="w-full bg-orange-600 hover:bg-orange-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <Zap size={14} /> Generate New Key
            </button>

            <div className="space-y-2">
              {keys.map((key) => (
                <button
                  key={key.id}
                  onClick={() => setSelectedKey(key)}
                  className={clsx(
                    "w-full text-left p-3 rounded-lg border-2 transition",
                    selectedKey?.id === key.id
                      ? "border-cyan-600 bg-cyan-600/10"
                      : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                  )}
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="font-semibold text-sm">{key.name}</div>
                    <span
                      className={clsx(
                        "text-xs px-2 py-1 rounded-md font-bold",
                        key.status === "active"
                          ? "bg-green-600/20 text-green-400"
                          : key.status === "disabled"
                          ? "bg-yellow-600/20 text-yellow-400"
                          : "bg-red-600/20 text-red-400"
                      )}
                    >
                      {key.status.toUpperCase()}
                    </span>
                  </div>

                  <div className="text-xs text-gray-400 font-mono mb-2">{key.key}</div>

                  <div className="flex justify-between text-xs space-x-2">
                    <span>Rate: {key.requestsToday} / {key.rateLimit}</span>
                    <span>Last: {key.lastUsed}</span>
                  </div>

                  <div className="flex-1 bg-[#2a2a35] rounded-full h-1.5 overflow-hidden mt-2">
                    <div
                      className="h-full bg-gradient-to-r from-orange-600 to-red-600"
                      style={{ width: `${(key.requestsToday / key.rateLimit) * 100}%` }}
                    />
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {activeTab === "quota" && (
          <div className="space-y-3">
            <div className="space-y-2">
              {quota.map((item, idx) => (
                <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                  <div className="flex justify-between items-center">
                    <span className="text-sm font-semibold">{item.date}</span>
                    <span className="font-bold text-orange-400">{item.percentage}%</span>
                  </div>

                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                    <div
                      className={clsx(
                        "h-full",
                        item.percentage < 70
                          ? "bg-green-600"
                          : item.percentage < 90
                          ? "bg-yellow-600"
                          : "bg-red-600"
                      )}
                      style={{ width: `${item.percentage}%` }}
                    />
                  </div>

                  <div className="flex justify-between text-xs text-gray-400">
                    <span>{item.requests.toLocaleString()} requests</span>
                    <span>Limit: {item.rateLimit.toLocaleString()}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === "rules" && (
          <div className="space-y-2">
            {rules.map((rule) => (
              <div key={rule.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                <div className="flex itms-center justify-between">
                  <div className="font-semibold text-sm">{rule.name}</div>
                  <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", rule.status === "active" ? "bg-green-600/20 text-green-400" : "bg-gray-600/20 text-gray-400")}>
                    {rule.status.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-xs text-gray-400">
                  <div className="flex justify-between">
                    <span>Sustained:</span>
                    <span className="font-semibold">{rule.requestsPerMinute.toLocaleString()} req/min</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Burst Limit:</span>
                    <span className="font-semibold">{rule.burstLimit.toLocaleString()} req</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Key Details */}
        {selectedKey && activeTab === "keys" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 text-sm">
            <h3 className="font-semibold">Key Details: {selectedKey.name}</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Key ID</span>
                <span className="font-mono text-xs">{selectedKey.key}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className={clsx("font-bold", selectedKey.status === "active" ? "text-green-400" : selectedKey.status === "disabled" ? "text-yellow-400" : "text-red-400")}>
                  {selectedKey.status.toUpperCase()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Created</span>
                <span className="font-semibold">{selectedKey.createdDate}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Last Used</span>
                <span className="font-semibold">{selectedKey.lastUsed}</span>
              </div>
              <div>
                <span className="text-gray-400 block mb-2">Permissions</span>
                <div className="flex flex-wrap gap-2">
                  {selectedKey.permissions.map((perm) => (
                    <span key={perm} className="bg-cyan-600/20 text-cyan-300 text-xs px-2 py-1 rounded-md font-semibold">
                      {perm}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              {selectedKey.status === "active" && (
                <button className="flex-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 py-2 rounded-lg text-xs font-semibold transition">
                  Disable Key
                </button>
              )}
              <button className="flex-1 bg-orange-600 hover:bg-orange-700 text-white py-2 rounded-lg text-xs font-semibold transition">
                Regenerate
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Rate limiting, quota tracking, and secure API key management.
      </div>
    </div>
  );
}
