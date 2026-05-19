import React, { useState } from "react";
import { Vote, CheckCircle, Clock, TrendingUp, BarChart3, AlertCircle } from "lucide-react";
import clsx from "clsx";
import { useProposalList } from "../../../hooks/useSubstrate";
import { useMyVotes } from "../../../hooks/useSubstrate";
import { useWalletStore } from "../../../stores/walletStore";
import { x3ChainService } from "../../../services/x3ChainService";
import { GovernanceProposal as ChainProposal } from "../../../lib/substrate/queries";

type TabType = "active" | "voted" | "create" | "history";

export default function GovernanceVotingPanel() {
  const { data: proposals, isLoading, error } = useProposalList();
  const { data: myVotes } = useMyVotes(null);
  const { activeAccountIndex, accounts } = useWalletStore();
  const activeAccount = accounts[activeAccountIndex];

  const [activeTab, setActiveTab] = useState<TabType>("active");
  const [selectedProposal, setSelectedProposal] = useState<string | null>(null);

  // Filter active proposals from chain data
  const activeProposals = proposals?.filter((p) => p.status === "Active") || [];

  // Handle voting via x3ChainService
  const handleVote = async (proposalId: number, voteType: "yes" | "no" | "abstain") => {
    if (!activeAccount) {
      console.error("No active account found");
      return;
    }

    try {
      const signer = await x3ChainService.getApi();
      const direction = voteType === "yes" ? "Aye" : voteType === "no" ? "Nay" : "Abstain";
      const result = await x3ChainService.castVote(signer, proposalId, direction, "1000000000000", "None");
      
      if (result.success) {
        console.log(`Vote cast successfully: ${result.txHash}`);
        window.dispatchEvent(new CustomEvent("proposal-updated"));
      } else {
        console.error(`Failed to cast vote: ${result.error}`);
      }
    } catch (err) {
      console.error("Error casting vote:", err);
    }
  };

  const getStatusColor = (status: string) => {
    if (status === "passed") return "bg-green-600/20 text-green-400";
    if (status === "failed") return "bg-red-600/20 text-red-400";
    if (status === "pending") return "bg-yellow-600/20 text-yellow-400";
    return "bg-cyan-600/20 text-cyan-400";
  };

  const getProposalTypeColor = (type: string) => {
    const colors: Record<string, string> = {
      parameter: "bg-blue-600/20 text-blue-400",
      upgrade: "bg-purple-600/20 text-purple-400",
      treasury: "bg-green-600/20 text-green-400",
      other: "bg-gray-600/20 text-gray-400",
    };
    return colors[type] || colors.other;
  };

  const calculatePassingChance = (yes: number, no: number) => {
    const total = yes + no;
    if (total === 0) return 0;
    return (yes / total) * 100;
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Vote size={20} className="text-purple-400" /> Governance & Voting
      </h2>

      {/* Tab Navigation */}
      <div className="flex gap-2 mb-4 border-b border-[#2a2a35] overflow-x-auto">
        {(["active", "voted", "create", "history"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={clsx(
              "px-4 py-2 text-sm font-semibold border-b-2 transition whitespace-nowrap",
              activeTab === tab
                ? "border-purple-400 text-purple-400"
                : "border-transparent text-gray-400 hover:text-white"
            )}
          >
            {tab === "active" && `Active (${activeProposals.length})`}
            {tab === "voted" && `My Votes (${myVotes?.length || 0})`}
            {tab === "create" && "Create Proposal"}
            {tab === "history" && "History"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto space-y-4">
        {/* Active Proposals */}
        {activeTab === "active" && (
          <div className="space-y-3">
            {activeProposals.length > 0 ? (
              activeProposals.map((proposal) => {
                const totalVotes = (proposal.ayes || 0) + (proposal.nays || 0);
                const passingChance = totalVotes > 0 ? (proposal.ayes / totalVotes) * 100 : 0;

                return (
                  <div
                    key={proposal.id}
                    className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 hover:border-[#3a3a45] transition cursor-pointer"
                    onClick={() => setSelectedProposal(selectedProposal === String(proposal.id) ? null : String(proposal.id))}
                  >
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <h3 className="font-semibold text-sm">{proposal.title}</h3>
                        <div className="flex gap-2 mt-2">
                          <span className={clsx("text-xs px-2 py-0.5 rounded", getStatusColor(proposal.status.toLowerCase()))}>
                            {proposal.status.charAt(0).toUpperCase() + proposal.status.slice(1)}
                          </span>
                        </div>
                      </div>
                      <div className="text-right text-xs">
                        <div className="text-gray-500 flex items-center gap-1 justify-end">
                          <Clock size={12} /> {proposal.votingStart ? "Active" : "Pending"}
                        </div>
                      </div>
                    </div>

                    {/* Vote Progress */}
                    <div className="mt-3 space-y-2">
                      <div className="flex gap-2">
                        <div className="flex-1">
                          <div className="text-xs text-gray-500 mb-1">Yes: {((proposal.ayes || 0) / 1000000000000).toFixed(1)}M</div>
                          <div className="bg-[#0a0a0f] rounded-full h-2">
                            <div
                              className="h-full bg-green-600 rounded-full"
                              style={{ width: `${passingChance}%` }}
                            />
                          </div>
                        </div>
                        <div className="flex-1">
                          <div className="text-xs text-gray-500 mb-1">No: {((proposal.nays || 0) / 1000000000000).toFixed(1)}M</div>
                          <div className="bg-[#0a0a0f] rounded-full h-2">
                            <div
                              className="h-full bg-red-600 rounded-full"
                              style={{ width: `${totalVotes > 0 ? ((proposal.nays || 0) / totalVotes) * 100 : 0}%` }}
                            />
                          </div>
                        </div>
                      </div>
                    </div>

                    {/* Expanded Details */}
                    {selectedProposal === String(proposal.id) && (
                      <div className="mt-4 pt-4 border-t border-[#2a2a35] space-y-3">
                        <p className="text-xs text-gray-400 leading-relaxed">{proposal.description}</p>
                        <div className="flex justify-between text-xs">
                          <span className="text-gray-500">Proposer:</span>
                          <span className="text-cyan-400 font-mono">{proposal.proposer}</span>
                        </div>
                        <div className="flex justify-between text-xs">
                          <span className="text-gray-500">Passing Chance:</span>
                          <span className="text-yellow-400 font-mono">{passingChance.toFixed(1)}%</span>
                        </div>

                        <div className="flex gap-2 mt-3">
                          <button
                            onClick={() => handleVote(proposal.id, "yes")}
                            className="flex-1 bg-green-600/20 border border-green-600 text-green-400 py-1.5 rounded text-xs font-semibold hover:bg-green-600/30 transition"
                          >
                            Vote Yes
                          </button>
                          <button
                            onClick={() => handleVote(proposal.id, "no")}
                            className="flex-1 bg-red-600/20 border border-red-600 text-red-400 py-1.5 rounded text-xs font-semibold hover:bg-red-600/30 transition"
                          >
                            Vote No
                          </button>
                          <button
                            onClick={() => handleVote(proposal.id, "abstain")}
                            className="flex-1 bg-gray-600/20 border border-gray-600 text-gray-400 py-1.5 rounded text-xs font-semibold hover:bg-gray-600/30 transition"
                          >
                            Abstain
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })
            ) : (
              <div className="text-center py-8 text-gray-500">
                {isLoading ? "Loading proposals..." : "No active proposals found"}
              </div>
            )}
          </div>
        )}

        {/* My Votes Tab */}
        {activeTab === "voted" && (
          <div className="space-y-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 text-xs">
              <div className="text-gray-500 mb-1">My Votes</div>
              <div className="text-lg font-bold text-purple-400">{myVotes?.length || 0} votes cast</div>
            </div>
            {myVotes && myVotes.length > 0 ? (
              myVotes.map((vote: any, idx: number) => {
                const proposal = proposals?.find((p) => p.id === vote.proposalId);
                return (
                  <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                    <div className="flex justify-between items-start">
                      <div>
                        <div className="text-sm font-semibold">{proposal?.title}</div>
                        <div className="text-xs text-gray-500 mt-1">Proposal: #{vote.proposalId}</div>
                        <div className="text-xs text-gray-600 mt-1">{vote.timestamp || "Recent"}</div>
                      </div>
                      <span
                        className={clsx(
                          "text-xs px-2 py-1 rounded font-semibold",
                          vote.vote?.toLowerCase() === "aye"
                            ? "bg-green-600/20 text-green-400"
                            : vote.vote?.toLowerCase() === "nay"
                            ? "bg-red-600/20 text-red-400"
                            : "bg-gray-600/20 text-gray-400"
                        )}
                      >
                        {vote.vote?.toUpperCase() || "VOTE"}
                      </span>
                    </div>
                  </div>
                );
              })
            ) : (
              <div className="text-center py-4 text-gray-500 text-sm">
                No votes cast yet
              </div>
            )}
          </div>
        )}

        {/* Create Proposal */}
        {activeTab === "create" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4 max-w-md">
            <h3 className="font-semibold text-sm">Create New Proposal</h3>
            <div>
              <label className="text-xs text-gray-400">Title</label>
              <input
                type="text"
                placeholder="Proposal title"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Description</label>
              <textarea
                placeholder="Detailed description"
                className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white placeholder-gray-600 h-24"
              />
            </div>
            <div>
              <label className="text-xs text-gray-400">Type</label>
              <select className="w-full mt-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-sm text-white">
                <option value="parameter">Parameter</option>
                <option value="upgrade">Upgrade</option>
                <option value="treasury">Treasury</option>
                <option value="other">Other</option>
              </select>
            </div>
            <button className="w-full bg-purple-600/20 border border-purple-600 text-purple-400 py-2 rounded font-semibold text-sm hover:bg-purple-600/30 transition">
              Submit Proposal
            </button>
          </div>
        )}

        {/* History Tab */}
        {activeTab === "history" && (
          <div className="text-center py-8 text-gray-500">
            <Vote size={32} className="mx-auto mb-3 opacity-50" />
            <p className="text-sm">No voting history yet</p>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        DAO governance voting with time-locks and quorum tracking
      </div>
    </div>
  );
}
