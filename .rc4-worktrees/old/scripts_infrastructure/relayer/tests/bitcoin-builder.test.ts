/// <reference types="mocha" />
const { expect } = require('chai');
const builder = require('../src/handlers/bitcoin-builder');

describe('Bitcoin builder', () => {
  it('throws when missing fields', async () => {
    try {
      await builder.buildAndSignRefund({} as any);
      throw new Error('expected to throw');
    } catch (err: any) {
      expect(err.message).to.match(/No UTXOs provided/);
    }
  }).timeout(5000);
});

export {};
