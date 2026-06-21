import { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../store';

export default function RelayerPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const [relayerStatus, setRelayerStatus] = useState<any[]>([]);

  useEffect(() => {
    if (!workspacePath) return;
    (async () => {
      const { stdout } = await window.x3studio.shell.exec('ls relayer* 2>/dev/null; ls **/relayer* 2>/dev/null; echo "done"', workspacePath);
      const lines = stdout.split('\n').filter(l => l && l !== 'done');
      setRelayerStatus(lines.map(l => ({ path: l, status: 'detected' })));
    })();
  }, [workspacePath]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Relayer Status</div>
      <div className="panel-body">
        {relayerStatus.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16 }}>
            No relayer directories detected. Run detection from Control Center.
          </div>
        )}
        {relayerStatus.map((r, i) => (
          <div key={i} className="dashboard-card" style={{ marginBottom: 4 }}>
            <div className="sub">{r.path}</div>
            <span className="badge badge-pass">detected</span>
          </div>
        ))}
        {relayerStatus.length > 0 && (
          <div style={{ marginTop: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
            Run 'cargo build' and check for running processes for live status.
          </div>
        )}
      </div>
    </div>
  );
}
