import React, { useState } from "react";
import { Eye, EyeOff, Lock, Zap, ArrowRight, CheckCircle, AlertTriangle, Copy } from "lucide-react";
import clsx from "clsx";

interface StealthAddress {
  id: string;
  address: string;
  createdAt: string;
  balance: number;
  transactionCount: number;
  status: "active" | "expired" | "compromised";
}

interface PrivacyTransaction {
  id: string;
  from: string;
  to: string;
  amount: number;
  timestamp: string;
  mixerStatus: "pending" | "mixed" | "completed";
  hops: number;
}

interface PrivacyAudit {
  timestamp: string;
  transactionsAnalyzed: number;
  linkabilityRisk: "low" | "medium" | "high";
  addressClusterSize: number;
  recommendedMixing: boolean;
}

const MOCK_ADDRESSES: StealthAddress[] = [
  {
    id: "1",
    address: "x3s7b...2f4a",
    createdAt: "2024-10-01",
    balance: 45.32,
    transactionCount: 8,
    status: "active",
  },
  {
    id: "2",
    address: "x3s9c...8e5b",
    createdAt: "2024-09-15",
    balance: 12.51,
    transactionCount: 3,
    status: "active",
  },
  {
    id: "3",
    address: "x3s2a...1k9f",
    createdAt: "2024-08-20",
    balance: 0,
    transactionCount: 15,
    status: "expired",
  },
];

const MOCK_TRANSACTIONS: PrivacyTransaction[] = [
  {
    id: "1",
    from: "x3c7b...2f4a",
    to: "x3s7b...2f4a",
    amount: 100,
    timestamp: "2 hours ago",
    mixerStatus: "completed",
    hops: 5,
  },
  {
    id: "2",
    from: "x3s7b...2f4a",
    to: "x3c9d...5e2c",
    amount: 50,
    timestamp: "1 hour ago",
    mixerStatus: "mixed",
    hops: 3,
  },
  {
    id: "3",
    from: "x3c5e...8f9g",
    to: "x3s9c...8e5b",
    amount: 25,
    timestamp: "30 mins ago",
    mixerStatus: "pending",
    hops: 0,
  },
];

const MOCK_AUDIT: PrivacyAudit = {
  timestamp: new Date().toISOString(),
  transactionsAnalyzed: 127,
  linkabilityRisk: "low",
  addressClusterSize: 43,
  recommendedMixing: false,
};

export default function PrivacyWalletPanel() {
  const [addresses, setAddresses] = useState<StealthAddress[]>(MOCK_ADDRESSES);
  const [transactions, setTransactions] = useState<PrivacyTransaction[]>(MOCK_TRANSACTIONS);
  const [audit, setAudit] = useState<PrivacyAudit>(MOCK_AUDIT);
  const [activeTab, setActiveTab] = useState<"stealth" | "mixer" | "audit">("stealth");
  const [showAddressDetails, setShowAddressDetails] = useState(false);
  const [selectedAddress, setSelectedAddress] = useState<StealthAddress | null>(addresses[0]);

  const handleGenerateStealthAddress = () => {
    const newAddress: StealthAddress = {
      id: (addresses.length + 1).toString(),
      address: `x3s${Math.random().toString(16).substring(2, 8)}...${Math.random().toString(16).substring(2, 6)}`,
      createdAt: new Date().toISOString().split("T")[0],
      balance: 0,
      transactionCount: 0,
      status: "active",
    };
    setAddresses([...addresses, newAddress]);
    setSelectedAddress(newAddress);
  };

  const handleMixTransaction = (txId: string) => {
    setTransactions(
      transactions.map((tx) =>
        tx.id === txId
          ? { ...tx, mixerStatus: "mixed" as const, hops: Math.floor(Math.random() * 5) + 3 }
          : tx
      )
    );
  };

  const activeCount = addresses.filter((a) => a.status === "active").length;
  const totalBalance = addresses.reduce((sum, a) => sum + a.balance, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Eye size={20} className="text-purple-400" /> Privacy Wallet
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Stealth Addresses</div>
            <div className="text-lg font-bold text-purple-400">{activeCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Balance</div>
            <div className="text-lg font-bold text-green-400">{totalBalance.toFixed(2)} X3</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Privacy Risk</div>
            <div className={clsx("text-lg font-bold", audit.linkabilityRisk === "low" ? "text-green-400" : "text-yellow-400")}>
              {audit.linkabilityRisk.toUpperCase()}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Tx Analyzed</div>
            <div className="text-lg font-bold text-cyan-400">{audit.transactionsAnalyzed}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["stealth", "mixer", "audit"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab
                  ? "border-purple-600 text-purple-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {activeTab === "stealth" && (
          <div className="space-y-3">
            {/* Generate Button */}
            <button
              onClick={handleGenerateStealthAddress}
              className="w-full bg-purple-600 hover:bg-purple-700 py-2 rounded-lg font-semibold text-sm transition"
            >
              + Generate Stealth Address
            </button>

            {/* Address List */}
            {addresses.map((addr) => (
              <div
                key={addr.id}
                onClick={() => setSelectedAddress(addr)}
                className={clsx(
                  "p-3 rounded-lg border transition cursor-pointer",
                  selectedAddress?.id === addr.id
                    ? "border-purple-600 bg-purple-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold font-mono text-sm">{addr.address}</div>
                    <div className="text-xs text-gray-400 mt-1">Created {addr.createdAt}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded font-semibold",
                      addr.status === "active"
                        ? "bg-green-600/20 text-green-400"
                        : addr.status === "expired"
                          ? "bg-yellow-600/20 text-yellow-400"
                          : "bg-red-600/20 text-red-400"
                    )}
                  >
                    {addr.status}
                  </span>
                </div>
                <div className="flex justify-between text-xs text-gray-400">
                  <span>Balance: {addr.balance.toFixed(2)} X3</span>
                  <span>{addr.transactionCount} txs</span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "mixer" && (
          <div className="space-y-3">
            {/* Mixer Status */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-2">
              <div className="text-sm font-bold">Transaction Mixer Status</div>
              <div className="text-xs text-gray-400">
                Break linkability between sender and recipient through multi-hop routing.
              </div>
            </div>

            {/* Transactions */}
            {transactions.map((tx) => (
              <div key={tx.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div className="text-xs">
                    <div className="font-semibold">{tx.amount} X3</div>
                    <div className="text-gray-400">{tx.timestamp}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded font-semibold",
                      tx.mixerStatus === "completed"
                        ? "bg-green-600/20 text-green-400"
                        : tx.mixerStatus === "mixed"
                          ? "bg-blue-600/20 text-blue-400"
                          : "bg-yellow-600/20 text-yellow-400"
                    )}
                  >
                    {tx.mixerStatus === "completed" && "✓ COMPLETED"}
                    {tx.mixerStatus === "mixed" && `${tx.hops} HOPS`}
                    {tx.mixerStatus === "pending" && "PENDING MIX"}
                  </span>
                </div>

                {tx.mixerStatus === "pending" && (
                  <button
                    onClick={() => handleMixTransaction(tx.id)}
                    className="w-full bg-purple-600 hover:bg-purple-700 py-1.5 rounded text-xs font-semibold transition"
                  >
                    Start Mixing
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        {activeTab === "audit" && (
          <div className="space-y-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm">Privacy Audit Report</h3>

              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Transactions Analyzed</span>
                  <span className="font-bold text-cyan-400">{audit.transactionsAnalyzed}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Linkability Risk</span>
                  <span
                    className={clsx(
                      "font-bold",
                      audit.linkabilityRisk === "low"
                        ? "text-green-400"
                        : audit.linkabilityRisk === "medium"
                          ? "text-yellow-400"
                          : "text-red-400"
                    )}
                  >
                    {audit.linkabilityRisk.toUpperCase()}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Address Cluster Size</span>
                  <span className="font-bold text-cyan-400">{audit.addressClusterSize}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Recommended Mixing</span>
                  <span className={clsx(audit.recommendedMixing ? "text-yellow-400" : "text-green-400")}>
                    {audit.recommendedMixing ? "Yes" : "No"}
                  </span>
                </div>
              </div>
            </div>

            {/* Recommendations */}
            <div className="bg-purple-600/10 border border-purple-600/30 rounded-lg p-3">
              <div className="text-xs font-semibold text-purple-400 mb-2">💡 Privacy Tips</div>
              <ul className="text-xs text-gray-300 space-y-1">
                <li>✓ Use separate stealth addresses for different counterparties</li>
                <li>✓ Enable mixer on large transactions to break chain analysis</li>
                <li>✓ Monitor linkability risk and re-generate addresses when needed</li>
              </ul>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Stealth addresses + transaction mixer for on-chain privacy with chain analysis resistance.
      </div>
    </div>
  );
}
