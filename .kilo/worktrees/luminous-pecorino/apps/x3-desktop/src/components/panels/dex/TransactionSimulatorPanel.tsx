import React, { useState } from "react";
import { ArrowRight, TrendingUp, AlertCircle, CheckCircle, Zap, DollarSign } from "lucide-react";
import clsx from "clsx";

interface SimulationResult {
  inputAmount: number;
  inputToken: string;
  outputAmount: number;
  outputToken: string;
  price: number;
  priceImpact: number;
  minimumReceived: number;
  liquidityProvider: number;
  networkFee: number;
  totalFee: number;
  slippage: number;
  route: string[];
  executionPrice: number;
  worstPrice: number;
}

export default function TransactionSimulatorPanel() {
  const [inputAmount, setInputAmount] = useState(100);
  const [inputToken, setInputToken] = useState("USDC");
  const [outputToken, setOutputToken] = useState("X3");
  const [slippage, setSlippage] = useState(0.5);
  const [simulation, setSimulation] = useState<SimulationResult | null>(null);
  const [showResult, setShowResult] = useState(false);

  const tokens = ["USDC", "X3", "ETH", "BTC", "SOL"];

  const handleSimulate = () => {
    // Mock simulation
    const exchangeRates: Record<string, Record<string, number>> = {
      USDC: { X3: 0.8, ETH: 0.00035, BTC: 0.000022, SOL: 0.53 },
      X3: { USDC: 1.25, ETH: 0.000438, BTC: 0.000027, SOL: 0.66 },
      ETH: { USDC: 2850, X3: 2283, BTC: 0.062, SOL: 1516 },
    };

    const rate = exchangeRates[inputToken]?.[outputToken] || 0.5;
    const rawOutput = inputAmount * rate;
    const priceImpact = inputAmount > 1000 ? 2.5 : inputAmount > 500 ? 1.2 : 0.35;
    const outputAfterImpact = rawOutput * (1 - priceImpact / 100);
    const lpFee = outputAfterImpact * 0.003;
    const networkFee = inputToken === "USDC" ? 1 : 0.02;
    const minimumReceived = outputAfterImpact * (1 - slippage / 100);

    setSimulation({
      inputAmount,
      inputToken,
      outputAmount: outputAfterImpact,
      outputToken,
      price: rate,
      priceImpact,
      minimumReceived,
      liquidityProvider: lpFee,
      networkFee,
      totalFee: lpFee + networkFee,
      slippage,
      route: [inputToken, "USDC", outputToken],
      executionPrice: outputAfterImpact / inputAmount,
      worstPrice: minimumReceived / inputAmount,
    });
    setShowResult(true);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-6">Transaction Simulator</h2>

      <div className="flex gap-6 h-full min-h-0">
        {/* Input Side */}
        <div className="flex-1 flex flex-col gap-4">
          <div className="bg-[#15151b] border border-[#2a2a35] p-6 rounded-lg">
            <label className="text-sm text-gray-400 block mb-3">You Send</label>
            <div className="space-y-3">
              <input
                type="number"
                value={inputAmount}
                onChange={(e) => setInputAmount(Number(e.target.value))}
                className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-3 text-2xl font-bold text-white"
              />
              <select
                value={inputToken}
                onChange={(e) => setInputToken(e.target.value)}
                className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white font-semibold"
              >
                {tokens.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>

            {/* Slippage Control */}
            <div className="mt-6 pt-6 border-t border-[#2a2a35]">
              <label className="text-sm text-gray-400 block mb-3">Slippage Tolerance</label>
              <div className="flex gap-2">
                {[0.1, 0.5, 1.0].map((s) => (
                  <button
                    key={s}
                    onClick={() => setSlippage(s)}
                    className={clsx(
                      "flex-1 py-2 rounded-lg font-semibold text-sm transition",
                      slippage === s ? "bg-blue-600" : "bg-[#2a2a35] hover:bg-[#3a3a45]"
                    )}
                  >
                    {s}%
                  </button>
                ))}
              </div>
              <input
                type="range"
                min="0"
                max="5"
                step="0.1"
                value={slippage}
                onChange={(e) => setSlippage(Number(e.target.value))}
                className="w-full mt-3"
              />
            </div>
          </div>

          {/* Swap Button */}
          <button
            onClick={() => {
              const temp = inputToken;
              setInputToken(outputToken);
              setOutputToken(temp);
            }}
            className="bg-[#1a5f4a] hover:bg-[#1a7f5a] px-4 py-3 rounded-lg font-semibold flex items-center justify-center gap-2 transition"
          >
            <ArrowRight size={18} className="rotate-90" /> Reverse
          </button>

          {/* Simulate Button */}
          <button
            onClick={handleSimulate}
            className="bg-blue-600 hover:bg-blue-700 px-4 py-3 rounded-lg font-semibold text-lg transition w-full"
          >
            <Zap size={18} className="inline mr-2" /> Simulate Swap
          </button>
        </div>

        {/* Output Side */}
        <div className="flex-1 flex flex-col gap-4">
          <div className="bg-[#15151b] border border-[#2a2a35] p-6 rounded-lg">
            <label className="text-sm text-gray-400 block mb-3">You Receive</label>
            <div className="space-y-3">
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-3 text-2xl font-bold text-white">
                {simulation ? simulation.outputAmount.toFixed(6) : "0.00"}
              </div>
              <select
                value={outputToken}
                onChange={(e) => setOutputToken(e.target.value)}
                className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white font-semibold"
              >
                {tokens.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>

            {/* Price Info */}
            {simulation && (
              <div className="mt-6 pt-6 border-t border-[#2a2a35] space-y-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Exchange Rate</span>
                  <span className="font-semibold">
                    1 {inputToken} = {simulation.price.toFixed(6)} {outputToken}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Price Impact</span>
                  <span className={clsx("font-semibold", simulation.priceImpact > 2 ? "text-red-400" : "text-yellow-400")}>
                    {simulation.priceImpact.toFixed(3)}%
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Minimum Received</span>
                  <span className="font-semibold">{simulation.minimumReceived.toFixed(6)} {outputToken}</span>
                </div>
              </div>
            )}
          </div>

          {/* Summary Card */}
          {showResult && simulation && (
            <div className="bg-[#15151b] border border-[#2a2a35] p-4 rounded-lg space-y-3">
              <h3 className="font-semibold text-sm flex items-center gap-2">
                <CheckCircle size={16} className="text-green-400" /> Simulation Summary
              </h3>
              <div className="space-y-2 text-xs">
                <div className="grid grid-cols-2 gap-2">
                  <div className="text-gray-400">LP Fee</div>
                  <div className="text-right font-semibold">{simulation.liquidityProvider.toFixed(6)} {outputToken}</div>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div className="text-gray-400">Network Fee</div>
                  <div className="text-right font-semibold">${simulation.networkFee.toFixed(2)}</div>
                </div>
                <div className="grid grid-cols-2 gap-2 pt-2 border-t border-[#2a2a35]">
                  <div className="text-gray-400">Total Cost</div>
                  <div className="text-right font-semibold text-red-400">
                    {simulation.totalFee.toFixed(6)} + $${simulation.networkFee.toFixed(2)}
                  </div>
                </div>
              </div>

              <div className="pt-3 border-t border-[#2a2a35]">
                <button className="w-full bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition">
                  Confirm Swap
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
