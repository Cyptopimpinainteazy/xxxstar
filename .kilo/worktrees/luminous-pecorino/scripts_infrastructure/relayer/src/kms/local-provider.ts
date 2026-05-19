import { KmsProvider } from './index';
import { Psbt } from 'bitcoinjs-lib';
import * as bitcoin from 'bitcoinjs-lib';

export interface LocalKmsOptions {
  // simple JSON file path or in-memory mapping for tests
  keyFilePath?: string; // not used in this initial mocked provider
}

// Local provider: for dev & testing only. Holds keys in memory.
export class LocalKmsProvider implements KmsProvider {
  name = 'local-file-keystore';
  private keys: Map<string, any> = new Map();

  constructor(opts?: LocalKmsOptions) {
    // In future, load keys from opts.keyFilePath and decrypt
  }

  addKey(keyId: string, wif: string, network?: bitcoin.Network) {
    const net = network || (process.env.BITCOIN_NETWORK === 'mainnet' ? bitcoin.networks.bitcoin : bitcoin.networks.regtest);
    const ecc = require('tiny-secp256k1');
    const { ECPairFactory } = require('ecpair');
    const ECPair = ECPairFactory(ecc);
    const kp = ECPair.fromWIF(wif, net);
    this.keys.set(keyId, kp);
  }

  async signPsbt(psbt: Psbt, keyId: string): Promise<void> {
    const key = this.keys.get(keyId);
    if (!key) throw new Error(`KMS key not found: ${keyId}`);

    // Sign all inputs with the key if they correspond to the key's pubkey
    for (let i = 0; i < psbt.inputCount; i++) {
      try {
        psbt.signInput(i, key as any);
      } catch (err) {
        // input may not be ours; continue
      }
    }

    // Note: do not finalize here - leave to caller to validate and finalize
  }
}

export default LocalKmsProvider;
