import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { withRetry, classifyError, logError, createErrorContext, isOnline } from '../../utils/errorHandler';

// Guarded Tauri invoke (avoids accessing __TAURI_INTERNALS__ in browser dev)
async function tauriInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
}

interface CpuMetrics {
  usage_percent: number;
  cores: number;
  frequency: number;
}

interface MemoryMetrics {
  used: number;
  total: number;
  usage_percent: number;
}

interface DiskMetrics {
  name: string;
  used: number;
  total: number;
  usage_percent: number;
}

interface SystemMetricsData {
  cpu: CpuMetrics;
  memory: MemoryMetrics;
  disk: DiskMetrics[];
  updated_at: string;
}

const formatBytes = (bytes: number): string => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;
  
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  
  return `${size.toFixed(1)} ${units[unitIndex]}`;
};

const MetricBar: React.FC<{ label: string; value: number; unit?: string }> = ({
  label,
  value,
  unit = '%',
}) => {
  const getColor = (percent: number) => {
    if (percent < 50) return 'bg-green-500';
    if (percent < 75) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <div className="mb-3">
      <div className="flex justify-between text-xs mb-1">
        <span className="text-gray-300">{label}</span>
        <span className="text-white font-semibold">
          {value.toFixed(1)}{unit}
        </span>
      </div>
      <div className="w-full bg-gray-700 rounded-full h-2 overflow-hidden">
        <div
          className={`h-full ${getColor(value)} transition-all duration-300`}
          style={{ width: `${Math.min(value, 100)}%` }}
        />
      </div>
    </div>
  );
};

export const SystemMetricsPanel: React.FC = () => {
  const [metrics, setMetrics] = useState<SystemMetricsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);
  const maxRetries = 3;

  const fetchMetricsWithRetry = async () => {
    try {
      setLoading(true);
      setError(null);

      const data = await withRetry(
        () => tauriInvoke<SystemMetricsData>('launch_system_metrics'),
        {
          maxAttempts: 3,
          delayMs: 1000,
          timeout: 10000,
        }
      );

      setMetrics(data);
      setRetryCount(0);
    } catch (err) {
      const appError = classifyError(err);
      const context = createErrorContext('SystemMetricsPanel', {
        retryCount: retryCount + 1,
      });
      appError.context = context;

      logError(appError);
      setError(appError.message);
      setRetryCount((prev) => prev + 1);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchMetricsWithRetry();

    // Listen for telemetry updates
    const unlistenPromise = listen<any>('telemetry_update', (event) => {
      if (event.payload.system) {
        setMetrics(event.payload.system);
        setError(null);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  const handleRetry = () => {
    if (retryCount < maxRetries) {
      fetchMetricsWithRetry();
    }
  };

  if (error) {
    return (
      <div className="p-4 bg-gray-900 rounded-lg border border-red-500/50">
        <div className="border-b border-gray-700 pb-3 mb-3">
          <h3 className="text-lg font-semibold text-white">System Metrics</h3>
        </div>
        <div className="space-y-3">
          <div className="flex items-start gap-2">
            <svg
              className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                clipRule="evenodd"
              />
            </svg>
            <div className="flex-1">
              <p className="text-red-400 font-medium">Failed to load metrics</p>
              <p className="text-red-300 text-xs mt-1">{error}</p>
            </div>
          </div>

          {!isOnline() && (
            <div className="bg-yellow-500/10 border border-yellow-500/30 rounded p-2">
              <p className="text-yellow-500 text-xs">
                💡 You appear to be offline. Please check your internet
                connection.
              </p>
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={handleRetry}
              disabled={retryCount >= maxRetries || loading}
              className="flex-1 px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm font-medium rounded transition-colors"
            >
              {loading ? 'Retrying...' : `Retry (${retryCount}/${maxRetries})`}
            </button>
          </div>

          {retryCount >= maxRetries && (
            <p className="text-xs text-gray-400">
              Max retries exceeded. Please verify your system and try restarting
              the application.
            </p>
          )}
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="p-4 bg-gray-900 rounded-lg border border-gray-700">
        <div className="border-b border-gray-700 pb-3 mb-3">
          <h3 className="text-lg font-semibold text-white">System Metrics</h3>
        </div>
        <div className="text-gray-400 text-sm">Loading system metrics...</div>
      </div>
    );
  }

  if (!metrics) {
    return (
      <div className="p-4 bg-gray-900 rounded-lg border border-gray-700">
        <div className="border-b border-gray-700 pb-3 mb-3">
          <h3 className="text-lg font-semibold text-white">System Metrics</h3>
        </div>
        <div className="text-red-400 text-sm">No system metrics available</div>
      </div>
    );
  }

  return (
    <div className="p-4 bg-gray-900 rounded-lg border border-gray-700 space-y-4">
      <div className="border-b border-gray-700 pb-3">
        <h3 className="text-lg font-semibold text-white mb-3">System Metrics</h3>
      </div>

      {/* CPU Section */}
      <div>
        <h4 className="text-sm font-semibold text-gray-300 mb-2">CPU</h4>
        <MetricBar label="CPU Usage" value={metrics.cpu.usage_percent} />
        <div className="text-xs text-gray-400 mt-1">
          {metrics.cpu.cores} cores @ {(metrics.cpu.frequency / 1000).toFixed(1)} GHz
        </div>
      </div>

      {/* Memory Section */}
      <div>
        <h4 className="text-sm font-semibold text-gray-300 mb-2">Memory</h4>
        <MetricBar label="Memory Usage" value={metrics.memory.usage_percent} />
        <div className="text-xs text-gray-400 mt-1">
          {formatBytes(metrics.memory.used)} / {formatBytes(metrics.memory.total)}
        </div>
      </div>

      {/* Disk Section */}
      {metrics.disk.length > 0 && (
        <div>
          <h4 className="text-sm font-semibold text-gray-300 mb-2">Storage</h4>
          {metrics.disk.map((disk) => (
            <div key={disk.name} className="mb-2">
              <MetricBar
                label={disk.name || 'Storage'}
                value={disk.usage_percent}
              />
              <div className="text-xs text-gray-400">
                {formatBytes(disk.used)} / {formatBytes(disk.total)}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Last Updated */}
      <div className="text-xs text-gray-500 pt-2 border-t border-gray-700">
        Updated: {new Date(metrics.updated_at).toLocaleTimeString()}
      </div>
    </div>
  );
};
