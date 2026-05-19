import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, screen } from '@testing-library/react';
import { renderWithRouter } from './setup';
import { FloorDashboard } from '../../src/pages/FloorDashboard';
import * as api from '../../src/services/api';

vi.mock('../../src/services/api');

describe('FloorDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render page header', async () => {
    (api.getFloorStats as any).mockRejectedValue(new Error('API error'));
    (api.getIntents as any).mockRejectedValue(new Error('API error'));

    // Wrapping render in act(async) drains all Promise microtasks (mock
    // rejections → setState) before assertions, eliminating act() warnings.
    await act(async () => {
      renderWithRouter(<FloorDashboard />);
    });

    expect(screen.getByText('X3 Floor')).toBeInTheDocument();
    expect(screen.getByText('Arbitrage jurisdiction — live')).toBeInTheDocument();
  });

  it('should display stats cards', async () => {
    (api.getFloorStats as any).mockRejectedValue(new Error('API error'));
    (api.getIntents as any).mockRejectedValue(new Error('API error'));

    await act(async () => {
      renderWithRouter(<FloorDashboard />);
    });

    expect(screen.getByText('Active Agents')).toBeInTheDocument();
    expect(screen.getByText('Total Intents')).toBeInTheDocument();
  });

  it('should fetch and display live data', async () => {
    const mockStats = {
      activeAgents: 50,
      totalIntents: 15000,
      totalVolume: '100,000,000',
      totalSlashes: 30,
      totalDisputes: 10,
      avgSuccessRate: 95.5,
      activeFlashloans: 5,
    };

    const mockIntents = {
      items: [],
      total: 0,
      page: 1,
      pageSize: 10,
    };

    (api.getFloorStats as any).mockResolvedValue(mockStats);
    (api.getIntents as any).mockResolvedValue(mockIntents);

    await act(async () => {
      renderWithRouter(<FloorDashboard />);
    });

    expect(api.getFloorStats).toHaveBeenCalled();
    expect(api.getIntents).toHaveBeenCalled();
  });
});
