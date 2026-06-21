import { Activity, Wifi, Cpu, Database, Loader2, Monitor, Server } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api } from '../api/client';

export function NetworkPanel() {
  const { data: net, loading, error, refresh } = useApi(() => api.networkStatus(), []);

  return (
    <div style={{ padding: 16, color: '#d4d4d4', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Activity size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Network Status</h2>
      </div>

      {loading && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><Loader2 size={14} className="spin" /> Loading...</div>}
      {error && <div style={{ color: '#f48771' }}>{error}</div>}

      {net && (
        <>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 16 }}>
            <StatCard icon={Wifi} label="Peers" value={String(net.peers)} />
            <StatCard icon={Cpu} label="Chain" value={net.chain} />
            <StatCard icon={Database} label="Token" value={net.tokenSymbol} />
            <StatCard icon={Activity} label="Syncing" value={net.syncing ? 'Yes' : 'No'} valueColor={net.syncing ? '#dcdcaa' : '#4ec9b0'} />
          </div>

          <div style={{ padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333', marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: '#888', marginBottom: 8 }}>RPC Endpoints</div>
            <div style={{ fontSize: 13, fontFamily: 'monospace' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 0' }}>
                <span><Server size={12} style={{ marginRight: 6 }} />HTTP RPC</span>
                <span style={{ color: '#4ec9b0' }}>{net.rpcUrl} ●</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 0' }}>
                <span><Monitor size={12} style={{ marginRight: 6 }} />IDE API</span>
                <span style={{ color: '#4ec9b0' }}>127.0.0.1:8765 ●</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 0' }}>
                <span><Database size={12} style={{ marginRight: 6 }} />Finalized Head</span>
                <span style={{ color: '#569cd6', fontSize: 12 }}>{net.finalizedHead.slice(0, 18)}...</span>
              </div>
            </div>
          </div>

          <div style={{ padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333' }}>
            <div style={{ fontSize: 12, color: '#888', marginBottom: 8 }}>Chain Info</div>
            <div style={{ fontFamily: 'monospace', fontSize: 13, lineHeight: 1.8 }}>
              <div><span style={{ color: '#888' }}>SS58 Format:</span> {net.ss58Format}</div>
              <div><span style={{ color: '#888' }}>Token Symbol:</span> {net.tokenSymbol}</div>
              <div><span style={{ color: '#888' }}>Chain Type:</span> {net.chain}</div>
            </div>
          </div>
        </>
      )}

      <button onClick={refresh} style={{
        marginTop: 12, padding: '6px 16px', border: '1px solid #333',
        borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
      }}>Refresh</button>
    </div>
  );
}

function StatCard({ icon: Icon, label, value, valueColor }: {
  icon: typeof Activity; label: string; value: string; valueColor?: string;
}) {
  return (
    <div style={{ padding: 12, background: '#252526', borderRadius: 8, border: '1px solid #333' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 8 }}>
        <Icon size={14} color="#888" />
        <span style={{ fontSize: 12, color: '#888' }}>{label}</span>
      </div>
      <div style={{ fontSize: 20, fontWeight: 700, color: valueColor || '#d4d4d4', fontFamily: 'monospace' }}>{value}</div>
    </div>
  );
}
