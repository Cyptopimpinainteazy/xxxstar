import { useState } from 'react';
import { FileJson, Download, Loader2, Copy, Play } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api, type Template } from '../api/client';

export function TemplatesPanel() {
  const { data: templates, loading, refresh } = useApi(() => api.templates(), []);
  const [selected, setSelected] = useState<Template | null>(null);
  const [templateContent, setTemplateContent] = useState<string | null>(null);
  const [loadingContent, setLoadingContent] = useState(false);
  const [scaffoldName, setScaffoldName] = useState('');
  const [scaffoldResult, setScaffoldResult] = useState<{ name: string; path: string; files: string[] } | null>(null);
  const [scaffolding, setScaffolding] = useState(false);

  const selectTemplate = async (t: Template) => {
    setSelected(t);
    setScaffoldResult(null);
    setScaffoldName(t.name);
    setLoadingContent(true);
    try {
      const data = await api.template(t.name);
      setTemplateContent(data.content);
    } catch {
      setTemplateContent('Error loading template');
    } finally {
      setLoadingContent(false);
    }
  };

  const doScaffold = async () => {
    if (!selected || !scaffoldName) return;
    setScaffolding(true);
    try {
      const result = await api.scaffold(selected.name, scaffoldName);
      setScaffoldResult(result);
    } catch (e) {
      console.error(e);
    } finally {
      setScaffolding(false);
    }
  };

  const copyContent = () => {
    if (templateContent) navigator.clipboard.writeText(templateContent);
  };

  return (
    <div style={{ display: 'flex', height: '100%', color: '#d4d4d4' }}>
      <div style={{ width: 260, borderRight: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
        <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526' }}>
          X3 PROJECT TEMPLATES ({templates?.length || 0})
        </div>
        {loading && <div style={{ padding: 12 }}><Loader2 size={14} className="spin" /> Loading...</div>}
        {templates?.map(t => (
          <div key={t.name} onClick={() => selectTemplate(t)}
            style={{
              padding: '10px 12px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer',
              background: selected?.name === t.name ? '#2a2a2a' : 'transparent',
            }}
            onMouseEnter={e => { if (selected?.name !== t.name) e.currentTarget.style.background = '#2a2a2a' }}
            onMouseLeave={e => { if (selected?.name !== t.name) e.currentTarget.style.background = 'transparent' }}
          >
            <div style={{ fontWeight: 600, color: '#569cd6', fontSize: 13 }}>
              <FileJson size={12} style={{ marginRight: 6 }} />
              {t.name.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
            </div>
            <div style={{ fontSize: 11, color: '#888', marginTop: 2, lineHeight: 1.4 }}>{t.description.slice(0, 100)}</div>
            <div style={{ fontSize: 10, color: '#666', marginTop: 2 }}>{t.lines} lines · {t.filename}</div>
          </div>
        ))}
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {!selected && (
          <div style={{ color: '#666', fontStyle: 'italic', textAlign: 'center', marginTop: 40 }}>
            Select a template to preview it, then scaffold a new project
          </div>
        )}

        {selected && (
          <>
            <div style={{ marginBottom: 16 }}>
              <h3 style={{ margin: '0 0 4px', fontSize: 15, color: '#569cd6' }}>
                {selected.name.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
              </h3>
              <p style={{ fontSize: 12, color: '#888', margin: 0 }}>{selected.description}</p>
            </div>

            <div style={{ display: 'flex', gap: 12, marginBottom: 16, alignItems: 'center' }}>
              <input value={scaffoldName} onChange={e => setScaffoldName(e.target.value)}
                placeholder="Project name"
                style={{ flex: 1, padding: '6px 10px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none' }}
              />
              <button onClick={doScaffold} disabled={scaffolding || !scaffoldName}
                style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '6px 14px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12, opacity: scaffolding ? 0.6 : 1 }}
              >
                {scaffolding ? <Loader2 size={14} className="spin" /> : <Download size={14} />} Scaffold
              </button>
              <button onClick={copyContent}
                style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '6px 10px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12 }}
              ><Copy size={12} /> Copy</button>
            </div>

            {scaffoldResult && (
              <div style={{ padding: '8px 12px', background: '#1a3a2a', border: '1px solid #4ec9b0', borderRadius: 6, marginBottom: 16, fontSize: 12 }}>
                <div style={{ color: '#4ec9b0' }}>✓ Project "{scaffoldResult.name}" created</div>
                <div style={{ color: '#888', marginTop: 4 }}>Path: apps/super-ide/projects/{scaffoldResult.name}/</div>
                <div style={{ color: '#888' }}>Files: {scaffoldResult.files.join(', ')}</div>
              </div>
            )}

            <pre style={{
              margin: 0, padding: 12, background: '#252526', border: '1px solid #333',
              borderRadius: 8, fontSize: 12, fontFamily: "'Fira Code', monospace",
              overflow: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-all',
              maxHeight: 'calc(100vh - 280px)',
            }}>
              {loadingContent ? 'Loading...' : templateContent}
            </pre>
          </>
        )}
      </div>
    </div>
  );
}
