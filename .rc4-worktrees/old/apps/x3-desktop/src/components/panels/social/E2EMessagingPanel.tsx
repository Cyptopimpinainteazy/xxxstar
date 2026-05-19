import React, { useState } from 'react';
import { MessageSquare, Lock, Eye, EyeOff, Send, Users } from 'lucide-react';

interface EncryptedMessage {
  id: string;
  sender: string;
  recipient: string;
  content: string;
  encrypted: boolean;
  keyExchangeStatus: 'pending' | 'completed' | 'failed';
  timestamp: string;
  readAt?: string;
}

export const E2EMessagingPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'conversations' | 'security' | 'contacts'>('conversations');
  const [selectedConversation, setSelectedConversation] = useState<string | null>(null);
  const [newMessage, setNewMessage] = useState('');
  const [showPassword, setShowPassword] = useState(false);

  const conversations: EncryptedMessage[] = [
    {
      id: '1',
      sender: 'You',
      recipient: 'alice.x3',
      content: 'Confirming validator specifications for mainnet launch...',
      encrypted: true,
      keyExchangeStatus: 'completed',
      timestamp: '2 mins ago',
      readAt: '1 min ago',
    },
    {
      id: '2',
      sender: 'bob.x3',
      recipient: 'You',
      content: 'DeFi integration complete, ready for testing',
      encrypted: true,
      keyExchangeStatus: 'completed',
      timestamp: '15 mins ago',
      readAt: undefined,
    },
    {
      id: '3',
      sender: 'You',
      recipient: 'treasury-council.x3',
      content: 'Multi-sig approval workflow configured with 5-of-7 threshold',
      encrypted: true,
      keyExchangeStatus: 'completed',
      timestamp: '1 hour ago',
      readAt: '45 mins ago',
    },
    {
      id: '4',
      sender: 'carol.x3',
      recipient: 'You',
      content: '[Encrypted Key Exchange in Progress]',
      encrypted: true,
      keyExchangeStatus: 'pending',
      timestamp: '3 hours ago',
      readAt: undefined,
    },
  ];

  const securityMetrics = [
    { name: 'X3DH Key Exchange', status: 'completed', curves: 'Curve25519', strength: '256-bit' },
    { name: 'Double Ratchet', status: 'completed', algorithm: 'ChaCha20-Poly1305', keyRotation: 'Per message' },
    { name: 'Forward Secrecy', status: 'completed', rotationType: 'Automatic', interval: 'Every message' },
    { name: 'Key Storage', status: 'completed', encryption: 'Argon2id + ChaCha20', location: 'Local encrypted DB' },
  ];

  const contacts = [
    { id: '1', name: 'Alice Network', address: 'alice.x3', verified: true, lastMessage: '2 mins ago' },
    { id: '2', name: 'Bob Trading', address: 'bob.x3', verified: true, lastMessage: '15 mins ago' },
    { id: '3', name: 'Treasury Council', address: 'treasury-council.x3', verified: true, lastMessage: '1 hour ago' },
    { id: '4', name: 'Carol Validator', address: 'carol.x3', verified: false, lastMessage: '3 hours ago' },
  ];

  const sendMessage = () => {
    if (newMessage.trim()) {
      console.log('Message sent:', newMessage);
      setNewMessage('');
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-purple-500/20 to-indigo-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Lock className="w-5 h-5 text-purple-400" />
          <h1 className="text-lg font-bold text-white">E2E Encrypted Messaging</h1>
        </div>
        <p className="text-sm text-gray-400">X3DH + Double Ratchet (Signal protocol) with forward secrecy</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['conversations', 'security', 'contacts'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-purple-400 border-b-2 border-purple-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'conversations' && 'Messages'}
            {tab === 'security' && 'Security'}
            {tab === 'contacts' && 'Contacts'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'conversations' && (
          <div className="h-full flex">
            {/* Conversation List */}
            <div className="w-80 border-r border-[#2a2a35] overflow-y-auto">
              <div className="p-3 space-y-2">
                {conversations.map((conv) => (
                  <div
                    key={conv.id}
                    onClick={() => setSelectedConversation(conv.id)}
                    className={`p-3 rounded-lg cursor-pointer transition ${
                      selectedConversation === conv.id
                        ? 'bg-purple-500/20 border border-purple-500/30'
                        : 'border border-[#2a2a35] hover:border-purple-500/30'
                    }`}
                  >
                    <div className="flex justify-between items-start mb-1">
                      <h3 className="font-semibold text-white text-sm">{conv.recipient}</h3>
                      <Lock className="w-3 h-3 text-purple-400" />
                    </div>
                    <p className="text-xs text-gray-500 truncate mb-1">{conv.content}</p>
                    <div className="flex justify-between items-center text-xs">
                      <span className="text-gray-600">{conv.timestamp}</span>
                      {!conv.readAt && conv.sender === 'You' ? (
                        <span className="px-2 py-0.5 bg-purple-600 rounded-full text-white text-xs">Unread</span>
                      ) : null}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Message View */}
            {selectedConversation && (
              <div className="flex-1 flex flex-col p-6">
                <div className="flex-1 space-y-4 mb-4 overflow-y-auto">
                  {conversations
                    .filter((c) => c.id === selectedConversation)
                    .map((conv) => (
                      <div key={conv.id} className="space-y-3">
                        <div className="p-3 bg-[#0f0f15] border border-[#2a2a35] rounded-lg">
                          <p className="text-sm text-gray-300 mb-2">{conv.content}</p>
                          <div className="flex items-center gap-2 text-xs text-gray-500">
                            <Lock className="w-3 h-3 text-purple-400" />
                            <span>End-to-end encrypted</span>
                          </div>
                        </div>
                      </div>
                    ))}
                </div>

                {/* Message Input */}
                <div className="flex gap-3">
                  <input
                    type="text"
                    value={newMessage}
                    onChange={(e) => setNewMessage(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && sendMessage()}
                    placeholder="Type encrypted message..."
                    className="flex-1 px-3 py-2 bg-[#0f0f15] border border-[#2a2a35] rounded text-white placeholder-gray-600 text-sm"
                  />
                  <button
                    onClick={sendMessage}
                    className="px-4 py-2 bg-purple-600 hover:bg-purple-700 rounded text-white font-semibold transition flex items-center gap-2"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'security' && (
          <div className="p-6 space-y-4">
            {securityMetrics.map((metric, idx) => (
              <div key={idx} className="p-4 border border-[#2a2a35] rounded-lg hover:border-purple-500/30 transition">
                <div className="flex justify-between items-start mb-2">
                  <h3 className="font-semibold text-white">{metric.name}</h3>
                  <span className="px-2 py-1 text-xs bg-emerald-500/20 text-emerald-400 rounded font-semibold">
                    {metric.status}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs text-gray-400">
                  {Object.entries(metric)
                    .filter(([key]) => key !== 'name' && key !== 'status')
                    .map(([key, value]) => (
                      <div key={key}>
                        <span className="text-gray-600 capitalize">{key}:</span> {value}
                      </div>
                    ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'contacts' && (
          <div className="p-6 space-y-3">
            {contacts.map((contact) => (
              <div
                key={contact.id}
                className="p-4 border border-[#2a2a35] rounded-lg hover:border-purple-500/30 transition cursor-pointer"
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <h3 className="font-semibold text-white">{contact.name}</h3>
                    <p className="text-xs text-gray-500 font-mono mt-1">{contact.address}</p>
                  </div>
                  {contact.verified && (
                    <span className="text-purple-400 text-xs font-bold">✓ Verified</span>
                  )}
                </div>
                <p className="text-xs text-gray-600">Last message: {contact.lastMessage}</p>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default E2EMessagingPanel;
