import { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../store';

export default function ValidatorPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const [nodeStatus, setNodeStatus] = useState<string>('Checking...');

  useEffect(() => {
    if (!workspacePath) return;
    (async () => {
      const { stdout } = await window.x3studio.shell.exec('ls validator* 2>/dev/null; ls node/src 2>/dev/null; ls chain-specs 2>/dev/null; echo "done"', workspacePath);
      const lines = stdout.split('\n').filter(l => l && l !== 'done');
      setNodeStatus(lines.length > 0 ? 'Validator configs detected' : 'No validator configs found');
    })();
  }, [workspacePath]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Validator Ops</div>
      <div className="panel-body">
        <div className="dashboard-card">
          <h3>Local Status</h3>
          <div className="sub">{nodeStatus}</div>
        </div>
        <div className="dashboard-card" style={{ marginTop: 6 }}>
          <h3>Chain Specs</h3>
          <div className="dashboard-actions">
            <button className="btn" onClick={async () => {
              const r = await window.x3studio.shell.exec('ls chain-specs/', workspacePath ?? undefined);
              alert(r.stdout || 'No chain-specs/ directory');
            }}>List Chain Specs</button>
          </div>
        </div>
        <div style={{ marginTop: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
          Connect to running node for live validator status, peer count, and block height.
        </div>
      </div>
    </div>
  );
}
