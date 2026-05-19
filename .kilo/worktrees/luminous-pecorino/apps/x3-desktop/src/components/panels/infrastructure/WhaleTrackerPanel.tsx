import React, { useState, useEffect } from 'react';
import { AlertTriangle, TrendingUp, TrendingDown, Bell, Search, Filter, Zap } from 'lucide-react';
import clsx from 'clsx';

interface WhaleAlert {
  id: string;
  wallet: string;
  action: 'send' | 'receive';
  amount: number;
  token: string;
  usdValue: number;
  timestamp: number;
  from: string;
  to: string;
  isTracked: boolean;
}

interface WalletWatch {
  address: string;
  label: string;
  alerts: number;
  totalVolume: number;
  lastAlert: number;
}

const MOCK_WHALES: WhaleAlert[] = [
  {
    id: '1',
    wallet: '0x123...abc',
    action: 'send',
    amount: 150000,
    token: 'X3',
    usdValue: 187500,
    timestamp: Date.now() - 300000,
    from: '0x123...abc',
    to: '0xdef...789',
    isTracked: true,
  },
  {
    id: '2',
    wallet: '0xabc...def',
    action: 'receive',
    amount: 250000,
    token: 'USDC',
    usdValue: 250000,
    timestamp: Date.now() - 1800000,
    from: '0x789...xyz',
    to: '0xabc...def',
    isTracked: false,
  },
  {
    id: '3',
    wallet: '0x999...111',
    action: 'send',
    amount: 500000,
    token: 'ETH',
    usdValue: 1625000,
    timestamp: Date.now() - 3600000,
    from: '0x999...111',
    to: 'Bridge Contract',
    isTracked: true,
  },
];

const MOCK_WATCHES: WalletWatch[] = [
  { address: '0x123...abc', label: 'Major Holder A', alerts: 12, totalVolume: 2500000, lastAlert: Date.now() - 300000 },
  { address: '0x999...111', label: 'Whale Bridge Bot', alerts: 8, totalVolume: 1800000, lastAlert: Date.now() - 3600000 },
  { address: '0xdef...456', label: 'Exchange Coldwallet', alerts: 24, totalVolume: 5200000, lastAlert: Date.now() - 86400000 },
];

const WhaleTrackerPanel: React.FC = () => {
  const [alerts, setAlerts] = useState<WhaleAlert[]>(MOCK_WHALES);
  const [watchlist, setWatchlist] = useState<WalletWatch[]>(MOCK_WATCHES);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterAction, setFilterAction] = useState<'all' | 'send' | 'receive'>('all');
  const [showAddWatch, setShowAddWatch] = useState(false);
  const [newWatchAddress, setNewWatchAddress] = useState('');
  const [newWatchLabel, setNewWatchLabel] = useState('');
  const [minValue, setMinValue] = useState(100); // $100K minimum

  const filteredAlerts = alerts.filter((alert) => {
    const matchesSearch = !searchQuery || 
      alert.wallet.toLowerCase().includes(searchQuery.toLowerCase()) ||
      alert.token.toLowerCase().includes(searchQuery.toLowerCase());
    
    const matchesAction = filterAction === 'all' || alert.action === filterAction;
    const matchesValue = alert.usdValue >= minValue * 1000;

    return matchesSearch && matchesAction && matchesValue;
  });

  const totalVolumeTracked = watchlist.reduce((sum, w) => sum + w.totalVolume, 0);
  const avgAlertValue = alerts.length > 0 
    ? alerts.reduce((sum, a) => sum + a.usdValue, 0) / alerts.length 
    : 0;

  const handleAddToWatchlist = () => {
    if (newWatchAddress && newWatchLabel) {
      const newWatch: WalletWatch = {
        address: newWatchAddress,
        label: newWatchLabel,
        alerts: 0,
        totalVolume: 0,
        lastAlert: Date.now(),
      };
      setWatchlist([...watchlist, newWatch]);
      setNewWatchAddress('');
      setNewWatchLabel('');
      setShowAddWatch(false);
    }
  };

  const handleRemoveWatch = (address: string) => {
    setWatchlist(watchlist.filter(w => w.address !== address));
  };

  const formatTime = (timestamp: number) => {
    const diff = Date.now() - timestamp;
    const hours = Math.floor(diff / 3600000);
    const mins = Math.floor((diff % 3600000) / 60000);
    
    if (hours > 24) return `${Math.floor(hours / 24)}d ago`;
    if (hours > 0) return `${hours}h ago`;
    return `${mins}m ago`;
  };

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <AlertTriangle size={18} className="text-orange-400" />
          <h1 className="text-lg font-bold">Whale Tracker</h1>
          <span className="text-xs bg-orange-500/20 text-orange-400 px-2 py-0.5 rounded">Monitor large transfers</span>
        </div>
        <button
          onClick={() => setShowAddWatch(true)}
          className="flex items-center gap-2 bg-gradient-to-r from-orange-500 to-orange-600 hover:from-orange-400 hover:to-orange-500 text-white px-4 py-2 rounded-lg font-semibold text-sm transition-all"
        >
          <Bell size={14} /> Watch Address
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4 border-b border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Alerts (24h)</div>
          <div className="text-lg font-bold text-orange-400">{alerts.length}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Avg Alert Value</div>
          <div className="text-lg font-bold text-white">${(avgAlertValue / 1000000).toFixed(1)}M</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Tracked Volume</div>
          <div className="text-lg font-bold text-green-400">${(totalVolumeTracked / 1000000).toFixed(1)}M</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Watchlist Size</div>
          <div className="text-lg font-bold text-blue-400">{watchlist.length}</div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 px-5 py-3 border-b border-[#1a1a1a]">
        <button className="px-4 py-2 rounded-lg text-sm font-medium text-orange-400 bg-orange-500/10 border border-orange-500/40">
          🚨 Live Alerts
        </button>
        <button className="px-4 py-2 rounded-lg text-sm font-medium text-gray-500 hover:text-white transition-colors">
          📋 Watchlist
        </button>
      </div>

      {/* Filters */}
      <div className="flex gap-3 px-5 py-3 border-b border-[#1a1a1a]">
        <div className="relative flex-1">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
          <input
            type="text"
            placeholder="Search wallet or token..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg pl-9 pr-3 py-2 text-xs text-white placeholder-gray-600 outline-none focus:border-orange-500/40"
          />
        </div>
        <select
          value={filterAction}
          onChange={(e) => setFilterAction(e.target.value as 'all' | 'send' | 'receive')}
          className="bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-orange-500/40"
        >
          <option value="all">All Transfers</option>
          <option value="send">Outgoing Only</option>
          <option value="receive">Incoming Only</option>
        </select>
        <select
          value={minValue}
          onChange={(e) => setMinValue(parseInt(e.target.value))}
          className="bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-orange-500/40"
        >
          <option value={100}>Min $100K</option>
          <option value={500}>Min $500K</option>
          <option value={1000}>Min $1M</option>
          <option value={5000}>Min $5M</option>
        </select>
      </div>

      {/* Alerts List */}
      <div className="flex-1 overflow-auto px-5 py-4 space-y-3">
        {filteredAlerts.map((alert) => (
          <div key={alert.id} className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 hover:border-[#2a2a2a] transition-colors">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className={clsx(
                  'p-2 rounded-lg',
                  alert.action === 'send' ? 'bg-red-500/20' : 'bg-green-500/20'
                )}>
                  {alert.action === 'send' 
                    ? <TrendingDown size={16} className="text-red-400" />
                    : <TrendingUp size={16} className="text-green-400" />
                  }
                </div>
                <div>
                  <div className="font-semibold text-white">{alert.wallet}</div>
                  <div className="text-xs text-gray-500">{formatTime(alert.timestamp)}</div>
                </div>
              </div>
              <div className="text-right">
                <div className={clsx(
                  'font-bold text-lg',
                  alert.action === 'send' ? 'text-red-400' : 'text-green-400'
                )}>
                  {alert.action === 'send' ? '-' : '+'}{alert.amount.toLocaleString()} {alert.token}
                </div>
                <div className="text-xs text-gray-500">${alert.usdValue.toLocaleString()}</div>
              </div>
            </div>

            <div className="bg-[#0a0a0f] rounded-lg p-3 mb-3 text-xs space-y-1">
              <div className="flex justify-between">
                <span className="text-gray-500">From:</span>
                <span className="text-white font-mono">{alert.from}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">To:</span>
                <span className="text-white font-mono">{alert.to}</span>
              </div>
            </div>

            {alert.isTracked && (
              <div className="flex items-center gap-2 px-2 py-1 rounded bg-orange-500/20 border border-orange-500/40 w-fit text-xs text-orange-400">
                <Bell size={12} /> Tracked
              </div>
            )}
          </div>
        ))}

        {filteredAlerts.length === 0 && (
          <div className="text-center py-12 text-gray-500">
            <AlertTriangle size={32} className="mx-auto mb-2 opacity-20" />
            <p>No alerts match your filters.</p>
          </div>
        )}
      </div>

      {/* Add to Watchlist Modal */}
      {showAddWatch && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
            <h3 className="font-bold text-white mb-4 flex items-center gap-2">
              <Bell size={18} className="text-orange-400" />
              Add To Watchlist
            </h3>

            <div className="space-y-4 mb-6">
              <div>
                <label className="block text-xs text-gray-500 mb-2">Wallet Address</label>
                <input
                  type="text"
                  value={newWatchAddress}
                  onChange={(e) => setNewWatchAddress(e.target.value)}
                  placeholder="0x..."
                  className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 text-white text-sm outline-none focus:border-orange-500/40"
                />
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-2">Label (e.g., "Binance Whale")</label>
                <input
                  type="text"
                  value={newWatchLabel}
                  onChange={(e) => setNewWatchLabel(e.target.value)}
                  placeholder="Nickname for this address"
                  className="w-full bg-[#0a0a0f] border border-[#1a1a1a] rounded-lg p-3 text-white text-sm outline-none focus:border-orange-500/40"
                />
              </div>

              <div className="bg-[#0a0a0f] rounded-lg p-3 text-sm text-gray-400">
                <p>You'll receive alerts when this address transfers over $100k.</p>
              </div>
            </div>

            <div className="flex gap-2 justify-end">
              <button
                onClick={() => setShowAddWatch(false)}
                className="px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleAddToWatchlist}
                disabled={!newWatchAddress || !newWatchLabel}
                className="px-4 py-2 rounded-lg bg-gradient-to-r from-orange-500 to-orange-600 text-white font-semibold disabled:from-gray-600 disabled:to-gray-600 transition-all"
              >
                Add Watch
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default WhaleTrackerPanel;

