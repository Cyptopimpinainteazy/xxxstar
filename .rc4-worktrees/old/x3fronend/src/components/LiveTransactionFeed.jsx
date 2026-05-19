import { useState, useEffect } from 'react';

/**
 * LiveTransactionFeed component for displaying recent blockchain transactions
 * @param {Array} transactions - Array of transaction objects
 * @param {boolean} loading - Loading state
 */
export default function LiveTransactionFeed({ transactions = [], loading }) {
  const [visibleTransactions, setVisibleTransactions] = useState([]);

  // Animate new transactions appearing
  useEffect(() => {
    setVisibleTransactions(transactions.slice(0, 5));
  }, [transactions]);

  /**
   * Format timestamp to relative time (e.g., "2m ago")
   * @param {string} isoString - ISO date string
   * @returns {string} Formatted relative time
   */
  const formatRelativeTime = (isoString) => {
    if (!isoString) return 'unknown';
    try {
      const date = new Date(isoString);
      const now = new Date();
      const diffMs = now - date;
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMs / 3600000);

      if (diffMins < 1) return 'just now';
      if (diffMins < 60) return `${diffMins}m ago`;
      if (diffHours < 24) return `${diffHours}h ago`;
      return date.toLocaleDateString();
    } catch {
      return 'unknown';
    }
  };

  /**
   * Get color for transaction type
   * @param {string} type - Transaction type
   * @returns {string} Tailwind color class
   */
  const getTypeColor = (type) => {
    switch (type) {
      case 'BUY': return 'text-emerald-400 bg-emerald-400/10';
      case 'SELL': return 'text-red-400 bg-red-400/10';
      case 'STAKE': return 'text-cyan bg-cyan/10';
      case 'VOTE': return 'text-purple-400 bg-purple-400/10';
      case 'MOVE': return 'text-amber-400 bg-amber-400/10';
      default: return 'text-gray-400 bg-gray-400/10';
    }
  };

  /**
   * Format currency value
   * @param {number} value - USD value
   * @returns {string} Formatted currency string
   */
  const formatCurrency = (value) => {
    if (!value) return '$0';
    if (value >= 1000000) return `$${(value / 1000000).toFixed(2)}M`;
    if (value >= 1000) return `$${(value / 1000).toFixed(1)}K`;
    return `$${value.toFixed(2)}`;
  };

  /**
   * Truncate wallet address
   * @param {string} address - Full wallet address
   * @returns {string} Truncated address
   */
  const truncateAddress = (address) => {
    if (!address) return 'unknown';
    if (address.length <= 12) return address;
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  // Loading skeleton
  if (loading) {
    return (
      <div className="live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5">
        <div className="h-4 bg-gray-700 rounded w-40 mb-6 animate-pulse"></div>
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="flex items-center gap-4 py-3 border-b border-white/5 animate-pulse">
            <div className="h-8 w-8 bg-gray-700 rounded-full"></div>
            <div className="flex-1">
              <div className="h-3 bg-gray-700 rounded w-24 mb-2"></div>
              <div className="h-2 bg-gray-700 rounded w-32"></div>
            </div>
            <div className="h-3 bg-gray-700 rounded w-16"></div>
          </div>
        ))}
      </div>
    );
  }

  // Empty state
  if (!transactions || transactions.length === 0) {
    return (
      <div className="live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5">
        <h3 className="text-sm uppercase tracking-widest opacity-60 mb-4 font-mono">
          Recent Transactions
        </h3>
        <div className="text-center py-8">
          <p className="text-gray-400 text-sm">No recent transactions</p>
          <p className="text-gray-500 text-xs mt-1">Transactions will appear here in real-time</p>
        </div>
      </div>
    );
  }

  return (
    <div className="live-transaction-feed bg-surface-dark rounded-2xl p-6 border border-white/5">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-sm uppercase tracking-widest opacity-60 font-mono">
          Recent Transactions
        </h3>
        <span className="flex items-center gap-2 text-xs text-emerald-400">
          <span className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse"></span>
          Live
        </span>
      </div>

      {/* Transaction list */}
      <div className="space-y-1">
        {visibleTransactions.map((tx, index) => (
          <div
            key={tx.id || index}
            className="flex items-center gap-4 py-3 border-b border-white/5 last:border-0 hover:bg-white/5 rounded-lg px-2 transition-colors duration-200"
          >
            {/* Type badge */}
            <div className={`px-2 py-1 rounded text-xs font-mono font-bold ${getTypeColor(tx.type)}`}>
              {tx.type}
            </div>

            {/* Transaction details */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm font-mono text-white/80">
                  {truncateAddress(tx.wallet)}
                </span>
                <span className="text-xs text-gray-500">
                  {formatRelativeTime(tx.timestamp)}
                </span>
              </div>
              <p className="text-xs text-gray-400 truncate mt-1">
                {tx.detail || 'Transaction'}
              </p>
            </div>

            {/* Amount */}
            <div className="text-right">
              <p className="text-sm font-mono text-white">
                {tx.amountX3S?.toLocaleString() || 0} X3S
              </p>
              <p className="text-xs text-gray-400">
                {formatCurrency(tx.amountUsd)}
              </p>
            </div>
          </div>
        ))}
      </div>

      {/* View more link */}
      {transactions.length > 5 && (
        <div className="mt-4 pt-4 border-t border-white/5 text-center">
          <button className="text-xs text-cyan hover:text-cyan/80 transition-colors font-mono">
            View all {transactions.length} transactions →
          </button>
        </div>
      )}
    </div>
  );
}