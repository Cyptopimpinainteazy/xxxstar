import React, { useState } from 'react';
import { Share2, MessageCircle, Lock, Users, Link2, TrendingUp } from 'lucide-react';

interface ActivityFeed {
  id: string;
  author: string;
  action: string;
  content: string;
  timestamp: number;
  likes: number;
  replies: number;
  visibility: 'public' | 'private' | 'followers';
}

interface SocialCommunity {
  id: string;
  name: string;
  description: string;
  members: number;
  moderators: number;
  posts24h: number;
  threadsActive: number;
  visibility: 'public' | 'private';
}

interface MediaUpload {
  id: string;
  filename: string;
  contentType: string;
  ipfsHash: string;
  encryptionStatus: 'encrypted' | 'plaintext';
  size: number;
  uploader: string;
  uploadedAt: number;
  views: number;
}

interface E2eMessage {
  id: string;
  from: string;
  to: string;
  encryptionAlgorithm: string;
  status: 'sent' | 'delivered' | 'read';
  timestamp: number;
  decryptedPreview: string;
  mediaAttachments: number;
}

export const SocialBackendPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'activity' | 'communities' | 'media' | 'messages'>('activity');

  const [activityFeed] = useState<ActivityFeed[]>([
    {
      id: 'a1',
      author: 'Alice Chen',
      action: 'shared',
      content: 'Just launched our new X3 integration API - checkout the docs!',
      timestamp: Date.now() - 3600000,
      likes: 243,
      replies: 18,
      visibility: 'public',
    },
    {
      id: 'a2',
      author: 'Bob Martinez',
      action: 'posted',
      content: 'Building a cross-chain bridge with X3Network - AMA! 🚀',
      timestamp: Date.now() - 7200000,
      likes: 567,
      replies: 92,
      visibility: 'public',
    },
    {
      id: 'a3',
      author: 'Carol Patel',
      action: 'shared',
      content: 'Performance benchmark results up +240% on v2.0',
      timestamp: Date.now() - 10800000,
      likes: 456,
      replies: 34,
      visibility: 'followers',
    },
  ]);

  const [communities] = useState<SocialCommunity[]>([
    {
      id: 'c1',
      name: 'X3 Developers',
      description: 'Official community for blockchain developers using X3Network',
      members: 8540,
      moderators: 12,
      posts24h: 340,
      threadsActive: 42,
      visibility: 'public',
    },
    {
      id: 'c2',
      name: 'X3 Validators',
      description: 'Private community for network validators and infrastructure operators',
      members: 2340,
      moderators: 8,
      posts24h: 125,
      threadsActive: 18,
      visibility: 'private',
    },
    {
      id: 'c3',
      name: 'X3 Enterprise',
      description: 'Enterprise adoption and integration discussions',
      members: 1820,
      moderators: 6,
      posts24h: 89,
      threadsActive: 14,
      visibility: 'private',
    },
  ]);

  const [mediaLibrary] = useState<MediaUpload[]>([
    {
      id: 'm1',
      filename: 'X3-whitepaper-2.0.pdf',
      contentType: 'application/pdf',
      ipfsHash: 'QmXrKn8zKLk4N9F2mP3Q4r5S6T7U8V9W0X1Y2Z3a4B5c',
      encryptionStatus: 'plaintext',
      size: 2.4 * 1024 * 1024,
      uploader: 'Technical Team',
      uploadedAt: Date.now() - 86400000 * 5,
      views: 3240,
    },
    {
      id: 'm2',
      filename: 'Demo-Trading-Video.mp4',
      contentType: 'video/mp4',
      ipfsHash: 'QmYsK9L0M1N2O3P4q5R6S7T8u9V0W1X2Y3Z4a5B6c7D',
      encryptionStatus: 'encrypted',
      size: 450 * 1024 * 1024,
      uploader: 'Marketing Team',
      uploadedAt: Date.now() - 86400000 * 2,
      views: 8450,
    },
    {
      id: 'm3',
      filename: 'Privacy-Standards-Doc.docx',
      contentType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      ipfsHash: 'QmZtL5M6N7O8P9q0R1S2T3u4V5W6X7Y8Z9a0B1c2D3e',
      encryptionStatus: 'encrypted',
      size: 3.2 * 1024 * 1024,
      uploader: 'Security Team',
      uploadedAt: Date.now() - 86400000 * 1,
      views: 450,
    },
  ]);

  const [e2eMessages] = useState<E2eMessage[]>([
    {
      id: 'msg1',
      from: 'Alice Chen',
      to: 'Bob Martinez',
      encryptionAlgorithm: 'X3DH + ChaCha20-Poly1305',
      status: 'read',
      timestamp: Date.now() - 3600000,
      decryptedPreview: 'Let\'s discuss the integration tomorrow...',
      mediaAttachments: 0,
    },
    {
      id: 'msg2',
      from: 'Carol Patel',
      to: 'Security Team',
      encryptionAlgorithm: 'X3DH + ChaCha20-Poly1305',
      status: 'delivered',
      timestamp: Date.now() - 7200000,
      decryptedPreview: 'Audit report attached - please review...',
      mediaAttachments: 1,
    },
    {
      id: 'msg3',
      from: 'DevOps Lead',
      to: 'Validators Group',
      encryptionAlgorithm: 'X3DH + ChaCha20-Poly1305',
      status: 'sent',
      timestamp: Date.now() - 1800000,
      decryptedPreview: 'Scheduled maintenance window: 2026-03-05 02:00 UTC',
      mediaAttachments: 0,
    },
  ]);

  const totalMembers = communities.reduce((sum, c) => sum + c.members, 0);
  const totalPosts24h = communities.reduce((sum, c) => sum + c.posts24h, 0);
  const totalMediaStorage = mediaLibrary.reduce((sum, m) => sum + m.size, 0);
  const totalMediaViews = mediaLibrary.reduce((sum, m) => sum + m.views, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-pink-400 to-rose-500 mb-2">
              Social Backend
            </h1>
            <p className="text-gray-400">ActivityPub Federation • E2E Encryption • IPFS Media • Communities</p>
          </div>
          <Share2 className="w-12 h-12 text-pink-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Total Community Members</div>
            <div className="text-2xl font-bold text-pink-400">{(totalMembers / 1000).toFixed(1)}K</div>
            <div className="text-xs text-gray-500 mt-2">Across 3 communities</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Posts (24h)</div>
            <div className="text-2xl font-bold text-purple-400">{totalPosts24h}</div>
            <div className="text-xs text-gray-500 mt-2">Active engagement</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Media Storage</div>
            <div className="text-2xl font-bold text-blue-400">{(totalMediaStorage / (1024 * 1024 * 1024)).toFixed(1)}GB</div>
            <div className="text-xs text-gray-500 mt-2">IPFS pinned</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">E2E Messages</div>
            <div className="text-2xl font-bold text-green-400">12.4K</div>
            <div className="text-xs text-gray-500 mt-2">End-to-end encrypted</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['activity', 'communities', 'media', 'messages'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-pink-400 border-b-2 border-pink-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'activity' && 'Activity Feed'}
              {tab === 'communities' && 'Communities'}
              {tab === 'media' && 'Media Library'}
              {tab === 'messages' && 'E2E Messages'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'activity' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Activity Feed (ActivityPub)</h3>
              <div className="space-y-4">
                {activityFeed.map((post) => (
                  <div key={post.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{post.author}</h4>
                        <p className="text-xs text-gray-400">{post.action}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          post.visibility === 'public'
                            ? 'bg-blue-500/20 text-blue-400'
                            : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {post.visibility.toUpperCase()}
                      </div>
                    </div>
                    <p className="text-gray-200 text-sm mb-3">{post.content}</p>
                    <div className="flex items-center gap-6 text-xs text-gray-400">
                      <span>{Math.round((Date.now() - post.timestamp) / 3600000)}h ago</span>
                      <div className="flex items-center gap-1">
                        <span>❤️ {post.likes}</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <MessageCircle className="w-3 h-3" /> {post.replies}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'communities' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Communities</h3>
              <div className="space-y-4">
                {communities.map((community) => (
                  <div key={community.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{community.name}</h4>
                        <p className="text-sm text-gray-400">{community.description}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          community.visibility === 'public'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-red-500/20 text-red-400'
                        }`}
                      >
                        {community.visibility.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Members</div>
                        <div className="text-white font-semibold">{(community.members / 1000).toFixed(1)}K</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Moderators</div>
                        <div className="text-white font-semibold">{community.moderators}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Posts (24h)</div>
                        <div className="text-blue-400 font-semibold">{community.posts24h}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Active Threads</div>
                        <div className="text-purple-400 font-semibold">{community.threadsActive}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Growth</div>
                        <div className="text-green-400 font-semibold">+12.5%</div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'media' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Media Library (IPFS)</h3>
              <div className="space-y-4">
                {mediaLibrary.map((media) => (
                  <div key={media.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <h4 className="text-white font-semibold">{media.filename}</h4>
                        <p className="text-xs text-gray-500 font-mono">{media.ipfsHash}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          media.encryptionStatus === 'encrypted'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {media.encryptionStatus === 'encrypted' ? (
                          <>
                            <Lock className="w-3 h-3 inline mr-1" /> ENCRYPTED
                          </>
                        ) : (
                          'PUBLIC'
                        )}
                      </div>
                    </div>
                    <div className="grid grid-cols-5 gap-4 text-sm">
                      <div>
                        <div className="text-gray-400">Type</div>
                        <div className="text-white font-semibold text-xs">{media.contentType.split('/')[1]}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Size</div>
                        <div className="text-white font-semibold">
                          {media.size < 1024 * 1024
                            ? `${(media.size / 1024).toFixed(1)}KB`
                            : `${(media.size / (1024 * 1024)).toFixed(1)}MB`}
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Uploader</div>
                        <div className="text-white font-semibold text-xs">{media.uploader}</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Views</div>
                        <div className="text-cyan-400 font-semibold">{(media.views / 1000).toFixed(1)}K</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Uploaded</div>
                        <div className="text-white font-semibold">
                          {Math.round((Date.now() - media.uploadedAt) / 86400000)}d ago
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'messages' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">E2E Encrypted Messages</h3>
              <div className="space-y-4">
                {e2eMessages.map((msg) => (
                  <div key={msg.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-2">
                          <Lock className="w-4 h-4 text-green-400" />
                          <h4 className="text-white font-semibold">
                            {msg.from} → {msg.to}
                          </h4>
                        </div>
                        <p className="text-xs text-gray-500">{msg.encryptionAlgorithm}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          msg.status === 'read'
                            ? 'bg-green-500/20 text-green-400'
                            : msg.status === 'delivered'
                              ? 'bg-blue-500/20 text-blue-400'
                              : 'bg-yellow-500/20 text-yellow-400'
                        }`}
                      >
                        {msg.status.toUpperCase()}
                      </div>
                    </div>
                    <p className="text-gray-300 text-sm mb-3">"{msg.decryptedPreview}"</p>
                    <div className="flex items-center justify-between text-xs text-gray-400">
                      <span>{Math.round((Date.now() - msg.timestamp) / 3600000)}h ago</span>
                      {msg.mediaAttachments > 0 && <span>📎 {msg.mediaAttachments} attachment(s)</span>}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default SocialBackendPanel;
