import assert from 'assert';
import fetch from 'node-fetch';
import { URL } from 'url';
import * as bitcoin from 'bitcoinjs-lib';

// Mocha-style test
describe('e2e: Local KMS + bitcoind (regtest)', function () {
  this.timeout(120000);

  const bitcoinRpc = process.env.BITCOIN_RPC_URL;
  const kmsWif = process.env.RELAYER_LOCAL_KMS_WIF;

  if (!bitcoinRpc || !kmsWif) {
    it('skips when BITCOIN_RPC_URL or RELAYER_LOCAL_KMS_WIF not set', function () {
      this.skip();
    });
    return;
  }

  async function rpc(method: string, params: any[] = []) {
    const u = new URL(bitcoinRpc as string);
    const auth = u.username && u.password ? { username: u.username, password: u.password } : null;
    const bodyObj = { jsonrpc: '1.0', id: 'e2e', method, params };
    const headers: any = { 'Content-Type': 'application/json' };
    if (auth) {
      const creds = Buffer.from(`${auth.username}:${auth.password}`).toString('base64');
      headers.Authorization = `Basic ${creds}`;
    }

    const maxAttempts = 6;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        const res = await fetch(bitcoinRpc as string, { method: 'POST', body: JSON.stringify(bodyObj), headers });
        if (!res.ok) {
          const txt = await res.text().catch(() => '<no-body>');
          throw new Error(`RPC HTTP ${res.status}: ${txt}`);
        }
        const j = await res.json();
        if (j.error) throw new Error(JSON.stringify(j.error));
        return j.result;
      } catch (err: any) {
        console.warn(`[TEST] rpc ${method} attempt ${attempt} failed: ${err && err.message ? err.message : err}`);
        if (attempt === maxAttempts) throw err;
        // exponential-ish backoff
        await new Promise((r) => setTimeout(r, attempt * 500));
      }
    }

    throw new Error('unreachable');
  }

  it('registers LocalKms from env, builds and broadcasts tx signed by KMS', async () => {
    // Init Local KMS provider from env
    const { initLocalKmsFromEnv } = require('../../src/kms/bootstrap');
    const provider = initLocalKmsFromEnv();
    console.info(`[TEST] initLocalKmsFromEnv returned provider: ${provider ? provider.name : 'null'}`);
    const kmsMod = require('../../src/kms');
    const currentProvider = kmsMod.getProvider && kmsMod.getProvider();
    console.info(`[TEST] getProvider() -> ${currentProvider ? currentProvider.name : 'null'}`);
    if (!currentProvider) throw new Error('KMS provider not registered by bootstrap');

    // Create wallet for miner (to fund the KMS address)
    try {
      await rpc('createwallet', ['e2e-wallet', false, false]);
      console.info('[TEST] created wallet e2e-wallet');
    } catch (err: any) {
      if (err.message && err.message.includes('already exists')) {
        console.info('[TEST] wallet e2e-wallet already exists, loading...');
        await rpc('loadwallet', ['e2e-wallet']);
      } else {
        throw err;
      }
    }

    // Generate miner address and mine 101 blocks to get funds
    const minerAddr = await rpc('getnewaddress', ['miner']);
    console.info(`[TEST] minerAddr=${minerAddr}`);
    await rpc('generatetoaddress', [101, minerAddr]);

    // Derive the KMS key's P2WPKH address
    const network = bitcoin.networks.regtest;
    const ecc = require('tiny-secp256k1');
    const { ECPairFactory } = require('ecpair');
    const ECPair = ECPairFactory(ecc);
    const keyPair = ECPair.fromWIF(kmsWif, network);
    const pubkey = Buffer.from(keyPair.publicKey);
    const { address: kmsAddress } = bitcoin.payments.p2wpkh({ pubkey, network });
    console.info(`[TEST] KMS-derived address: ${kmsAddress}`);

    // Fund the KMS address by sending from the wallet
    const depositTxid = await rpc('sendtoaddress', [kmsAddress, '1.0']);
    console.info(`[TEST] depositTxid=${depositTxid}`);
    await rpc('generatetoaddress', [1, minerAddr]); // confirm

    // Get the raw transaction for nonWitnessUtxo
    const rawtx = await rpc('getrawtransaction', [depositTxid, true]);
    // Find which output goes to the KMS address
    const vout = rawtx.vout.findIndex((o: any) => o.scriptPubKey.address === kmsAddress);
    assert(vout >= 0, 'KMS address not found in deposit tx outputs');
    const utxo = {
      txid: depositTxid,
      vout,
      value: BigInt(Math.floor(rawtx.vout[vout].value * 1e8)),
      hex: rawtx.hex,
    };
    console.info(`[TEST] utxo txid=${utxo.txid}, vout=${utxo.vout}, value=${utxo.value}`);

    // Destination address (can be any valid address; use a wallet address)
    const toAddr = await rpc('getnewaddress', ['to']);

    // Build and sign with KMS
    const { buildAndSignRefund } = require('../../src/handlers/bitcoin-builder');
    const payload = {
      lock: {
        utxos: [utxo],
        refundTo: toAddr,
        feeRate: 10,
        // privateKeyWIF needed as fallback but KMS should be used
        privateKeyWIF: kmsWif,
        kmsKeyId: process.env.RELAYER_KMS_KEY_ID || 'local-1',
      },
    };

    const hex = await buildAndSignRefund(payload);
    console.info(`[TEST] builder returned hex length=${hex ? hex.length : 0}`);
    assert(hex && typeof hex === 'string', 'builder did not return raw hex');

    // Broadcast via sendrawtransaction
    const txid = await rpc('sendrawtransaction', [hex]);
    console.info(`[TEST] broadcast returned txid=${txid}`);
    assert(txid && typeof txid === 'string', 'sendrawtransaction did not return txid');

    // Ensure mempool contains it
    const mempoolAfter = await rpc('getrawmempool');
    assert(mempoolAfter.includes(txid), 'tx not found in mempool after broadcast');
    console.info(`[TEST] E2E KMS signing and broadcast successful for keyId=${payload.lock.kmsKeyId}, txid=${txid}`);
  });
});
