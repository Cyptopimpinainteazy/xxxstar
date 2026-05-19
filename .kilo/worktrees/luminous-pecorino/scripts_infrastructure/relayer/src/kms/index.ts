import { Psbt } from 'bitcoinjs-lib';

export interface KmsProvider {
  name: string;
  signPsbt(psbt: Psbt, keyId: string): Promise<void>;
}

let provider: KmsProvider | null = null;

export function registerProvider(p: KmsProvider) {
  provider = p;
}

export function getProvider(): KmsProvider | null {
  return provider;
}
