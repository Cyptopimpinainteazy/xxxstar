import React, { useState } from "react";
import { HardDrive, Link as LinkIcon, Check, AlertTriangle, Zap, Settings, RefreshCw } from "lucide-react";
import clsx from "clsx";

interface HardwareDevice {
  id: string;
  name: string;
  type: "ledger" | "trezor";
  model: string;
  firmwareVersion: string;
  status: "connected" | "disconnected" | "locked";
  accounts: string[];
}

interface DerivationPath {
  path: string;
  type: "ethereum" | "solana" | "x3-native";
  accountIndex: number;
}

const MOCK_DEVICES: HardwareDevice[] = [
  {
    id: "1",
    name: "Ledger Nano X",
    type: "ledger",
    model: "Nano X (2024)",
    firmwareVersion: "2.1.0",
    status: "connected",
    accounts: ["0x742d35Cc6634C0532925a3b844Bc9e7595f...7e6f", "0x123...456"],
  },
  {
    id: "2",
    name: "Trezor One",
    type: "trezor",
    model: "Trezor Model One",
    firmwareVersion: "1.12.2",
    status: "disconnected",
    accounts: [],
  },
];

const MOCK_PATHS: DerivationPath[] = [
  { path: "m/44'/60'/0'/0/0", type: "ethereum", accountIndex: 0 },
  { path: "m/44'/60'/0'/0/1", type: "ethereum", accountIndex: 1 },
  { path: "m/44'/501'/0'/0'", type: "solana", accountIndex: 0 },
  { path: "m/44'/1399'/0'/0/0", type: "x3-native", accountIndex: 0 },
];

export default function HardwareWalletPanel() {
  const [devices, setDevices] = useState<HardwareDevice[]>(MOCK_DEVICES);
  const [selectedDevice, setSelectedDevice] = useState<HardwareDevice | null>(MOCK_DEVICES[0]);
  const [paths, setPaths] = useState<DerivationPath[]>(MOCK_PATHS);
  const [customPath, setCustomPath] = useState("");
  const [activeTab, setActiveTab] = useState<"devices" | "paths" | "settings">("devices");

  const connectedCount = devices.filter((d) => d.status === "connected").length;

  const handleConnectDevice = (deviceId: string) => {
    setDevices(devices.map(d => d.id === deviceId ? { ...d, status: "connected" as const } : d));
  };

  const handleTestConnection = (deviceId: string) => {
    alert(`Testing connection to device ${deviceId}...`);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <HardDrive size={20} className="text-purple-400" /> Hardware Wallet
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-3 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Connected Devices</div>
            <div className="text-lg font-bold text-green-400">{connectedCount}/{devices.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Total Accounts</div>
            <div className="text-lg font-bold text-cyan-400">
              {devices.reduce((sum, d) => sum + d.accounts.length, 0)}
            </div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Ledger + Trezor</div>
            <div className="text-lg font-bold text-purple-400">✓ Ready</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 border-b border-[#2a2a35]">
          {(["devices", "paths", "settings"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={clsx(
                "px-4 py-2 text-sm font-semibold transition border-b-2",
                activeTab === tab
                  ? "border-cyan-600 text-cyan-400"
                  : "border-transparent text-gray-400 hover:text-gray-300"
              )}
            >
              {tab === "devices" && "Devices"}
              {tab === "paths" && "Derivation Paths"}
              {tab === "settings" && "Settings"}
            </button>
          ))}
        </div>

        {activeTab === "devices" && (
          <div className="space-y-3">
            {devices.map((device) => (
              <button
                key={device.id}
                onClick={() => setSelectedDevice(device)}
                className={clsx(
                  "w-full text-left p-3 rounded-lg border-2 transition",
                  selectedDevice?.id === device.id
                    ? "border-cyan-600 bg-cyan-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div>
                    <div className="font-semibold">{device.name}</div>
                    <div className="text-xs text-gray-400">{device.model}</div>
                  </div>
                  <div
                    className={clsx(
                      "px-2 py-1 rounded-md text-xs font-bold",
                      device.status === "connected"
                        ? "bg-green-600/20 text-green-400"
                        : device.status === "locked"
                        ? "bg-yellow-600/20 text-yellow-400"
                        : "bg-red-600/20 text-red-400"
                    )}
                  >
                    {device.status === "connected" && <LinkIcon size={12} className="inline mr-1" />}
                    {device.status.toUpperCase()}
                  </div>
                </div>

                <div className="text-xs text-gray-400 mb-2">Firmware: {device.firmwareVersion}</div>
                <div className="text-xs text-gray-500">Accounts: {device.accounts.length}</div>
              </button>
            ))}
          </div>
        )}

        {activeTab === "paths" && (
          <div className="space-y-2">
            {paths.map((path, idx) => (
              <div key={idx} className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div className="font-mono text-xs font-semibold text-cyan-400">{path.path}</div>
                  <span className="text-xs px-2 py-1 bg-cyan-600/20 text-cyan-400 rounded-md capitalize">
                    {path.type}
                  </span>
                </div>
                <div className="flex justify-between text-xs text-gray-400">
                  <span>Account: {path.accountIndex}</span>
                  <button className="text-cyan-400 hover:text-cyan-300">Derive</button>
                </div>
              </div>
            ))}

            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3 space-y-2">
              <div className="text-xs font-semibold text-gray-400 mb-2">Custom Path</div>
              <input
                type="text"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                placeholder="m/44'/60'/0'/0/0"
                className="w-full bg-[#0a0a0f] border border-[#2a2a35] rounded px-2 py-2 text-xs font-mono focus:border-cyan-600 focus:outline-none"
              />
              <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded text-xs font-semibold transition">
                Add Path
              </button>
            </div>
          </div>
        )}

        {activeTab === "settings" && (
          <div className="space-y-3">
            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm">Firmware & Updates</h3>
              {selectedDevice && (
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Current Version</span>
                    <span className="font-semibold">{selectedDevice.firmwareVersion}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-400">Latest Version</span>
                    <span className="font-semibold text-green-400">2.1.0</span>
                  </div>
                  <button className="w-full bg-cyan-600 hover:bg-cyan-700 py-2 rounded text-sm font-semibold transition flex items-center justify-center gap-2">
                    <RefreshCw size={14} /> Check for Updates
                  </button>
                </div>
              )}
            </div>

            <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-3">
              <h3 className="font-semibold text-sm">Security</h3>
              <div className="space-y-2">
                <button className="w-full bg-purple-600/20 hover:bg-purple-600/30 text-purple-400 py-2 rounded text-sm font-semibold transition">
                  Verify PIN
                </button>
                <button className="w-full bg-yellow-600/20 hover:bg-yellow-600/30 text-yellow-400 py-2 rounded text-sm font-semibold transition">
                  Reset Device
                </button>
                <button className="w-full bg-red-600/20 hover:bg-red-600/30 text-red-400 py-2 rounded text-sm font-semibold transition">
                  Wipe & Recover
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Device Details */}
        {selectedDevice && activeTab === "devices" && (
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4 space-y-4">
            <h3 className="font-semibold">{selectedDevice.name} Details</h3>

            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-400">Type</span>
                <span className="font-semibold capitalize">{selectedDevice.type}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Model</span>
                <span className="font-semibold">{selectedDevice.model}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Status</span>
                <span
                  className={clsx(
                    "font-bold",
                    selectedDevice.status === "connected" ? "text-green-400" : "text-red-400"
                  )}
                >
                  {selectedDevice.status.toUpperCase()}
                </span>
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              {selectedDevice.status !== "connected" && (
                <button
                  onClick={() => handleConnectDevice(selectedDevice.id)}
                  className="flex-1 bg-green-600 hover:bg-green-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
                >
                  <LinkIcon size={14} /> Connect Device
                </button>
              )}
              <button
                onClick={() => handleTestConnection(selectedDevice.id)}
                className="flex-1 bg-cyan-600 hover:bg-cyan-700 py-2 rounded-lg font-semibold text-sm transition flex items-center justify-center gap-2"
              >
                <Zap size={14} /> Test Connection
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Secure hardware wallet integration with Ledger & Trezor devices.
      </div>
    </div>
  );
}
