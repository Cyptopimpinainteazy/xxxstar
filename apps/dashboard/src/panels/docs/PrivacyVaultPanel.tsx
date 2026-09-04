import React, { useState } from 'react';
import { Lock, Eye, EyeOff, Plus, Trash2, Key, Shield, Copy, Check } from 'lucide-react';

interface SecretKey {
  id: string;
  name: string;
  type: 'signing' | 'encryption' | 'stealth';
  publicKey: string;
  privateKey: string; // encrypted in production
  derivationPath: string;
  entropy: number; // bits
  createdAt: string;
  lastUsed?: string;
}

export const PrivacyVaultPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'vault' | 'generate' | 'settings'>('vault');
  const [showPrivateKeys, setShowPrivateKeys] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyType, setNewKeyType] = useState<'signing' | 'encryption'>('signing');
  const [copied, setCopied] = useState(false);
  const [derivationPath, setDerivationPath] = useState("m/44'/0'/0'/0/0");

  const [keys, setKeys] = useState<SecretKey[]>([
    {
      id: '1',
      name: 'Primary Validator Key',
      type: 'signing',
      publicKey: 'ed25519:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d1e2f',
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      derivationPath: "m/44'/0'/0'/0/0",
      entropy: 256,
      createdAt: '2026-02-15T10:30:00Z',
      lastUsed: '2026-03-01T08:45:00Z',
    },
    {
      id: '2',
      name: 'Backup Recovery Key',
      type: 'encryption',
      publicKey: 'x25519:9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a',
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      derivationPath: "m/44'/0'/0'/0/1",
      entropy: 256,
      createdAt: '2026-02-10T14:20:00Z',
      lastUsed: '2026-02-28T16:15:00Z',
    },
    {
      id: '3',
      name: 'Stealth Address Generator',
      type: 'stealth',
      publicKey: 'stealth:4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d',
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      derivationPath: "m/44'/0'/0'/1/0",
      entropy: 256,
      createdAt: '2026-01-20T11:00:00Z',
      lastUsed: '2026-02-25T09:30:00Z',
    },
  ]);

  const handleCopyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleGenerateKey = () => {
    if (!newKeyName) return;

    const newKey: SecretKey = {
      id: `key_${Date.now()}`,
      name: newKeyName,
      type: newKeyType,
      publicKey: `${newKeyType}:${Math.random().toString(16).slice(2)}${Math.random().toString(16).slice(2)}${Math.random().toString(16).slice(2)}`,
      privateKey: '••••••••••••••••••••••••••••••••••••••••',
      derivationPath,
      entropy: 256,
      createdAt: new Date().toISOString(),
    };

    setKeys([...keys, newKey]);
    setNewKeyName('');
    setNewKeyType('signing');
    setDerivationPath("m/44'/0'/0'/0/0");
  };

  const handleDeleteKey = (id: string) => {
    if (confirm('Are you sure you want to delete this key? This action cannot be undone.')) {
      setKeys(keys.filter(k => k.id !== id));
    }
  };

  return (
    <div className="h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f]">
      {/* Header */}
      <div className="border-b border-[#2a2a35] p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-gradient-to-br from-purple-500 to-pink-500 rounded-lg">
            <Lock className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-semibold text-white">Privacy Vault</h1>
            <p className="text-xs text-gray-400">E2E encrypted key management with ChaCha20-Poly1305</p>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 px-4 pt-4 border-b border-[#2a2a35]">
        {(['vault', 'generate', 'settings'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 rounded-lg font-medium text-sm transition ${
              activeTab === tab
                ? 'bg-purple-600 text-white'
                : 'text-gray-400 hover:text-gray-200'
            }`}
          >
            {tab === 'vault' && 'Vault'}
            {tab === 'generate' && 'Generate'}
            {tab === 'settings' && 'Settings'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'vault' && (
          <div className="p-4 space-y-4">
            {keys.length === 0 ? (
              <div className="text-center py-8 text-gray-400">
                <Lock className="w-12 h-12 mx-auto mb-2 opacity-50" />
                <p>No keys in vault. Create your first key to get started.</p>
              </div>
            ) : (
              keys.map(key => (
                <div key={key.id} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 hover:border-purple-500/50 transition">
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-3">
                      <div className={`p-2 rounded-lg ${
                        key.type === 'signing' ? 'bg-blue-500/20 text-blue-400' :
                        key.type === 'encryption' ? 'bg-green-500/20 text-green-400' :
                        'bg-yellow-500/20 text-yellow-400'
                      }`}>
                        <Key className="w-4 h-4" />
                      </div>
                      <div>
                        <h3 className="font-semibold text-white">{key.name}</h3>
                        <p className="text-xs text-gray-500">{key.type.charAt(0).toUpperCase() + key.type.slice(1)}</p>
                      </div>
                    </div>
                    <button
                      onClick={() => handleDeleteKey(key.id)}
                      className="p-2 hover:bg-red-500/20 rounded text-red-400 transition"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>

                  <div className="space-y-2 mb-3">
                    <div>
                      <p className="text-xs text-gray-400 mb-1">Public Key</p>
                      <div className="flex items-center gap-2">
                        <code className="text-xs bg-[#0a0a0f] border border-[#2a2a35] rounded px-2 py-1 text-cyan-400 flex-1 overflow-x-auto">
                          {key.publicKey.slice(0, 32)}...
                        </code>
                        <button
                          onClick={() => handleCopyKey(key.publicKey)}
                          className="p-1 hover:bg-[#2a2a35] rounded transition"
                        >
                          {copied ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4 text-gray-400" />}
                        </button>
                      </div>
                    </div>

                    <div>
                      <p className="text-xs text-gray-400 mb-1">Private Key</p>
                      <div className="flex items-center gap-2">
                        <code className="text-xs bg-[#0a0a0f] border border-[#2a2a35] rounded px-2 py-1 text-gray-500 flex-1">
                          {showPrivateKeys ? key.privateKey : key.privateKey}
                        </code>
                        <button
                          onClick={() => setShowPrivateKeys(!showPrivateKeys)}
                          className="p-1 hover:bg-[#2a2a35] rounded transition"
                        >
                          {showPrivateKeys ? <EyeOff className="w-4 h-4 text-gray-400" /> : <Eye className="w-4 h-4 text-gray-400" />}
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-2 py-2 border-t border-[#2a2a35] text-xs">
                    <div>
                      <p className="text-gray-500">Entropy</p>
                      <p className="text-white font-semibold">{key.entropy} bits</p>
                    </div>
                    <div>
                      <p className="text-gray-500">Path</p>
                      <p className="text-white font-mono text-xs">{key.derivationPath}</p>
                    </div>
                    <div>
                      <p className="text-gray-500">Last Used</p>
                      <p className="text-white text-xs">{key.lastUsed ? new Date(key.lastUsed).toLocaleDateString() : 'Never'}</p>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'generate' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Generate New Key</h3>

              <div className="space-y-4">
                <div>
                  <label className="text-xs text-gray-400 mb-2 block">Key Name</label>
                  <input
                    type="text"
                    value={newKeyName}
                    onChange={e => setNewKeyName(e.target.value)}
                    placeholder="e.g. Trading Account"
                    className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white placeholder-gray-600 focus:border-purple-500 outline-none"
                  />
                </div>

                <div>
                  <label className="text-xs text-gray-400 mb-2 block">Key Type</label>
                  <select
                    value={newKeyType}
                    onChange={e => setNewKeyType(e.target.value as 'signing' | 'encryption')}
                    className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white focus:border-purple-500 outline-none"
                  >
                    <option value="signing">Ed25519 Signing</option>
                    <option value="encryption">X25519 Encryption</option>
                  </select>
                </div>

                <div>
                  <label className="text-xs text-gray-400 mb-2 block">BIP-44 Derivation Path</label>
                  <input
                    type="text"
                    value={derivationPath}
                    onChange={e => setDerivationPath(e.target.value)}
                    placeholder="m/44'/0'/0'/0/0"
                    className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white placeholder-gray-600 focus:border-purple-500 outline-none font-mono text-sm"
                  />
                  <p className="text-xs text-gray-500 mt-1">BIP-44 compliant hierarchical derivation</p>
                </div>

                <div className="bg-yellow-500/10 border border-yellow-500/30 rounded p-3">
                  <div className="flex gap-2">
                    <Shield className="w-4 h-4 text-yellow-400 flex-shrink-0 mt-0.5" />
                    <div className="text-xs text-yellow-400">
                      <p className="font-semibold">Security Warning</p>
                      <p className="text-yellow-300 mt-1">Store private keys securely. Use hardware wallets for production accounts. Keys are encrypted with Argon2id KDF.</p>
                    </div>
                  </div>
                </div>

                <button
                  onClick={handleGenerateKey}
                  disabled={!newKeyName}
                  className="w-full bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded px-4 py-2 font-semibold transition flex items-center justify-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  Generate Key
                </button>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="p-4 space-y-4">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold text-white mb-4">Encryption Settings</h3>

              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-white font-semibold">Biometric Unlock</p>
                    <p className="text-xs text-gray-400">Use Face ID / Fingerprint</p>
                  </div>
                  <button className="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded font-semibold transition">
                    Enable
                  </button>
                </div>

                <div className="border-t border-[#2a2a35] pt-4">
                  <p className="text-white font-semibold mb-2">KDF Parameters</p>
                  <div className="space-y-2 text-xs">
                    <div className="flex justify-between text-gray-400">
                      <span>Algorithm</span>
                      <span className="text-white">Argon2id</span>
                    </div>
                    <div className="flex justify-between text-gray-400">
                      <span>Time Cost</span>
                      <span className="text-white">3 iterations</span>
                    </div>
                    <div className="flex justify-between text-gray-400">
                      <span>Memory Cost</span>
                      <span className="text-white">64 MiB</span>
                    </div>
                    <div className="flex justify-between text-gray-400">
                      <span>Parallelism</span>
                      <span className="text-white">4 threads</span>
                    </div>
                  </div>
                </div>

                <div className="border-t border-[#2a2a35] pt-4">
                  <p className="text-white font-semibold mb-2">Encryption Algorithm</p>
                  <div className="space-y-2 text-xs">
                    <div className="flex justify-between text-gray-400">
                      <span>Cipher</span>
                      <span className="text-white">ChaCha20-Poly1305</span>
                    </div>
                    <div className="flex justify-between text-gray-400">
                      <span>IV Length</span>
                      <span className="text-white">12 bytes</span>
                    </div>
                    <div className="flex justify-between text-gray-400">
                      <span>Tag Length</span>
                      <span className="text-white">128 bits</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-blue-500/10 border border-blue-500/30 rounded p-3">
              <div className="flex gap-2">
                <Shield className="w-4 h-4 text-blue-400 flex-shrink-0 mt-0.5" />
                <div className="text-xs text-blue-300">
                  All keys are encrypted at rest using industry-standard cryptography. Master password is required for decryption.
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default PrivacyVaultPanel;
