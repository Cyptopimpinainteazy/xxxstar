import React, { useState } from "react";
import { AlertTriangle, BarChart3, Settings, Clock } from "lucide-react";
import clsx from "clsx";

interface AntisniperConfig {
  enabled: boolean;
  blockDelay: number;
  maxBuyPercent: number;
  cooldownSeconds: number;
  blacklistMode: boolean;
  autoLiquify: boolean;
}

export default function AntisniperPanel() {
  const [config, setConfig] = useState<AntisniperConfig>({
    enabled: true,
    blockDelay: 5,
    maxBuyPercent: 1,
    cooldownSeconds: 30,
    blacklistMode: true,
    autoLiquify: false,
  });

  const [blockedBots, setBlockedBots] = useState([
    { id: "1", address: "0x1234...5678", reason: "Sandwich attack detected", time: "2 mins ago" },
    { id: "2", address: "0x9abc...def0", reason: "Multiple max buys", time: "15 mins ago" },
  ]);

  const [preDeployStats, setPreDeployStats] = useState({
    simulatedBotAttempts: 34,
    blockedByAntisniperLaunch: 32,
    effectivenessRate: 94,
    estimatedSaving: 450000,
  });

  const handleEnable = () => {
    setConfig({ ...config, enabled: !config.enabled });
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <AlertTriangle size={20} /> Anti-Sniper Protection
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Status */}
        <div className={clsx(
          "p-4 rounded-lg border flex items-start gap-3",
          config.enabled ? "bg-green-600/10 border-green-600" : "bg-gray-600/10 border-gray-600"
        )}>
          <div className={clsx(
            "w-2 h-2 rounded-full mt-2",
            config.enabled ? "bg-green-400" : "bg-gray-400"
          )} />
          <div>
            <div className={clsx("font-semibold", config.enabled ? "text-green-400" : "text-gray-400")}>
              {config.enabled ? "✓ Anti-Sniper Active" : "○ Anti-Sniper Disabled"}
            </div>
            <div className="text-xs text-gray-400 mt-1">
              {config.enabled
                ? "Bot detection & blocking enabled. New contracts inherit these settings."
                : "Sniper bots will not be blocked. Enable for launch protection."}
            </div>
          </div>
        </div>

        {/* Pre-Deploy Stats */}
        <div className="grid grid-cols-2 gap-3">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Simulated Bot Attempts</div>
            <div className="text-2xl font-bold text-blue-400">{preDeployStats.simulatedBotAttempts}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Would Block</div>
            <div className="text-2xl font-bold text-green-400">{preDeployStats.blockedByAntisniperLaunch}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Effectiveness</div>
            <div className="text-2xl font-bold text-yellow-400">{preDeployStats.effectivenessRate}%</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Est. Protected</div>
            <div className="text-2xl font-bold text-purple-400">${(preDeployStats.estimatedSaving / 1000).toFixed(0)}K</div>
          </div>
        </div>

        {/* Configuration */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="font-semibold flex items-center gap-2">
              <Settings size={16} /> Configuration
            </h3>
            <button
              onClick={handleEnable}
              className={clsx(
                "w-12 h-6 rounded-full transition relative",
                config.enabled ? "bg-green-600" : "bg-[#2a2a35]"
              )}
            >
              <div
                className={clsx(
                  "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                  config.enabled ? "right-0.5" : "left-0.5"
                )}
              />
            </button>
          </div>

          {config.enabled && (
            <div className="space-y-4 border-t border-[#2a2a35] pt-4">
              {/* Block Delay */}
              <div>
                <label className="text-sm font-medium block mb-2">
                  Launch Block Delay: {config.blockDelay} blocks
                </label>
                <input
                  type="range"
                  min="1"
                  max="10"
                  value={config.blockDelay}
                  onChange={(e) => setConfig({ ...config, blockDelay: parseInt(e.target.value) })}
                  className="w-full"
                />
                <p className="text-xs text-gray-400 mt-1">
                  Blocks from contract creation before buys allowed. Slows bots, allows humans to enter.
                </p>
              </div>

              {/* Max Buy Percent */}
              <div>
                <label className="text-sm font-medium block mb-2">
                  Max Buy in First 24h: {config.maxBuyPercent}% of supply
                </label>
                <input
                  type="range"
                  min="0.5"
                  max="5"
                  step="0.5"
                  value={config.maxBuyPercent}
                  onChange={(e) => setConfig({ ...config, maxBuyPercent: parseFloat(e.target.value) })}
                  className="w-full"
                />
                <p className="text-xs text-gray-400 mt-1">
                  No single wallet can buy more than this % of total supply. Prevents mass bot accumulation.
                </p>
              </div>

              {/* Cooldown */}
              <div>
                <label className="text-sm font-medium block mb-2">
                  <Clock size={14} className="inline mr-1" /> Buy Cooldown: {config.cooldownSeconds}s
                </label>
                <input
                  type="range"
                  min="5"
                  max="120"
                  step="5"
                  value={config.cooldownSeconds}
                  onChange={(e) => setConfig({ ...config, cooldownSeconds: parseInt(e.target.value) })}
                  className="w-full"
                />
                <p className="text-xs text-gray-400 mt-1">
                  Minimum seconds between buys for same wallet. Slows automated attack patterns.
                </p>
              </div>

              {/* Blacklist Mode */}
              <label className="flex items-center gap-3 p-2 rounded cursor-pointer hover:bg-[#2a2a35]">
                <input
                  type="checkbox"
                  checked={config.blacklistMode}
                  onChange={(e) => setConfig({ ...config, blacklistMode: e.target.checked })}
                  className="w-4 h-4 rounded"
                />
                <div>
                  <div className="text-sm font-medium">Permanent Blacklist Mode</div>
                  <div className="text-xs text-gray-400">Blocked addresses cannot trade forever</div>
                </div>
              </label>

              {/* Auto Liquify */}
              <label className="flex items-center gap-3 p-2 rounded cursor-pointer hover:bg-[#2a2a35]">
                <input
                  type="checkbox"
                  checked={config.autoLiquify}
                  onChange={(e) => setConfig({ ...config, autoLiquify: e.target.checked })}
                  className="w-4 h-4 rounded"
                />
                <div>
                  <div className="text-sm font-medium">Auto Liquify Sniper Buys</div>
                  <div className="text-xs text-gray-400">Liquify intercepted bot purchases back to project treasury</div>
                </div>
              </label>
            </div>
          )}
        </div>

        {/* Blocked Addresses */}
        {blockedBots.length > 0 && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <h3 className="font-semibold mb-3">Recently Blocked</h3>
            <div className="space-y-2">
              {blockedBots.map((bot) => (
                <div
                  key={bot.id}
                  className="p-2 rounded bg-[#2a2a35] flex items-center justify-between text-xs"
                >
                  <div>
                    <div className="text-gray-300 font-mono">{bot.address}</div>
                    <div className="text-gray-500">{bot.reason}</div>
                  </div>
                  <div className="text-gray-500">{bot.time}</div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Info */}
        <div className="bg-blue-600/10 border border-blue-600 rounded-lg p-4">
          <h3 className="font-semibold text-blue-400 mb-2">💡 Anti-Sniper Best Practices</h3>
          <ul className="text-xs text-gray-400 space-y-1">
            <li>• Set block delay to 3-5 blocks for human traders to enter before full liquidity</li>
            <li>• Max buy 0.5-1% prevents bots from accumulating major positions</li>
            <li>• Longer cooldown (30-60s) makes bot attacks unprofitable</li>
            <li>• Use blacklist permanently for known MEV exploiters</li>
          </ul>
        </div>
      </div>

      <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        Save Anti-Sniper Config
      </button>
    </div>
  );
}
