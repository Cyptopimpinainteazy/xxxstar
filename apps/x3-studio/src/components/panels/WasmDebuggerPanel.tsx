import { useState, useEffect } from 'react';
import { useWasmDebuggerStore, useWorkspaceStore } from '../../store';

export default function WasmDebuggerPanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const modules = useWasmDebuggerStore(s => s.modules);
  const activeModule = useWasmDebuggerStore(s => s.activeModule);
  const setModules = useWasmDebuggerStore(s => s.setModules);
  const setActive = useWasmDebuggerStore(s => s.setActive);
  const addModule = useWasmDebuggerStore(s => s.addModule);

  const [filePath, setFilePath] = useState('');
  const [wasmFiles, setWasmFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!wp) return;
    setLoading(true);
    window.x3studio.fs.glob(wp, '**/*.wasm')
      .then(files => setWasmFiles(files))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [wp]);

  const handleInspect = async (path: string) => {
    setLoading(true);
    try {
      const mod = await window.x3studio.wasm.inspect(path);
      addModule(mod);
      setActive(mod);
    } catch (e) {
      console.error('Wasm inspect failed', e);
    }
    setLoading(false);
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>WASM Debugger</span>
        <span className="badge badge-info">{modules.length} modules</span>
      </div>
      <div className="panel-body" style={{ padding: '8px', overflow: 'auto' }}>
        <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
          <input
            className="input-field"
            style={{ flex: 1, fontSize: 11 }}
            value={filePath}
            onChange={e => setFilePath(e.target.value)}
            placeholder="Path to .wasm file"
          />
          <button className="btn btn-primary" onClick={() => handleInspect(filePath)} disabled={loading || !filePath}>
            Inspect
          </button>
        </div>

        {activeModule && (
          <div style={{ display: 'flex', gap: 8, padding: 8, marginBottom: 8, background: 'var(--bg-surface)', borderRadius: 'var(--radius)', fontSize: 11 }}>
            <div><strong>Size:</strong> {formatSize(activeModule.size)}</div>
            <div><strong>Hex:</strong> 0x{activeModule.size.toString(16).toUpperCase()}</div>
            <div><strong>Functions:</strong> {activeModule.functions}</div>
            <div><strong>Memories:</strong> {activeModule.memories}</div>
            <div><strong>Tables:</strong> {activeModule.tables}</div>
          </div>
        )}

        {wasmFiles.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Workspace .wasm Files</div>
            {wasmFiles.map(f => (
              <div
                key={f}
                className="tree-node"
                style={{ fontSize: 'var(--font-size-sm)', cursor: 'pointer' }}
                onClick={() => handleInspect(f)}
              >
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{f}</span>
              </div>
            ))}
          </div>
        )}

        {modules.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Inspected Modules</div>
            <table className="data-table">
              <thead>
                <tr>
                  <th>Path</th>
                  <th>Size</th>
                </tr>
              </thead>
              <tbody>
                {modules.map(m => (
                  <tr
                    key={m.path}
                    onClick={() => setActive(m)}
                    style={{ cursor: 'pointer', background: activeModule?.path === m.path ? 'var(--bg-active)' : undefined }}
                  >
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{m.path}</td>
                    <td style={{ fontSize: 11 }}>{formatSize(m.size)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {activeModule && (
          <>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Sections</div>
            <table className="data-table">
              <thead><tr><th>Name</th><th>Size</th></tr></thead>
              <tbody>
                {activeModule.sections.map((s, i) => (
                  <tr key={i}>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{s.name}</td>
                    <td style={{ fontSize: 11 }}>{formatSize(s.size)}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Imports</div>
            <table className="data-table">
              <thead><tr><th>Module</th><th>Name</th><th>Kind</th></tr></thead>
              <tbody>
                {activeModule.imports.length === 0 && (
                  <tr><td colSpan={3} style={{ color: 'var(--text-muted)', fontSize: 11 }}>None</td></tr>
                )}
                {activeModule.imports.map((imp, i) => (
                  <tr key={i}>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{imp.module}</td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{imp.name}</td>
                    <td style={{ fontSize: 11 }}>{imp.kind}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Exports</div>
            <table className="data-table">
              <thead><tr><th>Name</th><th>Kind</th></tr></thead>
              <tbody>
                {activeModule.exports.length === 0 && (
                  <tr><td colSpan={2} style={{ color: 'var(--text-muted)', fontSize: 11 }}>None</td></tr>
                )}
                {activeModule.exports.map((exp, i) => (
                  <tr key={i}>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{exp.name}</td>
                    <td style={{ fontSize: 11 }}>{exp.kind}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        )}
      </div>
    </div>
  );
}
