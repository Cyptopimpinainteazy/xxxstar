import React, { useState } from "react";
import { Lock, Eye, EyeOff, Key, Zap, Copy, Settings, Shield, Trash2 } from "lucide-react";
import clsx from "clsx";

interface PrivateKey {
  id: string;
  name: string;
  type: "x3-native" | "evm" | "svm";
  address: string;
  encrypted: boolean;
  lastAccessed: string;
  derivation?: string;
}

interface StealthAddress {
  id: string;
  label: string;
  stealthAddress: string;
  viewingKey: string;
  spendingKey: string;
  balance: number;
  transactions: number;
  status: "active" | "inactive";
}

interface PrivacySettings {
  encryptionAlgorithm: string;
  keyDerivation: string;
  mixingLevel: "low" | "medium" | "high";
  autoShred: boolean;
  biometricUnlock: boolean;
}

const MOCK_KEYS: PrivateKey[] = [
  {
    id: "1",
    name: "Main Wallet",
    type: "x3-native",
    address: "x3c7b...2f4a",
    encrypted: true,
    lastAccessed: "2 mins ago",
    derivation: "m/44'/1399'/0'/0/0",
  },
  {
    id: "2",
    name: "Trading Account",
    type: "evm",
    address: "0x742d...f595",
    encrypted: true,
    lastAccessed: "1 hour ago",
    derivation: "m/44'/60'/0'/0/0",
  },
];

const MOCK_STEALTH: StealthAddress[] = [
  {
    id: "1",
    label: "Privacy Receiver 1",
    stealthAddress: "stealth:x3p1...k9m2",
    viewingKey: "vk:0x3a4b...7c9d",
    spendingKey: "sk:0x5e6f...2a1b",
    balance: 42.5,
    transactions: 12,
    status: "active",
  },
  {
    id: "2",
    label: "Privacy Receiver 2",
    stealthAddress: "stealth:x3p2...w4n7",
    viewingKey: "vk:0x8b9c...5d6e",
    spendingKey: "sk:0x1f2g...8h9i",
    balance: 0,
    transactions: 0,
    status: "inactive",
  },
];

const MOCK_SETTINGS: PrivacySettings = {
  encryptionAlgorithm: "ChaCha20-Poly1305",
  keyDerivation: "Argon2id (memory: 64MB, time: 3)",
  mixingLevel: "high",
  autoShred: true,
  biometricUnlock: true,
};

export default function PrivacyVaultPanel() {
  const [keys, setKeys] = useState<PrivateKey[]>(MOCK_KEYS);
  const [stealth, setStealth] = useState<StealthAddress[]>(MOCK_STEALTH);
  const [settings, setSettings] = useState<PrivacySettings>(MOCK_SETTINGS);
  const [activeTab, setActiveTab] = useState<"keys" | "stealth" | "settings">("keys");
  const [showSecrets, setShowSecrets] = useState<string | null>(null);
  const [selectedKey, setSelectedKey] = useState<PrivateKey | null>(keys[0]);

  const handleDeleteKey = (id: string) => {
    setKeys(keys.filter((k) => k.id !== id));
    if (selectedKey?.id === id) setSelectedKey(null);
  };

  const handleGenerateStealth = () => {
    const newStealth: StealthAddress = {
      id: (stealth.length + 1).toString(),
      label: `Privacy Receiver ${stealth.length + 1}`,
      stealthAddress: `stealth:x3p${stealth.length + 1}...${Math.random().toString(36).substring(2, 7)}`,
      viewingKey: `vk:0x${Math.random().toString(16).substring(2, 12)}`,
      spendingKey: `sk:0x${Math.random().toString(16).substring(2, 12)}`,
      balance: 0,
      transactions: 0,
      status: "active",
    };
    setStealth([...stealth, newStealth]);
  };

  const encryptedCount = keys.filter((k) => k.encrypted).length;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Lock size={20} className="text-red-400" /> Privacy Vault
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Keys</div>
            <div className="text-lg font-bold text-cyan-400">{keys.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Encrypted</div>
            <div className="text-lg font-bold text-green-400">{encryptedCount}/{keys.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Stealth Addrs</div>
            <div className="text-lg font-bold text-purple-400">{stealth.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Status</div>
            <div className="text-lg font-bold text-green-400">🔒 Secured</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["keys", "stealth", "settings"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2 capitalize",
                activeTab === tab ? "border-red-600 text-red-400" : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "stealth" ? "Stealth Addresses" : tab}
            </button>
          ))}
        </div>

        {activeTab === "keys" && (
          <div className="space-y-2">
            {keys.map((key) => (
              <div
                key={key.id}
                onClick={() => setSelectedKey(key)}
                className={clsx(
                  "p-3 rounded-lg border transition cursor-pointer",
                  selectedKey?.id === key.id
                    ? "border-red-600 bg-red-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-red-600/50"
                )}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{key.name}</div>
                    <div className="text-xs text-gray-400 font-mono">{key.address}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded-md font-bold",
                      key.encrypted ? "bg-green-600/20 text-green-400" : "bg-yellow-600/20 text-yellow-400"
                    )}
                  >
                    {key.encrypted ? "🔒" : "⚠️"}
                  </span>
                </div>

                <div className="flex justify-between text-xs text-gray-500 mb-2">
                  <span>{key.type}</span>
                  <span>{key.lastAccessed}</span>
                </div>

                {selectedKey?.id === key.id && (
                  <div className="bg-[#0a0a0f] rounded p-2 space-y-1 mb-2">
                    <div className="text-xs text-gray-400">Derivation Path</div>
                    <div className="text-xs font-mono text-cyan-400">{key.derivation}</div>
                  </div>
                )}

                <div className="flex gap-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setShowSecrets(showSecrets === key.id ? null : key.id);
                    }}
                    className="text-xs text-gray-400 hover:text-cyan-400 transition flex items-center gap-1"
                  >
                    {showSecrets === key.id ? <EyeOff size={12} /> : <Eye size={12} />}
                    View Key
                  </button>
                  <button className="text-xs text-gray-400 hover:text-cyan-400 transition flex items-center gap-1">
                    <Copy size={12} /> Copy
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteKey(key.id);
                    }}
                    className="text-xs text-gray-400 hover:text-red-400 transition flex items-center gap-1"
                  >
                    <Trash2 size={12} /> Delete
                  </button>
                </div>
              </div>
            ))}

            {/* Import Button */}
            <button className="w-full border border-dashed border-[#2a2a35] hover:border-red-600 rounded-lg p-3 text-center text-sm font-semibold text-gray-400 hover:text-red-400 transition">
              + Import Private Key
            </button>
          </div>
        )}

        {activeTab === "stealth" && (
          <div className="space-y-2">
            {stealth.map((addr) => (
              <div key={addr.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <div className="font-semibold text-sm">{addr.label}</div>
                    <div className="text-xs text-gray-400 font-mono">{addr.stealthAddress}</div>
                  </div>
                  <span
                    className={clsx(
                      "text-xs px-2 py-1 rounded-md font-bold",
                      addr.status === "active" ? "bg-green-600/20 text-green-400" : "bg-gray-600/20 text-gray-400"
                    )}
                  >
                    {addr.status}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-2 mb-2 text-xs">
                  <div>
                    <div className="text-gray-400">Balance</div>
                    <div className="font-bold text-cyan-400">{addr.balance} X3</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Transactions</div>
                    <div className="font-bold text-purple-400">{addr.transactions}</div>
                  </div>
                </div>

                <div className="space-y-1 text-xs mb-2">
                  <div className="text-gray-500">Viewing Key: <span className="font-mono text-gray-400 truncate">{addr.viewingKey}</span></div>
                  <div className="text-gray-500">Spending Key: <span className="font-mono text-gray-400 truncate">{addr.spendingKey}</span></div>
                </div>

                <div className="flex gap-2">
                  <button className="text-xs text-gray-400 hover:text-cyan-400 transition">Copy Address</button>
                  <button className="text-xs text-gray-400 hover:text-red-400 transition">Revoke</button>
                </div>
              </div>
            ))}

            <button
              onClick={handleGenerateStealth}
              className="w-full bg-[#15151b] border border-[#2a2a35] hover:border-red-600 rounded-lg p-3 text-center text-sm font-semibold text-gray-400 hover:text-red-400 transition"
            >
              + Generate New Stealth Address
            </button>
          </div>
        )}

        {activeTab === "settings" && (
          <div className="space-y-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm flex items-center gap-2">
                <Shield size={16} className="text-red-400" /> Security Settings
              </h3>

              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Encryption Algorithm</span>
                  <span className="font-mono text-xs">{settings.encryptionAlgorithm}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Key Derivation</span>
                  <span className="font-mono text-xs">{settings.keyDerivation}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Mixing Level</span>
                  <span className="font-semibold uppercase text-yellow-400">{settings.mixingLevel}</span>
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <label className="flex items-center p-3 bg-[#15151b] border border-[#2a2a35] rounded-lg cursor-pointer hover:border-red-600 transition">
                <input type="checkbox" defaultChecked={settings.autoShred} className="w-4 h-4 accent-red-600 mr-3" />
                <div className="flex-1">
                  <div className="text-xs font-semibold">Auto-Shred Keys on Logout</div>
                  <div className="text-xs text-gray-500">Securely erase keys from memory</div>
                </div>
              </label>

              <label className="flex items-center p-3 bg-[#15151b] border border-[#2a2a35] rounded-lg cursor-pointer hover:border-red-600 transition">
                <input type="checkbox" defaultChecked={settings.biometricUnlock} className="w-4 h-4 accent-red-600 mr-3" />
                <div className="flex-1">
                  <div className="text-xs font-semibold">Biometric Unlock</div>
                  <div className="text-xs text-gray-500">Use Face ID / fingerprint for vault access</div>
                </div>
              </label>
            </div>

            <button className="w-full bg-red-600/20 border border-red-600 hover:bg-red-600/30 rounded-lg p-3 text-sm font-semibold text-red-400 transition">
              🔄 Rotate All Encryption Keys
            </button>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        End-to-end encrypted private key vault with stealth address generation and Argon2id key derivation.
      </div>
    </div>
  );
}
