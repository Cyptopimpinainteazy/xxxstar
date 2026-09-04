import { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../store';

export default function GitDiffPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const branch = useWorkspaceStore(s => s.branch);
  const gitStatus = useWorkspaceStore(s => s.gitStatus);
  const [diffs, setDiffs] = useState<{ file: string; diff: string }[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<'files' | 'unstaged' | 'staged'>('files');

  const loadDiffs = async () => {
    if (!workspacePath) return;
    setLoading(true);
    const files = gitStatus.map(s => s.file);
    const result: { file: string; diff: string }[] = [];
    for (const file of files.slice(0, 20)) {
      try {
        const diff = await window.x3studio.git.diffFile(workspacePath, file);
        if (diff) result.push({ file, diff });
      } catch {}
    }
    setDiffs(result);
    if (result.length > 0) setSelectedFile(result[0].file);
    setLoading(false);
  };

  useEffect(() => { if (workspacePath) loadDiffs(); }, [workspacePath]);

  const handleStageFile = async (file: string) => {
    if (!workspacePath) return;
    await window.x3studio.shell.exec(`git add "${file}"`, workspacePath);
    loadDiffs();
  };

  const handleUnstageFile = async (file: string) => {
    if (!workspacePath) return;
    await window.x3studio.shell.exec(`git restore --staged "${file}"`, workspacePath);
    loadDiffs();
  };

  const handleDiscard = async (file: string) => {
    if (!workspacePath) return;
    await window.x3studio.shell.exec(`git checkout -- "${file}"`, workspacePath);
    loadDiffs();
  };

  if (!workspacePath) {
    return <div style={{ padding: 16, color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)' }}>Open a workspace to see git changes.</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Git Diff</span>
        <span className="badge badge-info">{branch}</span>
      </div>

      <div style={{ display: 'flex', gap: 4, padding: '4px 8px', borderBottom: '1px solid var(--border-color)' }}>
        <button className={`btn ${viewMode === 'files' ? 'btn-primary' : ''}`} onClick={() => setViewMode('files')} style={{ fontSize: 10, padding: '2px 6px' }}>Files</button>
        <button className="btn" onClick={loadDiffs} disabled={loading} style={{ fontSize: 10, padding: '2px 6px' }}>
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <div style={{ width: '40%', overflow: 'auto', borderRight: '1px solid var(--border-color)', fontSize: 'var(--font-size-sm)' }}>
          {diffs.map(d => (
            <div
              key={d.file}
              className={`tree-node ${selectedFile === d.file ? 'active' : ''}`}
              onClick={() => setSelectedFile(d.file)}
              style={{ cursor: 'pointer' }}
            >
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{d.file}</span>
              <div style={{ display: 'flex', gap: 2, marginTop: 2 }}>
                <button className="btn" onClick={(e) => { e.stopPropagation(); handleStageFile(d.file); }}
                  style={{ fontSize: 9, padding: '1px 4px' }} title="Stage file">+</button>
                <button className="btn" onClick={(e) => { e.stopPropagation(); handleDiscard(d.file); }}
                  style={{ fontSize: 9, padding: '1px 4px', color: 'var(--red)' }} title="Discard changes">✕</button>
              </div>
            </div>
          ))}
          {diffs.length === 0 && !loading && (
            <div style={{ color: 'var(--text-muted)', padding: 16, textAlign: 'center' }}>
              No changes detected
            </div>
          )}
        </div>

        <div style={{ flex: 1, overflow: 'auto', padding: 8 }}>
          {selectedFile ? (
            <pre style={{
              fontSize: 11, fontFamily: 'var(--font-mono)', whiteSpace: 'pre-wrap',
              background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)',
              minHeight: '100%',
            }}>
              {diffs.find(d => d.file === selectedFile)?.diff || 'No diff available'}
            </pre>
          ) : (
            <div style={{ color: 'var(--text-muted)', padding: 16, fontSize: 'var(--font-size-sm)' }}>
              Select a file to view diff
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
