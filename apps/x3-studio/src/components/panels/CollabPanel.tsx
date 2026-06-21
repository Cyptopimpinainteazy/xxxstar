import { useState } from 'react';
import { useCollabStore } from '../../store';

export default function CollabPanel() {
  const sessions = useCollabStore(s => s.sessions);
  const activeSessionId = useCollabStore(s => s.activeSessionId);
  const addSession = useCollabStore(s => s.addSession);
  const removeSession = useCollabStore(s => s.removeSession);
  const updateSession = useCollabStore(s => s.updateSession);
  const setActiveSession = useCollabStore(s => s.setActiveSession);

  const [room, setRoom] = useState('');
  const [host, setHost] = useState('localhost');
  const [wsUrl, setWsUrl] = useState('');
  const [creating, setCreating] = useState(false);
  const [joining, setJoining] = useState(false);
  const [error, setError] = useState('');

  const handleCreate = async () => {
    if (!room.trim() || !host.trim()) return;
    setCreating(true);
    setError('');
    try {
      const result = await window.x3studio.collab.createSession(room.trim(), host.trim());
      addSession({
        id: result.sessionId,
        room: room.trim(),
        host: host.trim(),
        peers: 0,
        connected: true,
        lastSync: new Date().toISOString(),
      });
      setRoom('');
    } catch (e: any) {
      setError(e.message || 'Failed to create session');
    } finally {
      setCreating(false);
    }
  };

  const handleJoin = async () => {
    if (!wsUrl.trim()) return;
    setJoining(true);
    setError('');
    try {
      const result = await window.x3studio.collab.joinSession(wsUrl.trim());
      addSession({
        id: `session_${Date.now()}`,
        room: wsUrl.trim(),
        host: 'remote',
        peers: 0,
        connected: true,
        lastSync: new Date().toISOString(),
      });
      setWsUrl('');
    } catch (e: any) {
      setError(e.message || 'Failed to join session');
    } finally {
      setJoining(false);
    }
  };

  const handleDisconnect = (id: string) => {
    removeSession(id);
    if (activeSessionId === id) setActiveSession(null);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Collab Sessions</span>
      </div>
      <div className="panel-body">
        {error && (
          <div style={{ color: 'var(--red)', fontSize: 'var(--font-size-sm)', marginBottom: 8, padding: '4px 8px', background: 'var(--surface2)', borderRadius: 4 }}>
            {error}
          </div>
        )}

        <div className="section-title">Create Session</div>
        <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
          <input className="input-field" value={room} onChange={e => setRoom(e.target.value)}
            placeholder="Room name" style={{ flex: 1 }} />
          <input className="input-field" value={host} onChange={e => setHost(e.target.value)}
            placeholder="Host" style={{ flex: 1 }} />
          <button className="btn btn-primary" onClick={handleCreate} disabled={creating || !room.trim() || !host.trim()}>
            {creating ? 'Creating...' : 'Create'}
          </button>
        </div>

        <div className="section-title">Join Session</div>
        <div style={{ display: 'flex', gap: 4, marginBottom: 12 }}>
          <input className="input-field" value={wsUrl} onChange={e => setWsUrl(e.target.value)}
            placeholder="ws://host:port/room" style={{ flex: 1 }} />
          <button className="btn btn-primary" onClick={handleJoin} disabled={joining || !wsUrl.trim()}>
            {joining ? 'Joining...' : 'Join'}
          </button>
        </div>

        <div className="section-title">Active Sessions ({sessions.length})</div>
        {sessions.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 8 }}>
            No active sessions. Create or join a session above.
          </div>
        )}
        {sessions.map(s => (
          <div key={s.id} className="tree-node" style={{
            border: activeSessionId === s.id ? '1px solid var(--accent)' : '1px solid var(--border)',
            borderRadius: 4, padding: 8, marginBottom: 6, cursor: 'pointer',
          }} onClick={() => setActiveSession(s.id)}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
              <span style={{ fontWeight: 600 }}>{s.room}</span>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{
                  width: 8, height: 8, borderRadius: '50%', display: 'inline-block',
                  background: s.connected ? 'var(--green)' : 'var(--red)',
                }} />
                <span style={{ fontSize: 'var(--font-size-sm)', color: s.connected ? 'var(--green)' : 'var(--red)' }}>
                  {s.connected ? 'Connected' : 'Disconnected'}
                </span>
              </div>
            </div>
            <div style={{ display: 'flex', gap: 12, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
              <span>Host: {s.host}</span>
              <span>Peers: {s.peers}</span>
              <span>Last sync: {new Date(s.lastSync).toLocaleTimeString()}</span>
            </div>
            <div style={{ marginTop: 4, textAlign: 'right' }}>
              <button className="btn" style={{ fontSize: 'var(--font-size-sm)', padding: '2px 8px' }}
                onClick={e => { e.stopPropagation(); handleDisconnect(s.id); }}>
                Disconnect
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
