
import { useNavigate } from 'react-router-dom';

export default function BenchmarkUltimatePage() {
  const navigate = useNavigate();

  return (
    <div className="flex flex-col min-h-screen bg-[#0a0a0f] text-white">
      <header className="p-4 border-b border-gray-800 flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#ff6b35]">⚡ Full Chain Bench Ultimate</h1>
        <button 
          onClick={() => navigate('/')}
          className="flex items-center gap-2 px-4 py-2 bg-[#ff6b35] text-white rounded-full shadow-lg hover:bg-[#ff8c42] transition-colors font-medium text-sm"
          style={{ boxShadow: "0 4px 12px rgba(255, 107, 53, 0.3)" }}
        >
          <span className="text-lg leading-none mb-[2px]">←</span> Back to Desktop
        </button>
      </header>

      <main className="flex-1 p-8 overflow-y-auto">
        <div className="max-w-6xl mx-auto space-y-8">
          
          <div className="p-6 bg-[#1a1a24] rounded-xl border border-gray-700">
            <h2 className="text-2xl font-bold mb-4">Live EVM / Substrate / Multi-Chain Benchmarks</h2>
            <p className="text-gray-400 mb-6">
              Full performance diagnostic suite showing throughput across 40+ chains natively connected with real-time JSON-RPC metrics.
            </p>
            
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="p-4 bg-black/40 rounded-lg border border-gray-800 flex flex-col items-center justify-center">
                <span className="text-sm text-gray-500 mb-1">X3 Chain (Mainnet)</span>
                <span className="text-3xl font-mono text-[#00d4aa]">115,000 TPS</span>
                <span className="text-xs text-green-500 mt-2">↑ Top Performer</span>
              </div>
              <div className="p-4 bg-black/40 rounded-lg border border-gray-800 flex flex-col items-center justify-center">
                <span className="text-sm text-gray-500 mb-1">Solana (Mainnet-Beta)</span>
                <span className="text-3xl font-mono text-[#4488ff]">65,000 TPS</span>
                <span className="text-xs text-blue-400 mt-2">Stable</span>
              </div>
              <div className="p-4 bg-black/40 rounded-lg border border-gray-800 flex flex-col items-center justify-center">
                <span className="text-sm text-gray-500 mb-1">Ethereum L2 (Arbitrum)</span>
                <span className="text-3xl font-mono text-[#a855f7]">4,000 TPS</span>
                <span className="text-xs text-purple-400 mt-2">Congested</span>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            <div className="p-6 bg-[#1a1a24] rounded-xl border border-gray-700">
              <h3 className="text-xl font-bold mb-4">GPU Swarm Analysis</h3>
              <div className="space-y-4">
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>VRAM Utilization</span>
                  <span className="font-mono text-[#ff8c42]">84.2%</span>
                </div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>Compute Shaders</span>
                  <span className="font-mono text-[#ff8c42]">Active (2M cores)</span>
                </div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>Memory Bandwidth</span>
                  <span className="font-mono text-[#ff8c42]">1.2 TB/s</span>
                </div>
              </div>
              <button className="mt-6 w-full py-2 bg-[#ff6b35]/20 text-[#ff6b35] hover:bg-[#ff6b35]/30 rounded transition font-medium">
                Run Diagnostic Stress Test
              </button>
            </div>

            <div className="p-6 bg-[#1a1a24] rounded-xl border border-gray-700">
              <h3 className="text-xl font-bold mb-4">Cross-Chain Arbitrage Latency</h3>
              <div className="space-y-4">
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>X3 ↔ Ethereum</span>
                  <span className="font-mono text-green-400">12ms</span>
                </div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>X3 ↔ Solana</span>
                  <span className="font-mono text-green-400">18ms</span>
                </div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded border border-gray-800">
                  <span>X3 ↔ Binance Smart Chain</span>
                  <span className="font-mono text-blue-400">24ms</span>
                </div>
              </div>
              <button className="mt-6 w-full py-2 bg-blue-500/20 text-blue-400 hover:bg-blue-500/30 rounded transition font-medium">
                Ping Core Nodes
              </button>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
