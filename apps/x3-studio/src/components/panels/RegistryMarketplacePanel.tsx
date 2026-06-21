import { useState, useEffect } from 'react';
import { useRegistryStore } from '../../store';

export default function RegistryMarketplacePanel() {
  const packages = useRegistryStore(s => s.packages);
  const isSearching = useRegistryStore(s => s.isSearching);
  const setPackages = useRegistryStore(s => s.setPackages);
  const setSearching = useRegistryStore(s => s.setSearching);

  const [query, setQuery] = useState('');
  const [status, setStatus] = useState('');
  const [installed, setInstalled] = useState<any[]>([]);
  const [tab, setTab] = useState<'search' | 'installed'>('search');

  useEffect(() => {
    if (tab === 'installed') loadInstalled();
  }, [tab]);

  const loadInstalled = async () => {
    try {
      const list = await window.x3studio.extensions.listInstalled();
      setInstalled(list);
    } catch {}
  };

  const search = async () => {
    setStatus('searching...');
    setSearching(true);
    try {
      const results = await window.x3studio.registry.search(query || 'x3studio');
      setPackages(results);
      setStatus('done');
    } catch (e: any) {
      setStatus('error: ' + e.message);
    } finally {
      setSearching(false);
    }
  };

  const installPkg = async (name: string, version: string) => {
    setStatus(`Installing ${name}...`);
    try {
      await window.x3studio.registry.installPackage(name, version);
      setStatus(`✓ Installed ${name}`);
    } catch (e: any) {
      setStatus('error: ' + e.message);
    }
  };

  const uninstallPkg = async (name: string) => {
    setStatus(`Uninstalling ${name}...`);
    try {
      await window.x3studio.extensions.uninstallExtension(name);
      setInstalled(prev => prev.filter(e => e.name !== name));
      setStatus(`✓ Uninstalled ${name}`);
    } catch (e: any) {
      setStatus('error: ' + e.message);
    }
  };

  const truncate = (s: string, len: number) => s.length > len ? s.slice(0, len) + '...' : s;

  return (
    <div className="panel-body">
      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className={'btn' + (tab === 'search' ? ' btn-active' : '')} onClick={() => setTab('search')}>Search</button>
        <button className={'btn' + (tab === 'installed' ? ' btn-active' : '')} onClick={() => setTab('installed')}>Installed</button>
      </div>

      {tab === 'search' && (
        <>
          <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
            <input className="input-field" style={{ flex: 1, fontSize: 11 }}
              value={query} onChange={e => setQuery(e.target.value)}
              placeholder="Search npm for x3studio packages..." />
            <button className="btn" onClick={search} disabled={isSearching}>Search</button>
          </div>

          {isSearching && <div className="status-indicator">searching...</div>}
          {!isSearching && status === 'done' && (
            <div className="status-indicator" style={{ color: 'var(--text-muted)' }}>
              Found {packages.length} package{packages.length !== 1 ? 's' : ''}
            </div>
          )}
          {!isSearching && status.startsWith('error') && (
            <div className="status-indicator" style={{ color: 'var(--error-color)' }}>{status}</div>
          )}

          {packages.map(pkg => (
            <div key={pkg.name} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
              <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>
                {pkg.name}
                <span style={{ color: 'var(--text-muted)', fontWeight: 400, marginLeft: 6 }}>v{pkg.version}</span>
              </div>
              <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 2 }}>
                {pkg.author} | {pkg.license} | {pkg.downloads.toLocaleString()} downloads
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginBottom: 4 }}>
                {truncate(pkg.description, 120)}
              </div>
              {pkg.keywords && pkg.keywords.length > 0 && (
                <div style={{ display: 'flex', gap: 3, flexWrap: 'wrap', marginBottom: 4 }}>
                  {pkg.keywords.map(kw => (
                    <span key={kw} style={{ fontSize: 9, background: 'var(--bg-surface-hover)', padding: '1px 5px', borderRadius: 3, color: 'var(--text-muted)' }}>{kw}</span>
                  ))}
                </div>
              )}
              <div style={{ display: 'flex', gap: 4 }}>
                <button className="btn" style={{ fontSize: 10 }} onClick={() => installPkg(pkg.name, pkg.version)}>Install</button>
                {pkg.homepage && (
                  <button className="btn" style={{ fontSize: 10 }} onClick={() => window.x3studio.shell.openExternal(pkg.homepage)}>View</button>
                )}
              </div>
            </div>
          ))}

          {!isSearching && packages.length === 0 && !status.startsWith('error') && (
            <div style={{ color: 'var(--text-muted)', fontSize: 11, textAlign: 'center', padding: 16 }}>
              Search for packages to get started
            </div>
          )}
        </>
      )}

      {tab === 'installed' && (
        <>
          {installed.map(ext => (
            <div key={ext.name} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
              <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>
                {ext.name}
                <span style={{ color: 'var(--text-muted)', fontWeight: 400, marginLeft: 6 }}>v{ext.version}</span>
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginBottom: 4 }}>{ext.description}</div>
              <button className="btn btn-danger" style={{ fontSize: 10 }} onClick={() => uninstallPkg(ext.name)}>Uninstall</button>
            </div>
          ))}
          {installed.length === 0 && (
            <div style={{ color: 'var(--text-muted)', fontSize: 11, textAlign: 'center', padding: 16 }}>
              No extensions installed
            </div>
          )}
        </>
      )}

      {status && !status.startsWith('searching') && !status.startsWith('done') && !status.startsWith('Found') && (
        <div style={{ marginTop: 8, fontSize: 11, color: status.startsWith('✓') ? 'var(--success-color)' : 'var(--accent-color)' }}>{status}</div>
      )}
    </div>
  );
}
