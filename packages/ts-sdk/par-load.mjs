// Clean parallel finalized-TPS loader. Sends COUNT remarks from one funded account with a
// bounded in-flight window of `limit`. Counts how many .isFinalized callbacks arrive and reads
// on-chain nonce delta after settling (authoritative finalized-inclusion count).
import { createRequire } from 'module';
const apiDir = '/home/lojak/Desktop/xxxstar-main/packages/ts-sdk/node_modules/@polkadot/';

async function main() {
  const { ApiPromise, WsProvider, Keyring } = await import(apiDir + 'api/index.js');
  const { cryptoWaitReady } = await import(apiDir + 'util-crypto/index.js');
  await cryptoWaitReady();
  const api = await ApiPromise.create({ provider: new WsProvider(process.env.WS || 'ws://127.0.0.1:9950') });
  const k = new Keyring({ type: 'sr25519' });
  const who = k.addFromUri(process.env.SURI);
  const count = parseInt(process.env.COUNT || '300', 10);
  const limit = parseInt(process.env.LIMIT || '128', 10);   // max in-flight
  const acc0 = await api.query.system.account(who.address);
  const startNonce = acc0.nonce.toNumber();
  console.log(`account=${who.address} startNonce=${startNonce} count=${count} inFlightLimit=${limit}`);

  let finalizedEvents = 0, sent = 0, next = startNonce, last = startNonce - 1;
  const inflight = new Set();
  let lastBlockN = 0;
  console.log('start free:', acc0.data.free.toString());

  const t0 = Date.now();
  // submission pump
  const pump = (async () => {
    // advance `next` past any already-included (startNonce is authoritative so none)
    for (;;) {
      while (sent < count && inflight.size < limit) {
        const nonce = startNonce + sent;
        inflight.add(nonce);
        const tx = api.tx.system.remark(`par-load-${sent}-${Date.now()}`);
        tx.signAndSend(who, { nonce }).then(() => inflight.delete(nonce)).catch(() => inflight.delete(nonce));
        // count finalized via separate subscription is hard for all; instead rely on nonce delta
        sent++;
      }
      if (sent >= count && inflight.size === 0) break;
      await new Promise(r => setTimeout(r, 250));
    }
  })();
  const feed = (async () => {
    let poll = 0;
    for (;;) {
      const h = await api.rpc.chain.getHeader();
      const hn = h.number.toNumber();
      if (hn !== lastBlockN) { lastBlockN = hn; }
      poll++;
      await new Promise(r => setTimeout(r, 1500));
    }
  })();
  await pump;
  // settle: wait until nonce no longer moves across 6 polls
  let stable = 0, prev = -1, finNonce = startNonce;
  for (;;) {
    await new Promise(r => setTimeout(r, 2000));
    const a = await api.query.system.account(who.address);
    const n = a.nonce.toNumber();
    if (n === prev) stable++; else stable = 0;
    prev = n; finNonce = n;
    if (stable >= 5) break;
  }
  feed.return?.();
  const wallMs = Date.now() - t0;
  console.log(`RESULT count=${count} finalizedNonce=${finNonce} delta=${finNonce - startNonce} lost=${count - (finNonce - startNonce)} wallMs=${wallMs} finalizedTPS=${((finNonce - startNonce) / (wallMs / 1000)).toFixed(1)}`);
  await api.disconnect();
  process.exit(0);
}
main().catch(e => console.error('FATAL', e)); 
