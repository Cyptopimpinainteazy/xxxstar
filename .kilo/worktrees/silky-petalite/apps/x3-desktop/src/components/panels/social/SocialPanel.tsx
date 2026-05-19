import React, { useState, useEffect } from 'react';
import { Heart, Send, User, Lock, Shield, Zap, Users, TrendingUp } from 'lucide-react';
import clsx from 'clsx';

interface Post {
  id: string;
  author: string;
  avatar: string;
  content: string;
  timestamp: number;
  tips: number;
  isCreator: boolean;
  subscriptionPrice?: number;
  isSubscribed?: boolean;
  hasProofOfHuman?: boolean;
  nftProfile?: string;
}

interface SocialUser {
  username: string;
  subscriptionPrice: number;
  subscribers: number;
  totalTips: number;
  reputation: number;
  isVerified: boolean;
  nftProfileUri?: string;
  proofOfHumanVerified?: boolean;
}

const MOCK_POSTS: Post[] = [
  {
    id: '1',
    author: '@alice',
    avatar: '👤',
    content: 'Just deployed my new X3-Lang contract! Check it out on testnet 🚀',
    timestamp: Date.now() - 3600000,
    tips: 12,
    isCreator: true,
    subscriptionPrice: 5,
    isSubscribed: false,
    hasProofOfHuman: true,
    nftProfile: 'Bold NFT #123',
  },
  {
    id: '2',
    author: '@builder',
    avatar: '🛠️',
    content: 'X3 cross-chain limit orders hit mainnet. DEX volume already 2x 📈',
    timestamp: Date.now() - 7200000,
    tips: 45,
    isCreator: true,
    subscriptionPrice: 10,
    isSubscribed: false,
  },
  {
    id: '3',
    author: '@dev',
    avatar: '💻',
    content: 'Integrated X3 into my trading bot. Latency is insane! ⚡',
    timestamp: Date.now() - 86400000,
    tips: 8,
    isCreator: false,
  },
];

const SocialPanel: React.FC = () => {
  const [posts, setPosts] = useState<Post[]>(MOCK_POSTS);
  const [creatorMode, setCreatorMode] = useState(false);
  const [currentUser, setCurrentUser] = useState<SocialUser>({
    username: '@you',
    subscriptionPrice: 0,
    subscribers: 0,
    totalTips: 0,
    reputation: 85,
    isVerified: false,
    proofOfHumanVerified: false,
  });
  const [showCreatorModal, setShowCreatorModal] = useState(false);
  const [newPostContent, setNewPostContent] = useState('');
  const [showTipModal, setShowTipModal] = useState<string | null>(null);
  const [tipAmount, setTipAmount] = useState(1);
  const [showNftUpload, setShowNftUpload] = useState(false);
  const [showProofOfHuman, setShowProofOfHuman] = useState(false);

  const handleTip = (postId: string) => {
    setPosts(posts.map(p => 
      p.id === postId 
        ? { ...p, tips: p.tips + tipAmount }
        : p
    ));
    setShowTipModal(null);
    setTipAmount(1);
  };

  const handleCreatePost = () => {
    if (newPostContent.trim()) {
      const newPost: Post = {
        id: Math.random().toString(),
        author: currentUser.username,
        avatar: '💬',
        content: newPostContent,
        timestamp: Date.now(),
        tips: 0,
        isCreator: creatorMode,
        subscriptionPrice: creatorMode ? currentUser.subscriptionPrice : undefined,
        isSubscribed: true,
      };
      setPosts([newPost, ...posts]);
      setNewPostContent('');
    }
  };

  const handleSubscribe = (price: number) => {
    setCurrentUser({
      ...currentUser,
      subscribers: currentUser.subscribers + 1,
      totalTips: currentUser.totalTips + price,
    });
    alert(`Subscribed for ${price} X3/month`);
  };

  const handleVerification = () => {
    if (!currentUser.proofOfHumanVerified) {
      setShowProofOfHuman(true);
    }
  };

  const confirmProofOfHuman = () => {
    setCurrentUser({
      ...currentUser,
      proofOfHumanVerified: true,
      reputation: currentUser.reputation + 15,
    });
    setShowProofOfHuman(false);
  };

  const handleNftUpload = () => {
    setCurrentUser({
      ...currentUser,
      nftProfileUri: 'ipfs://QmXxxx...verified',
      reputation: currentUser.reputation + 10,
    });
    setShowNftUpload(false);
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Users size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">X3 Social</h1>
        </div>
        <button
          onClick={() => setCreatorMode(!creatorMode)}
          className={clsx(
            'flex items-center gap-2 px-4 py-2 rounded-lg font-semibold text-sm transition-all',
            creatorMode
              ? 'bg-gradient-to-r from-purple-500 to-pink-500 shadow-lg shadow-purple-500/20'
              : 'bg-[#111111] border border-[#1a1a1a] hover:border-blue-500/40'
          )}
        >
          {creatorMode ? '✓ Creator Mode' : 'Enable Creator'}
        </button>
      </div>

      {/* User Profile & Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4 border-b border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-lg p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
          <div className="text-xs text-gray-500">Reputation</div>
          <div className="text-lg font-bold text-green-400">{currentUser.reputation}</div>
        </div>
        {creatorMode && (
          <>
            <div className="bg-[#111111] rounded-lg p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
              <div className="text-xs text-gray-500">Subscribers</div>
              <div className="text-lg font-bold text-blue-400">{currentUser.subscribers}</div>
            </div>
            <div className="bg-[#111111] rounded-lg p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
              <div className="text-xs text-gray-500">Tips Earned</div>
              <div className="text-lg font-bold text-yellow-400">{currentUser.totalTips} X3</div>
            </div>
            <div className="bg-[#111111] rounded-lg p-3 border border-[#1a1a1a] hover:border-[#2a2a2a] transition">
              <div className="text-xs text-gray-500">Sub Price</div>
              <div className="text-lg font-bold text-pink-400">{currentUser.subscriptionPrice} X3/mo</div>
            </div>
          </>
        )}
      </div>

      {/* Verification & Profile Options */}
      {creatorMode && (
        <div className="flex gap-3 px-5 py-4 border-b border-[#1a1a1a]">
          <button
            onClick={handleVerification}
            className={clsx(
              'flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition-all',
              currentUser.proofOfHumanVerified
                ? 'bg-green-500/20 text-green-400 border border-green-500/40'
                : 'bg-[#111111] border border-[#1a1a1a] hover:border-blue-500/40 text-gray-400'
            )}
          >
            <Shield size={12} />
            {currentUser.proofOfHumanVerified ? 'Verified Human' : 'Verify Human'}
          </button>
          <button
            onClick={() => setShowNftUpload(!showNftUpload)}
            className={clsx(
              'flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition-all',
              currentUser.nftProfileUri
                ? 'bg-purple-500/20 text-purple-400 border border-purple-500/40'
                : 'bg-[#111111] border border-[#1a1a1a] hover:border-blue-500/40 text-gray-400'
            )}
          >
            🎨 {currentUser.nftProfileUri ? 'NFT Profile Set' : 'Set NFT Profile'}
          </button>
          <input
            type="number"
            min="0"
            max="100"
            value={currentUser.subscriptionPrice}
            onChange={(e) => setCurrentUser({ ...currentUser, subscriptionPrice: parseInt(e.target.value) || 0 })}
            placeholder="Price/mo (X3)"
            className="bg-[#111111] border border-[#1a1a1a] px-3 py-2 rounded-lg text-xs text-white placeholder-gray-600 outline-none focus:border-blue-500/40 w-24"
          />
        </div>
      )}

      {/* New Post Input */}
      <div className="px-5 py-4 border-b border-[#1a1a1a] bg-[#111111]">
        <textarea
          value={newPostContent}
          onChange={(e) => setNewPostContent(e.target.value)}
          placeholder="What's happening in X3?"
          className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 text-white text-sm placeholder-gray-600 outline-none focus:border-blue-500/40 resize-none h-20"
        />
        <div className="flex justify-end mt-2">
          <button
            onClick={handleCreatePost}
            disabled={!newPostContent.trim()}
            className="flex items-center gap-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 disabled:from-gray-600 disabled:to-gray-600 text-white px-4 py-2 rounded-lg font-semibold text-sm transition-all"
          >
            <Send size={14} /> Post
          </button>
        </div>
      </div>

      {/* Feed */}
      <div className="flex-1 overflow-auto px-5 py-4 space-y-3">
        {posts.map((post) => (
          <div key={post.id} className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 hover:border-[#2a2a2a] transition-colors">
            {/* Post Header */}
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <span className="text-2xl">{post.avatar}</span>
                <div>
                  <div className="font-semibold text-white">{post.author}</div>
                  <div className="text-xs text-gray-500">{Math.floor((Date.now() - post.timestamp) / 3600000)}h ago</div>
                </div>
                {post.hasProofOfHuman && <Shield size={12} className="text-green-400 ml-2" />}
                {post.nftProfile && <span className="text-xs bg-purple-500/20 text-purple-400 px-2 py-0.5 rounded">🎨 {post.nftProfile}</span>}
              </div>
              {post.isCreator && post.subscriptionPrice && (
                <button
                  onClick={() => handleSubscribe(post.subscriptionPrice!)}
                  className="flex items-center gap-1 bg-pink-500/20 hover:bg-pink-500/30 text-pink-400 px-2 py-1 rounded text-xs font-medium transition-colors"
                >
                  <Lock size={10} /> Subscribe {post.subscriptionPrice} X3/mo
                </button>
              )}
            </div>

            {/* Post Content */}
            <p className="text-white mb-3 text-sm">{post.content}</p>

            {/* Post Actions */}
            <div className="flex items-center justify-between pt-3 border-t border-[#1a1a1a]">
              <div className="flex items-center gap-6 text-xs text-gray-500">
                <button className="flex items-center gap-1 hover:text-blue-400 transition-colors">
                  <Zap size={12} /> {post.tips} tips
                </button>
              </div>
              <button
                onClick={() => setShowTipModal(post.id)}
                className="flex items-center gap-1 bg-yellow-500/20 hover:bg-yellow-500/30 text-yellow-400 px-3 py-1 rounded text-xs font-medium transition-colors"
              >
                <Heart size={12} /> Tip
              </button>

              {/* Tip Modal */}
              {showTipModal === post.id && (
                <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
                  <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-80 shadow-2xl">
                    <h3 className="font-bold text-white mb-4">Send Tip to {post.author}</h3>
                    <input
                      type="number"
                      min="0.1"
                      step="0.1"
                      value={tipAmount}
                      onChange={(e) => setTipAmount(parseFloat(e.target.value) || 1)}
                      className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-2 text-white mb-4 outline-none focus:border-blue-500/40"
                      placeholder="Amount (X3)"
                    />
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => setShowTipModal(null)}
                        className="px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors text-sm"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={() => handleTip(post.id)}
                        className="px-4 py-2 rounded-lg bg-gradient-to-r from-yellow-500 to-yellow-600 text-white font-semibold text-sm transition-all"
                      >
                        Send {tipAmount} X3
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* NFT Profile Upload Modal */}
      {showNftUpload && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
            <h3 className="font-bold text-white mb-4">Set NFT Profile Picture</h3>
            <p className="text-gray-400 text-sm mb-4">Upload or select an NFT from your wallet. Ownership will be verified on-chain.</p>
            <button
              onClick={handleNftUpload}
              className="w-full py-2 rounded-lg bg-gradient-to-r from-purple-500 to-purple-600 text-white font-semibold mb-2 transition-all"
            >
              Upload NFT
            </button>
            <button
              onClick={() => setShowNftUpload(false)}
              className="w-full py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Proof of Human Modal */}
      {showProofOfHuman && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
            <h3 className="font-bold text-white mb-4 flex items-center gap-2">
              <Shield size={18} className="text-green-400" />
              Verify Proof of Human
            </h3>
            <p className="text-gray-400 text-sm mb-4">Link your Worldcoin or Proof of Humanity credential to get the bot-proof badge.</p>
            <button
              onClick={confirmProofOfHuman}
              className="w-full py-2 rounded-lg bg-gradient-to-r from-green-500 to-green-600 text-white font-semibold mb-2 transition-all"
            >
              Connect Worldcoin
            </button>
            <button
              onClick={() => setShowProofOfHuman(false)}
              className="w-full py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default SocialPanel;

