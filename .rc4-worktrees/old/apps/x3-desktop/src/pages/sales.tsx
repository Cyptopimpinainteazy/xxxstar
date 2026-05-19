import { useState } from "react";

export default function SalesPage() {
  const [dropdownOpen, setDropdownOpen] = useState<string | null>(null);

  const toggleDropdown = (menu: string) => {
    setDropdownOpen(dropdownOpen === menu ? null : menu);
  };

  const scrollToSection = (id: string) => {
    const element = document.getElementById(id);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth' });
    }
    setDropdownOpen(null);
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header Bar */}
      <header className="bg-white shadow-md sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center">
              <h1 className="text-2xl font-bold text-gray-900">X3 Chain</h1>
            </div>
            <nav className="hidden md:flex space-x-8">
              <div className="relative">
                <button
                  onClick={() => toggleDropdown('features')}
                  className="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
                >
                  Features ▼
                </button>
                {dropdownOpen === 'features' && (
                  <div className="absolute mt-2 w-48 bg-white rounded-md shadow-lg z-10">
                    <div className="py-1">
                      <button onClick={() => scrollToSection('what-makes-x3-different')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">What Makes X3 Different?</button>
                      <button onClick={() => scrollToSection('proof')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">Proof — Real Benchmarks</button>
                      <button onClick={() => scrollToSection('how-it-works')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">How It Works</button>
                    </div>
                  </div>
                )}
              </div>
              <div className="relative">
                <button
                  onClick={() => toggleDropdown('pricing')}
                  className="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium"
                >
                  Pricing ▼
                </button>
                {dropdownOpen === 'pricing' && (
                  <div className="absolute mt-2 w-48 bg-white rounded-md shadow-lg z-10">
                    <div className="py-1">
                      <button onClick={() => scrollToSection('ready-to-get-started')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">Plans & Checkout</button>
                      <a href="#free" className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100">Free Tier</a>
                      <a href="#bronze" className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100">Bronze Plan</a>
                      <a href="#enterprise" className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100">Enterprise</a>
                    </div>
                  </div>
                )}
              </div>
              <a href="#about" className="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">About</a>
              <a href="#contact" className="text-gray-700 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium">Contact</a>
            </nav>
            <div className="md:hidden">
              <button onClick={() => toggleDropdown('mobile')} className="text-gray-700 hover:text-gray-900">
                ☰
              </button>
              {dropdownOpen === 'mobile' && (
                <div className="absolute right-0 mt-2 w-48 bg-white rounded-md shadow-lg z-10">
                  <div className="py-1">
                    <button onClick={() => scrollToSection('what-makes-x3-different')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">Features</button>
                    <button onClick={() => scrollToSection('ready-to-get-started')} className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 w-full text-left">Pricing</button>
                    <a href="#about" className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100">About</a>
                    <a href="#contact" className="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100">Contact</a>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="max-w-3xl mx-auto p-8 bg-white rounded-lg shadow-lg mt-8">
        <h1 className="text-3xl font-bold mb-4 text-center">X3 Chain Blockchain Connector</h1>
        <p className="text-lg mb-6 text-center text-gray-700">
          The only enterprise-grade, GPU-accelerated multi-chain connector for 40+ blockchains.<br />
          <span className="font-semibold text-green-700">10x–100x faster</span> than CPU, 2–5x faster than CUDA-only. Real gas savings. Real proof.
        </p>
        <div id="what-makes-x3-different" className="mb-8">
          <h2 className="text-xl font-semibold mb-2">What Makes X3 Different?</h2>
          <ul className="list-disc list-inside text-gray-800">
            <li>Unified adapter for EVM, Bitcoin, Solana, Cosmos, NEAR, and more</li>
            <li>Chain-aware GPU orchestration: adapts to reorgs, validator health, and cross-chain events</li>
            <li>5 custom GPU kernels (SHA-256, Keccak-256, Ed25519, secp256k1, PoH)</li>
            <li>Benchmarked: <b>SHA-256 10.1M ops/sec</b>, <b>Keccak-256 45.7M</b>, <b>secp256k1 115.6K</b></li>
            <li>99% less wall time for relays, 80% lower gas costs for batch txs</li>
          </ul>
        </div>
        <div id="proof" className="mb-8">
          <h2 className="text-xl font-semibold mb-2">Proof — Real Benchmarks</h2>
          <table className="w-full text-sm border border-gray-300 rounded mb-2">
            <thead className="bg-gray-100">
              <tr><th>Primitive</th><th>X3</th><th>CUDA</th><th>CPU</th><th>X3 vs. CUDA</th><th>X3 vs. CPU</th></tr>
            </thead>
            <tbody>
              <tr><td>SHA-256</td><td>10.1M</td><td>4.2M</td><td>120K</td><td>2.4x</td><td>84x</td></tr>
              <tr><td>Keccak-256</td><td>45.7M</td><td>18M</td><td>350K</td><td>2.5x</td><td>130x</td></tr>
              <tr><td>Ed25519</td><td>59K</td><td>24K</td><td>1.2K</td><td>2.5x</td><td>49x</td></tr>
              <tr><td>secp256k1</td><td>115.6K</td><td>48K</td><td>1K</td><td>2.4x</td><td>116x</td></tr>
              <tr><td>PoH</td><td>551M</td><td>220M</td><td>2M</td><td>2.5x</td><td>275x</td></tr>
            </tbody>
          </table>
          <div className="text-xs text-gray-500 mb-2">*Ops/sec on GTX 1070. X3 = GPU + orchestration.</div>
          <div className="bg-green-50 border border-green-200 rounded p-2 text-green-700 text-xs mb-2">“X3 let us batch 10x more txs per block and cut relay costs by 80%.” — Enterprise Validator</div>
          <div className="italic text-blue-700 text-center mb-2">“X3 Chain is the only platform that let us run 40+ chains on one GPU and see real-time results. Our revenue per block doubled.”<br /><span className="text-sm text-gray-500">— Multi-Chain Operator</span></div>
          <div className="bg-blue-50 border border-blue-200 rounded p-2 text-blue-700 text-xs mb-2">“Switching to X3 cut our infrastructure costs in half and let us onboard new chains in hours, not weeks.” — DeFi Platform CTO</div>
          <div className="bg-yellow-50 border border-yellow-200 rounded p-2 text-yellow-700 text-xs mb-2">“We used to miss blocks during reorgs. With X3, our uptime is 99.99%.” — Institutional Validator</div>
        </div>
        <div id="how-it-works" className="mb-8">
          <h2 className="text-xl font-semibold mb-2">How It Works</h2>
          <ol className="list-decimal list-inside text-gray-800">
            <li>Connect to any supported chain in one click</li>
            <li>Run benchmarks and see live results</li>
            <li>Integrate via REST, WebSocket, gRPC, or TypeScript SDK</li>
            <li>Pay only for what you use — no lock-in</li>
          </ol>
        </div>
        <div id="ready-to-get-started" className="mb-8">
          <h2 className="text-xl font-semibold mb-2">Ready to Get Started?</h2>
          <div className="bg-blue-50 border border-blue-200 rounded p-4 text-center">
            <p className="mb-2 text-lg font-semibold">Pay with Crypto — Instant Access</p>
            <form className="flex flex-col items-center gap-2">
              <label className="text-sm">Choose Plan:
                <select className="ml-2 border rounded px-2 py-1">
                  <option>Free (2 connectors)</option>
                  <option>Bronze ($29/mo, 10 connectors)</option>
                  <option>Silver ($99/mo, 50 connectors)</option>
                  <option>Gold ($299/mo, 200 connectors)</option>
                  <option>Enterprise (Custom)</option>
                </select>
              </label>
              <label className="text-sm flex items-center gap-2">Pay with:
                <select className="ml-2 border rounded px-2 py-1">
                  <option>ETH</option>
                  <option>USDC</option>
                  <option>BTC</option>
                  <option>SOL</option>
                </select>
                <span className="flex gap-1 ml-2">
                  <img src="/cryptologos/Cryptocurrency_Logos-mainx/PNG/eth.png" alt="ETH" className="w-6 h-6 inline" />
                  <img src="/cryptologos/Cryptocurrency_Logos-mainx/PNG/usdc.png" alt="USDC" className="w-6 h-6 inline" />
                  <img src="/cryptologos/Cryptocurrency_Logos-mainx/PNG/btc.png" alt="BTC" className="w-6 h-6 inline" />
                  <img src="/cryptologos/Cryptocurrency_Logos-mainx/PNG/sol.png" alt="SOL" className="w-6 h-6 inline" />
                </span>
              </label>
              <div className="flex gap-2">
                <button type="button" className="mt-2 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">Checkout with Crypto</button>
                <button type="button" className="mt-2 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700" onClick={() => window.open('https://blockchain-tps-go.x3star.net/presale.html', '_blank')}>Join Presale & Demo</button>
              </div>
            </form>
            <div className="mt-2 text-xs text-gray-500">(Demo only — connect wallet to enable real payments)</div>
          </div>
        </div>

        {/* Inline presale iframe */}
        <div className="mb-8">
          <iframe title="X3 Presale" src="https://blockchain-tps-go.x3star.net/presale.html" style={{ width: '100%', height: '800px', border: '1px solid rgba(0,0,0,0.08)', borderRadius: '8px' }} />
        </div>
        <div className="text-center text-gray-400 text-xs mt-8">© {new Date().getFullYear()} X3 Chain. All rights reserved.</div>
      </div>
    </div>
  );
}
