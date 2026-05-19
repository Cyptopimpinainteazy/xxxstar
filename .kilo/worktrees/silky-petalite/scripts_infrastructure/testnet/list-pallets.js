const { ApiPromise, WsProvider } = require('@polkadot/api');
(async () => {
  const api = await ApiPromise.create({ provider: new WsProvider('ws://127.0.0.1:9933') });
  for (const [name, mod] of Object.entries(api.tx)) {
    const calls = Object.keys(mod);
    if (calls.some(c => c.toLowerCase().includes('author'))) {
      console.log(name, '->', calls.filter(c => c.toLowerCase().includes('author')));
    }
  }
  // also show x3Kernel calls if present
  if (api.tx.x3Kernel) console.log('x3Kernel calls:', Object.keys(api.tx.x3Kernel));
  process.exit(0);
})().catch(e => { console.error(e); process.exit(1); });
