import { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../store';

export default function GitPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const branch = useWorkspaceStore(s => s.branch);
  const gitStatus = useWorkspaceStore(s => s.gitStatus);
  const [log, setLog] = useState<{ hash: string; message: string }[]>([]);
  const [commitMsg, setCommitMsg] = useState('');

  useEffect(() => {
    if (!workspacePath) return;
    window.x3studio.git.log(workspacePath, 10).then(setLog);
  }, [workspacePath]);

  const doCommit = async () => {
    if (!workspacePath || !commitMsg) return;
    const { exitCode } = await window.x3studio.shell.exec(`git add -A && git commit -m "${commitMsg}"`, workspacePath);
    if (exitCode === 0) {
      setCommitMsg('');
      const l = await window.x3studio.git.log(workspacePath, 10);
      setLog(l);
      const status = await window.x3studio.git.status(workspacePath);
      useWorkspaceStore.getState().setGitStatus(status);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Git</span>
        <span className="badge badge-info">{branch}</span>
      </div>
      <div className="panel-body">
        <div className="section-title">Changed Files ({gitStatus.length})</div>
        {gitStatus.map((s, i) => (
          <div key={i} className="tree-node">
            <span className="badge badge-info" style={{ fontSize: 10, padding: '0 4px', marginRight: 4 }}>{s.status}</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--font-size-sm)' }}>{s.file}</span>
          </div>
        ))}
        {gitStatus.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 8 }}>
            Working tree clean
          </div>
        )}

        {gitStatus.length > 0 && (
          <>
            <div className="section-title">Commit</div>
            <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
              <input className="input-field" value={commitMsg} onChange={e => setCommitMsg(e.target.value)}
                placeholder="Commit message" onKeyDown={e => e.key === 'Enter' && doCommit()} />
              <button className="btn btn-primary" onClick={doCommit} disabled={!commitMsg}>Commit</button>
            </div>
          </>
        )}

        <div className="section-title">Recent Commits</div>
        {log.map(l => (
          <div key={l.hash} className="tree-node">
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--font-size-sm)', color: 'var(--yellow)', marginRight: 8 }}>
              {l.hash.substring(0, 7)}
            </span>
            <span style={{ fontSize: 'var(--font-size-sm)' }}>{l.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
