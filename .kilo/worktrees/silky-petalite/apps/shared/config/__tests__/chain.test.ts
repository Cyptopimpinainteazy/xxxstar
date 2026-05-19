import { getExplorerUrl } from '../chain';

describe('chain config - explorer URLs', () => {
  const OLD_ENV = process.env;

  beforeEach(() => {
    jest.resetModules(); // clear module cache
    process.env = { ...OLD_ENV };
  });

  afterAll(() => {
    process.env = OLD_ENV;
  });

  it('returns testnet explorer URL when NEXT_PUBLIC_NETWORK=testnet', () => {
    process.env.NEXT_PUBLIC_NETWORK = 'testnet';
    const url = getExplorerUrl();
    expect(url).toBe('https://explorer.testnet.x3-chain.io');
  });

  it('returns local explorer URL when NEXT_PUBLIC_NETWORK=local', () => {
    process.env.NEXT_PUBLIC_NETWORK = 'local';
    const url = getExplorerUrl();
    expect(url).toBe('http://localhost:3000/explorer');
  });
});
