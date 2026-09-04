import React, { useState } from 'react';

const RollbackButton: React.FC = () => {
  const [nodeId, setNodeId] = useState('');
  const [status, setStatus] = useState('');

  const handleRollback = () => {
    if (!nodeId.trim()) {
      setStatus('❌ Enter a node ID');
      return;
    }
    try {
      const socket = new WebSocket('ws://localhost:8765');
      socket.onopen = () => {
        socket.send(JSON.stringify({
          type: 'rollback_request',
          payload: { nodeId: nodeId.trim() }
        }));
        setStatus(`🔄 Rollback requested for ${nodeId}`);
        setTimeout(() => socket.close(), 1000);
      };
      socket.onerror = () => setStatus('❌ Connection failed');
    } catch {
      setStatus('❌ Connection failed');
    }
  };

  return (
    <div style={{ border: '1px solid #555', borderRadius: '6px', padding: '8px', background: '#1a1a2e' }}>
      <h4 style={{ color: '#fff', margin: '0 0 6px 0' }}>⏪ Rollback Node</h4>
      <div style={{ display: 'flex', gap: '4px' }}>
        <input
          type="text"
          value={nodeId}
          onChange={e => setNodeId(e.target.value)}
          placeholder="Node ID"
          style={{ flex: 1, padding: '4px', borderRadius: '3px', border: '1px solid #555', background: '#16213e', color: '#fff', fontSize: '12px' }}
        />
        <button
          onClick={handleRollback}
          style={{ padding: '4px 12px', background: '#cc4444', color: '#fff', border: 'none', borderRadius: '3px', cursor: 'pointer', fontWeight: 'bold', fontSize: '12px' }}
        >
          Rollback
        </button>
      </div>
      {status && <div style={{ fontSize: '11px', color: '#aaa', marginTop: '4px' }}>{status}</div>}
    </div>
  );
};

export default RollbackButton;