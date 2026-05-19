import React, { useState } from "react";
import { Music, TrendingUp, Zap, Eye, Download, Play, Volume2 } from "lucide-react";
import clsx from "clsx";

interface StreamContent {
  id: string;
  title: string;
  artist: string;
  type: "music" | "video" | "podcast";
  streams: number;
  earnings: number;
  duration: number;
  releaseDate: string;
  image: string;
}

interface CreatorProfile {
  id: string;
  name: string;
  followers: number;
  totalStreams: number;
  totalEarnings: number;
  contentCount: number;
  verified: boolean;
}

interface MicropaymentTransaction {
  id: string;
  content: string;
  listener: string;
  amount: number;
  timestamp: string;
  type: "stream" | "download" | "tip";
}

const MOCK_CONTENT: StreamContent[] = [
  {
    id: "1",
    title: "Quantum Horizons",
    artist: "Luna Echo",
    type: "music",
    streams: 125434,
    earnings: 4358,
    duration: 243,
    releaseDate: "2024-03-15",
    image: "🎵",
  },
  {
    id: "2",
    title: "Blockchain Dreams",
    artist: "Cosmic Waves",
    type: "music",
    streams: 89234,
    earnings: 3101,
    duration: 278,
    releaseDate: "2024-03-10",
    image: "🎵",
  },
  {
    id: "3",
    title: "Web3 Documentary",
    artist: "Tech Insights",
    type: "video",
    streams: 45123,
    earnings: 2256,
    duration: 1845,
    releaseDate: "2024-03-01",
    image: "🎬",
  },
];

const MOCK_CREATORS: CreatorProfile[] = [
  {
    id: "1",
    name: "Luna Echo",
    followers: 28456,
    totalStreams: 2834567,
    totalEarnings: 98450,
    contentCount: 34,
    verified: true,
  },
  {
    id: "2",
    name: "Cosmic Waves",
    followers: 15234,
    totalStreams: 1234567,
    totalEarnings: 42850,
    contentCount: 28,
    verified: true,
  },
];

const MOCK_TRANSACTIONS: MicropaymentTransaction[] = [
  { id: "1", content: "Quantum Horizons", listener: "0xListen...f123", amount: 0.025, timestamp: "2 mins ago", type: "stream" },
  { id: "2", content: "Blockchain Dreams", listener: "0xListen...a456", amount: 0.05, timestamp: "8 mins ago", type: "download" },
  { id: "3", content: "Quantum Horizons", listener: "0xListen...b789", amount: 1.0, timestamp: "15 mins ago", type: "tip" },
];

export default function MediaStreamingPanel() {
  const [content] = useState<StreamContent[]>(MOCK_CONTENT);
  const [creators] = useState<CreatorProfile[]>(MOCK_CREATORS);
  const [transactions] = useState<MicropaymentTransaction[]>(MOCK_TRANSACTIONS);
  const [activeTab, setActiveTab] = useState<"content" | "creators" | "transactions">("content");
  const [selectedContent, setSelectedContent] = useState<StreamContent | null>(content[0]);

  const totalStreams = content.reduce((sum, c) => sum + c.streams, 0);
  const totalEarnings = content.reduce((sum, c) => sum + c.earnings, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Music size={20} className="text-red-400" /> Media Streaming
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Streams</div>
            <div className="text-lg font-bold text-cyan-400">{(totalStreams / 1000).toFixed(0)}K</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Creator Earnings</div>
            <div className="text-lg font-bold text-green-400">${(totalEarnings / 1000).toFixed(1)}K</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active Creators</div>
            <div className="text-lg font-bold text-purple-400">{creators.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Content Pieces</div>
            <div className="text-lg font-bold text-orange-400">{content.length}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["content", "creators", "transactions"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-red-600 text-red-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Content Tab */}
        {activeTab === "content" && (
          <div className="space-y-2">
            {content.map((item) => (
              <div
                key={item.id}
                onClick={() => setSelectedContent(item)}
                className={clsx("bg-[#15151b] border rounded-lg p-3 cursor-pointer transition hover:border-red-600/50", selectedContent?.id === item.id && "border-red-600")}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <div className="text-3xl">{item.image}</div>
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <div className="font-semibold">{item.title}</div>
                        <span className={clsx("text-xs px-1.5 py-0.5 rounded font-bold", item.type === "music" ? "bg-red-600/20 text-red-400" : item.type === "video" ? "bg-purple-600/20 text-purple-400" : "bg-cyan-600/20 text-cyan-400")}>
                          {item.type}
                        </span>
                      </div>
                      <div className="text-xs text-gray-400">{item.artist}</div>
                    </div>
                  </div>
                  <Play size={16} className="text-red-400 flex-shrink-0" />
                </div>

                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div>
                    <div className="text-gray-400">Streams</div>
                    <div className="font-bold text-cyan-400">{(item.streams / 1000).toFixed(0)}K</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Creator Earnings</div>
                    <div className="font-bold text-green-400">${item.earnings.toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Duration</div>
                    <div className="font-bold text-orange-400">{Math.floor(item.duration / 60)}m {item.duration % 60}s</div>
                  </div>
                </div>

                <div className="mt-2 flex gap-2">
                  <button className="flex-1 bg-red-600/20 text-red-400 text-xs font-semibold py-1 rounded hover:bg-red-600/30 flex items-center justify-center gap-1">
                    <Play size={12} /> Play
                  </button>
                  <button className="flex-1 bg-purple-600/20 text-purple-400 text-xs font-semibold py-1 rounded hover:bg-purple-600/30">Support</button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Creators Tab */}
        {activeTab === "creators" && (
          <div className="space-y-2">
            {creators.map((creator) => (
              <div key={creator.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 hover:border-red-600/50 cursor-pointer transition">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <div className="font-semibold">{creator.name}</div>
                      {creator.verified && <span className="text-xs px-1.5 py-0.5 bg-blue-600/20 text-blue-400 rounded font-bold">✓</span>}
                    </div>
                    <div className="text-xs text-gray-400">{creator.followers.toLocaleString()} followers</div>
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-2 mb-3 text-xs pb-3 border-b border-[#2a2a35]">
                  <div>
                    <div className="text-gray-400">Total Streams</div>
                    <div className="font-bold text-cyan-400">{(creator.totalStreams / 1000000).toFixed(1)}M</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Total Earnings</div>
                    <div className="font-bold text-green-400">${(creator.totalEarnings / 1000).toFixed(1)}K</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Content Count</div>
                    <div className="font-bold text-purple-400">{creator.contentCount}</div>
                  </div>
                </div>

                <button className="w-full bg-red-600/20 text-red-400 text-sm font-semibold py-2 rounded hover:bg-red-600/30">View Profile</button>
              </div>
            ))}
          </div>
        )}

        {/* Transactions Tab */}
        {activeTab === "transactions" && (
          <div className="space-y-2">
            {transactions.map((tx) => (
              <div key={tx.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{tx.content}</div>
                    <div className="text-xs text-gray-400">{tx.timestamp}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-cyan-400">{tx.amount.toFixed(3)} X3</div>
                    <span
                      className={clsx(
                        "text-xs px-1.5 py-0.5 rounded font-bold",
                        tx.type === "stream" && "bg-red-600/20 text-red-400",
                        tx.type === "download" && "bg-blue-600/20 text-blue-400",
                        tx.type === "tip" && "bg-yellow-600/20 text-yellow-400"
                      )}
                    >
                      {tx.type}
                    </span>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 text-xs">
                  <div>
                    <div className="text-gray-400">Listener</div>
                    <div className="font-mono text-gray-500 text-xs">{tx.listener}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Micropayment</div>
                    <div className="font-bold text-green-400">${(tx.amount * 12.5).toFixed(2)}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Decentralized streaming, creator royalties, micropayments, and content monetization.
      </div>
    </div>
  );
}
