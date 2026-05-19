import React, { useState } from "react";
import { MessageCircle, Lock, Key, Shield, Send, Eye, CheckCircle, Clock } from "lucide-react";
import clsx from "clsx";

interface Message {
  id: string;
  from: string;
  content: string;
  encrypted: boolean;
  timestamp: string;
  status: "sent" | "delivered" | "read";
}

interface Conversation {
  id: string;
  participant: string;
  lastMessage: string;
  lastMessageTime: string;
  unread: number;
  encryptionStatus: "established" | "pending" | "failed";
  keyExchangeProgress: number;
}

interface EncryptionState {
  protocol: string;
  cipherSuite: string;
  keyRotationDays: number;
  nextRatchet: string;
  messageCount: number;
}

const MOCK_CONVERSATIONS: Conversation[] = [
  {
    id: "1",
    participant: "Alice",
    lastMessage: "See you tomorrow!",
    lastMessageTime: "2 mins ago",
    unread: 0,
    encryptionStatus: "established",
    keyExchangeProgress: 100,
  },
  {
    id: "2",
    participant: "Bob",
    lastMessage: "Can you review the proposal?",
    lastMessageTime: "1 hour ago",
    unread: 1,
    encryptionStatus: "established",
    keyExchangeProgress: 100,
  },
  {
    id: "3",
    participant: "Carol",
    lastMessage: "Starting key exchange...",
    lastMessageTime: "just now",
    unread: 0,
    encryptionStatus: "pending",
    keyExchangeProgress: 65,
  },
];

const MOCK_MESSAGES: Message[] = [
  {
    id: "1",
    from: "Alice",
    content: "Hey! How's the project going?",
    encrypted: true,
    timestamp: "2024-10-05T14:15:00Z",
    status: "read",
  },
  {
    id: "2",
    from: "me",
    content: "Great! Almost finished with the milestone.",
    encrypted: true,
    timestamp: "2024-10-05T14:17:00Z",
    status: "delivered",
  },
  {
    id: "3",
    from: "Alice",
    content: "See you tomorrow!",
    encrypted: true,
    timestamp: "2024-10-05T14:20:00Z",
    status: "read",
  },
];

const MOCK_ENCRYPTION: EncryptionState = {
  protocol: "X3DH + Double Ratchet (Signal Protocol)",
  cipherSuite: "ChaCha20-Poly1305 (256-bit)",
  keyRotationDays: 30,
  nextRatchet: "2024-10-15",
  messageCount: 47,
};

export default function E2eMessagesPanel() {
  const [conversations, setConversations] = useState<Conversation[]>(MOCK_CONVERSATIONS);
  const [selectedConversation, setSelectedConversation] = useState<Conversation | null>(MOCK_CONVERSATIONS[0]);
  const [messages, setMessages] = useState<Message[]>(MOCK_MESSAGES);
  const [messageInput, setMessageInput] = useState("");
  const [activeTab, setActiveTab] = useState<"chats" | "encryption">("chats");
  const [showKeyExchange, setShowKeyExchange] = useState(false);

  const pendingEncryption = conversations.filter((c) => c.encryptionStatus === "pending").length;

  const handleSendMessage = () => {
    if (messageInput.trim()) {
      const newMessage: Message = {
        id: (messages.length + 1).toString(),
        from: "me",
        content: messageInput,
        encrypted: true,
        timestamp: new Date().toISOString(),
        status: "sent",
      };
      setMessages([...messages, newMessage]);
      setMessageInput("");
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <MessageCircle size={20} className="text-green-400" /> E2E Encrypted Messages
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Conversations</div>
            <div className="text-lg font-bold text-green-400">{conversations.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Encrypted</div>
            <div className="text-lg font-bold text-cyan-400">
              {conversations.filter((c) => c.encryptionStatus === "established").length}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Pending Keys</div>
            <div className={clsx("text-lg font-bold", pendingEncryption > 0 ? "text-yellow-400" : "text-green-400")}>
              {pendingEncryption}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["chats", "encryption"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2",
                activeTab === tab
                  ? "border-cyan-600 text-cyan-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "chats" ? "Messages" : "Encryption"}
            </button>
          ))}
        </div>

        {activeTab === "chats" && (
          <div className="flex gap-3 h-96">
            {/* Conversation List */}
            <div className="w-48 flex flex-col space-y-2 border-r border-[#2a2a35] pr-3">
              {conversations.map((conv) => (
                <button
                  key={conv.id}
                  onClick={() => setSelectedConversation(conv)}
                  className={clsx(
                    "text-left p-2 rounded-lg border transition text-sm",
                    selectedConversation?.id === conv.id
                      ? "border-cyan-600 bg-cyan-600/10"
                      : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                  )}
                >
                  <div className="flex items-center justify-between mb-1">
                    <div className="font-semibold text-xs">{conv.participant}</div>
                    {conv.unread > 0 && (
                      <span className="bg-red-600 text-white text-xs px-1.5 py-0.5 rounded-full">{conv.unread}</span>
                    )}
                  </div>

                  <div className="text-xs text-gray-400 truncate">{conv.lastMessage}</div>

                  <div className="flex items-center gap-1 mt-1 text-xs text-gray-500">
                    {conv.encryptionStatus === "established" ? (
                      <Lock size={10} className="text-green-400" />
                    ) : (
                      <Clock size={10} className="text-yellow-400 animate-spin" />
                    )}
                    <span>{conv.lastMessageTime}</span>
                  </div>
                </button>
              ))}
            </div>

            {/* Messages */}
            {selectedConversation && (
              <div className="flex-1 flex flex-col">
                <div className="flex-1 overflow-y-auto space-y-2 mb-3">
                  {messages.map((msg) => (
                    <div
                      key={msg.id}
                      className={clsx("p-2 rounded-lg text-xs", msg.from === "me" ? "bg-cyan-600/20 ml-12" : "bg-[#2a2a35] mr-12")}
                    >
                      <div className="flex items-center gap-1 mb-1">
                        {msg.encrypted && <Lock size={10} className="text-green-400" />}
                        <span className="font-semibold">{msg.from === "me" ? "You" : msg.from}</span>
                        {msg.status === "read" && <CheckCircle size={10} className="text-blue-400" />}
                        {msg.status === "delivered" && <CheckCircle size={10} className="text-gray-400" />}
                      </div>
                      <div className="text-xs">{msg.content}</div>
                      <div className="text-xs text-gray-500 mt-1">{msg.timestamp.split("T")[1].substring(0, 5)}</div>
                    </div>
                  ))}
                </div>

                {/* Message Input */}
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={messageInput}
                    onChange={(e) => setMessageInput(e.target.value)}
                    onKeyPress={(e) => e.key === "Enter" && handleSendMessage()}
                    placeholder="Type encrypted message..."
                    className="flex-1 bg-[#15151b] border border-[#2a2a35] rounded px-2 py-2 text-xs focus:border-cyan-600 focus:outline-none"
                  />
                  <button onClick={handleSendMessage} className="bg-green-600 hover:bg-green-700 px-3 py-2 rounded text-xs font-semibold transition">
                    <Send size={12} />
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === "encryption" && (
          <div className="space-y-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm flex items-center gap-2">
                <Shield size={16} className="text-green-400" /> Signal Protocol
              </h3>

              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Protocol</span>
                  <span className="font-mono text-xs">{MOCK_ENCRYPTION.protocol}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Cipher Suite</span>
                  <span className="font-mono text-xs">{MOCK_ENCRYPTION.cipherSuite}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Messages Sent</span>
                  <span className="font-semibold">{MOCK_ENCRYPTION.messageCount}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Key Rotation</span>
                  <span className="font-semibold">Every {MOCK_ENCRYPTION.keyRotationDays} days</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Next Ratchet</span>
                  <span className="font-semibold">{MOCK_ENCRYPTION.nextRatchet}</span>
                </div>
              </div>
            </div>

            {/* Key Exchange Status */}
            <div className="space-y-2">
              {conversations.map((conv) => (
                <div key={conv.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="font-semibold text-sm">{conv.participant}</div>
                    <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", conv.encryptionStatus === "established" ? "bg-green-600/20 text-green-400" : "bg-yellow-600/20 text-yellow-400")}>
                      {conv.encryptionStatus === "established" ? "✓ ESTABLISHED" : "↻ KEY EXCHANGE"}
                    </span>
                  </div>

                  {conv.encryptionStatus === "pending" && (
                    <>
                      <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
                        <div
                          className="h-full bg-gradient-to-r from-green-600 to-cyan-600"
                          style={{ width: `${conv.keyExchangeProgress}%` }}
                        />
                      </div>
                      <div className="text-xs text-gray-400">{conv.keyExchangeProgress}% complete</div>
                    </>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        X3DH + Double Ratchet end-to-end encryption with automatic key rotation.
      </div>
    </div>
  );
}
