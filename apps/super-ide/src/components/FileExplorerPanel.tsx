import { useState, useEffect } from 'react';
import { FolderTree, ChevronRight, ChevronDown, FileType, Loader2, RefreshCw } from 'lucide-react';
import { api, type FileEntry } from '../api/client';

function TreeItem({ path, depth, onSelect }: { path: string; depth: number; onSelect: (p: string) => void }) {
  const [expanded, setExpanded] = useState(depth < 1);
  const [entries, setEntries] = useState<FileEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const name = path.split('/').pop() || path;

  useEffect(() => {
    if (expanded && !entries) {
      setLoading(true);
      api.files(path).then(setEntries).catch(() => setEntries([])).finally(() => setLoading(false));
    }
  }, [expanded, path]);

  return (
    <div>
      <div onClick={() => setExpanded(!expanded)}
        style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '3px 0', cursor: 'pointer', fontSize: 12, color: '#d4d4d4', paddingLeft: depth * 14 }}
        onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
        onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
      >
        {expanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        <FolderTree size={13} color="#dcdcaa" />
        <span>{name}</span>
      </div>
      {expanded && loading && <div style={{ paddingLeft: (depth + 1) * 14 + 16, color: '#666', fontSize: 11 }}>Loading...</div>}
      {expanded && entries?.map(e =>
        e.type === 'dir' ? (
          <TreeItem key={e.path} path={e.path} depth={depth + 1} onSelect={onSelect} />
        ) : (
          <div key={e.path} onClick={() => onSelect(e.path)}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '3px 0', cursor: 'pointer', fontSize: 12, color: '#ccc', paddingLeft: (depth + 1) * 14 + 16 }}
            onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
            onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
          >
            <FileType size={12} color="#569cd6" />
            <span>{e.name}</span>
            <span style={{ marginLeft: 'auto', color: '#555', fontSize: 10 }}>{e.size > 1024 ? `${(e.size / 1024).toFixed(1)}k` : `${e.size}B`}</span>
          </div>
        )
      )}
    </div>
  );
}

export function FileExplorerPanel() {
  const [fileContent, setFileContent] = useState<{ path: string; content: string } | null>(null);
  const [loading, setLoading] = useState(false);

  const openFile = async (path: string) => {
    setLoading(true);
    try {
      const file = await api.readFile(path);
      setFileContent(file);
    } catch (e) {
      setFileContent({ path, content: `Error loading file: ${e}` });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', height: '100%', color: '#d4d4d4' }}>
      <div style={{ width: 280, borderRight: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
        <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526' }}>
          X3 REPO BROWSER
        </div>
        <div style={{ padding: '4px 0' }}>
          <TreeItem path="." depth={0} onSelect={openFile} />
        </div>
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {loading && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><Loader2 size={14} className="spin" /> Loading...</div>}
        {fileContent && !loading && (
          <>
            <div style={{ color: '#569cd6', fontSize: 12, fontFamily: 'monospace', marginBottom: 8 }}>
              {fileContent.path}
            </div>
            <pre style={{
              margin: 0, padding: 12, background: '#252526', border: '1px solid #333',
              borderRadius: 8, fontSize: 12, fontFamily: "'Fira Code', monospace",
              overflow: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-all',
              maxHeight: 'calc(100vh - 120px)',
            }}>
              {fileContent.content}
            </pre>
          </>
        )}
        {!fileContent && !loading && (
          <div style={{ color: '#666', fontStyle: 'italic', textAlign: 'center', marginTop: 40 }}>
            Select a file from the tree to view its contents
          </div>
        )}
      </div>
    </div>
  );
}
