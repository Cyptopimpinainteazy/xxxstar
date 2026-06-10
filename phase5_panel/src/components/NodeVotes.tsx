import React, { useEffect, useState } from 'react';

interface VoteData {
  nodeId: string;
  votes: Record<string, boolean>;
}

const NodeVotes: React.FC = () => {
  const [voteData, setVoteData] = useState<Record<string, Record<string, boolean>>>({});

  useEffect(() => {
    const socket = new WebSocket('ws://localhost:8765');
    socket.onmessage = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.payload?.votes) {
          setVoteData(prev => ({ ...prev, [msg.payload.nodeId]: msg.payload.votes }));
        }
      } catch {}
    };
    return () => socket.close();
  }, []);

  return (
    <div style={{ border: '1px solid #555', borderRadius: '6px', padding: '8px', flex: 1, overflowY: 'auto', background: '#16213e' }}>
      <h4 style={{ color: '#fff', margin: '0 0 6px 0' }}>🗳️ Multi-Agent Votes</h4>
      {Object.entries(voteData).length === 0 && <div style={{ color: '#666', fontSize: '12px' }}>No votes yet</div>}
      {Object.entries(voteData).map(([nodeId, votes]) => (
        <div key={nodeId} style={{ fontSize: '12px', color: '#ccc', padding: '3px 0', borderBottom: '1px solid #333' }}>
          <strong style={{ color: '#fff' }}>{nodeId}</strong>:{' '}
          {Object.entries(votes).map(([agent, v]) => (
            <span key={agent} style={{ marginRight: '6px' }}>
              {agent}: {v ? '✅' : '❌'}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
};

export default NodeVotes;