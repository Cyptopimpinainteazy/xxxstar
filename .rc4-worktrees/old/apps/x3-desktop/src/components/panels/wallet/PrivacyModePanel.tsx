import React, { useState } from "react";
import { Eye, EyeOff, Shield, Lock, Volume2, Zap } from "lucide-react";
import clsx from "clsx";

interface PrivacySettings {
  stealthMode: boolean;
  hideBalance: boolean;
  mixerEnabled: boolean;
  hideActivity: boolean;
  anonBrowsing: boolean;
  autoLogout: number;
}

export default function PrivacyModePanel() {
  const [settings, setSettings] = useState<PrivacySettings>({
    stealthMode: false,
    hideBalance: false,
    mixerEnabled: false,
    hideActivity: false,
    anonBrowsing: false,
    autoLogout: 15,
  });

  const [mixingAmount, setMixingAmount] = useState("100");
  const [mixingProgress, setMixingProgress] = useState(0);
  const [isMixing, setIsMixing] = useState(false);
  const [mixingStatus, setMixingStatus] = useState<"idle" | "mixing" | "complete">("idle");

  const toggleSetting = (key: keyof PrivacySettings) => {
    setSettings({ ...settings, [key]: !settings[key] });
  };

  const handleMix = () => {
    setIsMixing(true);
    setMixingStatus("mixing");
    setMixingProgress(0);

    const interval = setInterval(() => {
      setMixingProgress((p) => {
        if (p >= 100) {
          clearInterval(interval);
          setIsMixing(false);
          setMixingStatus("complete");
          setTimeout(() => setMixingStatus("idle"), 3000);
          return 100;
        }
        return p + Math.random() * 30;
      });
    }, 300);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Shield size={20} /> Privacy Mode
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Stealth Mode Banner */}
        {settings.stealthMode && (
          <div className="bg-green-600/20 border border-green-600 rounded-lg p-3 flex items-center gap-2">
            <Shield size={16} className="text-green-400" />
            <span className="text-sm font-semibold text-green-400">🔐 Stealth Mode Active</span>
          </div>
        )}

        {/* Privacy Controls */}
        <div className="space-y-3">
          {/* Stealth Mode */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Eye size={16} className="text-blue-400" />
                <div>
                  <h3 className="font-semibold">Stealth Mode</h3>
                  <p className="text-xs text-gray-400">Hide all activity from blockchain explorers</p>
                </div>
              </div>
              <button
                onClick={() => toggleSetting("stealthMode")}
                className={clsx(
                  "w-12 h-6 rounded-full transition relative",
                  settings.stealthMode ? "bg-green-600" : "bg-[#2a2a35]"
                )}
              >
                <div
                  className={clsx(
                    "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                    settings.stealthMode ? "right-0.5" : "left-0.5"
                  )}
                />
              </button>
            </div>
            {settings.stealthMode && (
              <div className="text-xs text-gray-400 mt-2 p-2 bg-[#2a2a35] rounded">
                ⚠️ Transactions will use privacy protocols. Processing may take 30-60 seconds.
              </div>
            )}
          </div>

          {/* Hide Balance */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <EyeOff size={16} className="text-purple-400" />
                <div>
                  <h3 className="font-semibold">Hide Balance</h3>
                  <p className="text-xs text-gray-400">Hide portfolio value in UI</p>
                </div>
              </div>
              <button
                onClick={() => toggleSetting("hideBalance")}
                className={clsx(
                  "w-12 h-6 rounded-full transition relative",
                  settings.hideBalance ? "bg-purple-600" : "bg-[#2a2a35]"
                )}
              >
                <div
                  className={clsx(
                    "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                    settings.hideBalance ? "right-0.5" : "left-0.5"
                  )}
                />
              </button>
            </div>
          </div>

          {/* Transaction Mixer */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Zap size={16} className="text-yellow-400" />
                <div>
                  <h3 className="font-semibold">Transaction Mixer</h3>
                  <p className="text-xs text-gray-400">Obfuscate transaction trails</p>
                </div>
              </div>
              <button
                onClick={() => toggleSetting("mixerEnabled")}
                className={clsx(
                  "w-12 h-6 rounded-full transition relative",
                  settings.mixerEnabled ? "bg-yellow-600" : "bg-[#2a2a35]"
                )}
              >
                <div
                  className={clsx(
                    "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                    settings.mixerEnabled ? "right-0.5" : "left-0.5"
                  )}
                />
              </button>
            </div>

            {settings.mixerEnabled && (
              <div className="space-y-3">
                <div>
                  <label className="text-xs text-gray-400 block mb-1">Amount to Mix</label>
                  <input
                    type="number"
                    value={mixingAmount}
                    onChange={(e) => setMixingAmount(e.target.value)}
                    className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm"
                    placeholder="0"
                  />
                </div>

                {mixingStatus === "idle" && (
                  <button
                    onClick={handleMix}
                    disabled={!mixingAmount || isMixing}
                    className="w-full bg-yellow-600 hover:bg-yellow-700 disabled:opacity-50 py-2 rounded-lg font-semibold text-sm transition"
                  >
                    Start Mixing
                  </button>
                )}

                {mixingStatus === "mixing" && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-gray-400">Mixing in progress...</span>
                      <span className="text-xs font-semibold">{Math.floor(mixingProgress)}%</span>
                    </div>
                    <div className="bg-[#2a2a35] rounded-full h-2">
                      <div
                        className="bg-yellow-600 h-2 rounded-full transition-all"
                        style={{ width: `${mixingProgress}%` }}
                      />
                    </div>
                  </div>
                )}

                {mixingStatus === "complete" && (
                  <div className="bg-green-600/10 border border-green-600 p-2 rounded text-center">
                    <div className="text-xs font-semibold text-green-400">✓ Mixing Complete</div>
                    <div className="text-xs text-gray-400">Funds returned to wallet (mixed contract)</div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Hide Activity */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Lock size={16} className="text-red-400" />
                <div>
                  <h3 className="font-semibold">Hide Activity</h3>
                  <p className="text-xs text-gray-400">Hide trading history from UI</p>
                </div>
              </div>
              <button
                onClick={() => toggleSetting("hideActivity")}
                className={clsx(
                  "w-12 h-6 rounded-full transition relative",
                  settings.hideActivity ? "bg-red-600" : "bg-[#2a2a35]"
                )}
              >
                <div
                  className={clsx(
                    "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                    settings.hideActivity ? "right-0.5" : "left-0.5"
                  )}
                />
              </button>
            </div>
          </div>

          {/* Anonymous Browsing */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Volume2 size={16} className="text-cyan-400" />
                <div>
                  <h3 className="font-semibold">Anonymous Browsing</h3>
                  <p className="text-xs text-gray-400">Route through privacy VPN</p>
                </div>
              </div>
              <button
                onClick={() => toggleSetting("anonBrowsing")}
                className={clsx(
                  "w-12 h-6 rounded-full transition relative",
                  settings.anonBrowsing ? "bg-cyan-600" : "bg-[#2a2a35]"
                )}
              >
                <div
                  className={clsx(
                    "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                    settings.anonBrowsing ? "right-0.5" : "left-0.5"
                  )}
                />
              </button>
            </div>
          </div>

          {/* Auto Logout */}
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
            <div className="mb-3">
              <h3 className="font-semibold">Auto Logout</h3>
              <p className="text-xs text-gray-400">Lock wallet after inactivity</p>
            </div>
            <div className="flex items-center gap-3">
              <input
                type="range"
                min="5"
                max="120"
                step="5"
                value={settings.autoLogout}
                onChange={(e) => setSettings({ ...settings, autoLogout: parseInt(e.target.value) })}
                className="flex-1"
              />
              <span className="text-sm font-semibold w-16 text-right">{settings.autoLogout}m</span>
            </div>
          </div>
        </div>

        {/* Privacy Tips */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-2">🛡️ Privacy Tips</h3>
          <ul className="text-xs text-gray-400 space-y-1">
            <li>• Enable Stealth Mode for sensitive transactions</li>
            <li>• Use Transaction Mixer with multiple small amounts</li>
            <li>• Enable Anonymous Browsing for extra privacy</li>
            <li>• Keep auto-logout duration short if using shared devices</li>
          </ul>
        </div>
      </div>

      <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        Save Privacy Settings
      </button>
    </div>
  );
}
