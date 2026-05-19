import React, { useEffect } from 'react';
import Landing from './components/Landing';
import FrontendShell from './components/FrontendShell';
import LiveStatsDashboard from './components/LiveStatsDashboard';
import DriveWorld from './components/DriveWorld';
import { siteMeta } from './siteMap';

export default function App() {
  const params = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();
  const isShellSurface = params.get('surface') === 'shell';
  const isDriveMode = params.get('mode') === 'drive';
  const fromDrive = params.get('from') === 'drive';

  useEffect(() => {
    if (typeof document !== 'undefined') {
      document.title = isDriveMode ? 'X3 Chain — Drive' : siteMeta.title;
      const descriptionMeta = document.querySelector('meta[name="description"]');
      if (descriptionMeta) {
        descriptionMeta.setAttribute('content', siteMeta.description);
      }
    }
  }, [isDriveMode]);

  // "Back to World" floating button — shown when arriving from DriveWorld
  const BackToWorld = fromDrive ? (
    <a
      href="?mode=drive"
      title="Back to X3 Drive World"
      style={{
        position: 'fixed', bottom: 28, left: 28,
        background: 'rgba(6,6,15,0.88)',
        border: '1px solid rgba(0,212,255,0.4)',
        borderRadius: 50,
        padding: '10px 20px', fontSize: 14, fontWeight: 700,
        color: '#00d4ff', cursor: 'pointer',
        textDecoration: 'none', display: 'flex', alignItems: 'center', gap: 8,
        boxShadow: '0 0 20px rgba(0,212,255,0.2)',
        zIndex: 9999,
        backdropFilter: 'blur(12px)',
        transition: 'transform 0.15s, box-shadow 0.15s, border-color 0.15s',
        fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
        letterSpacing: 0.5,
      }}
      onMouseEnter={e => { e.currentTarget.style.transform = 'scale(1.05)'; e.currentTarget.style.borderColor = 'rgba(0,212,255,0.8)'; }}
      onMouseLeave={e => { e.currentTarget.style.transform = 'scale(1)'; e.currentTarget.style.borderColor = 'rgba(0,212,255,0.4)'; }}
    >
      ← Back to World
    </a>
  ) : null;

  if (isShellSurface) {
    return <>{BackToWorld}<FrontendShell /></>;
  }

  if (isDriveMode) {
    return <DriveWorld />;
  }

  return (
    <>
      <Landing />
      <LiveStatsDashboard />
      {BackToWorld}
      {/* Floating Drive button (only when not coming from drive, to avoid overlap) */}
      {!fromDrive && (
        <a
          href="?mode=drive"
          title="Drive the X3 world"
          style={{
            position: 'fixed', bottom: 28, right: 28,
            background: 'linear-gradient(135deg, #00d4ff, #a855f7)',
            border: 'none', borderRadius: 50,
            padding: '13px 22px', fontSize: 15, fontWeight: 800,
            color: '#06060f', cursor: 'pointer',
            textDecoration: 'none', display: 'flex', alignItems: 'center', gap: 8,
            boxShadow: '0 0 24px rgba(0,212,255,0.5)',
            zIndex: 9999,
            transition: 'transform 0.15s, box-shadow 0.15s',
            fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
            letterSpacing: 0.5,
          }}
          onMouseEnter={e => { e.currentTarget.style.transform = 'scale(1.06)'; e.currentTarget.style.boxShadow = '0 0 40px rgba(0,212,255,0.7)'; }}
          onMouseLeave={e => { e.currentTarget.style.transform = 'scale(1)'; e.currentTarget.style.boxShadow = '0 0 24px rgba(0,212,255,0.5)'; }}
        >
          🚗 DRIVE X3
        </a>
      )}
    </>
  );
}
