import React, { useState } from "react";
import { Vote, TrendingUp, Zap, Eye, Download, CheckCircle, AlertCircle } from "lucide-react";
import clsx from "clsx";
import { useProposalList } from "../../../hooks/useSubstrate";
import { useGovernanceSnapshot } from "../../../hooks/useSubstrate";
import { useWalletStore } from "../../../stores/walletStore";
import { x3ChainService } from "../../../services/x3ChainService";
import { GovernanceProposal as ChainProposal } from "../../../lib/substrate/queries";

interface VoteBreakdown {
  for: number;
  against: number;
  abstain: number;
}

export default function GovernanceProposalsPanel() {
  const { data: proposals, isLoading, error } = useProposalList();
  const { data: snapshot } = useGovernanceSnapshot();
  const { activeAccountIndex, accounts } = useWalletStore();
  const activeAccount = accounts[activeAccountIndex];

  const [selectedProposal, setSelectedProposal] = useState<ChainProposal | null>(null);
  const [activeTab, setActiveTab] = useState<"proposals" | "details">("proposals");

  // Derive metrics from snapshot data
  const activeProposals = proposals?.filter((p) => p.status === "Active").length || 0;
  const totalProposals = proposals?.length || 0;
  const totalVoters = snapshot?.voterCount || 0;
  const avgParticipation = snapshot?.avgParticipation || 0;

  // Handle voting via x3ChainService
  const handleVote = async (proposalId: number, direction: "Aye" | "Nay" | "Abstain") => {
    if (!activeAccount) {
      console.error("No active account found");
      return;
    }

    try {
      const signer = await x3ChainService.getApi();
      const result = await x3ChainService.castVote(signer, proposalId, direction, "1000000000000", "None");
      
      if (result.success) {
        console.log(`Vote cast successfully: ${result.txHash}`);
        // Trigger revalidation of proposal list
        window.dispatchEvent(new CustomEvent("proposal-updated"));
      } else {
        console.error(`Failed to cast vote: ${result.error}`);
      }
    } catch (err) {
      console.error("Error casting vote:", err);
    }
  };

  // Calculate total votes for a proposal
  const calculateTotalVotes = (proposal: ChainProposal | null) => {
    if (!proposal) return 0;
    return (proposal.ayes || 0) + (proposal.nays || 0);
  };

  // Calculate approval percentage
  const calculateApprovalPct = (proposal: ChainProposal | null) => {
    if (!proposal) return 0;
    const total = calculateTotalVotes(proposal);
    if (total === 0) return 0;
    return (proposal.ayes / total) * 100;
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Vote size={20} className="text-blue-400" /> Governance
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Proposals</div>
            <div className="text-lg font-bold text-blue-400">{activeProposals}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Proposals</div>
            <div className="text-lg font-bold text-purple-400">{totalProposals}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Voters</div>
            <div className="text-lg font-bold text-green-400">{totalVoters.toLocaleString()}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Avg Participation</div>
            <div className="text-lg font-bold text-orange-400">{avgParticipation}%</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["proposals", "details"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-blue-600 text-blue-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Proposals List */}
        {activeTab === "proposals" && (
          <div className="space-y-2">
            {proposals && proposals.length > 0 ? (
              proposals.map((proposal) => {
                const totalVotes = calculateTotalVotes(proposal);
                const approvalPct = calculateApprovalPct(proposal);
                const statusColor = proposal.status === "Active" ? "yellow" : proposal.status === "Passed" ? "green" : "red";

                return (
                  <div
                    key={proposal.id}
                    onClick={() => {
                      setSelectedProposal(proposal);
                      setActiveTab("details");
                    }}
                    className={clsx("bg-[#15151b] border rounded-lg p-3 cursor-pointer transition", selectedProposal?.id === proposal.id ? "border-blue-600" : "border-[#2a2a35] hover:border-blue-600/50")}
                  >
                    <div className="flex justify-between items-start mb-2">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <div className="font-semibold text-sm">{proposal.title}</div>
                          <span
                            className={clsx(
                              "text-xs px-2 py-1 rounded font-bold",
                              proposal.status === "Active" && "bg-yellow-600/20 text-yellow-400",
                              proposal.status === "Passed" && "bg-green-600/20 text-green-400",
                              proposal.status === "Rejected" && "bg-red-600/20 text-red-400",
                              proposal.status === "Enacted" && "bg-blue-600/20 text-blue-400"
                            )}
                          >
                            {proposal.status}
                          </span>
                        </div>
                        <div className="text-xs text-gray-400">{proposal.description.slice(0, 60)}...</div>
                      </div>
                      {proposal.status === "Passed" && <CheckCircle size={16} className="text-green-400 flex-shrink-0" />}
                      {proposal.status === "Rejected" && <AlertCircle size={16} className="text-red-400 flex-shrink-0" />}
                    </div>

                    <div className="bg-[#0a0a0f] rounded p-2 mb-2">
                      <div className="flex-1 bg-[#2a2a35] rounded-full h-2 flex overflow-hidden">
                        <div className="bg-green-600 h-full" style={{ width: `${approvalPct}%` }} />
                        <div className="bg-red-600 h-full" style={{ width: `${((proposal.nays || 0) / totalVotes) * 100}%` }} />
                        <div className="bg-gray-600 h-full" style={{ width: `${((totalVotes - (proposal.ayes || 0) - (proposal.nays || 0)) / totalVotes) * 100}%` }} />
                      </div>
                    </div>

                    <div className="grid grid-cols-3 gap-2 text-xs">
                      <div>
                        <div className="text-green-400 font-bold">{approvalPct.toFixed(1)}% For</div>
                        <div className="text-gray-500">{((proposal.ayes || 0) / 1000000000000).toFixed(1)}M X3</div>
                      </div>
                      <div>
                        <div className="text-red-400 font-bold">{(((proposal.nays || 0) / totalVotes) * 100).toFixed(1)}% Against</div>
                        <div className="text-gray-500">{((proposal.nays || 0) / 1000000000000).toFixed(1)}M X3</div>
                      </div>
                      <div>
                        <div className="text-gray-500 font-bold">{(((totalVotes - (proposal.ayes || 0) - (proposal.nays || 0)) / totalVotes) * 100).toFixed(1)}% Abstain</div>
                        <div className="text-gray-500">{(((totalVotes - (proposal.ayes || 0) - (proposal.nays || 0)) / 1000000000000).toFixed(1))}M X3</div>
                      </div>
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="text-center text-gray-500 py-8">
                {isLoading ? "Loading proposals..." : "No proposals found"}
              </div>
            )}
          </div>
        )}

        {/* Proposal Details */}
        {activeTab === "details" && selectedProposal && (
          <div className="space-y-3">
            {/* Full Details */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <div>
                <h3 className="text-sm font-semibold mb-1">{selectedProposal.title}</h3>
                <p className="text-xs text-gray-400 leading-relaxed">{selectedProposal.description}</p>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs pb-3 border-b border-[#2a2a35]">
                <div>
                  <div className="text-gray-400">Proposer</div>
                  <div className="font-mono text-gray-500">{selectedProposal.proposer}</div>
                </div>
                <div>
                  <div className="text-gray-400">Status</div>
                  <div className="font-semibold text-cyan-400">{selectedProposal.status}</div>
                </div>
              </div>
            </div>

            {/* Vote Breakdown */}
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-xs text-gray-400 mb-3 font-semibold">Vote Breakdown</div>

              <div className="space-y-3 mb-3">
                {/* For */}
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-green-400">For (Ayes)</span>
                    <span className="font-bold text-cyan-400">{((selectedProposal.ayes || 0) / 1000000000000).toFixed(1)}M X3</span>
                  </div>
                  <div className="bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-green-600 rounded-full" style={{ width: `${calculateApprovalPct(selectedProposal)}%` }} />
                  </div>
                </div>

                {/* Against */}
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-red-400">Against (Nays)</span>
                    <span className="font-bold text-cyan-400">{((selectedProposal.nays || 0) / 1000000000000).toFixed(1)}M X3</span>
                  </div>
                  <div className="bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-red-600 rounded-full" style={{ width: `${((selectedProposal.nays || 0) / calculateTotalVotes(selectedProposal)) * 100}%` }} />
                  </div>
                </div>

                {/* Abstain */}
                <div>
                  <div className="flex justify-between mb-1 text-xs">
                    <span className="text-gray-400">Abstain</span>
                    <span className="font-bold text-cyan-400">{(((calculateTotalVotes(selectedProposal) - (selectedProposal.ayes || 0) - (selectedProposal.nays || 0))) / 1000000000000).toFixed(1)}M X3</span>
                  </div>
                  <div className="bg-[#2a2a35] rounded-full h-2">
                    <div className="h-full bg-gray-600 rounded-full" style={{ width: `${((calculateTotalVotes(selectedProposal) - (selectedProposal.ayes || 0) - (selectedProposal.nays || 0)) / calculateTotalVotes(selectedProposal)) * 100}%` }} />
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-2 text-xs pt-3 border-t border-[#2a2a35]">
                <div>
                  <div className="text-gray-400">Total Votes</div>
                  <div className="font-bold text-cyan-400">{((calculateTotalVotes(selectedProposal)) / 1000000000000).toFixed(1)}M X3</div>
                </div>
                <div>
                  <div className="text-gray-400">Proposal ID</div>
                  <div className="font-bold text-cyan-400">#{selectedProposal.id}</div>
                </div>
              </div>
            </div>

            {/* Action Button */}
            <div className="flex gap-2">
              <button
                className="flex-1 bg-green-600/20 text-green-400 text-sm font-semibold py-2 rounded hover:bg-green-600/30"
                onClick={() => handleVote(selectedProposal.id, "Aye")}
              >
                Vote For
              </button>
              <button
                className="flex-1 bg-red-600/20 text-red-400 text-sm font-semibold py-2 rounded hover:bg-red-600/30"
                onClick={() => handleVote(selectedProposal.id, "Nay")}
              >
                Vote Against
              </button>
              <button
                className="flex-1 bg-gray-600/20 text-gray-400 text-sm font-semibold py-2 rounded hover:bg-gray-600/30"
                onClick={() => handleVote(selectedProposal.id, "Abstain")}
              >
                Abstain
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        DAO proposals, voting mechanics, quorum tracking, and proposal timeline.
      </div>
    </div>
  );
}
