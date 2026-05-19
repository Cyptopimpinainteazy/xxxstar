import React, { useState } from "react";
import { Wallet2 as Wallet, TrendingUp, Zap, Eye, Download, Lock, AlertCircle } from "lucide-react";
import clsx from "clsx";
import { useTreasurySnapshot } from "../../../hooks/useSubstrate";
import { useTreasuryBalance } from "../../../hooks/useSubstrate";
import { useTreasuryWallets } from "../../../hooks/useSubstrate";
import { useWalletStore } from "../../../stores/walletStore";
import { x3ChainService } from "../../../services/x3ChainService";

export default function TreasuryManagementPanel() {
  const { data: snapshot, isLoading: loadingSnapshot } = useTreasurySnapshot();
  const { data: balance, isLoading: loadingBalance } = useTreasuryBalance();
  const { data: wallets, isLoading: loadingWallets } = useTreasuryWallets();
  const { activeAccountIndex, accounts } = useWalletStore();
  const activeAccount = accounts[activeAccountIndex];

  const [activeTab, setActiveTab] = useState<"allocation" | "wallets" | "spending">("allocation");

  // Calculate totals from chain data
  const totalTreasury = parseFloat(balance || "0") / 1000000000000;
  const totalAllocated = snapshot?.totalAllocated ? parseFloat(snapshot.totalAllocated) / 1000000000000 : 0;
  const pendingApprovals = Array.isArray(snapshot?.pendingProposals)
    ? snapshot.pendingProposals.length
    : 0;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Wallet size={20} className="text-cyan-400" /> Treasury Management
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Treasury</div>
            <div className="text-lg font-bold text-cyan-400">${(totalTreasury / 1000000).toFixed(2)}M</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Allocated</div>
            <div className="text-lg font-bold text-purple-400">${(totalAllocated / 1000000).toFixed(2)}M</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Multi-Sig Wallets</div>
            <div className="text-lg font-bold text-green-400">{wallets?.length ?? 0}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Pending Approvals</div>
            <div className="text-lg font-bold text-orange-400">{pendingApprovals}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["allocation", "wallets", "spending"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-cyan-600 text-cyan-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "allocation" ? "Budget" : tab}
            </button>
          ))}
        </div>

        {/* Budget Allocation */}
        {activeTab === "allocation" && (
          <div className="space-y-3">
            {snapshot?.allocations && snapshot.allocations.length > 0 ? (
              snapshot.allocations.map((alloc: any, idx: number) => (
                <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <div className="font-semibold text-sm">{alloc.category || "Allocation"}</div>
                      <div className="text-xs text-gray-400">{alloc.description || ""}</div>
                    </div>
                    <span
                      className={clsx(
                        "text-xs px-2 py-1 rounded font-bold",
                        alloc.status === "allocated" && "bg-blue-600/20 text-blue-400",
                        alloc.status === "pending" && "bg-yellow-600/20 text-yellow-400",
                        alloc.status === "spent" && "bg-green-600/20 text-green-400"
                      )}
                    >
                      {alloc.status || "unknown"}
                    </span>
                  </div>

                  <div className="bg-[#0a0a0f] rounded p-2 mb-2">
                    <div className="text-xs text-gray-400 mb-1">
                      Budget: ${(parseFloat(alloc.amount || "0") / 1000000000000).toFixed(2)}M
                    </div>
                    <div className="bg-[#2a2a35] rounded-full h-2">
                      <div
                        className={clsx("h-full rounded-full", alloc.status === "allocated" ? "bg-blue-600" : alloc.status === "pending" ? "bg-yellow-600" : "bg-green-600")}
                        style={{ width: `${alloc.percentage || 0}%` }}
                      />
                    </div>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-500">
                {loadingSnapshot ? "Loading allocations..." : "No budget allocations found"}
              </div>
            )}

            {/* Budget Summary */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 mt-4">
              <div className="text-xs text-gray-400 mb-2 font-semibold">Budget Summary</div>
              <div className="space-y-1 text-xs">
                <div className="flex justify-between">
                  <span className="text-gray-400">Total Treasury</span>
                  <span className="font-bold text-cyan-400">${totalTreasury.toFixed(2)}M</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Total Allocated</span>
                  <span className="font-bold text-purple-400">${totalAllocated.toFixed(2)}M</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Percentage Allocated</span>
                  <span className="font-bold text-orange-400">{totalTreasury > 0 ? ((totalAllocated / totalTreasury) * 100).toFixed(1) : "0"}%</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Multi-Sig Wallets */}
        {activeTab === "wallets" && (
          <div className="space-y-2">
            {wallets && wallets.length > 0 ? (
              wallets.map((wallet: any, idx: number) => (
                <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                  <div className="flex justify-between items-start mb-2">
                    <div className="flex items-center gap-2">
                      <Lock size={16} className="text-green-400" />
                      <div>
                        <div className="font-semibold text-sm">{wallet.name || "Treasury Wallet"}</div>
                        <div className="text-xs text-gray-400 font-mono">{wallet.address || "N/A"}</div>
                      </div>
                    </div>
                    <span className="text-xs px-2 py-1 bg-green-600/20 text-green-400 rounded font-bold">{wallet.status || "active"}</span>
                  </div>

                  <div className="grid grid-cols-3 gap-2 mb-2 text-xs">
                    <div>
                      <div className="text-gray-400">Balance</div>
                      <div className="font-bold text-cyan-400">${(parseFloat(wallet.balance || "0") / 1000000000000).toFixed(2)}M</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Required Sigs</div>
                      <div className="font-bold text-purple-400">
                        {wallet.signaturesRequired || 2}/{wallet.signers || 3}
                      </div>
                    </div>
                    <div>
                      <div className="text-gray-400">% of Treasury</div>
                      <div className="font-bold text-orange-400">{totalTreasury > 0 ? ((parseFloat(wallet.balance || "0") / 1000000000000) / totalTreasury * 100).toFixed(1) : "0"}%</div>
                    </div>
                  </div>

                  <div className="text-xs text-gray-500">Multi-signature protection enabled</div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-500">
                {loadingWallets ? "Loading wallets..." : "No multi-signature wallets found"}
              </div>
            )}
          </div>
        )}

        {/* Spending History */}
        {activeTab === "spending" && (
          <div className="space-y-2">
            {snapshot?.proposals && snapshot.proposals.length > 0 ? (
              snapshot.proposals.map((proposal: any, idx: number) => (
                <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                  <div className="flex justify-between items-start mb-2">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <div className="font-semibold text-sm">{proposal.description || "Treasury Proposal"}</div>
                        <span
                          className={clsx(
                            "text-xs px-2 py-1 rounded font-bold",
                            proposal.status === "Approved" && "bg-green-600/20 text-green-400",
                            proposal.status === "Pending" && "bg-yellow-600/20 text-yellow-400",
                            proposal.status === "Rejected" && "bg-red-600/20 text-red-400"
                          )}
                        >
                          {proposal.status || "unknown"}
                        </span>
                      </div>
                      <div className="text-xs text-gray-400">{proposal.createdAt ? new Date(proposal.createdAt * 1000).toLocaleDateString() : "Unknown date"}</div>
                    </div>
                    <div className="text-right">
                      <div className="font-bold text-cyan-400">${(parseFloat(proposal.amount || "0") / 1000000000000).toFixed(2)}M</div>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs mb-2">
                    <div>
                      <div className="text-gray-400">Beneficiary</div>
                      <div className="font-mono text-gray-500 text-xs">{proposal.beneficiary || "N/A"}</div>
                    </div>
                    <div>
                      <div className="text-gray-400">Track</div>
                      <div className="font-bold text-purple-400">{proposal.track || "Standard"}</div>
                    </div>
                  </div>

                  {proposal.status === "Pending" && (
                    <div className="flex gap-2 mt-2">
                      <button
                        className="flex-1 bg-green-600/20 text-green-400 text-xs font-semibold py-1 rounded hover:bg-green-600/30"
                        onClick={() => console.log("Approve proposal:", proposal.id)}
                      >
                        Approve
                      </button>
                      <button
                        className="flex-1 bg-red-600/20 text-red-400 text-xs font-semibold py-1 rounded hover:bg-red-600/30"
                        onClick={() => console.log("Reject proposal:", proposal.id)}
                      >
                        Reject
                      </button>
                    </div>
                  )}
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-500">
                {loadingSnapshot ? "Loading proposals..." : "No spending proposals found"}
              </div>
            )}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Multi-signature treasury, budget allocation, spending history, and approval workflows.
      </div>
    </div>
  );
}
