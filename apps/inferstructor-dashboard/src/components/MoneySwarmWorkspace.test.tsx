import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MoneySwarmWorkspace } from './MoneySwarmWorkspace';

afterEach(() => vi.unstubAllGlobals());

const opportunity = {
  grant_id: 'fixture-grant', external_id: 'grants.gov:123',
  title: 'Fixture scientific computing program', sponsor: 'Fixture agency',
  status: 'open', stage: 'discovery', score: 0, amount_usd: 0,
  metadata: { source: 'grants.gov', official_url: 'https://www.grants.gov/search-results-detail/123',
    eligibility_status: 'UNKNOWN', deadline: '2027-01-01',
    last_verified_at: '2026-10-18T20:00:00Z', description: 'Research infrastructure' },
  created_at: '2026-10-18T20:00:00Z', updated_at: '2026-10-18T20:00:00Z',
};

function respond(grants = [opportunity]) {
  vi.stubGlobal('fetch', vi.fn(async (url: string) => new Response(JSON.stringify(
    url.includes('scoreboard') ? { total_grants: grants.length, open_grants: grants.length,
      submitted_proposals: 0, approved_grants: 0, funded_grants: 0, total_awarded_usd: 0,
      last_event_at: null } : url.includes('timeline') ? [] : grants,
  ), { status: 200 })));
}

describe('Money Swarm workspace', () => {
  it('shows source-backed opportunities and never calls relevance confirmed eligibility', async () => {
    respond();
    render(<MoneySwarmWorkspace />);
    expect(await screen.findByText(opportunity.title)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /official source/i })).toHaveAttribute('href', opportunity.metadata.official_url);
    expect(screen.getAllByText(/eligibility unknown/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/no verified awards/i)).toBeInTheDocument();
  });

  it('shows the backend failure instead of invented funding statistics', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 503 })));
    render(<MoneySwarmWorkspace />);
    expect(await screen.findByRole('alert')).toHaveTextContent(/unavailable/i);
    expect(screen.queryByText(opportunity.title)).not.toBeInTheDocument();
    expect(screen.queryByText('$120,000')).not.toBeInTheDocument();
  });

  it('filters actual loaded opportunities and exposes an actionable empty state', async () => {
    respond();
    render(<MoneySwarmWorkspace />);
    await screen.findByText(opportunity.title);
    fireEvent.change(screen.getByRole('searchbox', { name: /search opportunities/i }), { target: { value: 'no matching research' } });
    expect(screen.getByText(/no opportunities match/i)).toBeInTheDocument();
    expect(screen.queryByText(opportunity.title)).not.toBeInTheDocument();
  });

  it('does not render unsafe official-source URLs as links', async () => {
    respond([{ ...opportunity, metadata: { ...opportunity.metadata, official_url: 'javascript:alert(1)' } }]);
    render(<MoneySwarmWorkspace />);
    await screen.findByText(opportunity.title);
    expect(screen.queryByRole('link', { name: /official source/i })).not.toBeInTheDocument();
  });

  it('does not discover or modify records without an operator credential', async () => {
    respond([]);
    render(<MoneySwarmWorkspace />);
    await waitFor(() => expect(screen.getByRole('button', { name: /discover opportunities/i })).toBeDisabled());
    expect(vi.mocked(fetch).mock.calls.every(([, options]) => !options || options.method === 'GET')).toBe(true);
  });

  it('uses the canonical versioned admin route for authenticated discovery', async () => {
    const calls = vi.fn(async (url: string) => new Response(JSON.stringify(
      url.includes('/api/v1/admin/funding-swarm/discovery/grants-gov') ? { imported: 1, changed: 1 }
        : url.includes('scoreboard') ? { total_grants: 1, open_grants: 1, approved_grants: 0, funded_grants: 0, total_awarded_usd: 0 }
        : url.includes('timeline') ? [] : [opportunity],
    ), { status: 200 }));
    vi.stubGlobal('fetch', calls);
    render(<MoneySwarmWorkspace />);
    await screen.findByText(opportunity.title);
    fireEvent.change(screen.getByLabelText('Operator credential'), { target: { value: 'fixture-test-credential' } });
    fireEvent.click(screen.getByRole('button', { name: 'Discover opportunities' }));
    await screen.findByText(/Source refresh complete/);
    expect(calls.mock.calls.some(([url]) => url.endsWith('/api/v1/admin/funding-swarm/discovery/grants-gov'))).toBe(true);
  });

  it('keeps historical research credential-gated and separate from public awards', async () => {
    const archived = [{
      source_file: 'grants/C27-chain-grants.md', legacy_id: 'C27.1.1',
      name: 'Historical protocol treasury lead', category: 'C27-chain-grants',
      historical_status: 'AWARDED', current_availability: 'UNKNOWN',
      current_eligibility: 'UNKNOWN', review_state: 'UNVERIFIED',
    }];
    const fetchStub = vi.fn(async (url: string, options?: RequestInit) =>
      new Response(JSON.stringify(url.includes('/legacy-leads') ? archived
        : url.includes('scoreboard') ? { total_grants: 0, open_grants: 0, approved_grants: 0,
          funded_grants: 0, total_awarded_usd: 0 }
        : []), { status: 200 }));
    vi.stubGlobal('fetch', fetchStub);
    render(<MoneySwarmWorkspace />);
    await screen.findByText(/no verified awards/i);
    fireEvent.click(screen.getByRole('button', { name: /Historical leads/i }));
    expect(screen.getByRole('button', { name: /load private research/i })).toBeDisabled();
    expect(fetchStub.mock.calls.some(([url]) => url.includes('/legacy-leads'))).toBe(false);
    fireEvent.change(screen.getByLabelText('Operator credential'), { target: { value: 'private-operator-test-token' } });
    fireEvent.click(screen.getByRole('button', { name: /load private research/i }));
    expect(await screen.findByText('Historical protocol treasury lead')).toBeInTheDocument();
    expect(screen.getByText(/Historical label only: AWARDED/i)).toBeInTheDocument();
    expect(screen.getByText(/Current availability UNKNOWN/i)).toBeInTheDocument();
    expect(screen.getByText(/Eligibility UNKNOWN/i)).toBeInTheDocument();
    fireEvent.change(screen.getByRole('searchbox', { name: /search historical leads/i }), { target: { value: 'treasury' } });
    expect(screen.queryByText('Historical protocol treasury lead')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /load private research/i }));
    expect(await screen.findByText('Historical protocol treasury lead')).toBeInTheDocument();
    expect(fetchStub.mock.calls.some(([url]) => url.includes('search=treasury'))).toBe(true);
    expect(fetchStub.mock.calls.some(([url, options]) =>
      url.includes('/legacy-leads') && (options?.headers as Record<string, string>)?.['x-admin-token'] === 'private-operator-test-token'
    )).toBe(true);
    fireEvent.change(screen.getByLabelText('Operator credential'), { target: { value: '' } });
    expect(screen.queryByText('Historical protocol treasury lead')).not.toBeInTheDocument();
  });

  it('refuses to present forged current status in historical archive responses', async () => {
    vi.stubGlobal('fetch', vi.fn(async (url: string) => new Response(JSON.stringify(
      url.includes('/legacy-leads') ? [{
        source_file: 'grants/C27-chain-grants.md', legacy_id: 'C27.1.1',
        name: 'Bad archived lead', category: 'C27-chain-grants',
        historical_status: 'RESEARCH', current_availability: 'OPEN',
        current_eligibility: 'UNKNOWN', review_state: 'UNVERIFIED',
      }] : url.includes('scoreboard') ?
        { total_grants: 0, open_grants: 0, approved_grants: 0, funded_grants: 0, total_awarded_usd: 0 } : [],
    ), { status: 200 })));
    render(<MoneySwarmWorkspace />);
    await screen.findByText(/no verified awards/i);
    fireEvent.click(screen.getByRole('button', { name: /Historical leads/i }));
    fireEvent.change(screen.getByLabelText('Operator credential'), { target: { value: 'test-token' } });
    fireEvent.click(screen.getByRole('button', { name: /load private research/i }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/unexpected verification state/i);
    expect(screen.queryByText('Bad archived lead')).not.toBeInTheDocument();
  });
});
