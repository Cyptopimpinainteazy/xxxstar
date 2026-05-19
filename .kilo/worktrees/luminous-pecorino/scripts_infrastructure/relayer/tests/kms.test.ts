/// <reference types="mocha" />
const { expect } = require('chai');
const LocalKmsProvider = require('../src/kms/local-provider').default;
const { Psbt } = require('bitcoinjs-lib');
const bitcoin = require('bitcoinjs-lib');
const ecc = require('tiny-secp256k1');
const { ECPairFactory } = require('ecpair');
const ECPair = ECPairFactory(ecc);

describe('Local KMS provider', () => {
  it('signs PSBT inputs for registered key', async () => {
    const network = (process.env.BITCOIN_NETWORK === 'mainnet') ? bitcoin.networks.bitcoin : bitcoin.networks.regtest;
    const kp = ECPair.makeRandom({ network });
    const wif = kp.toWIF();

    const provider = new LocalKmsProvider();
    provider.addKey('test-key', wif, network);

    const pub = kp.publicKey;
    const p2wpkh = bitcoin.payments.p2wpkh({ pubkey: pub, network });

    // Build simple PSBT with one input (witnessUtxo) and one output
    const psbt = new Psbt({ network });
    const fakeHash = Buffer.alloc(32, 0x01);

    psbt.addInput({
      hash: fakeHash.toString('hex'),
      index: 0,
      witnessUtxo: { script: p2wpkh.output, value: 100000n },
    });

    psbt.addOutput({ address: p2wpkh.address, value: 90000n });

    // Sign using provider
    await provider.signPsbt(psbt, 'test-key');

    // Expect partialSig in input
    const input = psbt.data.inputs[0];
    expect(input.partialSig).to.exist;
  });
});
export {};
