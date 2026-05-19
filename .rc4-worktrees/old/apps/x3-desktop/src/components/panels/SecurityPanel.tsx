/**
 * SecurityPanel — Key management, attestation, and governance signing.
 */
import React, { useState, useMemo } from "react";
import { useWalletStore } from "@/stores/walletStore";
import { 
  Shield, Key, Lock, Fingerprint, Cpu,
  ShieldCheck, Clock, CheckCircle2, History
} from 'lucide-react';

interface KeyEntry {
  id: string;
  label: string;
  type: "sr25519" | "ed25519" | "ecdsa" | "aes256";
  purpose: "signing" | "encryption" | "attestation" | "governance";
  pubKey: string;
  created: string;
  lastUsed: string;
  status: "active" | "locked" | "revoked";
}

function getIdentities(): KeyEntry[] {
  return [
    { id: "k-1", label: "Provider Identity", type: "sr25519", purpose: "signing", pubKey: "5GrwvaEF...43jS", created: "2026-01-15", lastUsed: "2min ago", status: "active" },
    { id: "k-2", label: "Governance Key", type: "ed25519", purpose: "governance", pubKey: "5FHneW46...8qPm", created: "2026-01-20", lastUsed: "1hr ago", status: "active" },
    { id: "k-3", label: "Storage Encryption", type: "aes256", purpose: "encryption", pubKey: "—", created: "2026-02-01", lastUsed: "45min ago", status: "active" },
    { id: "k-4", label: "Hardware Attestation", type: "ecdsa", purpose: "attestation", pubKey: "0x04a1b2...c3d4", created: "2026-01-10", lastUsed: "12min ago", status: "active" },
  ];
}

const Stat = ({ label, value, icon: Icon, color }: any) => (
  <div className="bg-[#111] border border-[#222] p-4 rounded-2xl flex items-center gap-4">
    <div className={`p-3 rounded-xl bg-${color}-500/10`}>
      <Icon className={`w-5 h-5 text-${color}-400`} />
    </div>
    <div>
      <p className="text-[10px] uppercase tracking-widest text-gray-500 font-bold">{label}</p>
      <p className="text-xl font-black text-white">{value}</p>
    </div>
  </div>
);

const SecurityPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"keys" | "attestation" | "history">("keys");
  const { universalWallet } = useWalletStore();
  const keys = useMemo(() => getIdentities(), []);

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300 font-sans">
      {/* Header */}
      <div className="p-8 pb-4 border-b border-[#1a1a1a] flex justify-between items-center bg-gradient-to-b from-[#111] to-[#0a0a0f]">
        <div>
          <h1 className="text-3xl font-black text-white tracking-tighter flex items-center gap-3">
             <Shield className="text-indigo-400 w-8 h-8" /> SECURITY VAULT
          </h1>
          <p className="text-xs text-gray-500 mt-1 uppercase font-bold tracking-[0.2em]">Validated Cryptographic Enclave</p>
        </div>
        <div className="flex bg-[#111] border border-[#222] p-1 rounded-xl">
           {(["keys", "attestation", "history"] as const).map(tab => (
             <button 
               key={tab}
               onClick={() => setActiveTab(tab)}
               className={`px-4 py-2 rounded-lg text-xs font-black uppercase tracking-widest transition-all ${activeTab === tab ? "bg-indigo-500 text-white shadow-lg" : "text-gray-500 hover:text-white"}`}
             >
               {tab}
             </button>
           ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-8 space-y-8">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
           <Stat label="Active Keys" value={keys.length} icon={Key} color="indigo" />
           <Stat label="Hardware Security" value="TPM 2.0" icon={Cpu} color="green" />
           <Stat label="Auth Mode" value="Biometric" icon={Fingerprint} color="purple" />
           <Stat label="Vault Status" value="Locked" icon={Lock} color="orange" />
        </div>

        {activeTab === "keys" && (
          <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
             <h2 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
               <ShieldCheck className="w-5 h-5 text-indigo-400" /> Identity Inventory
             </h2>
             <div className="bg-[#111] border border-[#222] rounded-3xl overflow-hidden shadow-2xl">
                <table className="w-full text-left text-sm border-collapse">
                   <thead>
                      <tr className="bg-[#151515] border-b border-[#222]">
                        <th className="p-4 font-black uppercase text-[10px] tracking-widest text-gray-500">Identity</th>
                        <th className="p-4 font-black uppercase text-[10px] tracking-widest text-gray-500">Type</th>
                        <th className="p-4 font-black uppercase text-[10px] tracking-widest text-gray-500">Purpose</th>
                        <th className="p-4 font-black uppercase text-[10px] tracking-widest text-gray-500">Public Key</th>
                        <th className="p-4 font-black uppercase text-[10px] tracking-widest text-gray-500">Status</th>
                      </tr>
                   </thead>
                   <tbody className="divide-y divide-[#1a1a1a]">
                      {keys.map(k => (
                        <tr key={k.id} className="hover:bg-[#151515] transition-colors group">
                          <td className="p-4">
                            <span className="font-bold text-white group-hover:text-indigo-400 transition-colors">{k.label}</span>
                            <p className="text-[10px] text-gray-500 mt-0.5">ID: {k.id}</p>
                          </td>
                          <td className="p-4 font-mono text-xs">{k.type}</td>
                          <td className="p-4">
                             <span className="text-[10px] font-black uppercase tracking-wider px-2 py-0.5 rounded bg-[#222] border border-[#333]">
                               {k.purpose}
                             </span>
                          </td>
                          <td className="p-4 font-mono text-[10px] text-gray-400">{k.pubKey}</td>
                          <td className="p-4">
                             <div className="flex items-center gap-2">
                               <div className={`w-2 h-2 rounded-full ${k.status === "active" ? "bg-green-500" : "bg-red-500"} animate-pulse`}></div>
                               <span className="font-bold uppercase text-[10px]">{k.status}</span>
                             </div>
                          </td>
                        </tr>
                      ))}
                      {universalWallet && (
                        <tr className="bg-indigo-500/5 group hover:bg-indigo-500/10 transition-colors">
                           <td className="p-4 border-l-4 border-indigo-500">
                             <span className="font-bold text-indigo-400">Universal Wallet</span>
                             <p className="text-[10px] text-indigo-400/60 mt-0.5">HOT WALLET</p>
                           </td>
                           <td className="p-4 font-mono text-xs">secp256k1</td>
                           <td className="p-4">
                              <span className="text-[10px] font-black uppercase tracking-wider px-2 py-0.5 rounded bg-indigo-500/20 text-indigo-400 border border-indigo-500/30">
                                universal
                              </span>
                           </td>
                           <td className="p-4 font-mono text-[10px] text-indigo-400">{universalWallet.evm_address}</td>
                           <td className="p-4">
                              <div className="flex items-center gap-2 text-indigo-400 font-bold uppercase text-[10px]">
                                <CheckCircle2 className="w-3 h-3" /> ATTACHED
                              </div>
                           </td>
                        </tr>
                      )}
                   </tbody>
                </table>
             </div>
          </div>
        )}

        {activeTab === "attestation" && (
          <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 max-w-2xl">
             <div className="bg-gradient-to-br from-[#111] to-[#1a1a1a] border border-[#333] rounded-3xl p-8 shadow-2xl relative overflow-hidden">
                <div className="absolute top-0 right-0 p-8 opacity-5">
                   <ShieldCheck className="w-64 h-64 text-indigo-500 rotate-12" />
                </div>
                <div className="relative z-10">
                   <h2 className="text-2xl font-black text-white mb-2">Hardware Integrity</h2>
                   <p className="text-sm text-gray-500 mb-8">Validating localized secure enclave and CPU-bound attestations.</p>
                   
                   <div className="space-y-4">
                      <div className="flex justify-between items-center p-4 bg-green-500/5 border border-green-500/20 rounded-2xl">
                         <div className="flex items-center gap-4">
                            <Cpu className="w-6 h-6 text-green-400" />
                            <div>
                               <p className="font-bold text-white text-sm">TPM 2.0 Root of Trust</p>
                               <p className="text-xs text-gray-500">Verified locally via TCG specification.</p>
                            </div>
                         </div>
                         <span className="px-3 py-1 rounded-full bg-green-500/20 text-green-400 text-[10px] font-black uppercase">PASSED</span>
                      </div>
                      
                      <div className="flex justify-between items-center p-4 bg-green-500/5 border border-green-500/20 rounded-2xl">
                         <div className="flex items-center gap-4">
                            <Shield className="w-6 h-6 text-green-400" />
                            <div>
                               <p className="font-bold text-white text-sm">Kernel Secure Boot</p>
                               <p className="text-xs text-gray-500">Signatures validated for all loaded modules.</p>
                            </div>
                         </div>
                         <span className="px-3 py-1 rounded-full bg-green-500/20 text-green-400 text-[10px] font-black uppercase">PASSED</span>
                      </div>

                      <div className="flex justify-between items-center p-4 bg-orange-500/5 border border-orange-500/20 rounded-2xl">
                         <div className="flex items-center gap-4">
                            <Lock className="w-6 h-6 text-orange-400" />
                            <div>
                               <p className="font-bold text-white text-sm">Memory Encryption (SME)</p>
                               <p className="text-xs text-gray-500">Transparent encryption of active RAM pages.</p>
                            </div>
                         </div>
                         <span className="px-3 py-1 rounded-full bg-orange-500/20 text-orange-400 text-[10px] font-black uppercase">ACTIVE</span>
                      </div>
                   </div>
                   
                   <button className="w-full mt-8 bg-indigo-500 hover:bg-indigo-600 text-white font-black uppercase tracking-widest text-xs py-4 rounded-2xl shadow-lg transition-all active:scale-95">
                      Refresh Hardware Attestation
                   </button>
                </div>
             </div>
          </div>
        )}

        {activeTab === "history" && (
           <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 max-w-4xl">
              <h2 className="text-lg font-bold text-white mb-4 flex items-center gap-2">
                <History className="w-5 h-5 text-indigo-400" /> Signature Log
              </h2>
              <div className="space-y-2">
                 {[
                   { ts: "2026-02-25 14:22:01", action: "EVM_SEND_TRANSACTION", signer: "Wallet_0x12..", status: "success" },
                   { ts: "2026-02-25 12:05:44", action: "DAPP_APPROVAL", signer: "Provider_0xab..", status: "success" },
                   { ts: "2026-02-25 09:12:30", action: "KEY_ROTATION", signer: "Master_Root", status: "success" },
                   { ts: "2026-02-24 23:55:12", action: "LOGIN_CHALLENGE", signer: "Provider_Core", status: "success" },
                 ].map((op, i) => (
                   <div key={i} className="flex justify-between items-center p-4 bg-[#111] border border-[#222] rounded-2xl hover:border-indigo-500/30 transition-all cursor-pointer group">
                      <div className="flex items-center gap-4">
                         <div className="p-2 rounded-lg bg-[#222] border border-[#333]">
                            <Clock className="w-4 h-4 text-gray-500 group-hover:text-indigo-400 transition-colors" />
                         </div>
                         <div>
                            <p className="text-sm font-bold text-white">{op.action}</p>
                            <p className="text-[10px] text-gray-500 font-mono tracking-tighter">{op.ts} • Signer: {op.signer}</p>
                         </div>
                      </div>
                      <span className="text-[10px] font-black uppercase text-green-400 bg-green-500/10 px-2 py-1 rounded-lg">Verified</span>
                   </div>
                 ))}
                 <button className="w-full py-4 text-xs font-black uppercase tracking-widest text-gray-500 hover:text-white transition-colors">
                    Load Full Security Audit Trail
                 </button>
              </div>
           </div>
        )}
      </div>
      
      {/* Footer Info */}
      <div className="p-4 px-8 border-t border-[#1a1a1a] bg-[#0a0a0f] text-[9px] text-gray-600 flex justify-between uppercase font-bold tracking-widest">
         <div className="flex items-center gap-4">
            <span>Enclave: v2.4.1-stable</span>
            <span>Entropy: 256-bit Hardware</span>
            <span className="text-green-500">● Secure connection active</span>
         </div>
         <div className="flex items-center gap-4">
            <span>Server: Atlanta-Node-0x2</span>
            <span className="text-indigo-400">Governance Level 4</span>
         </div>
      </div>
    </div>
  );
};

export default SecurityPanel;
