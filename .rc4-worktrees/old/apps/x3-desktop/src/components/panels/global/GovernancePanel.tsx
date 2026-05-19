import React, { useState } from "react";
import { Vote, TrendingUp, Users, Zap, Plus, CheckCircle } from "lucide-react";
import clsx from "clsx";
import { useProposalList } from "../../../hooks/useSubstrate";
import { useGovernanceSnapshot } from "../../../hooks/useSubstrate";
import { useWalletStore } from "../../../stores/walletStore";
import { x3ChainService } from "../../../services/x3ChainService";
import { GovernanceProposal as ChainProposal } from "../../../lib/substrate/queries";

interface Proposal {
  id: string;
  title: string;
  description: string;
  type: "parameter" | "treasury" | "upgrade" | "action";
  status: "active" | "passed" | "failed" | "executed";
  votesFor: number;
  votesAgainst: number;
  totalVotes: number;
  quorumRequired: number;
  endsIn: string;
  proposer: string;
}

export default function GovernancePanel() {
  const { data: proposals, isLoading, error } = useProposalList();
  const { data: snapshot } = useGovernanceSnapshot();
  const { activeAccountIndex, accounts } = useWalletStore();
  const activeAccount = accounts[activeAccountIndex];

  const [selectedProposal, setSelectedProposal] = useState<Proposal | null>(null);
  const [userVotingPower, setUserVotingPower] = useState(250000);
  const [showCreateProposal, setShowCreateProposal] = useState(false);
  const [userVote, setUserVote] = useState<{ [key: string]: "for" | "against" | null }>({});

  const totalStaked = Number(snapshot?.totalStaked ?? 0) || 50000000;
  const currentVotingPower = userVotingPower;
  const delegatedVotingPower = 0;

  const handleVote = async (proposalId: string, direction: "for" | "against") => {
    if (!activeAccount) {
      console.error("No active account found");
      return;
    }
    try {
      const signer = await x3ChainService.getApi();
      const result = await x3ChainService.castVote(signer, parseInt(proposalId), direction === "for" ? "Aye" : "Nay", "1000000000000", "None");
      if (result.success) {
        console.log(`Vote cast successfully: ${result.txHash}`);
        setUserVote({ ...userVote, [proposalId]: direction });
        window.dispatchEvent(new CustomEvent("proposal-updated"));
      }
    } catch (err) {
      console.error("Error casting vote:", err);
    }
  };

  const getProposalColor = (type: string) => {
    switch (type) {
      case "parameter":
        return "border-blue-600 text-blue-400";
      case "treasury":
        return "border-green-600 text-green-400";
      case "upgrade":
        return "border-purple-600 text-purple-400";
      case "action":
        return "border-orange-600 text-orange-400";
      default:
        return "border-gray-600 text-gray-400";
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active":
        return "bg-blue-600/30 border-blue-600 text-blue-400";
      case "passed":
        return "bg-green-600/30 border-green-600 text-green-400";
      case "failed":
        return "bg-red-600/30 border-red-600 text-red-400";
      case "executed":
        return "bg-purple-600/30 border-purple-600 text-purple-400";
      default:
        return "bg-gray-600/30 border-gray-600 text-gray-400";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Vote size={20} className="text-blue-400" /> DAO Governance
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Voting Power Card */}
        <div className="bg-gradient-to-r from-blue-600/20 to-purple-600/20 border border-blue-600 rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">Your Voting Power</h3>
          <div className="grid grid-cols-3 gap-2 mb-3">
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Current</div>
              <div className="text-lg font-bold text-blue-400">{(currentVotingPower / 1000).toFixed(0)}K</div>
            </div>
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Delegated</div>
              <div className="text-lg font-bold">{delegatedVotingPower.toLocaleString()}</div>
            </div>
            <div className="bg-[#15151b] p-2 rounded text-center">
              <div className="text-xs text-gray-400">Total Staked</div>
              <div className="text-lg font-bold text-purple-400">{(totalStaked / 1000000).toFixed(1)}M</div>
            </div>
          </div>

          <div className="bg-[#2a2a35] rounded p-2 text-xs">
            <div className="flex justify-between mb-1">
              <span className="text-gray-400">Voting Power Share</span>
              <span className="font-semibold">{((currentVotingPower / totalStaked) * 100).toFixed(2)}%</span>
            </div>
            <div className="flex-1 bg-[#3a3a45] rounded-full h-2 overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-blue-600 to-purple-600"
                style={{ width: `${(currentVotingPower / totalStaked) * 100}%` }}
              />
            </div>
          </div>
        </div>

        {/* Create Proposal Button */}
        <button
          onClick={() => setShowCreateProposal(!showCreateProposal)}
          className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
        >
          <Plus size={14} /> Create Proposal
        </button>

        {/* Create Proposal Form */}
        {showCreateProposal && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
            <h3 className="font-semibold text-sm">New Proposal</h3>
            <input
              type="text"
              placeholder="Proposal title..."
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600"
            />
            <textarea
              placeholder="Description..."
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm h-20 focus:outline-none focus:border-blue-600 resize-none"
            />
            <select className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-600">
              <option>Parameter Change</option>
              <option>Treasury Allocation</option>
              <option>Smart Contract Upgrade</option>
              <option>Other Action</option>
            </select>
            <div className="flex gap-2">
              <button className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
                Submit
              </button>
              <button
                onClick={() => setShowCreateProposal(false)}
                className="flex-1 bg-[#2a2a35] hover:bg-[#3a3a45] py-2 rounded-lg font-semibold text-sm transition"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* Proposals List */}
        <div>
          <h3 className="font-semibold mb-3 text-sm">Active & Recent Proposals</h3>
          {isLoading ? (
            <div className="text-center py-8 text-gray-400">Loading proposals...</div>
          ) : error ? (
            <div className="text-center py-4 text-red-400 bg-red-900/20 rounded-lg border border-red-900/50">
              Error loading proposals: {error.message}
            </div>
          ) : (
            <div className="space-y-2">
              {proposals && proposals.length > 0 ? (
                proposals.map((proposal: ChainProposal) => {
                  const chainProposal: Proposal = {
                    id: proposal.id.toString(),
                    title: proposal.title,
                    description: proposal.description,
                    type: "parameter",
                    status: proposal.status === "Active" ? "active" : proposal.status === "Passed" ? "passed" : proposal.status === "Rejected" ? "failed" : "executed",
                    votesFor: proposal.ayes,
                    votesAgainst: proposal.nays,
                    totalVotes: proposal.ayes + proposal.nays,
                    quorumRequired: proposal.threshold,
                    endsIn: proposal.votingEnd > Date.now() / 1000 ? `${Math.ceil((proposal.votingEnd - Date.now() / 1000) / 86400)} days` : "Closed",
                    proposer: proposal.proposer,
                  };
                  return (
                    <button
                      key={chainProposal.id}
                      onClick={() => setSelectedProposal(chainProposal)}
                      className={clsx(
                        "w-full text-left p-3 rounded-lg border-2 transition",
                        selectedProposal?.id === chainProposal.id
                          ? "border-blue-600 bg-blue-600/10"
                          : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                      )}
                    >
                      <div className="flex items-start justify-between mb-2">
                        <div className="flex-1">
                          <div className="text-sm font-semibold mb-1">{chainProposal.title}</div>
                          <span className={clsx("text-xs px-2 py-1 rounded border-2", getProposalColor(chainProposal.type))}>
                            {chainProposal.type}
                          </span>
                        </div>
                        <span className={clsx("text-xs px-2 py-1 rounded border", getStatusColor(chainProposal.status))}>
                          {chainProposal.status}
                        </span>
                      </div>

                      <div className="mt-2 mb-2">
                        <div className="flex items-center gap-2 mb-1">
                          <div className="text-xs text-gray-400">Support</div>
                          <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                            <div
                              className="h-full bg-gradient-to-r from-green-600 to-blue-600"
                              style={{
                                width: `${(chainProposal.votesFor / chainProposal.totalVotes) * 100}%`,
                              }}
                            />
                          </div>
                          <span className="text-xs text-gray-400">
                            {((chainProposal.votesFor / chainProposal.totalVotes) * 100).toFixed(0)}%
                          </span>
                        </div>
                      </div>

                      <div className="text-xs text-gray-400">
                        {(chainProposal.votesFor / 1000000).toFixed(1)}M for • {(chainProposal.votesAgainst / 1000000).toFixed(1)}M against •{" "}
                        {chainProposal.endsIn}
                      </div>
                    </button>
                  );
                })
              ) : (
                <div className="text-center py-8 text-gray-400">No proposals found</div>
              )}
            </div>
          )}
        </div>

        {/* Selected Proposal Details */}
        {selectedProposal && (
          <>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 text-sm">Proposal Details</h3>

              <div className="space-y-3 text-sm">
                <div>
                  <div className="text-gray-400 mb-1">Description</div>
                  <div className="text-white">{selectedProposal.description}</div>
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400 mb-1">For</div>
                    <div className="font-bold text-green-400">
                      {(selectedProposal.votesFor / 1000000).toFixed(1)}M
                    </div>
                  </div>
                  <div className="bg-[#2a2a35] p-2 rounded">
                    <div className="text-xs text-gray-400 mb-1">Against</div>
                    <div className="font-bold text-red-400">
                      {(selectedProposal.votesAgainst / 1000000).toFixed(1)}M
                    </div>
                  </div>
                </div>

                <div className="border-t border-[#2a2a35] pt-3">
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-400">Quorum Required</span>
                    <span className="font-semibold">
                      {((selectedProposal.totalVotes / selectedProposal.quorumRequired) * 100).toFixed(0)}% complete
                    </span>
                  </div>
                  <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-orange-600 to-yellow-600"
                      style={{
                        width: `${(selectedProposal.totalVotes / selectedProposal.quorumRequired) * 100}%`,
                      }}
                    />
                  </div>
                </div>

                <div className="text-xs text-gray-400">
                  Proposed by: <span className="font-mono">{selectedProposal.proposer}</span>
                </div>
              </div>
            </div>

            {/* Voting Interface */}
            {selectedProposal.status === "active" && (
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <h3 className="font-semibold mb-3 text-sm">Cast Your Vote</h3>

                {userVote[selectedProposal.id] ? (
                  <div className="flex items-center gap-2 p-3 bg-green-600/20 border border-green-600 rounded-lg">
                    <CheckCircle size={16} className="text-green-400" />
                    <div className="text-sm font-semibold">
                      Voted {userVote[selectedProposal.id] === "for" ? "For" : "Against"}
                    </div>
                  </div>
                ) : (
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleVote(selectedProposal.id, "for")}
                      className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
                    >
                      <TrendingUp size={14} /> Vote For
                    </button>
                    <button
                      onClick={() => handleVote(selectedProposal.id, "against")}
                      className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition"
                    >
                      Vote Against
                    </button>
                  </div>
                )}

                <div className="mt-3 text-xs text-gray-400 text-center">
                  Your vote: {(currentVotingPower / 1000).toFixed(0)}K X3 tokens
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        DAO governance ensures community-driven protocol decisions.
      </div>
    </div>
  );
}
