/// <reference types="mocha" />
const { expect } = require('chai');
const sinon = require('sinon');
const btcHandler = require('../src/handlers/bitcoin');

describe('Bitcoin handler (unit)', () => {
  it('throws if RPC not configured', async () => {
    const saved = process.env.BITCOIN_RPC_URL;
    delete process.env.BITCOIN_RPC_URL;
    try {
      await btcHandler.bitcoinHandler({ swapId: 's', chain: 'bitcoin' } as any);
      throw new Error('expected to throw');
    } catch (err: any) {
      expect(err.message).to.match(/BITCOIN_RPC_URL is not configured/);
    } finally {
      process.env.BITCOIN_RPC_URL = saved;
    }
  });

  it('broadcasts raw tx when provided (mocked)', async () => {
    process.env.BITCOIN_RPC_URL = 'http://localhost:18332';

    const fakeFetch = sinon.stub().resolves({ json: sinon.stub().resolves({ result: 'txid123' }) });
    const origFetch = (globalThis as any).fetch;
    (globalThis as any).fetch = fakeFetch;

    const res = await btcHandler.bitcoinHandler({ swapId: 's', chain: 'bitcoin', lock: { rawTx: '0100' } } as any);
    expect(res).to.equal('txid123');

    // restore
    (globalThis as any).fetch = origFetch;
  });
});

export {};
