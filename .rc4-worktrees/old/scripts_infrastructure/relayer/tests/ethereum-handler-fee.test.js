"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/// <reference types="mocha" />
const { expect } = require('chai');
const sinon = require('sinon');
describe('Ethereum handler fee bumping', () => {
    it('retries and succeeds on subsequent attempt', async () => {
        const ethHandler = require('../src/handlers/ethereum');
        process.env.ETHEREUM_RPC_URL = 'http://localhost:8545';
        process.env.ETHEREUM_PRIVATE_KEY = '0x0123456789012345678901234567890123456789012345678901234567890123';
        // Mock provider.getFeeData and wallet/provider
        const fakeProvider = { getFeeData: sinon.stub().resolves({ maxFeePerGas: 100000n, maxPriorityFeePerGas: 100000n }) };
        const fakeWait = sinon.stub();
        fakeWait.onFirstCall().rejects(new Error('nonce error'));
        fakeWait.onSecondCall().resolves({ transactionHash: '0xabc' });
        const fakeTx = { wait: fakeWait };
        // Default withdraw resolves to fakeTx, but first call should reject
        const fakeContract = { withdraw: sinon.stub().resolves(fakeTx) };
        fakeContract.withdraw.onFirstCall().rejects(new Error('revert')); // low-level revert
        // Stub ethers provider and contract behavior by modifying prototypes
        const ethers = require('ethers');
        const origWithdraw = ethers.Contract.prototype.withdraw;
        ethers.Contract.prototype.withdraw = fakeContract.withdraw;
        // Stub JsonRpcProvider.getFeeData
        const origGetFeeData = ethers.JsonRpcProvider.prototype.getFeeData;
        ethers.JsonRpcProvider.prototype.getFeeData = fakeProvider.getFeeData;
        // Call handler
        try {
            const res = await ethHandler.ethereumHandler({ swapId: 's', chain: 'ethereum', preimage: '01', lock: { address: '0xdeadbeef', htlcId: '0x01' } });
            expect(res).to.equal('0xabc');
        }
        finally {
            ethers.Contract.prototype.withdraw = origWithdraw;
            ethers.JsonRpcProvider.prototype.getFeeData = origGetFeeData;
        }
    });
});
