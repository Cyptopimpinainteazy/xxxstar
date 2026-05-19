import AnimatedCounter from './AnimatedCounter';

/**
 * StatCard component for displaying a single statistic with animated counter
 * @param {string} label - Label for the statistic (e.g., "Validators")
 * @param {number} value - Numeric value to display
 * @param {string} prefix - Prefix for the value (e.g., "$")
 * @param {string} suffix - Suffix for the value (e.g., "%")
 * @param {number} trend - Percentage change (positive or negative)
 * @param {boolean} loading - Loading state
 * @param {string} error - Error message
 */
export default function StatCard({ label, value, prefix = '', suffix = '', trend, loading, error }) {
  // Loading skeleton state
  if (loading) {
    return (
      <div className="stat-card bg-surface-dark rounded-2xl p-6 animate-pulse">
        <div className="h-3 bg-gray-700 rounded w-24 mb-4"></div>
        <div className="h-10 bg-gray-700 rounded w-32 mb-2"></div>
        <div className="h-3 bg-gray-700 rounded w-16"></div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="stat-card bg-surface-dark rounded-2xl p-6 border border-red-500/30">
        <p className="text-xs uppercase tracking-widest opacity-60 mb-2">{label}</p>
        <p className="text-red-400 text-sm">{error}</p>
      </div>
    );
  }

  return (
    <div className="stat-card bg-surface-dark rounded-2xl p-6 border border-white/5 hover:border-cyan/20 transition-colors duration-300">
      {/* Label */}
      <p className="text-xs uppercase tracking-widest opacity-60 mb-3 font-mono">
        {label}
      </p>
      
      {/* Value with animated counter */}
      <p className="display gradient-text text-4xl lg:text-5xl font-bold mb-2">
        <AnimatedCounter value={value} prefix={prefix} suffix={suffix} duration={1500} />
      </p>
      
      {/* Trend indicator */}
      {trend !== undefined && trend !== null && (
        <div className={`flex items-center gap-1 text-sm mt-2 ${
          trend > 0 ? 'text-emerald-400' : trend < 0 ? 'text-red-400' : 'text-gray-400'
        }`}>
          <span className="text-lg">{trend > 0 ? '↑' : trend < 0 ? '↓' : '→'}</span>
          <span className="font-mono">{Math.abs(trend).toFixed(1)}%</span>
          <span className="opacity-60 text-xs ml-1">24h</span>
        </div>
      )}
    </div>
  );
}