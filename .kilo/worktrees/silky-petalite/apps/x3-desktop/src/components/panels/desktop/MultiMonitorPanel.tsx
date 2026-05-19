import React, { useState } from "react";
import { Monitor, Settings, Eye, Grid3x3 } from "lucide-react";
import clsx from "clsx";

interface Monitor {
  id: string;
  name: string;
  resolution: string;
  primary: boolean;
  position: "left" | "right" | "top" | "bottom" | "center";
}

export default function MultiMonitorPanel() {
  const [monitors, setMonitors] = useState<Monitor[]>([
    { id: "1", name: "Primary (HDMI)", resolution: "2560×1600", primary: true, position: "center" },
    { id: "2", name: "Secondary (DP)", resolution: "1920×1080", primary: false, position: "right" },
  ]);

  const [selectedPosition, setSelectedPosition] = useState<Monitor | null>(monitors[0]);
  const [windowSnapEnabled, setWindowSnapEnabled] = useState(true);
  const [mirrorMode, setMirrorMode] = useState(false);
  const [extendMode, setExtendMode] = useState(true);

  const handleSetPrimary = (monitorId: string) => {
    setMonitors(
      monitors.map((m) => ({
        ...m,
        primary: m.id === monitorId,
      }))
    );
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Monitor size={20} /> Multi-Monitor Setup
      </h2>

      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {/* Display Mode */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Display Mode</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="radio"
                checked={extendMode && !mirrorMode}
                onChange={() => {
                  setExtendMode(true);
                  setMirrorMode(false);
                }}
                className="w-4 h-4"
              />
              <div>
                <div className="text-sm font-medium">Extend Displays</div>
                <div className="text-xs text-gray-400">Use all monitors as one workspace</div>
              </div>
            </label>

            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="radio"
                checked={mirrorMode}
                onChange={() => {
                  setMirrorMode(true);
                  setExtendMode(false);
                }}
                className="w-4 h-4"
              />
              <div>
                <div className="text-sm font-medium">Mirror Displays</div>
                <div className="text-xs text-gray-400">Show same content on all monitors</div>
              </div>
            </label>
          </div>
        </div>

        {/* Connected Monitors */}
        <div>
          <h3 className="font-semibold mb-3">Connected Monitors</h3>
          <div className="space-y-3">
            {monitors.map((mon) => (
              <div
                key={mon.id}
                onClick={() => setSelectedPosition(mon)}
                className={clsx(
                  "p-4 rounded-lg border-2 cursor-pointer transition",
                  selectedPosition?.id === mon.id
                    ? "border-blue-400 bg-blue-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center justify-between mb-2">
                  <div>
                    <h4 className="font-semibold">{mon.name}</h4>
                    <p className="text-xs text-gray-400">{mon.resolution}</p>
                  </div>
                  {mon.primary && (
                    <span className="text-xs bg-blue-600 text-white px-2 py-1 rounded">
                      Primary
                    </span>
                  )}
                </div>

                {!mon.primary && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleSetPrimary(mon.id);
                    }}
                    className="text-xs bg-[#2a2a35] hover:bg-[#3a3a45] px-2 py-1 rounded text-blue-400"
                  >
                    Set as Primary
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* Monitor Layout */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Physical Layout</h3>
          <div className="relative bg-[#2a2a35] rounded p-4 h-48 flex items-center justify-center">
            <div className="flex gap-2">
              {monitors.map((mon) => (
                <div
                  key={mon.id}
                  className={clsx(
                    "flex items-center justify-center text-xs font-semibold rounded",
                    mon.primary ? "bg-blue-600 text-white w-32 h-24" : "bg-[#3a3a45] text-gray-400 w-24 h-20"
                  )}
                >
                  {mon.resolution}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Window Snapping */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <div>
              <h3 className="font-semibold">Window Snapping</h3>
              <p className="text-xs text-gray-400">Auto-snap windows at monitor boundaries</p>
            </div>
            <button
              onClick={() => setWindowSnapEnabled(!windowSnapEnabled)}
              className={clsx(
                "w-12 h-6 rounded-full transition relative",
                windowSnapEnabled ? "bg-green-600" : "bg-[#2a2a35]"
              )}
            >
              <div
                className={clsx(
                  "w-5 h-5 bg-white rounded-full absolute top-0.5 transition",
                  windowSnapEnabled ? "right-0.5" : "left-0.5"
                )}
              />
            </button>
          </div>

          {windowSnapEnabled && (
            <div className="space-y-2 text-xs text-gray-400">
              <label className="flex items-center gap-2">
                <input type="checkbox" defaultChecked className="w-3 h-3 rounded" />
                Drag to edges = snap
              </label>
              <label className="flex items-center gap-2">
                <input type="checkbox" defaultChecked className="w-3 h-3 rounded" />
                Across screens = continue drag
              </label>
              <label className="flex items-center gap-2">
                <input type="checkbox" defaultChecked className="w-3 h-3 rounded" />
                Corner snap (maximize)
              </label>
            </div>
          )}
        </div>

        {/* Resolution & Scaling */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Resolution & Scaling</h3>
          {selectedPosition && (
            <div className="space-y-3">
              <div>
                <label className="text-xs text-gray-400 block mb-1">Selected: {selectedPosition.name}</label>
              </div>
              <div>
                <label className="text-sm font-medium block mb-1">Resolution</label>
                <select defaultValue={selectedPosition.resolution} className="w-full bg-[#2a2a35] border border-[#3a3a45] rounded px-3 py-2 text-white text-sm">
                  <option>2560×1600 (Recommended)</option>
                  <option>2560×1440</option>
                  <option>1920×1200</option>
                  <option>1920×1080</option>
                </select>
              </div>
              <div>
                <label className="text-sm font-medium block mb-1">Scaling</label>
                <input
                  type="range"
                  min="100"
                  max="200"
                  step="10"
                  defaultValue="100"
                  className="w-full"
                />
              </div>
            </div>
          )}
        </div>
      </div>

      <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        Apply Monitor Settings
      </button>
    </div>
  );
}
