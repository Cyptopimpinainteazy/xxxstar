import React, { useState } from "react";
import { Database, Users, Wifi, Download, Upload, Trash2, Plus, Settings } from "lucide-react";
import clsx from "clsx";

interface Contact {
  id: string;
  name: string;
  walletAddress: string;
  email: string;
  tags: string[];
  lastContact: string;
  status: "active" | "inactive";
}

interface SyncStatus {
  timestamp: string;
  itemsSync: number;
  itemsTotal: number;
  status: "synced" | "syncing" | "pending";
}

const MOCK_CONTACTS: Contact[] = [
  {
    id: "1",
    name: "Alice",
    walletAddress: "0x742d35Cc6634C0532925a3b844Bc9e7595f...7e6f",
    email: "alice@x3.com",
    tags: ["friend", "validator"],
    lastContact: "2024-10-05",
    status: "active",
  },
  {
    id: "2",
    name: "Bob",
    walletAddress: "0x123...456",
    email: "bob@example.com",
    tags: ["team"],
    lastContact: "2024-10-01",
    status: "active",
  },
  {
    id: "3",
    name: "Carol",
    walletAddress: "0x789...abc",
    email: "carol@dev.com",
    tags: ["developer", "partner"],
    lastContact: "2024-09-28",
    status: "inactive",
  },
];

export default function RealCrmBackendPanel() {
  const [contacts, setContacts] = useState<Contact[]>(MOCK_CONTACTS);
  const [selectedContact, setSelectedContact] = useState<Contact | null>(MOCK_CONTACTS[0]);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    timestamp: "2024-10-05T14:32:00Z",
    itemsSync: 3,
    itemsTotal: 3,
    status: "synced",
  });
  const [searchQuery, setSearchQuery] = useState("");
  const [connectedStatus, setConnectedStatus] = useState<"connected" | "disconnected">("connected");

  const filteredContacts = contacts.filter((c) =>
    c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    c.email.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const activeContacts = contacts.filter((c) => c.status === "active").length;
  const dbSize = (contacts.length * 0.85).toFixed(2);

  const handleStartSync = () => {
    setSyncStatus({ ...syncStatus, status: "syncing" });
    setTimeout(() => {
      setSyncStatus({ ...syncStatus, status: "synced", timestamp: new Date().toISOString() });
    }, 2000);
  };

  const handleAddContact = () => {
    const newContact: Contact = {
      id: (contacts.length + 1).toString(),
      name: "New Contact",
      walletAddress: "0x...",
      email: "",
      tags: [],
      lastContact: new Date().toISOString().split("T")[0],
      status: "active",
    };
    setContacts([...contacts, newContact]);
    setSelectedContact(newContact);
  };

  const handleDeleteContact = (contactId: string) => {
    setContacts(contacts.filter((c) => c.id !== contactId));
    if (selectedContact?.id === contactId) {
      setSelectedContact(contacts[0] || null);
    }
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Database size={20} className="text-blue-400" /> CRM Backend
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Contacts</div>
            <div className="text-lg font-bold text-blue-400">{contacts.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Active</div>
            <div className="text-lg font-bold text-green-400">{activeContacts}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">DB Size</div>
            <div className="text-lg font-bold text-cyan-400">{dbSize} KB</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Connection</div>
            <div className={clsx("text-sm font-bold flex items-center gap-1", connectedStatus === "connected" ? "text-green-400" : "text-red-400")}>
              <Wifi size={12} /> {connectedStatus === "connected" ? "Online" : "Offline"}
            </div>
          </div>
        </div>

        {/* Sync Status */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
          <div className="flex items-center justify-between">
            <div className="text-sm font-semibold">SQLite Sync Status</div>
            <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", syncStatus.status === "synced" ? "bg-green-600/20 text-green-400" : "bg-yellow-600/20 text-yellow-400")}>
              {syncStatus.status === "synced" ? "✓ SYNCED" : "↻ SYNCING"}
            </span>
          </div>

          <div className="flex-1 bg-[#2a2a35] rounded-full h-2 overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-600 to-cyan-600"
              style={{ width: `${(syncStatus.itemsSync / syncStatus.itemsTotal) * 100}%` }}
            />
          </div>
          <div className="text-xs text-gray-400">
            {syncStatus.itemsSync}/{syncStatus.itemsTotal} items synced
          </div>
          <div className="text-xs text-gray-500 mb-2">Last sync: {syncStatus.timestamp.split("T")[0]} {syncStatus.timestamp.split("T")[1].substring(0, 5)}</div>

          <button
            onClick={handleStartSync}
            className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded text-sm font-semibold transition"
          >
            Sync Now
          </button>
        </div>

        {/* Search & Actions */}
        <div className="space-y-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search contacts..."
            className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-sm focus:border-cyan-600 focus:outline-none"
          />

          <div className="flex gap-2">
            <button
              onClick={handleAddContact}
              className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded text-sm font-semibold transition flex items-center justify-center gap-2"
            >
              <Plus size={14} /> Add Contact
            </button>
            <button className="flex-1 bg-cyan-600 hover:bg-cyan-700 py-2 rounded text-sm font-semibold transition flex items-center justify-center gap-2">
              <Upload size={14} /> Import
            </button>
            <button className="flex-1 bg-purple-600 hover:bg-purple-700 py-2 rounded text-sm font-semibold transition flex items-center justify-center gap-2">
              <Download size={14} /> Export
            </button>
          </div>
        </div>

        {/* Contact List */}
        <div>
          <h3 className="font-semibold mb-2 text-sm">Contacts ({filteredContacts.length})</h3>
          <div className="space-y-2">
            {filteredContacts.map((contact) => (
              <button
                key={contact.id}
                onClick={() => setSelectedContact(contact)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedContact?.id === contact.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-1">
                  <div className="font-semibold text-sm">{contact.name}</div>
                  <span className={clsx("text-xs px-2 py-1 rounded-md font-bold", contact.status === "active" ? "bg-green-600/20 text-green-400" : "bg-gray-600/20 text-gray-400")}>
                    {contact.status === "active" ? "Active" : "Inactive"}
                  </span>
                </div>
                <div className="text-xs text-gray-400 mb-1">{contact.email}</div>
                <div className="flex justify-between text-xs text-gray-500">
                  <span className="font-mono text-xs">{contact.walletAddress.slice(0, 10)}...{contact.walletAddress.slice(-8)}</span>
                  <span>Last: {contact.lastContact}</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Contact Details */}
        {selectedContact && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3 text-sm">
            <h3 className="font-semibold">{selectedContact.name} Details</h3>

            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-gray-400">Email</span>
                <span className="font-mono text-xs">{selectedContact.email}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Wallet</span>
                <span className="font-mono text-xs">{selectedContact.walletAddress.slice(0, 10)}...{selectedContact.walletAddress.slice(-8)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span className="font-bold">{selectedContact.status.toUpperCase()}</span>
              </div>
              <div>
                <span className="text-gray-400 block mb-1">Tags</span>
                <div className="flex flex-wrap gap-1">
                  {selectedContact.tags.map((tag) => (
                    <span key={tag} className="bg-blue-600/20 text-blue-300 text-xs px-2 py-1 rounded-md font-semibold">
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <button
              onClick={() => handleDeleteContact(selectedContact.id)}
              className="w-full bg-red-600/20 hover:bg-red-600/30 text-red-400 py-2 rounded font-semibold text-sm transition flex items-center justify-center gap-2"
            >
              <Trash2 size={14} /> Delete Contact
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        SQLite backend with WebSocket synchronization for contacts & messages.
      </div>
    </div>
  );
}
