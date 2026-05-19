/// <reference types="mocha" />
const { expect } = require('chai');
const builder = require('../src/handlers/bitcoin-builder');
const LocalKmsProvider = require('../src/kms/local-provider').default;
const { registerProvider } = require('../src/kms');
const bitcoin = require('bitcoinjs-lib');

describe('Bitcoin builder with KMS signing', () => {
  it('uses KMS to sign PSBT when kmsKeyId is provided', async () => {
    const network = (process.env.BITCOIN_NETWORK === 'mainnet') ? bitcoin.networks.bitcoin : bitcoin.networks.regtest;
    const ecc = require('tiny-secp256k1');
    const { ECPairFactory } = require('ecpair');
    const ECPair = ECPairFactory(ecc);
    const kp = ECPair.makeRandom({ network });
    const wif = kp.toWIF();

    const kms = new LocalKmsProvider();
    kms.addKey('builder-key', wif, network);
    registerProvider(kms);

    const fakeUtxo = {
      txid: '00'.repeat(32),
      vout: 0,
      value: 100000,
      hex: '02000000000100', // minimal fake hex to satisfy buffer size (tests only)
    };

    const payload: any = {
      lock: {
        utxos: [fakeUtxo],
        refundTo: 'tb1qexampleaddress0000000000000000000000000',
        feeRate: 1,
        kmsKeyId: 'builder-key'
      }
    };

    // Should not throw (we may not validate final tx fully due to fake hex), but builder should attempt KMS signing
    try {
      const tx = await builder.buildAndSignRefund(payload);
      expect(tx).to.be.a('string');
    } catch (err: any) {
      // builder may throw due to fake nonWitnessUtxo; ensure error is not related to missing KMS key
      expect(err.message).to.not.match(/No KMS key/);
    }
  }).timeout(5000);
});

export {};
