import React, { useState } from 'react';
import { MessageSquare, Mail, Phone, Globe, Send, Check } from 'lucide-react';

interface Message {
  id: number;
  senderName: string;
  subject: string;
  preview: string;
  timestamp: string;
  isRead: boolean;
  category: 'support' | 'update' | 'alert';
}

export const CommunicationCenterPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'inbox' | 'compose'>('inbox');
  const [messages, setMessages] = useState<Message[]>([
    {
      id: 1,
      senderName: 'Support Team',
      subject: 'Your validator setup is complete',
      preview: 'Your validator node has been successfully configured and is running...',
      timestamp: '2 hours ago',
      isRead: false,
      category: 'support',
    },
    {
      id: 2,
      senderName: 'Network Update',
      subject: 'Protocol upgrade scheduled for next week',
      preview: 'The X3 protocol will undergo a scheduled upgrade next Tuesday...',
      timestamp: '1 day ago',
      isRead: true,
      category: 'update',
    },
    {
      id: 3,
      senderName: 'Alerts',
      subject: 'High memory usage detected',
      preview: 'Your validator is experiencing higher than normal memory usage...',
      timestamp: '3 days ago',
      isRead: true,
      category: 'alert',
    },
  ]);
  const [selectedMessage, setSelectedMessage] = useState<Message | null>(null);
  const [newMessage, setNewMessage] = useState({ to: '', subject: '', body: '' });

  const handleMarkAsRead = (id: number) => {
    setMessages(messages.map((m) => (m.id === id ? { ...m, isRead: true } : m)));
  };

  const handleSendMessage = () => {
    if (newMessage.to && newMessage.subject && newMessage.body) {
      const message: Message = {
        id: messages.length + 1,
        senderName: newMessage.to,
        subject: newMessage.subject,
        preview: newMessage.body.substring(0, 50),
        timestamp: 'Just now',
        isRead: true,
        category: 'support',
      };
      setMessages([message, ...messages]);
      setNewMessage({ to: '', subject: '', body: '' });
      setActiveTab('inbox');
    }
  };

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'support':
        return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
      case 'update':
        return 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20';
      case 'alert':
        return 'bg-orange-500/10 text-orange-400 border-orange-500/20';
      default:
        return 'bg-gray-500/10 text-gray-400 border-gray-500/20';
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Communication Center
            </h1>
            <p className="text-gray-400">Messages, alerts, and notifications</p>
          </div>
          <MessageSquare className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['inbox', 'compose'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'inbox' ? `Inbox (${messages.filter((m) => !m.isRead).length})` : 'New Message'}
            </button>
          ))}
        </div>

        {activeTab === 'inbox' ? (
          <div className="grid grid-cols-3 gap-6">
            {/* Message List */}
            <div className="col-span-2 bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
              {messages.length === 0 ? (
                <div className="p-8 text-center">
                  <MessageSquare className="w-12 h-12 text-gray-500 mx-auto mb-4 opacity-50" />
                  <p className="text-gray-400">No messages</p>
                </div>
              ) : (
                <div className="divide-y divide-[#2a2a35]">
                  {messages.map((msg) => (
                    <div
                      key={msg.id}
                      onClick={() => {
                        setSelectedMessage(msg);
                        handleMarkAsRead(msg.id);
                      }}
                      className={`p-4 cursor-pointer transition ${
                        selectedMessage?.id === msg.id
                          ? 'bg-[#2a2a35]'
                          : 'hover:bg-[#0a0a0f]'
                      } ${!msg.isRead ? 'bg-[#0a0a0f]/50' : ''}`}
                    >
                      <div className="flex items-start justify-between mb-2">
                        <h3 className={`font-semibold ${msg.isRead ? 'text-gray-300' : 'text-white'}`}>
                          {msg.senderName}
                        </h3>
                        <span className="text-gray-500 text-xs">{msg.timestamp}</span>
                      </div>
                      <p className={`text-sm mb-2 ${msg.isRead ? 'text-gray-500' : 'text-cyan-300'}`}>
                        {msg.subject}
                      </p>
                      <p className="text-gray-500 text-xs line-clamp-2">{msg.preview}</p>
                      <div className="mt-2">
                        <span
                          className={`inline-block text-xs px-2 py-1 rounded border ${getCategoryColor(
                            msg.category
                          )}`}
                        >
                          {msg.category}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Message Detail */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              {selectedMessage ? (
                <div>
                  <h2 className="text-xl font-bold text-white mb-1">{selectedMessage.subject}</h2>
                  <p className="text-gray-400 text-sm mb-4">From: {selectedMessage.senderName}</p>
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4 mb-4">
                    <p className="text-gray-300">{selectedMessage.preview}</p>
                  </div>
                  <div className="text-xs text-gray-500">
                    <p>Received {selectedMessage.timestamp}</p>
                  </div>
                </div>
              ) : (
                <p className="text-gray-400 text-center py-8">Select a message to read</p>
              )}
            </div>
          </div>
        ) : (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h2 className="text-xl font-bold text-white mb-6">Compose New Message</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">To</label>
                <input
                  type="email"
                  value={newMessage.to}
                  onChange={(e) => setNewMessage({ ...newMessage, to: e.target.value })}
                  placeholder="recipient@example.com"
                  className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
                />
              </div>
              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Subject</label>
                <input
                  type="text"
                  value={newMessage.subject}
                  onChange={(e) => setNewMessage({ ...newMessage, subject: e.target.value })}
                  placeholder="Message subject"
                  className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
                />
              </div>
              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Message</label>
                <textarea
                  value={newMessage.body}
                  onChange={(e) => setNewMessage({ ...newMessage, body: e.target.value })}
                  placeholder="Your message"
                  className="w-full h-48 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400 resize-none"
                />
              </div>
              <button
                onClick={handleSendMessage}
                className="w-full px-4 py-3 bg-cyan-600 hover:bg-cyan-700 text-white font-bold rounded-lg transition flex items-center justify-center gap-2"
              >
                <Send className="w-5 h-5" /> Send Message
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default CommunicationCenterPanel;
