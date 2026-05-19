import React, { useState } from "react";
import { Users, MessageSquare, Zap, TrendingUp, Plus, Trash2, Shield, Eye } from "lucide-react";
import clsx from "clsx";

interface Community {
  id: string;
  name: string;
  description: string;
  members: number;
  posts: number;
  icon: string;
  verified: boolean;
  trending: boolean;
}

interface Post {
  id: string;
  author: string;
  content: string;
  likes: number;
  comments: number;
  timestamp: string;
  likes_by_user: boolean;
}

interface Moderator {
  id: string;
  name: string;
  role: "owner" | "moderator" | "helper";
  joinedDate: string;
  actions: number;
}

interface CommunityRules {
  id: string;
  title: string;
  description: string;
}

const MOCK_COMMUNITIES: Community[] = [
  {
    id: "1",
    name: "X3-Development",
    description: "For developers building on X3-Chain",
    members: 2841,
    posts: 15420,
    icon: "🔧",
    verified: true,
    trending: true,
  },
  {
    id: "2",
    name: "Token-Economics",
    description: "Discuss X3 economics and tokenomics",
    members: 1523,
    posts: 8342,
    icon: "📊",
    verified: true,
    trending: false,
  },
  {
    id: "3",
    name: "Trading-Strategies",
    description: "Share trading insights and strategies",
    members: 892,
    posts: 4521,
    icon: "📈",
    verified: false,
    trending: true,
  },
];

const MOCK_POSTS: Post[] = [
  {
    id: "1",
    author: "Alice",
    content: "Just deployed my first X3 smart contract! Works perfectly on testnet.",
    likes: 142,
    comments: 23,
    timestamp: "2 hours ago",
    likes_by_user: false,
  },
  {
    id: "2",
    author: "Bob",
    content: "The new SDK is absolutely amazing. Development velocity increased 10x!",
    likes: 89,
    comments: 12,
    timestamp: "4 hours ago",
    likes_by_user: true,
  },
  {
    id: "3",
    author: "Carol",
    content: "Looking for collaborators on a DeFi protocol. Reply if interested!",
    likes: 76,
    comments: 34,
    timestamp: "6 hours ago",
    likes_by_user: false,
  },
];

const MOCK_MODS: Moderator[] = [
  {
    id: "1",
    name: "Alice",
    role: "owner",
    joinedDate: "2024-01-01",
    actions: 234,
  },
  {
    id: "2",
    name: "Bob",
    role: "moderator",
    joinedDate: "2024-02-15",
    actions: 156,
  },
  {
    id: "3",
    name: "Carol",
    role: "helper",
    joinedDate: "2024-03-20",
    actions: 78,
  },
];

const MOCK_RULES: CommunityRules[] = [
  {
    id: "1",
    title: "Be Respectful",
    description: "Treat others with dignity and respect. No harassment or hate speech.",
  },
  {
    id: "2",
    title: "Stay On Topic",
    description: "Keep discussions relevant to X3 development and tokenomics.",
  },
  {
    id: "3",
    title: "No Spam",
    description: "Don't post promotional content, scams, or irrelevant links.",
  },
];

export default function CommunitiesPanel() {
  const [communities, setCommunities] = useState<Community[]>(MOCK_COMMUNITIES);
  const [selectedCommunity, setSelectedCommunity] = useState<Community | null>(MOCK_COMMUNITIES[0]);
  const [posts, setPosts] = useState<Post[]>(MOCK_POSTS);
  const [mods, setMods] = useState<Moderator[]>(MOCK_MODS);
  const [newPostContent, setNewPostContent] = useState("");
  const [activeTab, setActiveTab] = useState<"feed" | "moderators" | "rules">("feed");

  const handleLike = (postId: string) => {
    setPosts(
      posts.map((p) =>
        p.id === postId
          ? { ...p, likes: p.likes_by_user ? p.likes - 1 : p.likes + 1, likes_by_user: !p.likes_by_user }
          : p
      )
    );
  };

  const handlePostMessage = () => {
    if (newPostContent.trim()) {
      const newPost: Post = {
        id: (posts.length + 1).toString(),
        author: "You",
        content: newPostContent,
        likes: 0,
        comments: 0,
        timestamp: "just now",
        likes_by_user: false,
      };
      setPosts([newPost, ...posts]);
      setNewPostContent("");
    }
  };

  const totalMembers = communities.reduce((sum, c) => sum + c.members, 0);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Users size={20} className="text-purple-400" /> Communities
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Communities</div>
            <div className="text-lg font-bold text-purple-400">{communities.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Members</div>
            <div className="text-lg font-bold text-cyan-400">{(totalMembers / 1000).toFixed(1)}k</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Posts</div>
            <div className="text-lg font-bold text-yellow-400">{communities.reduce((s, c) => s + c.posts, 0)}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Trending</div>
            <div className="text-lg font-bold text-green-400">{communities.filter((c) => c.trending).length}</div>
          </div>
        </div>

        {/* Community List + Feed Layout */}
        <div className="flex gap-3 h-96">
          {/* Community Sidebar */}
          <div className="w-40 flex flex-col space-y-2 border-r border-[#2a2a35] pr-3 overflow-y-auto">
            {communities.map((comm) => (
              <button
                key={comm.id}
                onClick={() => setSelectedCommunity(comm)}
                className={clsx(
                  "text-left p-2 rounded-lg border transition text-sm",
                  selectedCommunity?.id === comm.id
                    ? "border-purple-600 bg-purple-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-lg">{comm.icon}</span>
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold text-xs truncate">{comm.name}</div>
                    {comm.verified && <Shield size={10} className="text-green-400 mt-0.5" />}
                  </div>
                </div>
                <div className="text-xs text-gray-400">{(comm.members / 1000).toFixed(1)}k members</div>
                {comm.trending && <div className="text-xs text-yellow-400 font-semibold flex items-center gap-1 mt-1">
                  <TrendingUp size={10} /> Trending
                </div>}
              </button>
            ))}
          </div>

          {/* Main Feed */}
          {selectedCommunity && (
            <div className="flex-1 flex flex-col">
              {/* Community Header */}
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 mb-3">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="text-2xl">{selectedCommunity.icon}</span>
                    <div>
                      <div className="font-bold">{selectedCommunity.name}</div>
                      <div className="text-xs text-gray-400">{selectedCommunity.description}</div>
                    </div>
                  </div>
                  {selectedCommunity.verified && (
                    <Shield size={16} className="text-green-400" />
                  )}
                </div>
                <div className="flex gap-3 text-xs">
                  <span className="text-gray-400">{selectedCommunity.members} members</span>
                  <span className="text-gray-400">{selectedCommunity.posts} posts</span>
                </div>
              </div>

              {/* Tabs */}
              <div className="flex gap-2 border-b border-[#2a2a35] mb-3">
                {(["feed", "moderators", "rules"] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveTab(tab)}
                    className={clsx(
                      "px-3 py-1 text-sm font-semibold transition border-b-2",
                      activeTab === tab
                        ? "border-purple-600 text-purple-400"
                        : "border-transparent text-gray-400 hover:text-gray-300"
                    )}
                  >
                    {tab === "feed" ? "Posts" : tab === "moderators" ? "Mods" : "Rules"}
                  </button>
                ))}
              </div>

              {/* Tab Content */}
              {activeTab === "feed" && (
                <div className="flex-1 overflow-y-auto space-y-2">
                  {/* Post Input */}
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-2 mb-2">
                    <textarea
                      value={newPostContent}
                      onChange={(e) => setNewPostContent(e.target.value)}
                      placeholder="Share your thoughts..."
                      className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded px-2 py-2 text-xs focus:border-purple-600 focus:outline-none resize-none"
                      rows={2}
                    />
                    <button
                      onClick={handlePostMessage}
                      className="mt-1 bg-purple-600 hover:bg-purple-700 px-3 py-1 rounded text-xs font-semibold transition w-full"
                    >
                      Post
                    </button>
                  </div>

                  {/* Posts */}
                  {posts.map((post) => (
                    <div key={post.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-2">
                      <div className="flex justify-between mb-1">
                        <div className="font-semibold text-xs">{post.author}</div>
                        <div className="text-xs text-gray-500">{post.timestamp}</div>
                      </div>
                      <div className="text-xs text-gray-300 mb-2">{post.content}</div>
                      <div className="flex gap-3 text-xs">
                        <button
                          onClick={() => handleLike(post.id)}
                          className={clsx(
                            "flex items-center gap-1 transition",
                            post.likes_by_user ? "text-pink-400" : "text-gray-400 hover:text-pink-400"
                          )}
                        >
                          ♥ {post.likes}
                        </button>
                        <button className="flex items-center gap-1 text-gray-400 hover:text-cyan-400 transition">
                          <MessageSquare size={12} /> {post.comments}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "moderators" && (
                <div className="flex-1 overflow-y-auto space-y-2">
                  {mods.map((mod) => (
                    <div key={mod.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-2">
                      <div className="flex justify-between items-start">
                        <div>
                          <div className="flex items-center gap-2 mb-1">
                            <div className="font-semibold text-xs">{mod.name}</div>
                            <span
                              className={clsx(
                                "text-xs px-1.5 py-0.5 rounded font-semibold",
                                mod.role === "owner"
                                  ? "bg-red-600/20 text-red-400"
                                  : mod.role === "moderator"
                                    ? "bg-purple-600/20 text-purple-400"
                                    : "bg-blue-600/20 text-blue-400"
                              )}
                            >
                              {mod.role}
                            </span>
                          </div>
                          <div className="text-xs text-gray-400">Joined {mod.joinedDate}</div>
                          <div className="text-xs text-gray-400 mt-0.5">{mod.actions} actions</div>
                        </div>
                        <Shield size={16} className={clsx(mod.role === "owner" ? "text-red-400" : "text-purple-400")} />
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {activeTab === "rules" && (
                <div className="flex-1 overflow-y-auto space-y-2">
                  {MOCK_RULES.map((rule) => (
                    <div key={rule.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                      <div className="font-semibold text-xs mb-1">{rule.title}</div>
                      <div className="text-xs text-gray-400">{rule.description}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        SubReddit-style communities with topic-based feeds, moderators, and community rules.
      </div>
    </div>
  );
}
