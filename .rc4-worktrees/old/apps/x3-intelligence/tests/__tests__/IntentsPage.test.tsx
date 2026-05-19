import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithRouter } from './setup';
import { IntentsPage } from '../../src/pages/IntentsPage';
import * as api from '../../src/services/api';

vi.mock('../../src/services/api');

describe('IntentsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render page header', async () => {
    (api.getIntents as any).mockRejectedValue(new Error('API error'));

    await act(async () => {
      renderWithRouter(<IntentsPage />);
    });

    expect(screen.getByText('Arb Intents')).toBeInTheDocument();
  });

  it('should display stats cards', async () => {
    (api.getIntents as any).mockRejectedValue(new Error('API error'));

    await act(async () => {
      renderWithRouter(<IntentsPage />);
    });

    expect(screen.getByText('Total Intents')).toBeInTheDocument();
    // 'Finalized' appears in both stat card and filter buttons
    expect(screen.getAllByText('Finalized').length).toBeGreaterThan(0);
  });

  it('should filter intents by state', async () => {
    (api.getIntents as any).mockRejectedValue(new Error('API error'));
    const user = userEvent.setup();

    await act(async () => {
      renderWithRouter(<IntentsPage />);
    });

    // All DEMO_INTENTS (5) shown initially
    expect(screen.getByText('5 shown')).toBeInTheDocument();

    // Click the Finalized filter button — wrap in act so the filter change
    // triggers the useEffect re-fetch and all resulting state updates settle.
    const finalizedBtn = screen.getByText('Finalized', { selector: 'button' });
    await act(async () => {
      await user.click(finalizedBtn);
    });

    // After filtering, only the 1 finalized intent from DEMO_INTENTS is shown
    expect(screen.getByText('1 shown')).toBeInTheDocument();
  });

  it('should display intent ledger table', async () => {
    (api.getIntents as any).mockRejectedValue(new Error('API error'));

    await act(async () => {
      renderWithRouter(<IntentsPage />);
    });

    expect(screen.getByText('Intent Ledger')).toBeInTheDocument();
  });
});
