import React, { useState } from "react";
import { Maximize2, Grid3x3, Layout, RotateCcw } from "lucide-react";
import clsx from "clsx";

interface Layout {
  id: string;
  name: string;
  description: string;
  icon: React.ReactNode;
  preview: JSX.Element;
}

const LAYOUTS: Layout[] = [
  {
    id: "2x2",
    name: "2x2 Grid",
    description: "Four equal panels in 2x2 grid",
    icon: <Grid3x3 size={20} />,
    preview: (
      <div className="grid grid-cols-2 gap-1 w-full h-24">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="bg-blue-600 rounded-sm opacity-70" />
        ))}
      </div>
    ),
  },
  {
    id: "1+2",
    name: "Main + Split",
    description: "Large panel left, two smaller right",
    icon: <Layout size={20} />,
    preview: (
      <div className="flex gap-1 w-full h-24">
        <div className="flex-1 bg-blue-600 rounded-sm opacity-70" />
        <div className="flex flex-col gap-1 flex-1">
          <div className="flex-1 bg-blue-600 rounded-sm opacity-70" />
          <div className="flex-1 bg-blue-600 rounded-sm opacity-70" />
        </div>
      </div>
    ),
  },
  {
    id: "fullscreen",
    name: "Fullscreen",
    description: "One panel maximized",
    icon: <Maximize2 size={20} />,
    preview: (
      <div className="w-full h-24 bg-blue-600 rounded-sm opacity-70" />
    ),
  },
  {
    id: "3col",
    name: "Three Columns",
    description: "Three equal vertical panels",
    icon: <Layout size={20} />,
    preview: (
      <div className="grid grid-cols-3 gap-1 w-full h-24">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="bg-blue-600 rounded-sm opacity-70" />
        ))}
      </div>
    ),
  },
];

export default function WindowLayoutsPanel() {
  const [selectedLayout, setSelectedLayout] = useState("2x2");
  const [windowSnapEnabled, setWindowSnapEnabled] = useState(true);
  const [gridSize, setGridSize] = useState(20);

  const handleApplyLayout = (layoutId: string) => {
    setSelectedLayout(layoutId);
  };

  const handleResetLayout = () => {
    setSelectedLayout("2x2");
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold flex items-center gap-2">
          <Maximize2 size={20} /> Window Layouts
        </h2>
        <button
          onClick={handleResetLayout}
          className="flex items-center gap-1 bg-[#15151b] hover:bg-[#1a1a20] px-3 py-1 rounded text-sm transition"
        >
          <RotateCcw size={14} /> Reset
        </button>
      </div>

      <div className="flex-1 overflow-y-auto mb-4 space-y-4">
        {/* Window Snap Settings */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Snap Settings</h3>
          
          <div className="space-y-4">
            {/* Window Snap Toggle */}
            <div className="flex items-center justify-between">
              <div>
                <h4 className="text-sm font-medium">Window Snapping</h4>
                <p className="text-xs text-gray-400">Auto-snap windows to grid positions</p>
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
              <div>
                <label className="text-sm text-gray-300 block mb-2">Grid Size: {gridSize}px</label>
                <input
                  type="range"
                  min="10"
                  max="40"
                  step="5"
                  value={gridSize}
                  onChange={(e) => setGridSize(parseInt(e.target.value))}
                  className="w-full"
                />
                <p className="text-xs text-gray-500 mt-1">Smaller = more precise snap positions</p>
              </div>
            )}
          </div>
        </div>

        {/* Layout Grid */}
        <div>
          <h3 className="font-semibold mb-3">Quick Layouts</h3>
          <div className="grid grid-cols-2 gap-3">
            {LAYOUTS.map((layout) => (
              <button
                key={layout.id}
                onClick={() => handleApplyLayout(layout.id)}
                className={clsx(
                  "p-3 rounded-lg border-2 transition text-left",
                  selectedLayout === layout.id
                    ? "border-blue-400 bg-blue-600/10"
                    : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
                )}
              >
                <div className="flex items-center gap-2 mb-2">
                  <div className={selectedLayout === layout.id ? "text-blue-400" : "text-gray-400"}>
                    {layout.icon}
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">{layout.name}</h4>
                    <p className="text-xs text-gray-400">{layout.description}</p>
                  </div>
                </div>
                <div className="rounded bg-[#2a2a35] p-2">
                  {layout.preview}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Advanced Options */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Advanced Options</h3>
          
          <div className="space-y-3">
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                defaultChecked
                className="w-4 h-4 rounded"
              />
              <span className="text-sm">Remember window positions</span>
            </label>

            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                defaultChecked
                className="w-4 h-4 rounded"
              />
              <span className="text-sm">Animate layout transitions</span>
            </label>

            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                className="w-4 h-4 rounded"
              />
              <span className="text-sm">Lock layout changes</span>
            </label>

            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                defaultChecked
                className="w-4 h-4 rounded"
              />
              <span className="text-sm">Show snap guides</span>
            </label>
          </div>
        </div>

        {/* Saved Layouts */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Saved Layouts</h3>
          
          <div className="space-y-2">
            {[
              { name: "Trading Setup", layout: "1+2", created: "Today" },
              { name: "Analysis View", layout: "3col", created: "2 days ago" },
              { name: "Wallet Focus", layout: "fullscreen", created: "1 week ago" },
            ].map((saved, i) => (
              <button
                key={i}
                className="w-full text-left p-2 bg-[#2a2a35] hover:bg-[#3a3a45] rounded transition flex items-center justify-between"
              >
                <div className="text-sm">
                  <div className="font-medium">{saved.name}</div>
                  <div className="text-xs text-gray-400">{saved.layout} layout • {saved.created}</div>
                </div>
                <span className="text-xs bg-blue-600/20 text-blue-400 px-2 py-1 rounded">Load</span>
              </button>
            ))}
          </div>
        </div>

        {/* Current Layout Preview */}
        <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-4">
          <h3 className="font-semibold mb-3">Current Layout</h3>
          <div className="bg-[#2a2a35] p-3 rounded">
            {LAYOUTS.find((l) => l.id === selectedLayout)?.preview}
          </div>
          <p className="text-xs text-gray-400 mt-2">
            Active Layout: {LAYOUTS.find((l) => l.id === selectedLayout)?.name}
          </p>
        </div>
      </div>

      <button className="w-full bg-blue-600 hover:bg-blue-700 py-2 rounded-lg font-semibold text-sm transition">
        Apply Layout
      </button>
    </div>
  );
}
