import React, { useState } from 'react';
import { Dna, FileCode, ShieldCheck, Cpu, Play, RotateCcw, Zap, Gavel, TrendingUp, Clock } from 'lucide-react';

type Tab = 'evolution' | 'scripts' | 'verifier' | 'swarm';
type SwarmSub = 'gpu' | 'auctions' | 'predictions';

const evolutionEvents = [
  { version: 'v0.1.0', changes: 'Initial genesis mutation — base neural scaffold deployed with 12 attention heads and cross-VM perception bridge.', fitnessDelta: +0.42 },
  { version: 'v0.2.0', changes: 'Memory consolidation upgrade — long-term state compression and selective recall for context windows up to 128k tokens.', fitnessDelta: +0.18 },
  { version: 'v0.3.0', changes: 'Multi-modal integration — added vision encoder and audio preprocessing pipelines to the inference stack.', fitnessDelta: +0.31 },
  { version: 'v0.4.0', changes: 'Swarm consensus layer — distributed decision-making across GPU nodes with Byzantine fault tolerance.', fitnessDelta: +0.15 },
  { version: 'v0.5.0', changes: 'Quantum-resistant signing — lattice-based cryptographic verification for all X3 inter-agent communications.', fitnessDelta: +0.09 },
];

const scriptTemplates = [
  { name: 'Balance Check', code: `// Check cross-VM balance\nconst evm = await x3.query("evm", { method: "eth_getBalance", params: [address] });\nconst svm = await x3.query("svm", { method: "getBalance", params: [pubkey] });\nconsole.log("EVM:", evm, "SVM:", svm);` },
  { name: 'Swap', code: `// Execute atomic swap\nconst comit = x3.comit([\n  { vm: "evm", action: "approve", token: "USDC", amount: "1000" },\n  { vm: "svm", action: "swap", pair: "USDC/X3", amount: "1000" },\n]);\nawait comit.execute();` },
  { name: 'Bridge', code: `// Bridge assets EVM → SVM\nconst bridge = await x3.bridge({\n  from: "evm", to: "svm",\n  token: "X3", amount: "500",\n  recipient: svmPubkey,\n});\nconsole.log("Bridge tx:", bridge.hash);` },
];

const verifications = [
  { id: 'vrfy-001', proof: 'ZKP-Groth16', block: 1284520, status: 'verified', time: '2.3s' },
  { id: 'vrfy-002', proof: 'STARK-FRI', block: 1284519, status: 'verified', time: '4.1s' },
  { id: 'vrfy-003', proof: 'ZKP-Plonk', block: 1284518, status: 'verified', time: '1.8s' },
  { id: 'vrfy-004', proof: 'ZKP-Groth16', block: 1284517, status: 'pending', time: '—' },
  { id: 'vrfy-005', proof: 'STARK-FRI', block: 1284516, status: 'verified', time: '3.9s' },
  { id: 'vrfy-006', proof: 'ZKP-Plonk', block: 1284515, status: 'failed', time: '—' },
  { id: 'vrfy-007', proof: 'ZKP-Groth16', block: 1284514, status: 'verified', time: '2.0s' },
  { id: 'vrfy-008', proof: 'STARK-FRI', block: 1284513, status: 'verified', time: '5.2s' },
];

const gpuNodes = [
  { id: 'gpu-01', name: 'Sphere-A100-01', gpu: 'A100 80GB', utilization: 87, jobs: 142, earnings: 2450 },
  { id: 'gpu-02', name: 'Sphere-A100-02', gpu: 'A100 80GB', utilization: 92, jobs: 168, earnings: 2890 },
  { id: 'gpu-03', name: 'Sphere-H100-01', gpu: 'H100 80GB', utilization: 95, jobs: 201, earnings: 4100 },
  { id: 'gpu-04', name: 'Sphere-4090-01', gpu: 'RTX 4090', utilization: 78, jobs: 89, earnings: 1200 },
  { id: 'gpu-05', name: 'Sphere-4090-02', gpu: 'RTX 4090', utilization: 65, jobs: 67, earnings: 980 },
];

const auctions = [
  { id: 'auc-01', model: 'Llama-70B', requester: '0x4a2f...', bidFloor: 0.05, topBid: 0.12, timeLeft: '2m 34s', bids: 8 },
  { id: 'auc-02', model: 'Stable Diffusion XL', requester: '0x8bc1...', bidFloor: 0.02, topBid: 0.04, timeLeft: '5m 12s', bids: 3 },
  { id: 'auc-03', model: 'Whisper Large', requester: '0xd3e5...', bidFloor: 0.01, topBid: 0.02, timeLeft: '8m 45s', bids: 5 },
  { id: 'auc-04', model: 'CodeLlama-34B', requester: '0x1f9a...', bidFloor: 0.03, topBid: 0.08, timeLeft: '1m 02s', bids: 12 },
];

const predictions = [
  { id: 'pred-01', question: 'X3 > $50 by Q2 2026?', yesPrice: 0.62, volume: 125000, endDate: 'Jun 30, 2026' },
  { id: 'pred-02', question: 'GPU Swarm > 1000 nodes by March?', yesPrice: 0.78, volume: 45000, endDate: 'Mar 31, 2026' },
  { id: 'pred-03', question: 'EVM-SVM bridge TVL > $100M?', yesPrice: 0.45, volume: 88000, endDate: 'Apr 15, 2026' },
  { id: 'pred-04', question: 'X3 mainnet TPS > 10,000?', yesPrice: 0.33, volume: 62000, endDate: 'May 01, 2026' },
];

const X3SubPagesPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<Tab>('evolution');
  const [swarmSub, setSwarmSub] = useState<SwarmSub>('gpu');
  const [scriptCode, setScriptCode] = useState(scriptTemplates[0].code);
  const [scriptOutput, setScriptOutput] = useState('');

  const tabs: { key: Tab; label: string; icon: React.ReactNode }[] = [
    { key: 'evolution', label: 'Evolution', icon: <Dna size={14} /> },
    { key: 'scripts', label: 'Scripts', icon: <FileCode size={14} /> },
    { key: 'verifier', label: 'Verifier', icon: <ShieldCheck size={14} /> },
    { key: 'swarm', label: 'Swarm', icon: <Cpu size={14} /> },
  ];

  const handleRun = () => {
    setScriptOutput('> Executing script...\n> Connected to X3 devnet (127.0.0.1:9944)\n> EVM Balance: 142.58 X3\n> SVM Balance: 89.32 X3\n> Script completed in 0.34s');
  };

  const statusColor = (s: string) => s === 'verified' ? 'text-green-400' : s === 'pending' ? 'text-yellow-400' : 'text-red-400';

  return (
    <div className="flex flex-col h-full bg-[#0a0a0f] text-gray-300">
      <div className="flex items-center gap-4 px-5 py-3 border-b border-[#1a1a1a]">
        <Zap size={18} className="text-[#ff6b35]" />
        <h1 className="text-lg font-semibold text-white">X3 Intelligence</h1>
        <div className="flex gap-1 ml-4">
          {tabs.map(t => (
            <button key={t.key} onClick={() => setActiveTab(t.key)}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-sm rounded transition-colors ${activeTab === t.key ? 'bg-[#ff6b35]/10 text-[#ff6b35]' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}`}>
              {t.icon} {t.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-5">
        {activeTab === 'evolution' && (
          <div className="max-w-3xl">
            <h2 className="text-lg font-bold text-white mb-1">Mutation Timeline</h2>
            <p className="text-xs text-gray-500 mb-4">Current Generation: 5 | Total Fitness Score: {evolutionEvents.reduce((s, e) => s + e.fitnessDelta, 0).toFixed(2)}</p>
            <div className="space-y-1">
              {evolutionEvents.map((ev, i) => (
                <div key={i} className="flex gap-4 group">
                  <div className="flex flex-col items-center">
                    <div className="w-3 h-3 rounded-full bg-[#ff6b35] border-2 border-[#0a0a0f] z-10" />
                    {i < evolutionEvents.length - 1 && <div className="w-px flex-1 bg-[#1a1a1a]" />}
                  </div>
                  <div className="pb-5 flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-sm font-semibold text-white">{ev.version}</span>
                      <span className={`text-xs px-1.5 py-0.5 rounded ${ev.fitnessDelta > 0.2 ? 'bg-green-500/10 text-green-400' : 'bg-blue-500/10 text-blue-400'}`}>
                        +{ev.fitnessDelta.toFixed(2)} fitness
                      </span>
                    </div>
                    <p className="text-xs text-gray-400">{ev.changes}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'scripts' && (
          <div className="max-w-3xl">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-lg font-bold text-white">Script Editor</h2>
              <div className="flex gap-2">
                {scriptTemplates.map((t, i) => (
                  <button key={i} onClick={() => { setScriptCode(t.code); setScriptOutput(''); }}
                    className="text-xs px-2 py-1 bg-[#111118] border border-[#1a1a1a] rounded text-gray-400 hover:text-gray-200 hover:border-gray-600 transition-colors">
                    {t.name}
                  </button>
                ))}
              </div>
            </div>
            <textarea
              value={scriptCode}
              onChange={e => setScriptCode(e.target.value)}
              className="w-full h-48 bg-[#050508] border border-[#1a1a1a] rounded-lg p-4 text-sm font-mono text-green-400/80 resize-none focus:outline-none focus:border-[#ff6b35]/30"
              spellCheck={false}
            />
            <div className="flex gap-2 my-3">
              <button onClick={handleRun} className="flex items-center gap-1.5 px-4 py-1.5 bg-[#ff6b35] text-white text-sm rounded hover:bg-[#ff6b35]/80 transition-colors">
                <Play size={12} /> Run
              </button>
              <button onClick={() => setScriptOutput('')} className="flex items-center gap-1.5 px-3 py-1.5 border border-[#1a1a1a] text-gray-400 text-sm rounded hover:border-gray-600 transition-colors">
                <RotateCcw size={12} /> Clear
              </button>
            </div>
            <div className="bg-[#050508] border border-[#1a1a1a] rounded-lg p-4 min-h-[100px]">
              <p className="text-xs text-gray-500 mb-1">Output</p>
              <pre className="text-xs font-mono text-gray-400 whitespace-pre-wrap">{scriptOutput || 'No output yet. Click Run to execute.'}</pre>
            </div>
          </div>
        )}

        {activeTab === 'verifier' && (
          <div className="max-w-3xl">
            <div className="grid grid-cols-3 gap-3 mb-5">
              <div className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <p className="text-xs text-gray-500">Proofs Verified</p>
                <p className="text-xl font-bold text-green-400">14,892</p>
              </div>
              <div className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <p className="text-xs text-gray-500">Pending</p>
                <p className="text-xl font-bold text-yellow-400">23</p>
              </div>
              <div className="p-3 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                <p className="text-xs text-gray-500">Failed</p>
                <p className="text-xl font-bold text-red-400">7</p>
              </div>
            </div>
            <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs uppercase tracking-wider">
                    <th className="text-left px-4 py-2.5">ID</th>
                    <th className="text-left px-4 py-2.5">Proof Type</th>
                    <th className="text-right px-4 py-2.5">Block</th>
                    <th className="text-center px-4 py-2.5">Status</th>
                    <th className="text-right px-4 py-2.5">Time</th>
                  </tr>
                </thead>
                <tbody>
                  {verifications.map((v, i) => (
                    <tr key={i} className="border-b border-[#1a1a1a]/50 hover:bg-white/[0.02]">
                      <td className="px-4 py-2 font-mono text-xs text-gray-400">{v.id}</td>
                      <td className="px-4 py-2 text-white">{v.proof}</td>
                      <td className="px-4 py-2 text-right font-mono text-xs">#{v.block.toLocaleString()}</td>
                      <td className={`px-4 py-2 text-center text-xs font-medium ${statusColor(v.status)}`}>{v.status}</td>
                      <td className="px-4 py-2 text-right text-xs text-gray-400">{v.time}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {activeTab === 'swarm' && (
          <div className="max-w-4xl">
            <div className="flex gap-2 mb-4">
              {([['gpu', 'GPU Nodes', <Cpu size={12} key="g" />], ['auctions', 'Auctions', <Gavel size={12} key="a" />], ['predictions', 'Predictions', <TrendingUp size={12} key="p" />]] as [SwarmSub, string, React.ReactNode][]).map(([key, label, icon]) => (
                <button key={key} onClick={() => setSwarmSub(key)}
                  className={`flex items-center gap-1.5 px-3 py-1.5 text-xs rounded transition-colors ${swarmSub === key ? 'bg-[#ff6b35]/10 text-[#ff6b35] border border-[#ff6b35]/30' : 'text-gray-400 border border-[#1a1a1a] hover:border-gray-600'}`}>
                  {icon} {label}
                </button>
              ))}
            </div>

            {swarmSub === 'gpu' && (
              <div className="bg-[#111118] border border-[#1a1a1a] rounded-lg overflow-hidden">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs uppercase tracking-wider">
                      <th className="text-left px-4 py-2.5">Node</th>
                      <th className="text-left px-4 py-2.5">GPU</th>
                      <th className="text-right px-4 py-2.5">Utilization</th>
                      <th className="text-right px-4 py-2.5">Jobs</th>
                      <th className="text-right px-4 py-2.5">Earnings (X3)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {gpuNodes.map(n => (
                      <tr key={n.id} className="border-b border-[#1a1a1a]/50 hover:bg-white/[0.02]">
                        <td className="px-4 py-2 text-white font-medium">{n.name}</td>
                        <td className="px-4 py-2 text-gray-400">{n.gpu}</td>
                        <td className="px-4 py-2 text-right">
                          <div className="flex items-center justify-end gap-2">
                            <div className="w-16 h-1.5 bg-[#1a1a1a] rounded-full overflow-hidden">
                              <div className="h-full bg-[#ff6b35] rounded-full" style={{ width: `${n.utilization}%` }} />
                            </div>
                            <span className="text-xs">{n.utilization}%</span>
                          </div>
                        </td>
                        <td className="px-4 py-2 text-right">{n.jobs}</td>
                        <td className="px-4 py-2 text-right text-[#ff6b35]">{n.earnings.toLocaleString()}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {swarmSub === 'auctions' && (
              <div className="space-y-3">
                {auctions.map(a => (
                  <div key={a.id} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="text-sm font-semibold text-white">{a.model}</h3>
                      <span className="flex items-center gap-1 text-xs text-yellow-400"><Clock size={10} /> {a.timeLeft}</span>
                    </div>
                    <div className="grid grid-cols-4 gap-4 text-xs">
                      <div><span className="text-gray-500">Requester</span><p className="text-gray-300 font-mono">{a.requester}</p></div>
                      <div><span className="text-gray-500">Bid Floor</span><p className="text-gray-300">{a.bidFloor} X3</p></div>
                      <div><span className="text-gray-500">Top Bid</span><p className="text-[#ff6b35] font-semibold">{a.topBid} X3</p></div>
                      <div><span className="text-gray-500">Bids</span><p className="text-gray-300">{a.bids}</p></div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {swarmSub === 'predictions' && (
              <div className="space-y-3">
                {predictions.map(p => (
                  <div key={p.id} className="p-4 bg-[#111118] border border-[#1a1a1a] rounded-lg">
                    <h3 className="text-sm font-semibold text-white mb-2">{p.question}</h3>
                    <div className="flex items-center gap-6 text-xs">
                      <div>
                        <span className="text-gray-500">Yes Price: </span>
                        <span className="text-green-400 font-semibold">${p.yesPrice.toFixed(2)}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">No Price: </span>
                        <span className="text-red-400 font-semibold">${(1 - p.yesPrice).toFixed(2)}</span>
                      </div>
                      <div>
                        <span className="text-gray-500">Volume: </span>
                        <span className="text-gray-300">${(p.volume / 1000).toFixed(0)}K</span>
                      </div>
                      <div>
                        <span className="text-gray-500">Ends: </span>
                        <span className="text-gray-300">{p.endDate}</span>
                      </div>
                    </div>
                    <div className="mt-2 h-2 bg-[#1a1a1a] rounded-full overflow-hidden">
                      <div className="h-full bg-green-500/40 rounded-full" style={{ width: `${p.yesPrice * 100}%` }} />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default X3SubPagesPanel;
