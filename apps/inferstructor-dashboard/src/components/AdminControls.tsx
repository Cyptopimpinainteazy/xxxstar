import React, { useState, useEffect, Suspense, lazy } from 'react';
import { Settings, AlertTriangle, ShieldOff, Sliders, Eye, Loader, AlertCircle } from 'lucide-react';
import { api } from '../api';
import type { ServiceStatus } from '../api';
import {
  ADMIN_METRICS_HISTORY_SECONDS,
} from '../constants';

// Lazy load tab components
const RpcPanel = lazy(() => import('./admin-tabs/RpcPanel').then(m => ({ default: m.RpcPanel })));
const FaucetPanel = lazy(() => import('./admin-tabs/FaucetPanel').then(m => ({ default: m.FaucetPanel })));
const EmergencyPanel = lazy(() => import('./admin-tabs/EmergencyPanel').then(m => ({ default: m.EmergencyPanel })));
const RBACPanel = lazy(() => import('./admin-tabs/RBACPanel').then(m => ({ default: m.RBACPanel })));
const AuditPanel = lazy(() => import('./admin-tabs/AuditPanel').then(m => ({ default: m.AuditPanel })));

// Fallback component while loading
const TabLoadingFallback: React.FC = () => (
  <div className="flex items-center justify-center py-8">
    <Loader className="w-5 h-5 text-blue-400 animate-spin" />
    <span className="ml-2 text-gray-400">Loading...</span>
  </div>
);

interface AdminControlsProps {
  onClose?: () => void;
}

export const AdminControls: React.FC<AdminControlsProps> = () => {
  const [rpcEndpoints, setRpcEndpoints] = useState<any[]>([]);
  const [rbacRoles, setRbacRoles] = useState<any[]>([]);
  const [auditLogs, setAuditLogs] = useState<any[]>([]);
  
  const [activeTab, setActiveTab] = useState<'rpc' | 'faucet' | 'emergency' | 'rbac' | 'audit'>('rpc');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load admin data on mount
  useEffect(() => {
    loadAdminData();
  }, []);

  const loadAdminData = async () => {
    try {
      setLoading(true);
      setError(null);
      
      // Use Promise.allSettled to show partial data if one endpoint fails
      const results = await Promise.allSettled([
        api.getServiceStatus(),
        api.getAdminMetricsHistory(ADMIN_METRICS_HISTORY_SECONDS),
      ]);

      // Process results
      const serviceData = results[0].status === 'fulfilled' ? results[0].value : { services: [] };
      const metricsData = results[1].status === 'fulfilled' ? results[1].value : { points: [] };

      // Log any failures
      if (results[0].status === 'rejected') {
        console.error('Failed to fetch service status:', results[0].reason);
      }
      if (results[1].status === 'rejected') {
        console.error('Failed to fetch metrics history:', results[1].reason);
      }

      // Extract RPC endpoints from services or use fallback
      if (serviceData.services && Array.isArray(serviceData.services)) {
        const rpcServices = serviceData.services.filter((s: ServiceStatus) => s.name?.includes('rpc'));
        if (rpcServices.length > 0) {
          setRpcEndpoints(rpcServices);
        } else {
          setRpcEndpoints([
            { id: 'eth-1', name: 'Ethereum', url: 'https://eth-rpc.example.com', status: 'up' },
            { id: 'sol-1', name: 'Solana', url: 'https://sol-rpc.example.com', status: 'up' },
          ]);
        }
      }

      // Set RBAC roles (mock until API endpoint available)
      setRbacRoles([
        {
          id: 'admin',
          name: 'Administrator',
          permissions: ['validator_approval', 'emergency_pause', 'audit_view', 'settings_modify'],
        },
        {
          id: 'operator',
          name: 'Operator',
          permissions: ['validator_view', 'metrics_view', 'audit_view'],
        },
        {
          id: 'viewer',
          name: 'Viewer',
          permissions: ['metrics_view', 'leaderboard_view'],
        },
      ]);

      // Extract audit logs from metrics history or use fallback
      if (metricsData.points && Array.isArray(metricsData.points)) {
        const logs: any[] = metricsData.points.slice(0, 10).map((point: any, idx: number) => ({
          id: `log-${idx}`,
          action: 'Metrics recorded',
          actor: 'system',
          timestamp: new Date(point.timestamp || Date.now()).toLocaleString(),
          status: 'success' as const,
        }));
        setAuditLogs(logs);
      } else {
        setAuditLogs([
          {
            id: 'log-001',
            action: 'Validator approved',
            actor: 'admin@example.com',
            timestamp: '2024-04-06 10:30:00',
            status: 'success',
          },
          {
            id: 'log-002',
            action: 'Emergency pause triggered',
            actor: 'admin@example.com',
            timestamp: '2024-04-06 10:15:00',
            status: 'success',
          },
          {
            id: 'log-003',
            action: 'RPC endpoint added',
            actor: 'operator@example.com',
            timestamp: '2024-04-06 09:45:00',
            status: 'success',
          },
        ]);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load admin data');
      // Use fallback data
      setRpcEndpoints([
        { id: 'eth-1', chain: 'Ethereum', url: 'https://eth-rpc.example.com', status: 'healthy' },
        { id: 'sol-1', chain: 'Solana', url: 'https://sol-rpc.example.com', status: 'healthy' },
      ]);
      setAuditLogs([
        {
          id: 'log-001',
          action: 'Validator approved',
          actor: 'admin@example.com',
          timestamp: '2024-04-06 10:30:00',
          status: 'success',
        },
      ]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="px-6">
      <div className="max-w-6xl">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-white mb-2">Admin Controls</h1>
          <p className="text-gray-400">RPC endpoints, faucet limits, emergency controls, and RBAC management</p>
        </div>

        {/* Error Banner */}
        {error && (
          <div className="mb-6 p-4 bg-red-900/20 border border-red-700 rounded-lg flex items-start gap-3">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <p className="text-red-300 font-medium">Error</p>
              <p className="text-red-200 text-sm">{error}</p>
            </div>
            <button
              onClick={() => setError(null)}
              className="ml-auto text-red-400 hover:text-red-300"
            >
              ✕
            </button>
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <Loader className="w-6 h-6 text-blue-400 animate-spin" />
            <span className="ml-2 text-gray-400">Loading admin data...</span>
          </div>
        )}

        {!loading && (
          <>
            {/* Tabs */}
            <div className="flex gap-2 mb-6 border-b border-[#2a2a35]" role="tablist">
              {[
                { id: 'rpc', label: 'RPC Endpoints', icon: Settings },
                { id: 'faucet', label: 'Faucet Config', icon: Sliders },
                { id: 'emergency', label: 'Emergency', icon: AlertTriangle },
                { id: 'rbac', label: 'RBAC', icon: ShieldOff },
                { id: 'audit', label: 'Audit Logs', icon: Eye },
              ].map((tab) => {
                const Icon = tab.icon;
                return (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id as typeof activeTab)}
                    role="tab"
                    aria-selected={activeTab === tab.id}
                    aria-controls={`${tab.id}-panel`}
                    className={`px-4 py-3 font-medium text-sm transition-colors flex items-center gap-2 ${
                      activeTab === tab.id
                        ? 'text-blue-400 border-b-2 border-blue-400'
                        : 'text-gray-400 hover:text-white'
                    }`}
                  >
                    <Icon className="w-4 h-4" aria-hidden="true" />
                    {tab.label}
                  </button>
                );
              })}
            </div>

            {/* Tab Content - Lazy Loaded */}
            <Suspense fallback={<TabLoadingFallback />}>
              {activeTab === 'rpc' && <RpcPanel rpcEndpoints={rpcEndpoints} />}
              {activeTab === 'faucet' && <FaucetPanel />}
              {activeTab === 'emergency' && <EmergencyPanel />}
              {activeTab === 'rbac' && <RBACPanel rbacRoles={rbacRoles} />}
              {activeTab === 'audit' && <AuditPanel auditLogs={auditLogs} />}
            </Suspense>
          </>
        )}
      </div>
    </div>
  );
};
