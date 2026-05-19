import React, { useState } from "react";
import { Lock, Users, Check, Clock, AlertTriangle, Copy, Plus, Trash2 } from "lucide-react";
import clsx from "clsx";

interface MultiSigWallet {
  id: string;
  name: string;
  address: string;
  threshold: number;
  signers: number;
  balance: number;
  status: "active" | "archived";
  createdDate: string;
}

interface PendingApproval {
  id: string;
  title: string;
  description: string;
  proposedBy: string;
  approvals: number;
  requiredApprovals: number;
  timestamp: string;
  status: "pending" | "approved" | "rejected";
}

interface CoSigner {
  id: string;
  name: string;
  address: string;
  status: "active" | "removed";
  joinedDate: string;
}

const MOCK_WALLETS: MultiSigWallet[] = [
  {
    id: "1",
    name: "Treasury 2-of-3",
    address: "0x742d35Cc6634C0532925a3b844Bc9e7595f...7e6f",
    threshold: 2,
    signers: 3,
    balance: 50000,
    status: "active",
    createdDate: "2024-01-15",
  },
  {
    id: "2",
    name: "DAO Safe 3-of-5",
    address: "0xabcd1234ef5678...9abc",
    threshold: 3,
    signers: 5,
    balance: 125000,
    status: "active",
    createdDate: "2024-02-20",
  },
];

const MOCK_PENDING: PendingApproval[] = [
  {
    id: "1",
    title: "Treasury Transfer",
    description: "Transfer 5000 X3 to marketing fund",
    proposedBy: "Alice",
    approvals: 1,
    requiredApprovals: 2,
    timestamp: "2024-10-05T14:32:00Z",
    status: "pending",
  },
  {
    id: "2",
    title: "Add New Co-Signer",
    description: "Add Bob as co-signer (replace Carol)",
    proposedBy: "Alice",
    approvals: 2,
    requiredApprovals: 2,
    timestamp: "2024-10-05T12:00:00Z",
    status: "approved",
  },
];

const MOCK_COSIGNERS: CoSigner[] = [
  { id: "1", name: "Alice", address: "0x123...456", status: "active", joinedDate: "2024-01-15" },
  { id: "2", name: "Bob", address: "0x789...abc", status: "active", joinedDate: "2024-01-15" },
  { id: "3", name: "Carol", address: "0xdef...789", status: "active", joinedDate: "2024-03-10" },
];

export default function MultiSignaturePanel() {
  const [wallets, setWallets] = useState<MultiSigWallet[]>(MOCK_WALLETS);
  const [pending, setPending] = useState<PendingApproval[]>(MOCK_PENDING);
  const [cosigners, setCosigners] = useState<CoSigner[]>(MOCK_COSIGNERS);
  const [selectedWallet, setSelectedWallet] = useState<MultiSigWallet | null>(MOCK_WALLETS[0]);
  const [activeTab, setActiveTab] = useState<"wallets" | "approvals" | "cosigners">("wallets");

  const activeWallets = wallets.filter((w) => w.status === "active").length;
  const pendingApprovals = pending.filter((p) => p.status === "pending").length;

  const handleApprove = (approvalId: string) => {
    setPending(pending.map((p) =>
      p.id === approvalId
        ? { ...p, approvals: p.approvals + 1, status: p.approvals + 1 >= p.requiredApprovals ? "approved" as const : "pending" as const }
        : p
    ));
  };

  const handleReject = (approvalId: string) => {
    setPending(pending.map((p) =>
      p.id === approvalId ? { ...p, status: "rejected" as const } : p
    ));
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Lock size={20} className="text-orange-400" /> Multi-Signature Wallets
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Wallets</div>
            <div className="text-lg font-bold text-orange-400">{activeWallets}/{wallets.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Value</div>
            <div className="text-lg font-bold text-cyan-400">${wallets.reduce((sum, w) => sum + w.balance, 0).toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Pending Approvals</div>
            <div className="text-lg font-bold text-yellow-400">{pendingApprovals}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["wallets", "approvals", "cosigners"] as const).map((tab) => (
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
              {tab === "wallets" && "Wallets"}
              {tab === "approvals" && "Approvals"}
              {tab === "cosigners" && "Co-Signers"}
            </button>
          ))}
        </div>

        {activeTab === "wallets" && (
          <div className="space-y-3">
            <button className="w-full bg-orange-600 hover:bg-orange-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2">
              <Plus size={14} /> Create New Wallet
            </button>

            {wallets.map((wallet) => (
              <button
                key={wallet.id}
                onClick={() => setSelectedWallet(wallet)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedWallet?.id === wallet.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="font-semibold">{wallet.name}</div>
                  <span className="text-xs px-2 py-1 bg-orange-600/20 text-orange-400 rounded-md font-bold">
                    {wallet.threshold}-of-{wallet.signers}
                  </span>
                </div>

                <div className="text-xs text-gray-400 font-mono mb-2">{wallet.address}</div>

                <div className="flex justify-between text-xs">
                  <span className="text-gray-500">Created: {wallet.createdDate}</span>
                  <span className="font-bold text-cyan-400">${wallet.balance.toLocaleString()}</span>
                </div>
              </button>
            ))}
          </div>
        )}

        {activeTab === "approvals" && (
          <div className="space-y-2">
            {pending.map((approval) => (
              <div key={approval.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-semibold text-sm">{approval.title}</div>
                    <div className="text-xs text-gray-400 mt-1">{approval.description}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded-md font-bold",
                      approval.status === "pending"
                        ? "bg-yellow-600/20 text-yellow-400"
                        : approval.status === "approved"
                        ? "bg-green-600/20 text-green-400"
                        : "bg-red-600/20 text-red-400"
                    )}
                  >
                    {approval.status.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-xs text-gray-400">
                  <div>Proposed by: {approval.proposedBy}</div>
                  <div>{approval.timestamp}</div>
                </div>

                <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-orange-600 to-red-600"
                    style={{ width: `${(approval.approvals / approval.requiredApprovals) * 100}%` }}
                  />
                </div>
                <div className="text-xs text-gray-400">
                  {approval.approvals}/{approval.requiredApprovals} signatures
                </div>

                {approval.status === "pending" && (
                  <div className="flex gap-2 pt-2">
                    <button
                      onClick={() => handleApprove(approval.id)}
                      className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded text-xs font-semibold transition"
                    >
                      Approve
                    </button>
                    <button
                      onClick={() => handleReject(approval.id)}
                      className="flex-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 py-2 rounded text-xs font-semibold transition"
                    >
                      Reject
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {activeTab === "cosigners" && (
          <div className="space-y-2">
            {cosigners.map((signer) => (
              <div key={signer.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-semibold text-sm">{signer.name}</div>
                    <div className="text-xs text-gray-400 font-mono">{signer.address}</div>
                  </div>
                  <span className="text-xs px-2 py-1 bg-green-600/20 text-green-400 rounded-md font-bold">
                    {signer.status.toUpperCase()}
                  </span>
                </div>
                <div className="text-xs text-gray-500">Joined: {signer.joinedDate}</div>
              </div>
            ))}

            <button className="w-full bg-orange-600 hover:bg-orange-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2 mt-2">
              <Users size={14} /> Add Co-Signer
            </button>
          </div>
        )}

        {/* Wallet Details */}
        {selectedWallet && activeTab === "wallets" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 text-sm">
            <h3 className="font-semibold">{selectedWallet.name} Details</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Threshold</span>
                <span className="font-bold">{selectedWallet.threshold}-of-{selectedWallet.signers}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Balance</span>
                <span className="font-bold text-cyan-400">${selectedWallet.balance.toLocaleString()}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className="font-bold text-green-400">ACTIVE</span>
              </div>
            </div>

            <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition">
              View All Transactions
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        M-of-N multi-signature wallet management with approval workflows.
      </div>
    </div>
  );
}
