import React, { useState } from "react";
import { Plus, TrendingUp, PieChart, LineChart, Zap } from "lucide-react";
import clsx from "clsx";

interface TokenConfig {
  name: string;
  symbol: string;
  totalSupply: string;
  decimals: string;
  presalePrice: string;
  presaleTarget: string;
  vestingPeriod: string;
}

interface VestingSchedule {
  id: string;
  name: string;
  percentage: number;
  duration: string;
  cliffDays: string;
}

export default function TokenLaunchpadPanel() {
  const [step, setStep] = useState(1);
  const [tokenConfig, setTokenConfig] = useState<TokenConfig>({
    name: "My Token",
    symbol: "MYTKN",
    totalSupply: "1000000",
    decimals: "18",
    presalePrice: "0.05",
    presaleTarget: "100000",
    vestingPeriod: "12",
  });

  const [vestingSchedules, setVestingSchedules] = useState<VestingSchedule[]>([
    { id: "1", name: "Team", percentage: 20, duration: "24", cliffDays: "180" },
    { id: "2", name: "Investors", percentage: 30, duration: "12", cliffDays: "0" },
    { id: "3", name: "Marketing", percentage: 15, duration: "6", cliffDays: "30" },
  ]);

  const [showAddVesting, setShowAddVesting] = useState(false);

  const totalSupply = parseFloat(tokenConfig.totalSupply) || 0;
  const presaleAmount = Math.floor((totalSupply * 0.30) / 1);
  const tokensPerUsd = 1 / (parseFloat(tokenConfig.presalePrice) || 1);

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4">Token Launchpad</h2>

      {/* Step Indicator */}
      <div className="flex gap-2 mb-6">
        {[1, 2, 3].map((s) => (
          <button
            key={s}
            onClick={() => setStep(s)}
            className={clsx(
              "flex-1 py-2 rounded-lg font-semibold text-sm transition",
              step === s
                ? "bg-blue-600 text-white"
                : "bg-[#15151b] text-gray-400 hover:bg-[#1a1a20]"
            )}
          >
            {s === 1 && "Token Config"}
            {s === 2 && "Vesting"}
            {s === 3 && "Preview"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto mb-4">
        {/* Step 1: Token Configuration */}
        {step === 1 && (
          <div className="space-y-4">
            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Token Name</label>
              <input
                type="text"
                value={tokenConfig.name}
                onChange={(e) => setTokenConfig({ ...tokenConfig, name: e.target.value })}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-sm font-semibold text-gray-300 block mb-2">Symbol</label>
                <input
                  type="text"
                  value={tokenConfig.symbol}
                  onChange={(e) => setTokenConfig({ ...tokenConfig, symbol: e.target.value })}
                  className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
                />
              </div>
              <div>
                <label className="text-sm font-semibold text-gray-300 block mb-2">Decimals</label>
                <input
                  type="number"
                  value={tokenConfig.decimals}
                  onChange={(e) => setTokenConfig({ ...tokenConfig, decimals: e.target.value })}
                  className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
                />
              </div>
            </div>

            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Total Supply</label>
              <input
                type="number"
                value={tokenConfig.totalSupply}
                onChange={(e) => setTokenConfig({ ...tokenConfig, totalSupply: e.target.value })}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
              />
            </div>

            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Presale Price (USD)</label>
              <input
                type="number"
                step="0.01"
                value={tokenConfig.presalePrice}
                onChange={(e) => setTokenConfig({ ...tokenConfig, presalePrice: e.target.value })}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
              />
            </div>

            <div>
              <label className="text-sm font-semibold text-gray-300 block mb-2">Presale Target (USD)</label>
              <input
                type="number"
                value={tokenConfig.presaleTarget}
                onChange={(e) => setTokenConfig({ ...tokenConfig, presaleTarget: e.target.value })}
                className="w-full bg-[#15151b] border border-[#2a2a35] rounded px-3 py-2 text-white text-sm"
              />
            </div>

            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3">Quick Info</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-400">Presale Amount</span>
                  <span className="font-semibold">{presaleAmount.toLocaleString()} {tokenConfig.symbol}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Tokens per USD</span>
                  <span className="font-semibold text-green-400">{tokensPerUsd.toFixed(2)}</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Step 2: Vesting Schedule */}
        {step === 2 && (
          <div className="space-y-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="font-semibold">Vesting Groups</h3>
              <button
                onClick={() => setShowAddVesting(true)}
                className="flex items-center gap-1 bg-blue-600 hover:bg-blue-700 px-3 py-1 rounded text-sm transition"
              >
                <Plus size={14} /> Add
              </button>
            </div>

            {vestingSchedules.map((schedule) => (
              <div key={schedule.id} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="font-semibold">{schedule.name}</h4>
                  <span className="text-blue-400 font-semibold">{schedule.percentage}%</span>
                </div>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between text-gray-400">
                    <span>Duration</span>
                    <span>{schedule.duration} months</span>
                  </div>
                  <div className="flex justify-between text-gray-400">
                    <span>Cliff Period</span>
                    <span>{schedule.cliffDays} days</span>
                  </div>
                </div>
                <div className="mt-3 bg-[#2a2a35] rounded-full h-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full transition-all"
                    style={{ width: `${schedule.percentage}%` }}
                  />
                </div>
              </div>
            ))}

            {/* Add Vesting Modal */}
            {showAddVesting && (
              <div className="bg-[#15151b] border border-blue-600 rounded-lg p-4 space-y-3">
                <input
                  type="text"
                  placeholder="Group name"
                  className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
                />
                <input
                  type="number"
                  placeholder="Percentage"
                  className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
                />
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowAddVesting(false)}
                    className="flex-1 bg-[#2a2a35] py-2 rounded text-sm hover:bg-[#3a3a45]"
                  >
                    Cancel
                  </button>
                  <button className="flex-1 bg-blue-600 hover:bg-blue-700 py-2 rounded text-sm font-semibold">
                    Add
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {/* Step 3: Preview */}
        {step === 3 && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-xs text-gray-400 mb-1">Token Name</div>
                <div className="font-bold">{tokenConfig.name}</div>
              </div>
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-xs text-gray-400 mb-1">Symbol</div>
                <div className="font-bold">{tokenConfig.symbol}</div>
              </div>
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-xs text-gray-400 mb-1">Total Supply</div>
                <div className="font-bold">{parseFloat(tokenConfig.totalSupply).toLocaleString()}</div>
              </div>
              <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
                <div className="text-xs text-gray-400 mb-1">Presale Price</div>
                <div className="font-bold text-green-400">${tokenConfig.presalePrice}</div>
              </div>
            </div>

            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
              <h3 className="font-semibold mb-3 flex items-center gap-2">
                <PieChart size={16} /> Allocation
              </h3>
              <div className="space-y-2">
                {vestingSchedules.map((s) => (
                  <div key={s.id} className="flex items-center justify-between text-sm">
                    <span className="text-gray-400">{s.name}</span>
                    <div className="flex-1 mx-3 bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full"
                        style={{ width: `${s.percentage}%` }}
                      />
                    </div>
                    <span className="font-semibold w-12 text-right">{s.percentage}%</span>
                  </div>
                ))}
              </div>
            </div>

            <button className="w-full bg-green-600 hover:bg-green-700 py-3 rounded-lg font-bold transition flex items-center justify-center gap-2">
              <Zap size={16} /> Deploy Token
            </button>
          </div>
        )}
      </div>

      {/* Navigation */}
      <div className="flex gap-2">
        <button
          onClick={() => setStep(Math.max(1, step - 1))}
          disabled={step === 1}
          className="flex-1 bg-[#15151b] hover:bg-[#1a1a20] disabled:opacity-50 py-2 rounded-lg font-semibold text-sm transition"
        >
          Previous
        </button>
        <button
          onClick={() => setStep(Math.min(3, step + 1))}
          disabled={step === 3}
          className="flex-1 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 py-2 rounded-lg font-semibold text-sm transition"
        >
          Next
        </button>
      </div>
    </div>
  );
}
