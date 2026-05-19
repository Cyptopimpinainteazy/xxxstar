import LocalKmsProvider from './local-provider';
import { registerProvider } from './index';
import * as bitcoin from 'bitcoinjs-lib';

export function initLocalKmsFromEnv() {
  const wif = process.env.RELAYER_LOCAL_KMS_WIF;
  const keyId = process.env.RELAYER_LOCAL_KMS_KEY_ID || 'local-test-key';
  if (!wif) return null;

  const provider = new LocalKmsProvider();
  const network = (process.env.BITCOIN_NETWORK === 'mainnet') ? bitcoin.networks.bitcoin : bitcoin.networks.regtest;
  provider.addKey(keyId, wif, network);
  registerProvider(provider as any);
  console.info(`Local KMS provider registered with keyId=${keyId}`);
  return provider;
}
