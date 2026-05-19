import useLiveStats from '../hooks/useLiveStats';
import StatCard from './StatCard';
import LiveTransactionFeed from './LiveTransactionFeed';
import Eyebrow from './Eyebrow';
import Button from './Button';

/**
 * LiveStatsDashboard component - Main dashboard for displaying real-time X3 Chain statistics
 * Displays validator count, TPS, block height, TVL, and recent transactions
 * Auto-refreshes every 5 seconds
 */
export default function LiveStatsDashboard() {
  const {
    validators,
    tps,
    blockHeight,
    tvlUsd,
    recentTransactions,
    loading,
    error,
    refresh,
  } = useLiveStats(5000); // Refresh every 5 seconds

  return (
    <section className="live-stats-dashboard section py-20 px-6" id="stats">
      {/* Section header */}
      <Eyebrow text="Live Network" />
      <h2 className="section-title text-4xl lg:text-6xl font-bold mb-4">
        X3 Chain Statistics
      </h2>
      <p className="section-sub max-w-2xl mx-auto text-gray-400 mb-12">
        Real-time network health, validator activity, and transaction flow.
        <span className="text-cyan ml-2">Auto-refreshes every 5 seconds.</span>
      </p>

      {/* Error banner */}
      {error && (
        <div className="max-w-4xl mx-auto mb-8 p-4 bg-red-500/10 border border-red-500/30 rounded-xl flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-red-400">⚠️</span>
            <span className="text-red-300 text-sm">{error}</span>
          </div>
          <Button outline onClick={refresh}>
            Retry
          </Button>
        </div>
      )}

      {/* Stats grid */}
      <div className="max-w-6xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-12">
        <StatCard
          label="Active Validators"
          value={validators}
          loading={loading}
          error={error && !validators ? 'Failed to load' : null}
        />
        <StatCard
          label="Avg TPS"
          value={tps}
          suffix=" tx/s"
          loading={loading}
          error={error && !tps ? 'Failed to load' : null}
        />
        <StatCard
          label="Block Height"
          value={blockHeight}
          prefix="#"
          loading={loading}
          error={error && !blockHeight ? 'Failed to load' : null}
        />
        <StatCard
          label="Total Value Locked"
          value={tvlUsd}
          prefix="$"
          loading={loading}
          error={error && !tvlUsd ? 'Failed to load' : null}
        />
      </div>

      {/* Transaction feed and refresh button */}
      <div className="max-w-6xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-mono text-white/80">Network Activity</h3>
          <Button outline onClick={refresh} disabled={loading}>
            {loading ? 'Refreshing...' : '↻ Refresh Now'}
          </Button>
        </div>
        
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Transaction feed */}
          <LiveTransactionFeed
            transactions={recentTransactions}
            loading={loading}
          />

          {/* Network status card */}
          <div className="bg-surface-dark rounded-2xl p-6 border border-white/5">
            <h3 className="text-sm uppercase tracking-widest opacity-60 mb-6 font-mono">
              Network Status
            </h3>
            
            <div className="space-y-4">
              {/* Status indicator */}
              <div className="flex items-center justify-between p-3 bg-white/5 rounded-lg">
                <span className="text-sm text-gray-400">Network Status</span>
                <span className="flex items-center gap-2 text-emerald-400">
                  <span className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse"></span>
                  Operational
                </span>
              </div>

              {/* Finality */}
              <div className="flex items-center justify-between p-3 bg-white/5 rounded-lg">
                <span className="text-sm text-gray-400">Avg Finality</span>
                <span className="font-mono text-white">0.4s</span>
              </div>

              {/* Uptime */}
              <div className="flex items-center justify-between p-3 bg-white/5 rounded-lg">
                <span className="text-sm text-gray-400">Network Uptime</span>
                <span className="font-mono text-emerald-400">99.8%</span>
              </div>

              {/* Fee */}
              <div className="flex items-center justify-between p-3 bg-white/5 rounded-lg">
                <span className="text-sm text-gray-400">Avg Tx Fee</span>
                <span className="font-mono text-cyan">$0.0001</span>
              </div>

              {/* Last updated */}
              <div className="pt-4 border-t border-white/5">
                <p className="text-xs text-gray-500 text-center">
                  Last updated: {new Date().toLocaleTimeString()}
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}