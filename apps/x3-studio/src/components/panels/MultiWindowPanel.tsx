import { useState, useEffect } from 'react';

export default function MultiWindowPanel() {
  const [windows, setWindows] = useState<{ id: string; title: string; url: string; width: number; height: number }[]>([]);
  const [url, setUrl] = useState('http://localhost:5173');
  const [title, setTitle] = useState('X3 Studio - Dev Tools');
  const [width, setWidth] = useState('800');
  const [height, setHeight] = useState('600');
  const [savedState, setSavedState] = useState<any>(null);

  useEffect(() => { loadState(); }, []);

  const loadState = async () => {
    try {
      const state = await window.x3studio.windowState.load();
      if (state) { setSavedState(state); }
    } catch {}
  };

  const openWindow = async () => {
    try {
      const id = await window.x3studio.window.create(url, { width: parseInt(width), height: parseInt(height), title });
      setWindows(prev => [...prev, { id, title, url, width: parseInt(width), height: parseInt(height) }]);
      await window.x3studio.windowState.save();
    } catch (e: any) { console.error(e); }
  };

  const closeWindow = async (id: string) => {
    try {
      await window.x3studio.window.close(id);
      setWindows(prev => prev.filter(w => w.id !== id));
      await window.x3studio.windowState.save();
    } catch {}
  };

  const saveCurrentState = async () => {
    await window.x3studio.windowState.save();
    await loadState();
  };

  const restoreState = async () => {
    const state = await window.x3studio.windowState.load();
    if (state?.secondaryWindows) {
      for (const win of state.secondaryWindows) {
        try {
          const id = await window.x3studio.window.create(win.url, { width: win.width || 800, height: win.height || 600, title: win.title || 'X3 Studio' });
          setWindows(prev => [...prev, { id, title: win.title || '', url: win.url || '', width: win.width || 800, height: win.height || 600 }]);
        } catch {}
      }
    }
  };

  return (
    <div style={{ padding: 8, overflow: 'auto', height: '100%' }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Multi-Window Manager</div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className="btn" onClick={saveCurrentState} style={{ fontSize: 10 }}>Save State</button>
        <button className="btn" onClick={restoreState} style={{ fontSize: 10 }}>Restore State</button>
        <button className="btn" onClick={loadState} style={{ fontSize: 10 }}>Load Saved Info</button>
      </div>

      {savedState && (
        <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 8, padding: 4, background: 'var(--bg-surface)', borderRadius: 'var(--radius)' }}>
          <div>Saved: {savedState.bounds ? `${savedState.bounds.width}x${savedState.bounds.height}` : 'N/A'}</div>
          <div>Secondary windows saved: {savedState.secondaryWindows?.length || 0}</div>
        </div>
      )}

      <div className="section-title">New Window</div>
      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>URL</label>
        <input className="input-field" value={url} onChange={e => setUrl(e.target.value)} placeholder="http://localhost:5173" />
      </div>
      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>Title</label>
        <input className="input-field" value={title} onChange={e => setTitle(e.target.value)} placeholder="Window title" />
      </div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        <div className="form-group" style={{ flex: 1 }}>
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Width</label>
          <input className="input-field" value={width} onChange={e => setWidth(e.target.value)} />
        </div>
        <div className="form-group" style={{ flex: 1 }}>
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Height</label>
          <input className="input-field" value={height} onChange={e => setHeight(e.target.value)} />
        </div>
      </div>
      <button className="btn btn-primary" onClick={openWindow}>Open Window</button>

      <div className="section-title">Open Windows ({windows.length})</div>
      {windows.map(w => (
        <div key={w.id} className="tree-node" style={{ fontSize: 11 }}>
          <span>{w.title}</span>
          <span style={{ color: 'var(--text-muted)', marginLeft: 8, fontFamily: 'var(--font-mono)', fontSize: 10 }}>{w.id}</span>
          <button className="btn" style={{ marginLeft: 8, fontSize: 9, padding: '2px 6px' }} onClick={() => closeWindow(w.id)}>Close</button>
        </div>
      ))}
      {windows.length === 0 && <div style={{ color: 'var(--text-muted)', fontSize: 11, padding: 8 }}>No secondary windows open.</div>}
    </div>
  );
}
