import React, { useEffect, useState, useCallback } from 'react';

interface ASTNodeData {
  nodeId: string;
  pnl: number;
  votes?: Record<string, boolean>;
  runtime?: any;
  merged?: boolean;
}

const ASTHeatmap: React.FC = () => {
  const [nodes, setNodes] = useState<ASTNodeData[]>([]);
  const [ws, setWs] = useState<WebSocket | null>(null);

  useEffect(() => {
    const socket = new WebSocket('ws://localhost:8765');
    socket.onopen = () => {
      document.getElementById('connection-status')!.textContent = '🟢 connected';
      document.getElementById('connection-status')!.style.color = '#00ff00';
    };
    socket.onclose = () => {
      document.getElementById('connection-status')!.textContent = '🔴 disconnected';
    };
    socket.onmessage = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'autopilot_update' || msg.type === 'replay_update' || msg.type === 'rollback_update') {
          setNodes((prev) => {
            const filtered = prev.filter(n => n.nodeId !== msg.payload.nodeId);
            return [...filtered, msg.payload];
          });
        }
      } catch {}
    };
    setWs(socket);
    return () => socket.close();
  }, []);

  const getColor = useCallback((pnl: number) => {
    if (pnl >= 2) return '#00cc44';
    if (pnl >= 0) return '#88dd88';
    if (pnl >= -1) return '#ffcc44';
    return '#ff4444';
  }, []);

  return (
    <div style={{ border: '1px solid #555', borderRadius: '6px', padding: '10px', flex: 1, overflowY: 'auto', background: '#1a1a2e' }}>
      <h3 style={{ color: '#fff', margin: '0 0 8px 0' }}>📊 AST Node Heatmap</h3>
      {nodes.length === 0 && <div style={{ color: '#888', fontSize: '13px' }}>Waiting for data...</div>}
      {nodes.map(node => (
        <div key={node.nodeId} style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          background: getColor(node.pnl), color: '#111',
          padding: '4px 8px', margin: '2px 0', borderRadius: '4px',
          fontSize: '13px', fontWeight: 'bold'
        }}>
          <span>{node.nodeId}</span>
          <span style={{ marginLeft: '8px' }}>PnL: {node.pnl.toFixed(2)}</span>
          {node.merged !== undefined && (
            <span style={{ marginLeft: '8px' }}>{node.merged ? '✅ Merged' : '⛔ Blocked'}</span>
          )}
        </div>
      ))}
    </div>
  );
};

export default ASTHeatmap;