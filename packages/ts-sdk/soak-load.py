# Soak loader: continuously submits remarks (bounded in-flight) for `durSec` seconds from one
# funded account and reports cumulative finalized nonce delta + avg finalized TPS over the soak.
import json, os, subprocess, sys, time

WS = os.environ.get("WS", "ws://127.0.0.1:9980")
DUR = int(os.environ.get("DUR", "90"))
SURI = os.environ["SURI"]
api_dir = "/home/lojak/Desktop/xxxstar-main/packages/ts-sdk/node_modules/@polkadot/"
script = r'''
import { createRequire } from 'module';
const apiDir = process.env.APID;
async function main() {
  const { ApiPromise, WsProvider, Keyring } = await import(apiDir + 'api/index.js');
  const { cryptoWaitReady } = await import(apiDir + 'util-crypto/index.js');
  await cryptoWaitReady();
  const api = await ApiPromise.create({ provider: new WsProvider(process.env.WS) });
  const k = new Keyring({ type: 'sr25519' });
  const who = k.addFromUri(process.env.SURI);
  const dur = parseInt(process.env.DUR || '90', 10);
  const acc0 = await api.query.system.account(who.address);
  const startNonce = acc0.nonce.toNumber();
  console.log('acc=' + who.address + ' startNonce=' + startNonce + ' dur=' + dur);
  const inflight = new Set(); let sent = 0; const t0 = Date.now();
  while (Date.now() - t0 < dur * 1000 || true) {
    while (sent < 999999 && inflight.size < 120) {
      const nonce = startNonce + sent; inflight.add(nonce);
      api.tx.system.remark('soak-' + sent + '-' + Date.now())
        .signAndSend(who, { nonce }).then(()=>inflight.delete(nonce)).catch(()=>inflight.delete(nonce));
      sent++; // attempt one new per iteration? guard loop, allow idle wait below
    }
    if (inflight.size === 0) break;
    await new Promise(r=>setTimeout(r, 200));
    if (Date.now() - t0 > dur*1000 && inflight.size===0) break;
    // stop sending after dur, let drain
  }
  // drain window: stop new sends after `dur`; wait remaining finalize for up to 120s
  // (we already stop when inflight empty). wait steady nonce
  let stable=0, prev=-1, fin=startNonce;
  for (;;) {
    await new Promise(r=>setTimeout(r, 2000));
    const a = await api.query.system.account(who.address);
    const n = a.nonce.toNumber();
    if (n===prev) stable++; else stable=0;
    prev=n; fin=n;
    if (stable>=5) break;
  }
  const wall=Date.now()-t0;
  console.log('SOAKRES ult ' + JSON.stringify({sent, finNonce: fin, delta: fin-startNonce, wallMs: wall, finTPS:+(((fin-startNonce)/(wall/1000)).toFixed(2))}));
  await api.disconnect(); process.exit(0);
}
main().catch(e=>{console.error('FATAL', e); process.exit(1);});
'''
env = {**os.environ, "APID": api_dir, "SURI": SURI, "WS": WS, "DUR": str(DUR)}
open("/tmp/soak.mjs","w").write(script)
# run via node from ts-sdk cwd
r = subprocess.run(["node", "/tmp/soak.mjs"], env=env, capture_output=True, text=True, timeout=360)
print(r.stdout[-1500:])
if r.stderr: print("STDERR", r.stderr[-1200:])
