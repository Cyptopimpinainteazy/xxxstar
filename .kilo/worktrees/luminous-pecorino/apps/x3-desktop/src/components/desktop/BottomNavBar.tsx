/**
 * BottomNavBar.tsx — bottom navigation bar with two-column icon layout.
 *
 * Features:
 * - Two columns of navigation icons (left and right)
 * - Quick access to key applications
 * - Terminal icon on the bottom right
 */
import React from "react";
import { useWindowManager } from "@/hooks/useWindowManager";
import { useApplicationStore } from "@/stores/applicationStore";
import { useDesktopStore } from "@/stores/desktopStore";

interface NavItem {
  appId: string;
  label: string;
  emoji: string;
  accentColor?: string;
}

const BottomNavBar: React.FC<{
  onTerminalToggle?: () => void;
  isTerminalOpen?: boolean;
}> = ({ onTerminalToggle, isTerminalOpen = true }) => {
  const { launch } = useWindowManager();
  const { isRunning } = useApplicationStore();
  const clearAllWindows = useDesktopStore((s) => s.clearAllWindows);

  // Define navigation items in two columns
  const leftColumnItems: NavItem[] = [
    { appId: "x3-floor-dashboard", label: "X3 Intelligence", emoji: "🤖" },
    { appId: "defi-metrics", label: "Analytics", emoji: "📊" },
    { appId: "block-explorer", label: "Explorer", emoji: "🔍" },
  ];

  const rightColumnItems: NavItem[] = [
    { appId: "wallet", label: "Wallet", emoji: "💰" },
    { appId: "dex", label: "DEX", emoji: "💱" },
    { appId: "dex-orderbook", label: "Orderbook", emoji: "📊" },
    { appId: "infrastructure", label: "Infrastructure", emoji: "🏗️" },
    { appId: "system-monitoring", label: "System Monitor", emoji: "📈" },
    { appId: "admin-dashboard", label: "Admin", emoji: "🛡️" },
    { appId: "documentation", label: "Documentation", emoji: "📚" },
  ];

  const handleAppClick = (appId: string) => {
    launch(appId);
  };

  return (
    <div
      className="fixed bottom-6 left-1/2 transform -translate-x-1/2 h-auto z-[9998]"
      role="navigation"
      aria-label="Bottom navigation"
    >
      <div className="glass-panel flex items-center gap-3 px-6 py-3 rounded-full border border-[#1a9fb5]/40 shadow-2xl"
        style={{
          boxShadow: '0 0 40px rgba(26, 159, 181, 0.7), 0 0 20px rgba(26, 159, 181, 0.5), 0 8px 16px rgba(0, 0, 0, 0.4)'
        }}>
        {/* Left column items */}
        <div className="flex gap-3">
          {leftColumnItems.map((item) => {
            const running = isRunning(item.appId);

            return (
              <button
                key={item.appId}
                onClick={() => handleAppClick(item.appId)}
                className={`flex items-center justify-center w-12 h-12 rounded-lg
                  transition-all duration-200 relative
                  ${
                    running
                      ? "bg-white/10 border border-green-500/60 hover:border-green-500"
                      : "bg-white/5 border border-red-500/60 hover:border-red-500"
                  }`}
                title={item.label}
                style={running ? {
                  boxShadow: '0 0 20px rgba(34, 197, 94, 0.6), inset 0 0 15px rgba(34, 197, 94, 0.2)'
                } : {
                  boxShadow: '0 0 20px rgba(239, 68, 68, 0.6), inset 0 0 15px rgba(239, 68, 68, 0.2)'
                }}
              >
                <span className="text-xl">{item.emoji}</span>
              </button>
            );
          })}
        </div>

        {/* Divider */}
        <div className="w-px h-6 bg-white/10" />

        {/* Right column items */}
        <div className="flex gap-3">
          {rightColumnItems.map((item) => {
            const running = isRunning(item.appId);

            return (
              <button
                key={item.appId}
                onClick={() => handleAppClick(item.appId)}
                className={`flex items-center justify-center w-12 h-12 rounded-lg
                  transition-all duration-200 relative
                  ${
                    running
                      ? "bg-white/10 border border-green-500/60 hover:border-green-500"
                      : "bg-white/5 border border-red-500/60 hover:border-red-500"
                  }`}
                title={item.label}
                style={running ? {
                  boxShadow: '0 0 20px rgba(34, 197, 94, 0.6), inset 0 0 15px rgba(34, 197, 94, 0.2)'
                } : {
                  boxShadow: '0 0 20px rgba(239, 68, 68, 0.6), inset 0 0 15px rgba(239, 68, 68, 0.2)'
                }}
              >
                <span className="text-xl">{item.emoji}</span>
              </button>
            );
          })}
        </div>

        {/* Divider */}
        <div className="w-px h-6 bg-white/10" />

        {/* Clear Windows button */}
        <button
          onClick={clearAllWindows}
          className="flex items-center justify-center w-12 h-12 rounded-lg
            transition-all duration-200 relative
            bg-white/5 border border-orange-500/60 hover:border-orange-500"
          title="Clear All Windows"
          style={{
            boxShadow: '0 0 20px rgba(249, 115, 22, 0.6), inset 0 0 15px rgba(249, 115, 22, 0.2)'
          }}
        >
          <span className="text-xl">🗑️</span>
        </button>

        {/* Divider */}
        <div className="w-px h-6 bg-white/10" />

        {/* Terminal icon - on the right side */}
        <button
          onClick={onTerminalToggle}
          className={`flex items-center justify-center w-12 h-12 rounded-lg
            transition-all duration-200 relative
            ${
              isTerminalOpen
                ? "bg-white/10 border border-green-500/60 hover:border-green-500"
                : "bg-white/5 border border-red-500/60 hover:border-red-500"
            }`}
          title="Terminal (Ctrl+Alt+T)"
          style={isTerminalOpen ? {
            boxShadow: '0 0 20px rgba(34, 197, 94, 0.6), inset 0 0 15px rgba(34, 197, 94, 0.2)'
          } : {
            boxShadow: '0 0 20px rgba(239, 68, 68, 0.6), inset 0 0 15px rgba(239, 68, 68, 0.2)'
          }}
        >
          <span className="text-xl">⌨</span>
        </button>
      </div>
    </div>
  );
};

export default React.memo(BottomNavBar);
