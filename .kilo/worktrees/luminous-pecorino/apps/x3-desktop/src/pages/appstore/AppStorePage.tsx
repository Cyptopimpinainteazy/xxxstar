/**
 * App Store Page
 * 
 * Marketplace for third-party apps integrated with X3 Desktop.
 * All apps are configured to route 50% of earnings to X3 Treasury.
 */
import React, { useState, useMemo } from "react";
import { 
  APP_STORE_APPS, 
  getAppsByCategory, 
  getTreasuryIntegratedApps,
  type AppCategory,
  type AppStoreApp 
} from "../../config/appstore.config";
import { X3_TREASURY_CONFIG, calculateTreasurySplit } from "../../config/treasury.config";
import { Download, Play, Settings, TrendingUp, Shield, Zap } from "lucide-react";
import { appLauncher } from "../../services/AppLauncherService";

type FilterType = "all" | "installed" | "treasury" | AppCategory;

export const AppStorePage: React.FC = () => {
  const [filter, setFilter] = useState<FilterType>("all");
  const [selectedApp, setSelectedApp] = useState<AppStoreApp | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const filteredApps = useMemo(() => {
    let apps = APP_STORE_APPS;

    // Apply filter
    if (filter !== "all") {
      if (filter === "installed") {
        apps = apps.filter(app => app.installed);
      } else if (filter === "treasury") {
        apps = getTreasuryIntegratedApps();
      } else {
        apps = getAppsByCategory(filter as AppCategory);
      }
    }

    // Apply search
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      apps = apps.filter(app => 
        app.name.toLowerCase().includes(query) ||
        app.description.toLowerCase().includes(query) ||
        app.features.some(f => f.toLowerCase().includes(query))
      );
    }

    return apps;
  }, [filter, searchQuery]);

  const treasuryStats = useMemo(() => {
    const integratedApps = getTreasuryIntegratedApps();
    const totalApps = APP_STORE_APPS.length;
    const percentage = Math.round((integratedApps.length / totalApps) * 100);
    return {
      total: totalApps,
      integrated: integratedApps.length,
      percentage
    };
  }, []);

  return (
    <div className="flex flex-col h-full bg-gradient-to-br from-[#0a0a0a] via-[#1a1a1a] to-[#0a0a0a] text-white p-6 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold bg-gradient-to-r from-[#ff6b35] via-[#f7931a] to-[#ff6b35] bg-clip-text text-transparent">
            X3 App Store
          </h1>
          <p className="text-sm text-gray-400 mt-1">
            Third-party apps integrated with X3 Treasury
          </p>
        </div>

        {/* Treasury Status Badge */}
        <div className="flex items-center gap-4">
          <div className="bg-gradient-to-r from-[#ff6b35]/20 to-[#f7931a]/20 rounded-lg px-4 py-2 border border-[#ff6b35]/30">
            <div className="flex items-center gap-2">
              <Shield className="w-4 h-4 text-[#ff6b35]" />
              <div className="text-xs">
                <div className="text-gray-400">Treasury Integration</div>
                <div className="font-bold text-white">
                  {treasuryStats.integrated}/{treasuryStats.total} apps ({treasuryStats.percentage}%)
                </div>
              </div>
            </div>
          </div>
          
          <div className="bg-[#ff6b35]/10 rounded-lg px-4 py-2 border border-[#ff6b35]/30">
            <div className="text-xs text-gray-400">Treasury Share</div>
            <div className="text-lg font-bold text-[#ff6b35]">
              {X3_TREASURY_CONFIG.treasuryShare}%
            </div>
          </div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="mb-4">
        <input
          type="text"
          placeholder="Search apps, features, or categories..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full bg-[#2a2a2a] border border-gray-700 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:border-[#ff6b35] transition-colors"
        />
      </div>

      {/* Filter Tabs */}
      <div className="flex gap-2 mb-6 overflow-x-auto pb-2">
        {[
          { id: "all", label: "All Apps", icon: "📦" },
          { id: "installed", label: "Installed", icon: "✅" },
          { id: "treasury", label: "Treasury Integrated", icon: "💰" },
          { id: "trading", label: "Trading", icon: "📈" },
          { id: "wallet", label: "Wallets", icon: "👛" },
          { id: "defi", label: "DeFi", icon: "🏦" },
          { id: "mining", label: "Mining", icon: "⛏️" },
          { id: "ai", label: "AI", icon: "🤖" },
          { id: "tools", label: "Tools", icon: "🔧" },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setFilter(tab.id as FilterType)}
            className={`px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all ${
              filter === tab.id
                ? "bg-gradient-to-r from-[#ff6b35] to-[#f7931a] text-white shadow-lg"
                : "bg-[#2a2a2a] text-gray-400 hover:bg-[#3a3a3a] hover:text-white"
            }`}
          >
            <span className="mr-2">{tab.icon}</span>
            {tab.label}
          </button>
        ))}
      </div>

      {/* App Grid */}
      <div className="flex-1 overflow-y-auto">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-6">
          {filteredApps.map((app) => (
            <AppCard 
              key={app.id} 
              app={app} 
              onSelect={setSelectedApp}
            />
          ))}
        </div>

        {filteredApps.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-gray-500">
            <div className="text-4xl mb-4">🔍</div>
            <div className="text-lg font-medium">No apps found</div>
            <div className="text-sm">Try adjusting your search or filters</div>
          </div>
        )}
      </div>

      {/* App Detail Modal */}
      {selectedApp && (
        <AppDetailModal 
          app={selectedApp} 
          onClose={() => setSelectedApp(null)} 
        />
      )}
    </div>
  );
};

/* ── App Card Component ──────────────────────────────────── */

interface AppCardProps {
  app: AppStoreApp;
  onSelect: (app: AppStoreApp) => void;
}

const AppCard: React.FC<AppCardProps> = ({ app, onSelect }) => {
  const getCategoryColor = (category: AppCategory) => {
    const colors: Record<AppCategory, string> = {
      trading: "from-blue-500/20 to-cyan-500/20 border-blue-500/30",
      wallet: "from-purple-500/20 to-pink-500/20 border-purple-500/30",
      defi: "from-green-500/20 to-emerald-500/20 border-green-500/30",
      mining: "from-orange-500/20 to-yellow-500/20 border-orange-500/30",
      ai: "from-red-500/20 to-pink-500/20 border-red-500/30",
      agent: "from-indigo-500/20 to-purple-500/20 border-indigo-500/30",
      tools: "from-gray-500/20 to-slate-500/20 border-gray-500/30",
      gaming: "from-pink-500/20 to-rose-500/20 border-pink-500/30",
    };
    return colors[category];
  };

  return (
    <div 
      onClick={() => onSelect(app)}
      className="bg-gradient-to-br from-[#1a1a1a] to-[#2a2a2a] rounded-xl p-4 border border-gray-800 hover:border-[#ff6b35] transition-all cursor-pointer group"
    >
      {/* App Icon & Status */}
      <div className="flex items-start justify-between mb-3">
        <div className={`text-4xl bg-gradient-to-br ${getCategoryColor(app.category)} rounded-lg p-3 border`}>
          {app.icon}
        </div>
        <div className="flex flex-col gap-1 items-end">
          {app.installed && (
            <span className="text-xs bg-green-500/20 text-green-400 px-2 py-1 rounded border border-green-500/30">
              Installed
            </span>
          )}
          {app.treasuryIntegrated && (
            <span className="text-xs bg-[#ff6b35]/20 text-[#ff6b35] px-2 py-1 rounded border border-[#ff6b35]/30">
              X3 Treasury
            </span>
          )}
        </div>
      </div>

      {/* App Info */}
      <h3 className="text-lg font-bold text-white mb-1 group-hover:text-[#ff6b35] transition-colors">
        {app.name}
      </h3>
      <p className="text-xs text-gray-500 mb-2">{app.chain} • {app.category}</p>
      <p className="text-sm text-gray-400 mb-3 line-clamp-2">
        {app.description}
      </p>

      {/* Quick Stats */}
      <div className="flex items-center justify-between text-xs text-gray-500 mt-auto pt-3 border-t border-gray-800">
        <span>v{app.version}</span>
        <span>{app.size}</span>
      </div>
    </div>
  );
};

/* ── App Detail Modal ───────────────────────────────────── */

interface AppDetailModalProps {
  app: AppStoreApp;
  onClose: () => void;
}

const AppDetailModal: React.FC<AppDetailModalProps> = ({ app, onClose }) => {
  const [isLaunching, setIsLaunching] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);

  const handleLaunch = async () => {
    setIsLaunching(true);
    
    try {
      const result = await appLauncher.launchApp(app);
      
      if (result.success) {
        console.log(`[AppStore] ${app.name} launched successfully (PID: ${result.pid})`);
        alert(`✅ ${app.name} launched successfully!\n\nTreasury integration: ${app.treasuryIntegrated ? 'ENABLED (50% to X3)' : 'N/A'}`);
      } else {
        console.error(`[AppStore] Failed to launch ${app.name}: ${result.message}`);
        alert(`❌ Failed to launch ${app.name}\n\n${result.message}\n\nError: ${result.error || 'Unknown'}`);
      }
    } catch (error: any) {
      console.error(`[AppStore] Launch error:`, error);
      alert(`❌ Failed to launch ${app.name}\n\n${error.message}`);
    } finally {
      setIsLaunching(false);
    }
  };

  const handleInstall = async () => {
    setIsInstalling(true);
    try {
      const result = await appLauncher.installApp(app as AppStoreApp);
      if (result.success) {
        alert(`✅ ${app.name} installed/repaired successfully`);
      } else {
        alert(`❌ Install failed: ${result.message}\n\n${result.error || ''}`);
      }
    } catch (err: any) {
      alert(`❌ Install failed: ${err?.message || String(err)}`);
    } finally {
      setIsInstalling(false);
    }
  };

  const treasurySplit = calculateTreasurySplit(100);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-6">
      <div className="bg-gradient-to-br from-[#1a1a1a] to-[#2a2a2a] rounded-2xl max-w-3xl w-full max-h-[90vh] overflow-y-auto border border-gray-800">
        {/* Header */}
        <div className="sticky top-0 bg-gradient-to-r from-[#1a1a1a] to-[#2a2a2a] p-6 border-b border-gray-800 backdrop-blur-sm">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <div className="text-5xl">{app.icon}</div>
              <div>
                <h2 className="text-2xl font-bold text-white">{app.name}</h2>
                <p className="text-sm text-gray-400">
                  by {app.author} • v{app.version}
                </p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-white transition-colors text-2xl"
            >
              ×
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {/* Description */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-2">Description</h3>
            <p className="text-gray-400">{app.description}</p>
          </div>

          {/* Treasury Integration */}
          {app.treasuryIntegrated && (
            <div className="bg-gradient-to-r from-[#ff6b35]/10 to-[#f7931a]/10 rounded-lg p-4 border border-[#ff6b35]/30">
              <div className="flex items-center gap-2 mb-2">
                <TrendingUp className="w-5 h-5 text-[#ff6b35]" />
                <h4 className="font-semibold text-white">X3 Treasury Integration</h4>
              </div>
              <p className="text-sm text-gray-300 mb-3">
                This app automatically routes <span className="font-bold text-[#ff6b35]">50%</span> of all earnings to the X3 Treasury.
              </p>
              <div className="flex gap-4 text-sm">
                <div className="flex-1 bg-[#ff6b35]/20 rounded p-3">
                  <div className="text-gray-400">To Treasury</div>
                  <div className="text-xl font-bold text-[#ff6b35]">{treasurySplit.treasury}%</div>
                </div>
                <div className="flex-1 bg-white/10 rounded p-3">
                  <div className="text-gray-400">To You</div>
                  <div className="text-xl font-bold text-white">{treasurySplit.user}%</div>
                </div>
              </div>
            </div>
          )}

          {/* Features */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-3">Features</h3>
            <ul className="space-y-2">
              {app.features.map((feature, idx) => (
                <li key={idx} className="flex items-start gap-2 text-gray-400">
                  <Zap className="w-4 h-4 text-[#ff6b35] mt-0.5 flex-shrink-0" />
                  <span>{feature}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Requirements */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-3">Requirements</h3>
            <ul className="space-y-2">
              {app.requirements.map((req, idx) => (
                <li key={idx} className="flex items-start gap-2 text-gray-400">
                  <Settings className="w-4 h-4 text-gray-500 mt-0.5 flex-shrink-0" />
                  <span>{req}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Metadata */}
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div className="bg-[#2a2a2a] rounded-lg p-3">
              <div className="text-gray-400">Chain</div>
              <div className="font-medium text-white">{app.chain}</div>
            </div>
            <div className="bg-[#2a2a2a] rounded-lg p-3">
              <div className="text-gray-400">Category</div>
              <div className="font-medium text-white">{app.category}</div>
            </div>
            <div className="bg-[#2a2a2a] rounded-lg p-3">
              <div className="text-gray-400">Size</div>
              <div className="font-medium text-white">{app.size}</div>
            </div>
            <div className="bg-[#2a2a2a] rounded-lg p-3">
              <div className="text-gray-400">Status</div>
              <div className={`font-medium ${app.installed ? "text-green-400" : "text-gray-400"}`}>
                {app.installed ? "Installed" : "Not Installed"}
              </div>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="sticky bottom-0 bg-gradient-to-r from-[#1a1a1a] to-[#2a2a2a] p-6 border-t border-gray-800 backdrop-blur-sm">
          <div className="flex gap-3">
            {app.installed ? (
              <>
                <button
                  onClick={handleLaunch}
                  disabled={isLaunching}
                  className="flex-1 bg-gradient-to-r from-[#ff6b35] to-[#f7931a] text-white py-3 rounded-lg font-medium hover:opacity-90 transition-opacity flex items-center justify-center gap-2 disabled:opacity-50"
                >
                  <Play className="w-5 h-5" />
                  {isLaunching ? "Launching..." : "Launch App"}
                </button>

                <button
                  onClick={handleInstall}
                  disabled={isInstalling}
                  className="px-6 py-3 bg-[#1f6feb] text-white rounded-lg hover:brightness-105 transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  {isInstalling ? 'Installing...' : 'Install / Repair'}
                </button>

                <button className="px-6 py-3 bg-[#2a2a2a] text-white rounded-lg hover:bg-[#3a3a3a] transition-colors flex items-center gap-2">
                  <Settings className="w-5 h-5" />
                  Configure
                </button>
              </>
            ) : (
              <button className="flex-1 bg-gradient-to-r from-[#ff6b35] to-[#f7931a] text-white py-3 rounded-lg font-medium hover:opacity-90 transition-opacity flex items-center justify-center gap-2">
                <Download className="w-5 h-5" />
                Install App
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
