import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import WorldMonitorPanel from '@/components/panels/WorldMonitorPanel';

// Mock the guarded tauriInvoke used by the panel
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('WorldMonitorPanel', () => {
  beforeEach(() => {
    // Provide a mocked Tauri runtime presence so guarded tauriInvoke passes the runtime check
    (window as any).__TAURI__ = {};
  });

  afterEach(() => {
    vi.resetAllMocks();
    delete (window as any).__TAURI__;
  });

  test('uses IPFS gateway when launch_ipfs_storage returns matching pinned object', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockResolvedValueOnce({
      pinned_objects: [
        { cid: 'QmWorldMonitor', name: 'world-monitor-bundle', size: 1234 },
      ],
    });

    render(<WorldMonitorPanel />);

    // In the test environment the iframe health-check will fail (no local gateway).
    // Verify the component chose the IPFS gateway URL (displayed in the unreachable overlay).
    await waitFor(() => {
      expect(screen.getByText(/QmWorldMonitor/)).toBeInTheDocument();
      const codeEl = screen.getByText(/QmWorldMonitor/).closest('code');
      expect(codeEl).toBeTruthy();
      expect(codeEl?.textContent).toContain('http://127.0.0.1:8080/ipfs/QmWorldMonitor/');
    });
  });

  test('detects SES and shows safe native fallback when SES present and no IPFS', async () => {
    // Simulate a global lockdown function
    (window as any).lockdown = () => {};

    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockRejectedValueOnce(new Error('IPFS unavailable'));

    render(<WorldMonitorPanel />);

    await waitFor(() => {
      expect(screen.getByText(/Secure JS lockdown/i)).toBeInTheDocument();
    });

    delete (window as any).lockdown;
  });

  test('uses configured ISPF CID from localStorage and surfaces the CID in the UI', async () => {
    const cid = 'bafytestcid12345';
    localStorage.setItem('WORLDMONITOR_ISPF_CID', cid);

    // Ensure tauri invoke is not required for this path
    const { invoke } = await import('@tauri-apps/api/core');
    (invoke as any).mockRejectedValue(new Error('should not be called'));

    render(<WorldMonitorPanel />);

    // The panel will show the configured ISPF CID (the iframe health-check may fail in tests)
    await waitFor(() => expect(screen.getByText(new RegExp(cid.slice(0, 8)))).toBeInTheDocument());

    // Ensure the displayed URL includes the ISPF scheme (reflects chosen target)
    const codeEl = screen.getByText(new RegExp(cid.slice(0, 8))).closest('code');
    expect(codeEl).toBeTruthy();
    expect(codeEl?.textContent).toContain(`ispf://${cid}`);
  });
});
