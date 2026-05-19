"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/// <reference types="mocha" />
const { expect } = require('chai');
const builder = require('../src/handlers/bitcoin-builder');
describe('Bitcoin builder', () => {
    it('throws when missing fields', async () => {
        try {
            await builder.buildAndSignRefund({});
            throw new Error('expected to throw');
        }
        catch (err) {
            expect(err.message).to.match(/No UTXOs provided/);
        }
    }).timeout(5000);
});
