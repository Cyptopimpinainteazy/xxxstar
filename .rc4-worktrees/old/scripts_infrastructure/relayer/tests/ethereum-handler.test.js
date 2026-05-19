"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/// <reference types="mocha" />
const { expect } = require('chai');
const sinon = require('sinon');
const ethHandler = require('../src/handlers/ethereum');
describe('Ethereum handler (unit)', () => {
    it('throws if RPC not configured', async () => {
        const envRpc = process.env.ETHEREUM_RPC_URL;
        delete process.env.ETHEREUM_RPC_URL;
        try {
            await ethHandler.ethereumHandler({ swapId: 's', chain: 'ethereum' });
            throw new Error('expected to throw');
        }
        catch (err) {
            expect(err.message).to.match(/ETHEREUM_RPC_URL is not configured/);
        }
        finally {
            process.env.ETHEREUM_RPC_URL = envRpc;
        }
    });
    it('uses contract withdraw when configured (mocked)', async () => {
        // Mock ethers.Contract and wallet behavior
        process.env.ETHEREUM_RPC_URL = 'http://localhost:8545';
        process.env.ETHEREUM_PRIVATE_KEY = '0x0123456789012345678901234567890123456789012345678901234567890123';
        const stubContract = {
            withdraw: sinon.stub().resolves({ wait: sinon.stub().resolves({ transactionHash: '0xabc' }) }),
        };
        const ethers = require('ethers');
        const origWithdraw = ethers.Contract.prototype.withdraw;
        ethers.Contract.prototype.withdraw = stubContract.withdraw;
        // Stub provider fee data to avoid real RPC calls
        const origGetFeeData = ethers.JsonRpcProvider.prototype.getFeeData;
        ethers.JsonRpcProvider.prototype.getFeeData = sinon.stub().resolves({ maxFeePerGas: 100000n, maxPriorityFeePerGas: 100000n });
        process.env.ETHEREUM_RPC_URL = 'http://localhost:8545';
        const res = await ethHandler.ethereumHandler({
            swapId: 's1',
            chain: 'ethereum',
            preimage: '01',
            lock: { address: '0xdeadbeef', htlcId: '0x01' },
        });
        expect(res).to.equal('0xabc');
        // restore
        ethers.Contract.prototype.withdraw = origWithdraw;
        ethers.JsonRpcProvider.prototype.getFeeData = origGetFeeData;
    });
});
