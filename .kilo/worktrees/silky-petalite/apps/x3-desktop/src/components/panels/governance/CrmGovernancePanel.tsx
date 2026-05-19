import { useState } from "react";
import { ShieldAlert, Users, BrainCircuit, Activity, CheckCircle, Clock, ArrowRight, XCircle } from "lucide-react";
import clsx from "clsx";
import { useAIProposals } from "../../../hooks/useSubstrate";
import { useWalletStore } from "../../../stores/walletStore";
import { x3ChainService } from "../../../services/x3ChainService";
import { GovernanceProposal as ChainProposal } from "../../../lib/substrate/queries";

export default function CrmGovernancePanel() {
  const { data: proposals, isLoading, error } = useAIProposals();
  const { activeAccountIndex, accounts } = useWalletStore();
  const activeAccount = accounts[activeAccountIndex];

  const [selectedProposal, setSelectedProposal] = useState<ChainProposal | null>(null);

  // Handle voting via x3ChainService
  const handleVote = async (proposalId: number, voteType: "yes" | "no") => {
    if (!activeAccount) {
      console.error("No active account found");
      return;
    }

    try {
      const signer = await x3ChainService.getApi();
      const direction = voteType === "yes" ? "Aye" : "Nay";
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

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case "Critical": return "text-red-500 bg-red-500/10 border-red-500/20";
      case "High": return "text-orange-500 bg-orange-500/10 border-orange-500/20";
      case "Medium": return "text-yellow-500 bg-yellow-500/10 border-yellow-500/20";
      case "Low": return "text-green-500 bg-green-500/10 border-green-500/20";
      default: return "text-gray-400 bg-gray-500/10 border-gray-500/20";
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col font-sans">
      <div className="flex justify-between items-center mb-6 border-b border-[#2a2a35] pb-4">
        <div>
          <h2 className="text-2xl font-bold flex items-center gap-2 bg-gradient-to-r from-purple-400 to-cyan-400 bg-clip-text text-transparent">
            <Users size={24} className="text-purple-400" /> CRM Governance Window
          </h2>
          <p className="text-sm text-gray-400 mt-1">
            Human-in-the-Loop Override & Authorizations
          </p>
        </div>
        <div className="bg-[#15151b] px-4 py-2 rounded-lg border border-[#2a2a35] flex items-center gap-3 shadow-lg">
          <Activity className="text-cyan-400 animate-pulse" size={16} />
          <span className="text-sm font-semibold">Live Swarm Connection</span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto space-y-4 pr-2 custom-scrollbar">
        {proposals && proposals.length > 0 ? (
          proposals.map((proposal) => (
            <div
              key={proposal.id}
              className={clsx(
                "rounded-xl border p-5 backdrop-blur-md transition-all duration-300",
                "bg-gradient-to-br from-[#1c1525] to-[#15151b] border-purple-500/30 hover:border-purple-500/60 shadow-[0_0_15px_rgba(168,85,247,0.05)]"
              )}
            >
              <div className="flex justify-between items-start mb-3">
                <div className="flex items-center gap-3">
                  <ShieldAlert className="text-purple-400" size={20} />
                  <div>
                    <h3 className="font-bold text-lg text-gray-100">{proposal.title}</h3>
                    <div className="text-xs font-mono text-gray-500 uppercase tracking-widest mt-1">
                      Source: <span className="text-gray-300">{proposal.proposer}</span>
                    </div>
                  </div>
                </div>
                
                <div className="flex flex-col items-end gap-2">
                  <span className={clsx(
                    "px-3 py-1 rounded-full text-xs font-bold border",
                    getRiskColor("Medium")
                  )}>
                    AI Proposal
                  </span>
                  {proposal.status === "Active" && (
                    <span className="flex items-center gap-1 text-xs text-orange-400 bg-orange-400/10 px-2 py-1 rounded">
                      <Clock size={12} /> Active
                    </span>
                  )}
                </div>
              </div>

              <p className="text-sm text-gray-400 leading-relaxed max-w-3xl mb-4">
                {proposal.description}
              </p>

              <div className="bg-[#0f0f13] border border-[#2a2a35] rounded-lg p-4 mt-2">
                <div className="flex justify-between items-center mb-3">
                  <div className="text-sm font-semibold text-gray-300">
                    Human Authorization Matrix
                  </div>
                  <div className="text-xs text-gray-500">
                    {proposal.ayes || 0} / 5 Votes Reached
                  </div>
                </div>

                <div className="w-full bg-[#15151b] rounded-full h-2.5 mb-4 overflow-hidden border border-[#2a2a35]">
                  <div
                    className={clsx(
                      "h-full transition-all duration-500 ease-out",
                      proposal.status === "Passed" ? "bg-green-500" : "bg-purple-500 shadow-[0_0_10px_rgba(168,85,247,0.5)]"
                    )}
                    style={{ width: `${Math.min(((proposal.ayes || 0) / 5) * 100, 100)}%` }}
                  ></div>
                </div>

                {proposal.status === "Active" ? (
                  <div className="flex gap-3">
                    <button
                      onClick={() => handleVote(proposal.id, "yes")}
                      className="flex-1 flex justify-center items-center gap-2 bg-gradient-to-r from-green-600/20 to-green-500/10 hover:from-green-500/30 hover:to-green-400/20 border border-green-500/50 text-green-400 py-2 rounded-md font-bold text-sm transition-all"
                    >
                      <CheckCircle size={16} /> Approve
                    </button>
                    <button
                      onClick={() => handleVote(proposal.id, "no")}
                      className="flex-1 flex justify-center items-center gap-2 bg-gradient-to-r from-red-600/20 to-red-500/10 hover:from-red-500/30 hover:to-red-400/20 border border-red-500/50 text-red-400 py-2 rounded-md font-bold text-sm transition-all"
                    >
                      <XCircle size={16} /> Reject
                    </button>
                  </div>
                ) : (
                  <div className="flex justify-center items-center py-2 bg-[#1c2a20] border border-green-900 rounded-md text-green-400 text-sm font-bold gap-2">
                    <CheckCircle size={16} /> Execution Authorized. Sending to Agent Swarm...
                  </div>
                )}
              </div>
            </div>
          ))
        ) : (
          <div className="text-center py-8 text-gray-500">
            {isLoading ? "Loading AI proposals..." : "No AI proposals pending approval"}
          </div>
        )}
      </div>
    </div>
  );
}
