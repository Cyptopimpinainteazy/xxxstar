import React, { useState } from "react";
import { Route, Zap, ArrowRight, BarChart3, Shield } from "lucide-react";
import clsx from "clsx";

interface AMM {
  id: string;
  name: string;
  liquidity: number;
  fee: number;
  slippage: number;
}

interface RouteOption {
  id: string;
  path: string[];
  expectedOutput: number;
  priceImpact: number;
  gasEstimate: number;
  mevRisk: "low" | "medium" | "high";
}

const MOCK_AMMS: AMM[] = [
  { id: "1", name: "Uniswap V3", liquidity: 2500000, fee: 0.01, slippage: 0.15 },
  { id: "2", name: "Raydium", liquidity: 1200000, fee: 0.025, slippage: 0.22 },
  { id: "3", name: "X3 DEX", liquidity: 890000, fee: 0.02, slippage: 0.18 },
];

const MOCK_ROUTES: RouteOption[] = [
  {
    id: "1",
    path: ["X3", "→", "USDC", "→", "ETH"],
    expectedOutput: 0.253,
    priceImpact: 0.18,
    gasEstimate: 45000,
    mevRisk: "low",
  },
  {
    id: "2",
    path: ["X3", "→", "USDT", "→", "USDC", "→", "ETH"],
    expectedOutput: 0.251,
    priceImpact: 0.21,
    gasEstimate: 78000,
    mevRisk: "medium",
  },
  {
    id: "3",
    path: ["X3", "→", "ETH"],
    expectedOutput: 0.248,
    priceImpact: 0.35,
    gasEstimate: 28000,
    mevRisk: "high",
  },
];

export default function AdvancedDexPanel() {
  const [routes, setRoutes] = useState<RouteOption[]>(MOCK_ROUTES);
  const [selectedRoute, setSelectedRoute] = useState<RouteOption | null>(MOCK_ROUTES[0]);
  const [inputAmount, setInputAmount] = useState(1000);
  const [mevProtection, setMevProtection] = useState(true);
  const [slippageTolerance, setSlippageTolerance] = useState(0.5);

  const bestRoute = routes[0];
  const totalSavings = routes.reduce((sum, r, idx) => sum + (bestRoute.expectedOutput - r.expectedOutput), 0);

  const handleExecuteSwap = () => {
    alert(
      `Executing swap via best route: ${selectedRoute?.path.join(" ")}. Expected output: ${selectedRoute?.expectedOutput.toFixed(3)} ETH`
    );
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Route size={20} className="text-green-400" /> Advanced DEX Routing
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* DEX Liquidity Overview */}
        <div>
          <h3 className="font-semibold mb-2 text-sm">Available AMMs</h3>
          <div className="grid grid-cols-3 gap-2">
            {MOCK_AMMS.map((amm) => (
              <div key={amm.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">{amm.name}</div>
                <div className="text-sm font-bold text-green-400 mb-1">
                  ${(amm.liquidity / 1000000).toFixed(1)}M
                </div>
                <div className="text-xs text-gray-400">
                  Fee: {amm.fee}% • Slip: {amm.slippage.toFixed(2)}%
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Swap Input */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
          <h3 className="font-semibold text-sm">Swap Parameters</h3>

          <div>
            <label className="text-xs text-gray-400 block mb-2">Input Amount (X3)</label>
            <input
              type="number"
              value={inputAmount}
              onChange={(e) => setInputAmount(Number(e.target.value))}
              className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-green-600"
            />
          </div>

          <div>
            <label className="text-xs text-gray-400 block mb-2">Slippage Tolerance (%)</label>
            <div className="flex items-center gap-2">
              <input
                type="range"
                min="0.1"
                max="5"
                step="0.1"
                value={slippageTolerance}
                onChange={(e) => setSlippageTolerance(Number(e.target.value))}
                className="flex-1"
              />
              <span className="text-sm font-bold w-12 text-right">{slippageTolerance.toFixed(1)}%</span>
            </div>
          </div>

          <div className="flex items-center gap-2 p-3 bg-[#2a2a35] rounded-lg">
            <Shield size={14} className="text-blue-400" />
            <label className="flex items-center gap-2 cursor-pointer flex-1 text-sm">
              <input
                type="checkbox"
                checked={mevProtection}
                onChange={(e) => setMevProtection(e.target.checked)}
                className="w-4 h-4"
              />
              MEV Protection (Commit-Reveal)
            </label>
          </div>
        </div>

        {/* Best Route Highlight */}
        <div className="bg-gradient-to-r from-green-600/20 to-blue-600/20 border border-green-600 rounded-lg p-4">
          <h3 className="font-semibold mb-3 text-sm">✓ Best Route (Recommended)</h3>
          <div className="space-y-2 text-sm">
            <div className="flex items-center justify-between">
              <span className="text-gray-400">Route</span>
              <span className="font-mono text-xs">{bestRoute.path.join(" ")}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-400">Expected Output</span>
              <span className="font-bold text-green-400">{bestRoute.expectedOutput.toFixed(6)} ETH</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-400">Price Impact</span>
              <span className="font-semibold">{bestRoute.priceImpact.toFixed(2)}%</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-400">Gas Estimate</span>
              <span className="font-mono text-xs">{bestRoute.gasEstimate.toLocaleString()} gas</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-gray-400">MEV Risk</span>
              <span className={clsx("font-bold", getBestRouteColor(bestRoute.mevRisk))}>
                {bestRoute.mevRisk.toUpperCase()}
              </span>
            </div>
          </div>

          <button
            onClick={handleExecuteSwap}
            className="w-full bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition mt-4"
          >
            Execute Best Route
          </button>
        </div>

        {/* Alternative Routes */}
        <div>
          <h3 className="font-semibold mb-2 text-sm">Alternative Routes</h3>
          <div className="space-y-2">
            {routes.slice(1).map((route) => (
              <button
                key={route.id}
                onClick={() => setSelectedRoute(route)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedRoute?.id === route.id
                    ? "border-green-600 bg-green-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="text-xs font-mono">{route.path.join(" ")}</div>
                  <span
                    className={clsx("text-xs px-2 py-1 rounded border", getBestRouteColor(route.mevRisk))}
                  >
                    {route.mevRisk}
                  </span>
                </div>
                <div className="flex justify-between text-xs text-gray-400">
                  <span>{route.expectedOutput.toFixed(6)} ETH • {route.priceImpact.toFixed(2)}%</span>
                  <span>{route.gasEstimate.toLocaleString()} gas</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* MEV Protection Info */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 text-xs space-y-2">
          <div className="flex gap-2">
            <Zap size={14} className="text-yellow-400 flex-shrink-0" />
            <div>
              <strong>Commit-Reveal Scheme:</strong> Your swap is committed private, then revealed only after block inclusion. Protects against sandwich attacks.
            </div>
          </div>
          <div className="flex gap-2">
            <BarChart3 size={14} className="text-blue-400 flex-shrink-0" />
            <div>
              <strong>Price Impact:</strong> Lower is better. High impact = slippage risk.
            </div>
          </div>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Intelligent routing finds the best path across all DEXes with MEV protection.
      </div>
    </div>
  );
}

function getBestRouteColor(risk: string) {
  switch (risk) {
    case "low":
      return "bg-green-600/30 border-green-600 text-green-400";
    case "medium":
      return "bg-yellow-600/30 border-yellow-600 text-yellow-400";
    case "high":
      return "bg-red-600/30 border-red-600 text-red-400";
    default:
      return "bg-gray-600/30 border-gray-600 text-gray-400";
  }
}
