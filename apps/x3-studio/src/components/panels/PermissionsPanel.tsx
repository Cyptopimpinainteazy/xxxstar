import { useState, useEffect } from 'react';
import { usePermissionStore } from '../../store';

export default function PermissionsPanel() {
  const permissions = usePermissionStore(s => s.permissions);
  const setPermissions = usePermissionStore(s => s.setPermissions);
  const updatePermission = usePermissionStore(s => s.updatePermission);
  const [status, setStatus] = useState('');

  useEffect(() => { loadPermissions(); }, []);

  const loadPermissions = async () => {
    try {
      const perms = await window.x3studio.permissions.getPermissions();
      setPermissions(perms);
    } catch {}
  };

  const togglePermission = async (channel: string, allowed: boolean) => {
    try {
      await window.x3studio.permissions.setPermission(channel, allowed);
      updatePermission(channel, allowed);
      setStatus(`${channel}: ${allowed ? 'Allowed' : 'Denied'}`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  const requestTest = async (channel: string) => {
    setStatus(`Requesting permission for ${channel}...`);
    try {
      const result = await window.x3studio.permissions.request(channel, []);
      setStatus(`${channel}: ${result ? 'Granted' : 'Denied'}`);
      await loadPermissions();
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  return (
    <div style={{ padding: 8, overflow: 'auto', height: '100%' }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>IPC Permissions</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Control which IPC channels can access the system. Permissions are remembered for the session.
      </p>

      <table className="data-table" style={{ fontSize: 10 }}>
        <thead><tr><th>Channel</th><th>Status</th><th>Requests</th><th>Last Request</th><th>Actions</th></tr></thead>
        <tbody>
          {permissions.map(p => (
            <tr key={p.channel}>
              <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{p.channel}</td>
              <td><span className={`badge badge-${p.allowed ? 'pass' : 'fail'}`} style={{ fontSize: 9 }}>{p.allowed ? 'Allowed' : 'Denied'}</span></td>
              <td style={{ fontSize: 10 }}>{p.count}</td>
              <td style={{ fontSize: 10, color: 'var(--text-muted)' }}>{p.lastRequest ? new Date(p.lastRequest).toLocaleString() : '—'}</td>
              <td>
                <button className="btn" style={{ fontSize: 9, padding: '2px 6px', marginRight: 4 }}
                  onClick={() => togglePermission(p.channel, !p.allowed)}>
                  {p.allowed ? 'Deny' : 'Allow'}
                </button>
                <button className="btn" style={{ fontSize: 9, padding: '2px 6px' }}
                  onClick={() => requestTest(p.channel)}>Test</button>
              </td>
            </tr>
          ))}
          {permissions.length === 0 && (
            <tr><td colSpan={5} style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 16 }}>
              No permissions have been requested yet. Permissions are prompted automatically when IPC calls are made.
            </td></tr>
          )}
        </tbody>
      </table>

      <div className="section-title">Common Channels</div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 8 }}>
        {['shell:exec', 'fs:readFile', 'fs:writeFile', 'fs:deleteFile', 'chain:rpcCall'].map(ch => (
          !permissions.find(p => p.channel === ch) && (
            <button key={ch} className="btn" style={{ fontSize: 9, padding: '2px 6px' }}
              onClick={() => requestTest(ch)}>
              Pre-approve {ch}
            </button>
          )
        ))}
      </div>

      {status && <div style={{ fontSize: 11, color: 'var(--accent-color)' }}>{status}</div>}
    </div>
  );
}
