/// <reference types="mocha" />
const { expect } = require('chai');
const request = require('supertest');
const appModule = require('../src/http-server');
const app = appModule.default || appModule;

describe('Relayer HTTP stub', () => {
  it('responds with txid on /settlement', async () => {
    // Ensure no auth env vars interfere with this test
    delete process.env.RELAYER_TOKEN;
    delete process.env.RELAYER_HMAC_SECRET;

    // Register mock handler to avoid depending on full chain config
    if (typeof appModule.registerHandler === 'function') {
      appModule.registerHandler('ethereum', async (_payload: any) => 'txid-http');
    }

    const res = await request(app)
      .post('/settlement')
      .send({ swapId: 'swap-1', chain: 'ethereum', type: 'settlement' })
      .expect(200);

    expect(res.body.txid).to.be.a('string');
    expect(res.body.txid.length).to.be.greaterThan(0);
  });
});

export {};
