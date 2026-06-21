import { useState, useEffect } from 'react';
import { useExtensionStore, useLayoutStore } from '../../store';
import type { ExtensionPanel } from '../../types';

export default function ExtensionManagerPanel() {
  const registeredPanels = useExtensionStore(s => s.panels);
  const registerPanel = useExtensionStore(s => s.registerPanel);
  const unregisterPanel = useExtensionStore(s => s.unregisterPanel);
  const [candidates, setCandidates] = useState<any[]>([]);
  const [installed, setInstalled] = useState<any[]>([]);
  const [scanPath, setScanPath] = useState('');
  const [status, setStatus] = useState('');

  useEffect(() => { loadInstalled(); }, []);

  const loadInstalled = async () => {
    try {
      const list = await window.x3studio.extensions.listInstalled();
      setInstalled(list);
    } catch {}
  };

  const scanDir = async () => {
    if (!scanPath) return;
    setStatus('Scanning...');
    try {
      const results = await window.x3studio.extensions.scanDirectory(scanPath);
      setCandidates(results);
      setStatus(`Found ${results.length} extensions`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  const installExt = async (cand: any) => {
    setStatus(`Installing ${cand.name}...`);
    try {
      await window.x3studio.extensions.installExtension(cand.path, cand.name);
      const ep: ExtensionPanel = { id: `ext-${cand.name}`, label: cand.name, icon: cand.icon || '📦', component: 'dynamic', description: cand.description, version: cand.version };
      registerPanel(ep);
      await loadInstalled();
      setStatus(`✓ Installed ${cand.name}`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  const uninstallExt = async (name: string) => {
    setStatus(`Uninstalling ${name}...`);
    try {
      await window.x3studio.extensions.uninstallExtension(name);
      unregisterPanel(`ext-${name}`);
      await loadInstalled();
      setStatus(`✓ Uninstalled ${name}`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  return (
    <div style={{ padding: 8, overflow: 'auto', height: '100%' }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Extension Manager</div>

      <div className="section-title">Scan Directory for Extensions</div>
      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <input className="input-field" style={{ flex: 1, fontSize: 11 }} value={scanPath}
          onChange={e => setScanPath(e.target.value)} placeholder="/path/to/extensions" />
        <button className="btn" onClick={scanDir}>Scan</button>
      </div>

      {candidates.length > 0 && (
        <>
          <div className="section-title">Available Extensions ({candidates.length})</div>
          {candidates.map(c => (
            <div key={c.name} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
              <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>{c.icon} {c.name} <span style={{ color: 'var(--text-muted)', fontWeight: 400 }}>v{c.version}</span></div>
              <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>{c.description}</div>
              <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>Panels: {c.panels.join(', ') || 'none'}</div>
              <button className="btn" style={{ marginTop: 4, fontSize: 10 }} onClick={() => installExt(c)}>Install</button>
            </div>
          ))}
        </>
      )}

      <div className="section-title">Installed Extensions ({installed.length})</div>
      {installed.map(ext => (
        <div key={ext.name} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
          <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>{ext.name} <span style={{ color: 'var(--text-muted)', fontWeight: 400 }}>v{ext.version}</span></div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>{ext.description}</div>
          <button className="btn btn-danger" style={{ marginTop: 4, fontSize: 10 }} onClick={() => uninstallExt(ext.name)}>Uninstall</button>
        </div>
      ))}

      {installed.length === 0 && candidates.length === 0 && (
        <div style={{ color: 'var(--text-muted)', fontSize: 11, textAlign: 'center', padding: 16 }}>
          No extensions installed. Scan a directory to find extensions.
        </div>
      )}

      <div className="section-title">Registered Runtime Panels ({registeredPanels.length})</div>
      {registeredPanels.map(p => (
        <div key={p.id} className="tree-node" style={{ fontSize: 11 }}>
          <span>{p.icon} </span>
          <span>{p.label}</span>
          <span style={{ color: 'var(--text-muted)', marginLeft: 8 }}>v{p.version}</span>
        </div>
      ))}

      {status && <div style={{ marginTop: 8, fontSize: 11, color: 'var(--accent-color)' }}>{status}</div>}
    </div>
  );
}
