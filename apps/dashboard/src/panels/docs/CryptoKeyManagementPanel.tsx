import React, { useState } from 'react';
import { Zap, Lock, Key, AlertCircle, CheckCircle } from 'lucide-react';

interface KeyPair {
  name: string;
  publicKey: string;
  privateKey: string;
  created: string;
  type: 'signing' | 'encryption';
}

export const CryptoKeyManagementPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'generate' | 'manage'>('manage');
  const [keyPairs, setKeyPairs] = useState<KeyPair[]>([
    {
      name: 'Primary Validator Key',
      publicKey: '0x742d35Cc6634C0532925a3b844Bc9e7595f42bE',
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      created: '2024-01-15',
      type: 'signing',
    },
    {
      name: 'Backup Key',
      publicKey: '0x1234567890abcdef1234567890abcdef12345678',
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      created: '2024-01-20',
      type: 'encryption',
    },
  ]);
  const [showPrivateKey, setShowPrivateKey] = useState<boolean>(false);
  const [newKeyName, setNewKeyName] = useState('');

  const handleGenerateKey = () => {
    const newKey: KeyPair = {
      name: newKeyName || 'New Key',
      publicKey: '0x' + Array(40).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join(''),
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      created: new Date().toLocaleDateString(),
      type: 'signing',
    };
    setKeyPairs([...keyPairs, newKey]);
    setNewKeyName('');
    setActiveTab('manage');
  };

  const handleDeleteKey = (index: number) => {
    setKeyPairs(keyPairs.filter((_, i) => i !== index));
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Cryptographic Key Management
            </h1>
            <p className="text-gray-400">Generate, secure, and manage your signing and encryption keys</p>
          </div>
          <Key className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['manage', 'generate'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'manage' ? 'Manage Keys' : 'Generate New'}
            </button>
          ))}
        </div>

        {/* Content */}
        {activeTab === 'manage' ? (
          <div className="space-y-4">
            {keyPairs.length === 0 ? (
              <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-8 text-center">
                <Lock className="w-12 h-12 text-gray-500 mx-auto mb-4 opacity-50" />
                <p className="text-gray-400">No keys yet. Generate your first key.</p>
              </div>
            ) : (
              keyPairs.map((key, idx) => (
                <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <h3 className="text-white font-bold text-lg mb-1">{key.name}</h3>
                      <p className="text-gray-400 text-sm">Type: {key.type === 'signing' ? 'Signing' : 'Encryption'} • Created: {key.created}</p>
                    </div>
                    <button
                      onClick={() => handleDeleteKey(idx)}
                      className="px-3 py-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded text-sm font-semibold transition"
                    >
                      Delete
                    </button>
                  </div>

                  <div className="space-y-3">
                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                      <div className="text-gray-400 text-xs mb-1">Public Key</div>
                      <div className="font-mono text-sm text-cyan-400 break-all">{key.publicKey}</div>
                    </div>

                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                      <div className="flex justify-between items-start mb-1">
                        <div className="text-gray-400 text-xs">Private Key</div>
                        <button
                          onClick={() => setShowPrivateKey(!showPrivateKey)}
                          className="text-cyan-400 hover:text-cyan-300 text-xs font-semibold transition"
                        >
                          {showPrivateKey ? 'Hide' : 'Show'}
                        </button>
                      </div>
                      <div className="font-mono text-sm text-gray-400 break-all">
                        {showPrivateKey ? key.privateKey : '••••••••••••••••••••••••••••••••••••••••'}
                      </div>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        ) : (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h2 className="text-xl font-bold text-white mb-6">Generate New Key Pair</h2>

            <div className="space-y-4">
              <div>
                <label className="block text-gray-400 text-sm font-semibold mb-2">Key Name</label>
                <input
                  type="text"
                  value={newKeyName}
                  onChange={(e) => setNewKeyName(e.target.value)}
                  placeholder="e.g., Hot Wallet, Archive Node"
                  className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
                />
              </div>

              <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4">
                <div className="flex gap-3">
                  <AlertCircle className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="text-blue-400 font-semibold mb-1">Security Notice</p>
                    <p className="text-blue-300 text-sm">
                      Store your private key securely. Never share it with anyone. Consider using a hardware wallet for signing keys.
                    </p>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 text-xs font-semibold mb-2">Key Type</div>
                  <div className="text-white font-semibold">Ed25519 (Signing)</div>
                  <p className="text-gray-500 text-xs mt-1">For validator signatures and transactions</p>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 text-xs font-semibold mb-2">Entropy</div>
                  <div className="text-white font-semibold">256-bit</div>
                  <p className="text-gray-500 text-xs mt-1">Cryptographically secure random</p>
                </div>
              </div>

              <button
                onClick={handleGenerateKey}
                className="w-full px-4 py-3 bg-cyan-600 hover:bg-cyan-700 text-white font-bold rounded-lg transition flex items-center justify-center gap-2"
              >
                <Zap className="w-5 h-5" /> Generate Key Pair
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default CryptoKeyManagementPanel;
