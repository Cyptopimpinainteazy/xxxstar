import React, { useState } from 'react';
import { Database, RefreshCw, Copy, Download, Upload } from 'lucide-react';

interface DataEntry {
  id: string;
  key: string;
  value: string;
  type: 'string' | 'number' | 'json' | 'boolean';
  lastModified: string;
}

export const DatastoreManagementPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'view' | 'create' | 'search'>('view');
  const [entries, setEntries] = useState<DataEntry[]>([
    {
      id: '1',
      key: 'validator_address',
      value: '0x742d35Cc6634C0532925a3b844Bc9e7595f42bE',
      type: 'string',
      lastModified: '2024-01-20',
    },
    {
      id: '2',
      key: 'stake_amount',
      value: '32',
      type: 'number',
      lastModified: '2024-01-15',
    },
    {
      id: '3',
      key: 'config',
      value: '{"timeout": 30, "retries": 3}',
      type: 'json',
      lastModified: '2024-01-10',
    },
  ]);
  const [newEntry, setNewEntry] = useState({ key: '', value: '', type: 'string' as const });
  const [searchQuery, setSearchQuery] = useState('');

  const handleAddEntry = () => {
    if (newEntry.key && newEntry.value) {
      const entry: DataEntry = {
        id: Date.now().toString(),
        key: newEntry.key,
        value: newEntry.value,
        type: newEntry.type,
        lastModified: new Date().toLocaleDateString(),
      };
      setEntries([entry, ...entries]);
      setNewEntry({ key: '', value: '', type: 'string' });
      setActiveTab('view');
    }
  };

  const handleDeleteEntry = (id: string) => {
    setEntries(entries.filter((e) => e.id !== id));
  };

  const handleCopyValue = (value: string) => {
    navigator.clipboard.writeText(value);
  };

  const filteredEntries = entries.filter(
    (e) =>
      e.key.toLowerCase().includes(searchQuery.toLowerCase()) ||
      e.value.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'string':
        return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
      case 'number':
        return 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20';
      case 'json':
        return 'bg-purple-500/10 text-purple-400 border-purple-500/20';
      case 'boolean':
        return 'bg-green-500/10 text-green-400 border-green-500/20';
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
              Datastore Management
            </h1>
            <p className="text-gray-400">Key-value storage for application configuration and state</p>
          </div>
          <Database className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-1">TOTAL ENTRIES</div>
            <div className="text-3xl font-bold text-cyan-400">{entries.length}</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-1">STORAGE USED</div>
            <div className="text-3xl font-bold text-blue-400">
              {(entries.reduce((sum, e) => sum + e.value.length, 0) / 1024).toFixed(1)} KB
            </div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-1">LAST SYNC</div>
            <div className="text-3xl font-bold text-teal-400">Now</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['view', 'create', 'search'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'view' && 'View Entries'}
              {tab === 'create' && 'Add New'}
              {tab === 'search' && 'Search'}
            </button>
          ))}
        </div>

        {/* Content */}
        {activeTab === 'view' && (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
            {entries.length === 0 ? (
              <div className="p-8 text-center">
                <Database className="w-12 h-12 text-gray-500 mx-auto mb-4 opacity-50" />
                <p className="text-gray-400">No entries yet. Create your first entry.</p>
              </div>
            ) : (
              <div className="divide-y divide-[#2a2a35]">
                {entries.map((entry) => (
                  <div key={entry.id} className="p-4 hover:bg-[#0a0a0f]/50 transition">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex-1">
                        <h3 className="text-white font-bold font-mono">{entry.key}</h3>
                      </div>
                      <span className={`text-xs px-2 py-1 rounded border ${getTypeColor(entry.type)}`}>
                        {entry.type}
                      </span>
                    </div>

                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3 mb-3">
                      <p className="text-gray-300 font-mono text-sm break-all">{entry.value}</p>
                    </div>

                    <div className="flex items-center justify-between">
                      <p className="text-gray-500 text-xs">Modified {entry.lastModified}</p>
                      <div className="flex gap-2">
                        <button
                          onClick={() => handleCopyValue(entry.value)}
                          className="px-2 py-1 bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-400 rounded text-xs font-semibold transition flex items-center gap-1"
                        >
                          <Copy className="w-3 h-3" /> Copy
                        </button>
                        <button
                          onClick={() => handleDeleteEntry(entry.id)}
                          className="px-2 py-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded text-xs font-semibold transition"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === 'create' && (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h2 className="text-xl font-bold text-white mb-6">Create New Entry</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Key</label>
                <input
                  type="text"
                  value={newEntry.key}
                  onChange={(e) => setNewEntry({ ...newEntry, key: e.target.value })}
                  placeholder="e.g., configuration_timeout"
                  className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400 font-mono"
                />
              </div>

              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Type</label>
                <select
                  value={newEntry.type}
                  onChange={(e) => setNewEntry({ ...newEntry, type: e.target.value as any })}
                  className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white focus:outline-none focus:border-cyan-400"
                >
                  <option value="string">String</option>
                  <option value="number">Number</option>
                  <option value="json">JSON</option>
                  <option value="boolean">Boolean</option>
                </select>
              </div>

              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Value</label>
                <textarea
                  value={newEntry.value}
                  onChange={(e) => setNewEntry({ ...newEntry, value: e.target.value })}
                  placeholder="Enter the value"
                  className="w-full h-32 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400 resize-none font-mono"
                />
              </div>

              <button
                onClick={handleAddEntry}
                className="w-full px-4 py-3 bg-cyan-600 hover:bg-cyan-700 text-white font-bold rounded-lg transition"
              >
                Add Entry
              </button>
            </div>
          </div>
        )}

        {activeTab === 'search' && (
          <div className="space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search by key or value..."
                className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
              />
            </div>

            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
              {filteredEntries.length === 0 ? (
                <div className="p-8 text-center">
                  <p className="text-gray-400">No entries found</p>
                </div>
              ) : (
                <div className="divide-y divide-[#2a2a35]">
                  {filteredEntries.map((entry) => (
                    <div key={entry.id} className="p-4">
                      <div className="flex items-center justify-between">
                        <div>
                          <h3 className="text-white font-bold font-mono">{entry.key}</h3>
                          <p className="text-gray-500 text-sm mt-1">{entry.value}</p>
                        </div>
                        <span className={`text-xs px-2 py-1 rounded border ${getTypeColor(entry.type)}`}>
                          {entry.type}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default DatastoreManagementPanel;
