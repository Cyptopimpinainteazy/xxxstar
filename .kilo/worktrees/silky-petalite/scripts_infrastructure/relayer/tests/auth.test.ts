/// <reference types="mocha" />
const request = require('supertest');
const { expect } = require('chai');
const appModule = require('../src/http-server');
const app = appModule.default || appModule;

describe('Relayer auth middleware', () => {
  it('rejects unauthorized when RELAYER_TOKEN set', async () => {
    process.env.RELAYER_TOKEN = 'secret123';

    const res = await request(app).post('/settlement').send({ swapId: 's', chain: 'bitcoin' });

    expect(res.status).to.be.oneOf([401, 403]);

    delete process.env.RELAYER_TOKEN;
  });

  it('accepts authorized requests', async () => {
    process.env.RELAYER_TOKEN = 'secret123';
    // Register a mock handler to ensure the endpoint returns success for the test
    if (typeof appModule.registerHandler === 'function') {
      appModule.registerHandler('bitcoin', async (_payload: any) => 'txid-mock');
    }

    const res = await request(app)
      .post('/settlement')
      .set('Authorization', 'Bearer secret123')
      .send({ swapId: 's', chain: 'bitcoin' });

    expect(res.status).to.equal(200);
    delete process.env.RELAYER_TOKEN;
  });

  it('rejects invalid hmac', async () => {
    process.env.RELAYER_HMAC_SECRET = 'hmac-secret';
    const res = await request(app).post('/settlement').send({ swapId: 's', chain: 'bitcoin' });
    expect(res.status).to.equal(401);
    delete process.env.RELAYER_HMAC_SECRET;
  });

  it('accepts valid hmac', async () => {
    process.env.RELAYER_HMAC_SECRET = 'hmac-secret';
    // Register mock handler
    if (typeof appModule.registerHandler === 'function') {
      appModule.registerHandler('bitcoin', async (_payload: any) => 'txid-hmac');
    }
    const payload = { swapId: 's', chain: 'bitcoin' };
    const sig = require('crypto').createHmac('sha256', 'hmac-secret').update(JSON.stringify(payload)).digest('hex');
    const res = await request(app).post('/settlement').set('X-Signature', sig).send(payload);
    expect(res.status).to.equal(200);
    delete process.env.RELAYER_HMAC_SECRET;
  });
});

export {};
