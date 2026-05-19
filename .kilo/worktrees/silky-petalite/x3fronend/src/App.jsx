import React from 'react';
import Landing from './components/Landing';
import FrontendShell from './components/FrontendShell';
import LiveStatsDashboard from './components/LiveStatsDashboard';

export default function App() {
  const isShellSurface =
    typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('surface') === 'shell';

  if (isShellSurface) {
    return <FrontendShell />;
  }

  return (
    <>
      <Landing />
      <LiveStatsDashboard />
    </>
  );
}
