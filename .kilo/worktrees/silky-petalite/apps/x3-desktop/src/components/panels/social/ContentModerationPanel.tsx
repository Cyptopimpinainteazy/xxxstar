import React, { useState } from "react";
import { MessageSquare, Flag, CheckCircle, Trash2, Users, TrendingUp } from "lucide-react";
import clsx from "clsx";

interface Content {
  id: string;
  author: string;
  type: "post" | "comment" | "message";
  content: string;
  flagReason: string;
  flags: number;
  votes: { support: number; remove: number };
  status: "pending" | "approved" | "removed";
  timestamp: string;
}

const MOCK_CONTENT: Content[] = [
  {
    id: "1",
    author: "0x1234...5678",
    type: "post",
    content: "Check out my new trading bot strategy...",
    flagReason: "Suspected spam/scam",
    flags: 12,
    votes: { support: 8, remove: 28 },
    status: "pending",
    timestamp: "2 hours ago",
  },
  {
    id: "2",
    author: "0x8765...4321",
    type: "comment",
    content: "Great analysis on the market trends...",
    flagReason: "None",
    flags: 0,
    votes: { support: 45, remove: 2 },
    status: "approved",
    timestamp: "4 hours ago",
  },
  {
    id: "3",
    author: "0xabcd...efgh",
    type: "post",
    content: "Offensive language in response to governance vote",
    flagReason: "Harassment",
    flags: 18,
    votes: { support: 3, remove: 42 },
    status: "pending",
    timestamp: "6 hours ago",
  },
];

export default function ContentModerationPanel() {
  const [content, setContent] = useState<Content[]>(MOCK_CONTENT);
  const [selectedContent, setSelectedContent] = useState<Content | null>(MOCK_CONTENT[0]);
  const [filterStatus, setFilterStatus] = useState<"all" | "pending" | "approved" | "removed">("pending");
  const [userVote, setUserVote] = useState<{ [key: string]: "support" | "remove" | null }>({});

  const pendingCount = content.filter((c) => c.status === "pending").length;
  const approvedCount = content.filter((c) => c.status === "approved").length;
  const removedCount = content.filter((c) => c.status === "removed").length;
  const totalFlags = content.reduce((sum, c) => sum + c.flags, 0);

  const filteredContent = content.filter((c) => filterStatus === "all" || c.status === filterStatus);

  const handleVote = (contentId: string, direction: "support" | "remove") => {
    setUserVote({ ...userVote, [contentId]: direction });

    setContent(
      content.map((c) => {
        if (c.id === contentId) {
          const totalVotes = c.votes.support + c.votes.remove;
          const removeThreshold = Math.ceil(totalVotes * 0.66); // 2/3 majority

          return {
            ...c,
            votes: {
              support: direction === "support" ? c.votes.support + 1 : c.votes.support,
              remove: direction === "remove" ? c.votes.remove + 1 : c.votes.remove,
            },
            status:
              direction === "remove" && c.votes.remove + 1 >= removeThreshold ? "removed" : c.status,
          };
        }
        return c;
      })
    );
  };

  const handleApprove = (contentId: string) => {
    setContent(content.map((c) => (c.id === contentId ? { ...c, status: "approved" } : c)));
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <MessageSquare size={20} className="text-purple-400" /> Content Moderation
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview Stats */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Pending</div>
            <div className="text-lg font-bold text-orange-400">{pendingCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Approved</div>
            <div className="text-lg font-bold text-green-400">{approvedCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Removed</div>
            <div className="text-lg font-bold text-red-400">{removedCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Flags</div>
            <div className="text-lg font-bold text-yellow-400">{totalFlags}</div>
          </div>
        </div>

        {/* Filter Tabs */}
        <div className="flex gap-2">
          {(["all", "pending", "approved", "removed"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setFilterStatus(tab)}
              className={clsx(
                "px-3 py-1 rounded text-xs font-semibold transition",
                filterStatus === tab
                  ? "bg-purple-600 text-white"
                  : "bg-[#15151b] text-gray-400 hover:bg-[#2a2a35]"
              )}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
          ))}
        </div>

        {/* Content Queue */}
        <div className="space-y-2">
          {filteredContent.map((item) => (
            <button
              key={item.id}
              onClick={() => setSelectedContent(item)}
              className={clsx(
                "w-full text-left p-3 rounded-lg border-2 transition",
                selectedContent?.id === item.id
                  ? "border-purple-600 bg-purple-600/10"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              <div className="flex items-start justify-between mb-2">
                <div>
                  <div className="text-sm font-semibold flex items-center gap-2">
                    {item.type === "post" && "📄"}
                    {item.type === "comment" && "💬"}
                    {item.type === "message" && "✉️"}
                    {item.author}
                  </div>
                  <div className="text-xs text-gray-400">{item.timestamp}</div>
                </div>
                <span
                  className={clsx(
                    "text-xs px-2 py-1 rounded border",
                    item.status === "pending"
                      ? "bg-orange-600/30 border-orange-600 text-orange-400"
                      : item.status === "approved"
                      ? "bg-green-600/30 border-green-600 text-green-400"
                      : "bg-red-600/30 border-red-600 text-red-400"
                  )}
                >
                  {item.status}
                </span>
              </div>

              <div className="text-sm text-gray-300 mb-2">{item.content.substring(0, 80)}...</div>

              <div className="flex items-center gap-3 text-xs text-gray-400">
                {item.flags > 0 && (
                  <span className="flex items-center gap-1">
                    <Flag size={12} className="text-red-400" /> {item.flags} flags
                  </span>
                )}
                <span className="flex items-center gap-1">
                  ✓ {item.votes.support} • ✗ {item.votes.remove}
                </span>
              </div>
            </button>
          ))}
        </div>

        {/* Selected Content Details */}
        {selectedContent && (
          <>
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 text-sm">Content Details</h3>

              <div className="space-y-3 text-sm">
                <div>
                  <div className="text-gray-400 mb-1">Author</div>
                  <div className="font-mono text-xs">{selectedContent.author}</div>
                </div>

                <div>
                  <div className="text-gray-400 mb-1">Content</div>
                  <div className="bg-[#2a2a35] p-2 rounded text-xs leading-relaxed">
                    {selectedContent.content}
                  </div>
                </div>

                {selectedContent.flags > 0 && (
                  <div>
                    <div className="text-gray-400 mb-1">Flag Reason</div>
                    <div className="text-sm text-red-400">{selectedContent.flagReason}</div>
                  </div>
                )}

                <div className="bg-[#2a2a35] p-3 rounded">
                  <div className="flex justify-between mb-2">
                    <span>Keep Content</span>
                    <span className="font-bold text-green-400">{selectedContent.votes.support}</span>
                  </div>
                  <div className="flex-1 bg-[#1a1a1f] rounded-full h-2 overflow-hidden mb-3">
                    <div
                      className="h-full bg-green-600"
                      style={{
                        width: `${(selectedContent.votes.support / (selectedContent.votes.support + selectedContent.votes.remove)) * 100}%`,
                      }}
                    />
                  </div>

                  <div className="flex justify-between">
                    <span>Remove Content</span>
                    <span className="font-bold text-red-400">{selectedContent.votes.remove}</span>
                  </div>
                  <div className="flex-1 bg-[#1a1a1f] rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-red-600"
                      style={{
                        width: `${(selectedContent.votes.remove / (selectedContent.votes.support + selectedContent.votes.remove)) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              </div>
            </div>

            {/* Community Vote */}
            {selectedContent.status === "pending" && (
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <h3 className="font-semibold mb-3 text-sm">Cast Your Moderation Vote</h3>

                {userVote[selectedContent.id] ? (
                  <div className="flex items-center gap-2 p-3 bg-blue-600/20 border border-blue-600 rounded-lg">
                    <CheckCircle size={16} className="text-blue-400" />
                    <div className="text-sm font-semibold">
                      Voted to {userVote[selectedContent.id] === "support" ? "Keep" : "Remove"}
                    </div>
                  </div>
                ) : (
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleVote(selectedContent.id, "support")}
                      className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition"
                    >
                      ✓ Keep Content
                    </button>
                    <button
                      onClick={() => handleVote(selectedContent.id, "remove")}
                      className="flex-1 bg-red-600 hover:bg-red-700 py-2 rounded-lg font-semibold text-sm transition"
                    >
                      ✗ Remove Content
                    </button>
                  </div>
                )}

                <div className="mt-2 text-xs text-gray-400 text-center">
                  2/3 majority required to remove. Current: {selectedContent.votes.remove}/{Math.ceil((selectedContent.votes.support + selectedContent.votes.remove) * 0.66)}
                </div>
              </div>
            )}

            {selectedContent.status === "approved" && (
              <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
                View Similar Content
              </button>
            )}
          </>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Community-governed moderation ensures decentralized content standards.
      </div>
    </div>
  );
}
