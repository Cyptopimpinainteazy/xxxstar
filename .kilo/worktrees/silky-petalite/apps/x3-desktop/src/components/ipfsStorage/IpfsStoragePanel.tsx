import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { withRetry, classifyError, logError, createErrorContext, isOnline } from '../../utils/errorHandler';

// Guarded Tauri invoke helper
async function tauriInvoke<T>(cmd: string, args?: any): Promise<T> {
  if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
    throw new Error('Tauri runtime not available');
  }
  const mod = await import('@tauri-apps/api/core');
  return mod.invoke<T>(cmd, args);
}

interface PinnedContent {
  cid: string;
  name: string;
  size: number;
  pinned_at: string;
  replicas: number;
  earning_potential: number;
}

interface StorageDeal {
  id: string;
  client: string;
  size: number;
  price_per_epoch: number;
  duration_epochs: number;
  status: string;
  earned: number;
}

interface IpfsStorageData {
  node_id: string;
  pinned_objects: PinnedContent[];
  storage_used: number;
  storage_capacity: number;
  storage_market: StorageDeal[];
  total_pins: number;
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

const StorageUsageBar: React.FC<{ used: number; capacity: number }> = ({
  used,
  capacity,
}) => {
  const percentage = (used / capacity) * 100;
  const color =
    percentage < 50 ? 'bg-green-500' : percentage < 75 ? 'bg-yellow-500' : 'bg-red-500';

  return (
    <div>
      <div className="flex justify-between text-xs mb-2">
        <span className="text-gray-300">Storage Capacity</span>
        <span className="text-white font-semibold">{percentage.toFixed(1)}%</span>
      </div>
      <div className="w-full bg-gray-700 rounded-full h-3 overflow-hidden">
        <div
          className={`h-full ${color} transition-all duration-300`}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />
      </div>
      <div className="text-xs text-gray-400 mt-1">
        {formatBytes(used)} / {formatBytes(capacity)}
      </div>
    </div>
  );
};

export const IpfsStoragePanel: React.FC = () => {
  const [data, setData] = useState<IpfsStorageData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);
  const maxRetries = 3;

  const fetchDataWithRetry = async () => {
    try {
      setLoading(true);
      setError(null);

      const result = await withRetry(
        () => tauriInvoke<IpfsStorageData>('launch_ipfs_storage'),
        {
          maxAttempts: 3,
          delayMs: 1000,
          timeout: 10000,
        }
      );

      setData(result);
      setRetryCount(0);
    } catch (err) {
      const appError = classifyError(err);
      const context = createErrorContext('IpfsStoragePanel', {
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
    fetchDataWithRetry();

    // Listen for telemetry updates
    const unlistenPromise = listen<any>('telemetry_update', (event) => {
      if (event.payload.ipfs) {
        setData(event.payload.ipfs);
        setError(null);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  const handleRetry = () => {
    if (retryCount < maxRetries) {
      fetchDataWithRetry();
    }
  };

  if (error) {
    return (
      <div className="p-4 bg-gray-900 rounded-lg border border-red-500/50">
        <div className="border-b border-gray-700 pb-3 mb-3">
          <h3 className="text-lg font-semibold text-white">IPFS Storage Marketplace</h3>
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
              <p className="text-red-400 font-medium">Failed to load IPFS data</p>
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
              Max retries exceeded. Please verify your IPFS backend and try
              restarting the application.
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
          <h3 className="text-lg font-semibold text-white">IPFS Storage Marketplace</h3>
        </div>
        <div className="text-gray-400 text-sm">Loading IPFS storage data...</div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="p-4 bg-gray-900 rounded-lg border border-gray-700">
        <div className="border-b border-gray-700 pb-3 mb-3">
          <h3 className="text-lg font-semibold text-white">IPFS Storage Marketplace</h3>
        </div>
        <div className="text-red-400 text-sm">No IPFS storage data available</div>
      </div>
    );
  }

  const totalEarnings = data.storage_market.reduce((sum, deal) => sum + deal.earned, 0);
  const activeDeals = data.storage_market.filter((deal) => deal.status === 'Active').length;

  return (
    <div className="p-4 bg-gray-900 rounded-lg border border-gray-700 space-y-4">
      <div className="border-b border-gray-700 pb-3">
        <h3 className="text-lg font-semibold text-white mb-1">IPFS Storage Marketplace</h3>
        <div className="text-xs text-gray-400">
          Node: {data.node_id.substring(0, 12)}...
        </div>
      </div>

      {/* Storage Capacity */}
      <div>
        <StorageUsageBar used={data.storage_used} capacity={data.storage_capacity} />
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-3 gap-2 py-2 border-y border-gray-700">
        <div className="text-center">
          <div className="text-lg font-bold text-green-400">{data.pinned_objects.length}</div>
          <div className="text-xs text-gray-400">Pinned Objects</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-blue-400">{activeDeals}</div>
          <div className="text-xs text-gray-400">Active Deals</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-yellow-400">${totalEarnings.toFixed(2)}</div>
          <div className="text-xs text-gray-400">Total Earned</div>
        </div>
      </div>

      {/* Storage Deals */}
      {data.storage_market.length > 0 && (
        <div>
          <h4 className="text-sm font-semibold text-gray-300 mb-2">Storage Deals</h4>
          <div className="space-y-2">
            {data.storage_market.map((deal) => (
              <div
                key={deal.id}
                className="p-2 bg-gray-800 rounded border border-gray-700 text-xs"
              >
                <div className="flex justify-between mb-1">
                  <span className="text-white font-semibold">{deal.client}</span>
                  <span
                    className={`${
                      deal.status === 'Active' ? 'text-green-400' : 'text-gray-400'
                    }`}
                  >
                    {deal.status}
                  </span>
                </div>
                <div className="flex justify-between text-gray-400">
                  <span>{formatBytes(deal.size)}</span>
                  <span>${deal.earned.toFixed(2)} earned</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Pinned Content */}
      {data.pinned_objects.length > 0 && (
        <div>
          <h4 className="text-sm font-semibold text-gray-300 mb-2">Pinned Content</h4>
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {data.pinned_objects.map((pin) => (
              <div
                key={pin.cid}
                className="p-2 bg-gray-800 rounded border border-gray-700 text-xs"
              >
                <div className="flex justify-between items-start mb-1">
                  <div>
                    <div className="text-white font-semibold truncate">{pin.name}</div>
                    <div className="text-gray-400 truncate font-mono text-[10px]">
                      {pin.cid.substring(0, 16)}...
                    </div>
                  </div>
                  <div className="text-right whitespace-nowrap ml-2">
                    <div className="text-yellow-400 font-semibold">${pin.earning_potential.toFixed(2)}</div>
                    <div className="text-gray-400">{pin.replicas} replicas</div>
                  </div>
                </div>
                <div className="text-gray-500">{formatBytes(pin.size)}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Last Updated */}
      <div className="text-xs text-gray-500 pt-2 border-t border-gray-700">
        Updated: {new Date(data.updated_at).toLocaleTimeString()}
      </div>
    </div>
  );
};
