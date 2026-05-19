#!/usr/bin/env node
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

async function main() {
  const wsUrl = process.env.RPC_WS || 'ws://127.0.0.1:9944';
  const suri = process.env.SURI || '//Alice';
  const remark = process.env.REMARK || `x3-harness-${Date.now()}`;

  await cryptoWaitReady();
  const provider = new WsProvider(wsUrl);
  const api = await ApiPromise.create({ provider });
  const keyring = new Keyring({ type: 'sr25519' });
  const signer = keyring.addFromUri(suri);

  return new Promise((resolve, reject) => {
    let unsubscribe;

    api.tx.system
      .remark(remark)
      .signAndSend(signer, async (result) => {
        if (result.dispatchError) {
          if (result.dispatchError.isModule) {
            const decoded = api.registry.findMetaError(result.dispatchError.asModule);
            const { section, name } = decoded;
            reject(new Error(`Dispatch error: ${section}.${name}`));
          } else {
            reject(new Error(result.dispatchError.toString()));
          }
          return;
        }

        if (result.status.isFinalized) {
          console.log(`remark finalized at ${result.status.asFinalized.toHex()}`);
          if (unsubscribe) {
            unsubscribe();
          }
          await api.disconnect();
          resolve();
        }
      })
      .then((unsub) => {
        unsubscribe = unsub;
      })
      .catch(async (err) => {
        await api.disconnect();
        reject(err);
      });
  });
}

main().catch((err) => {
  console.error(err.message || err);
  process.exit(1);
});
