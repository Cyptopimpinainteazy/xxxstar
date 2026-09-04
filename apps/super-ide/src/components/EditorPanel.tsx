import { useState, useRef, useEffect, useCallback } from 'react';
import { FileCode, Play, Save, FolderOpen, Plus, X, ChevronRight, ChevronDown, File, FileType } from 'lucide-react';
import { api, type FileEntry } from '../api/client';

interface Tab {
  path: string;
  name: string;
  content: string;
  modified: boolean;
  language: string;
}

const LANG_MAP: Record<string, string> = {
  ts: 'typescript', tsx: 'typescript', js: 'javascript', jsx: 'javascript',
  sol: 'solidity', rs: 'rust', py: 'python', json: 'json',
  md: 'markdown', html: 'html', css: 'css', x3: 'x3',
  yaml: 'yaml', yml: 'yaml', toml: 'toml',
};

function getLang(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  return LANG_MAP[ext] || 'plaintext';
}

function FileTree({ path, depth = 0, onOpen }: { path: string; depth?: number; onOpen: (path: string) => void }) {
  const [expanded, setExpanded] = useState(depth < 1);
  const [entries, setEntries] = useState<FileEntry[] | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (expanded && !entries) {
      setLoading(true);
      api.files(path).then(setEntries).catch(() => setEntries([])).finally(() => setLoading(false));
    }
  }, [expanded, path]);

  return (
    <div>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '2px 0', cursor: 'pointer', fontSize: 12, color: '#d4d4d4', paddingLeft: depth * 16 }}
        onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
        onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <FolderOpen size={14} color="#dcdcaa" />
        <span>{path.split('/').pop() || path}</span>
      </div>
      {expanded && loading && <div style={{ paddingLeft: (depth + 1) * 16 + 16, color: '#666', fontSize: 11 }}>Loading...</div>}
      {expanded && entries?.map(e => (
        e.type === 'dir' ? (
          <FileTree key={e.path} path={e.path} depth={depth + 1} onOpen={onOpen} />
        ) : (
          <div key={e.path} onClick={() => onOpen(e.path)}
            style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '2px 0', cursor: 'pointer', fontSize: 12, color: '#ccc', paddingLeft: (depth + 1) * 16 + 16 }}
            onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
            onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
          >
            <FileType size={13} color="#569cd6" />
            <span>{e.name}</span>
          </div>
        )
      ))}
    </div>
  );
}

export function EditorPanel() {
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTab, setActiveTab] = useState(0);
  const [showFileTree, setShowFileTree] = useState(true);
  const [output, setOutput] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeFile = tabs[activeTab];

  const openFile = useCallback(async (path: string) => {
    try {
      const existing = tabs.findIndex(t => t.path === path);
      if (existing >= 0) { setActiveTab(existing); return; }
      const file = await api.readFile(path);
      const name = path.split('/').pop() || path;
      setTabs(prev => [...prev, { path, name, content: file.content, modified: false, language: getLang(name) }]);
      setActiveTab(tabs.length);
    } catch (e) { console.error(e); }
  }, [tabs]);

  const saveFile = async () => {
    if (!activeFile) return;
    try {
      await api.writeFile(activeFile.path, activeFile.content);
      setTabs(prev => prev.map((t, i) => i === activeTab ? { ...t, modified: false } : t));
    } catch (e) { console.error(e); }
  };

  const closeTab = (idx: number) => {
    setTabs(prev => prev.filter((_, i) => i !== idx));
    if (activeTab >= idx && activeTab > 0) setActiveTab(prev => prev - 1);
  };

  const updateContent = (content: string) => {
    setTabs(prev => prev.map((t, i) => i === activeTab ? { ...t, content, modified: true } : t));
  };

  const runCode = () => {
    if (!activeFile) return;
    setOutput('');
    try {
      if (activeFile.language === 'x3') {
        api.compile(activeFile.content, 'x3').then(r => {
          setOutput(r.output || r.errors || (r.success ? 'Compiled successfully' : 'Compilation failed'));
        }).catch(e => setOutput(`Error: ${e.message}`));
      } else if (activeFile.language === 'javascript' || activeFile.language === 'typescript') {
        const logs: string[] = [];
        const fn = new Function('console', `
          ${activeFile.content}
          return logs;
        `);
        const mockConsole = { log: (...args: unknown[]) => logs.push(args.map(String).join(' ')) };
        const result = fn(mockConsole);
        setOutput(logs.join('\n') + (result?.length ? '\nDone.' : ''));
      } else {
        setOutput(`Cannot run ${activeFile.language} files. Use the Compiler panel to compile.`);
      }
    } catch (e) {
      setOutput(`Error: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', color: '#d4d4d4' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '4px 8px', borderBottom: '1px solid #333', background: '#252526', fontSize: 12 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <button onClick={() => setShowFileTree(!showFileTree)}
            style={{ background: 'none', border: 'none', color: '#888', cursor: 'pointer', padding: 2 }}>
            {showFileTree ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
          <FileCode size={14} />
          <span>Editor</span>
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          {activeFile && (
            <>
              <button onClick={runCode} style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '3px 10px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12 }}>
                <Play size={12} /> Run
              </button>
              <button onClick={saveFile} disabled={!activeFile?.modified}
                style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '3px 10px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: activeFile?.modified ? '#ccc' : '#666', cursor: activeFile?.modified ? 'pointer' : 'default', fontSize: 12 }}>
                <Save size={12} /> Save
              </button>
            </>
          )}
        </div>
      </div>

      <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
        {showFileTree && (
          <div style={{ width: 220, borderRight: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
            <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span>EXPLORER</span>
              <button onClick={() => openFile('x3-templates')} style={{ background: 'none', border: 'none', color: '#569cd6', cursor: 'pointer', fontSize: 11 }}>X3</button>
            </div>
            <FileTree path="." depth={0} onOpen={openFile} />
          </div>
        )}

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          {tabs.length > 0 ? (
            <>
              <div style={{ display: 'flex', background: '#252526', borderBottom: '1px solid #333', overflowX: 'auto' }}>
                {tabs.map((tab, i) => (
                  <div key={tab.path} onClick={() => setActiveTab(i)}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 6, padding: '4px 12px',
                      cursor: 'pointer', borderRight: '1px solid #333',
                      background: i === activeTab ? '#1e1e1e' : '#2d2d2d',
                      color: i === activeTab ? '#fff' : '#888',
                      fontSize: 12, whiteSpace: 'nowrap',
                      borderTop: i === activeTab ? '1px solid #0e639c' : '1px solid transparent',
                    }}
                  >
                    <FileCode size={12} />
                    <span>{tab.name}</span>
                    {tab.modified && <span style={{ color: '#dcdcaa' }}>●</span>}
                    <button onClick={(e) => { e.stopPropagation(); closeTab(i); }}
                      style={{ background: 'none', border: 'none', color: '#666', cursor: 'pointer', padding: 0, fontSize: 14 }}
                    >×</button>
                  </div>
                ))}
              </div>

              <textarea
                ref={textareaRef}
                value={activeFile?.content || ''}
                onChange={e => updateContent(e.target.value)}
                spellCheck={false}
                style={{
                  flex: 1, width: '100%', border: 'none', outline: 'none', resize: 'none',
                  padding: 16, fontFamily: "'Fira Code', 'Cascadia Code', 'Consolas', monospace",
                  fontSize: 14, lineHeight: 1.6, background: '#1e1e1e', color: '#d4d4d4',
                  tabSize: 2,
                }}
              />

              {output && (
                <div style={{
                  height: 100, borderTop: '1px solid #333', background: '#1e1e1e',
                  padding: '8px 16px', fontFamily: 'monospace', fontSize: 13,
                  overflow: 'auto', whiteSpace: 'pre-wrap', color: '#4ec9b0',
                }}>
                  <div style={{ color: '#888', fontSize: 11, marginBottom: 4 }}>OUTPUT / {activeFile?.language || 'text'}</div>
                  {output}
                </div>
              )}
            </>
          ) : (
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: '#666' }}>
              <FileCode size={48} opacity={0.3} />
              <p style={{ marginTop: 12 }}>Open a file from the explorer</p>
              <p style={{ fontSize: 12, color: '#555' }}>Browse X3-contracts/, x3-templates/, or x3-lang/</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
