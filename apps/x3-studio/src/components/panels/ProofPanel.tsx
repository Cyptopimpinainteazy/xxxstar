import { useState } from 'react';
import { useWorkspaceStore, useProofStore } from '../../store';
import { runProofCommand } from '../../services/proofGenerator';

export default function ProofPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const records = useProofStore(s => s.records);
  const isRunning = useProofStore(s => s.isRunning);
  const addRecord = useProofStore(s => s.addRecord);
  const setRunning = useProofStore(s => s.setRunning);
  const clear = useProofStore(s => s.clear);
  const [customCmd, setCustomCmd] = useState('');

  const runCommand = async (cmd: string, label: string) => {
    if (!workspacePath) return;
    setRunning(true);
    const rec = await runProofCommand(cmd, workspacePath, label);
    addRecord(rec);
    setRunning(false);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Proof Mode</span>
        <span className="badge badge-pass" style={{ fontSize: 10 }}>ACTIVE</span>
      </div>
      <div className="panel-body">
        <div className="dashboard-actions">
          <button className="btn btn-primary" onClick={() => runCommand('cargo check 2>&1', 'Cargo Check')} disabled={isRunning}>
            {isRunning ? 'Running...' : 'Cargo Check'}
          </button>
          <button className="btn" onClick={() => runCommand('cargo test 2>&1 | tail -20', 'Cargo Test')} disabled={isRunning}>
            Cargo Test
          </button>
          <button className="btn" onClick={() => runCommand('forge build 2>&1', 'Forge Build')} disabled={isRunning}>
            Forge Build
          </button>
          <button className="btn" onClick={() => runCommand('forge test 2>&1 | tail -20', 'Forge Test')} disabled={isRunning}>
            Forge Test
          </button>
        </div>

        <div className="form-group">
          <label>Custom Command</label>
          <div style={{ display: 'flex', gap: 4 }}>
            <input className="input-field" value={customCmd} onChange={e => setCustomCmd(e.target.value)}
              placeholder="e.g. cargo clippy -- -D warnings" onKeyDown={e => {
                if (e.key === 'Enter' && customCmd) runCommand(customCmd, customCmd);
              }} />
            <button className="btn btn-primary" onClick={() => customCmd && runCommand(customCmd, customCmd)}
              disabled={!customCmd || isRunning}>Run</button>
          </div>
        </div>

        <div className="section-title">Proof History ({records.length})</div>
        {records.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 8 }}>
            No proofs recorded yet. Run a command above.
          </div>
        )}
        {records.map(r => (
          <div key={r.id} style={{ background: 'var(--bg-surface)', border: '1px solid var(--border)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--font-size-sm)' }}>{r.command.substring(0, 60)}</span>
              <span className={`badge badge-${r.status === 'PASS' ? 'pass' : 'fail'}`}>{r.status}</span>
            </div>
            <div style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
              Exit: {r.exitCode} • {r.duration}ms • {r.changedFiles.length} files changed
            </div>
            {r.stderr && r.exitCode !== 0 && (
              <div className="code-block" style={{ maxHeight: 60, marginTop: 4, color: 'var(--red)' }}>
                {r.stderr.substring(0, 300)}
              </div>
            )}
          </div>
        ))}
        {records.length > 0 && (
          <button className="btn" onClick={clear} style={{ marginTop: 8 }}>Clear History</button>
        )}
      </div>
    </div>
  );
}
