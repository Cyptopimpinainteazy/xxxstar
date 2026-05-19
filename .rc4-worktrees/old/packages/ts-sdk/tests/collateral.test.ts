import { CollateralManagerClient } from '../src/collateral';

describe('CollateralManagerClient', () => {
  beforeEach(() => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ result: { bondId: 'bond-123', txHash: '0xabc' } }),
    } as any);
  });

  it('creates a deposit receipt', async () => {
    const client = new CollateralManagerClient('http://localhost');
    const r = await client.depositBond('acct1', 'USDC', 100n);
    expect(r.bondId).toMatch(/^bond-/);
  });
});
