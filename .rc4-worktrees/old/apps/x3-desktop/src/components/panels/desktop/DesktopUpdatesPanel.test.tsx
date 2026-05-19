import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import DesktopUpdatesPanel from './DesktopUpdatesPanel';

describe('DesktopUpdatesPanel', () => {
  it('shows guarded desktop readiness instead of a fake update download', () => {
    render(<DesktopUpdatesPanel />);

    expect(screen.getByText('Desktop Readiness')).toBeInTheDocument();
    expect(screen.getByText('Guarded testnet desktop')).toBeInTheDocument();
    expect(screen.queryByText('Update Now')).not.toBeInTheDocument();
    expect(screen.queryByText(/Downloading update/i)).not.toBeInTheDocument();
  });

  it('renders feature modes from the readiness snapshot', () => {
    render(<DesktopUpdatesPanel />);

    expect(screen.getAllByText('Live testnet').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Guarded testnet').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Simulation testnet').length).toBeGreaterThan(0);
    expect(screen.getByText('Feature Registry Snapshot')).toBeInTheDocument();
  });
});