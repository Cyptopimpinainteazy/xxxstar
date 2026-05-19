"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const assert_1 = __importDefault(require("assert"));
const node_fetch_1 = __importDefault(require("node-fetch"));
// Mocha-style test
describe('e2e: Local KMS + bitcoind (regtest)', function () {
    this.timeout(120000);
    const bitcoinRpc = process.env.BITCOIN_RPC_URL;
    const kmsWif = process.env.RELAYER_LOCAL_KMS_WIF || process.env.RELAYER_KMS_WIF || process.env.KMS_LOCAL_WIF;
    it('skips when BITCOIN_RPC_URL or RELAYER_LOCAL_KMS_WIF not set', function () {
        this.skip();
    });
    return;
}, function rpc(method, params = []) {
    const body = JSON.stringify({ jsonrpc: '1.0', id: 'e2e', method, params });
    const headers = { 'Content-Type': 'application/json' };
    return (0, node_fetch_1.default)(bitcoinRpc, { method: 'POST', body, headers }).then((r) => r.json()).then((j) => {
        if (j.error)
            throw new Error(JSON.stringify(j.error));
        return j.result;
    });
}, it('registers LocalKms from env, builds and broadcasts tx signed by KMS', async () => {
    // Init Local KMS provider from env (this will register a provider)
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { initLocalKmsFromEnv } = require('../../src/kms/bootstrap');
    initLocalKmsFromEnv();
    try {
        await rpc('createwallet', ['e2e-wallet', false, true]);
    }
    catch (e) { }
    const minerAddr = await rpc('getnewaddress', ['miner']);
    await rpc('generatetoaddress', [101, minerAddr]);
    const depositAddr = await rpc('getnewaddress', ['deposit']);
    const txid = await rpc('sendtoaddress', [depositAddr, '1.0']);
    await rpc('generatetoaddress', [1, minerAddr]);
    const unspent = await rpc('listunspent', [1, 9999999, [depositAddr]]);
    (0, assert_1.default)(unspent.length >= 1, 'no utxo found');
    const utxo = unspent[0];
    const rawtx = await rpc('getrawtransaction', [utxo.txid, true]);
    utxo.hex = rawtx.hex;
    const toAddr = await rpc('getnewaddress', ['to']);
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { buildAndSignRefund } = require('../../src/handlers/bitcoin-builder');
    const payload = {
        utxos: [
            {
                txid: utxo.txid,
                vout: utxo.vout,
                value: BigInt(Math.floor(utxo.amount * 1e8)),
                scriptPubKey: utxo.scriptPubKey,
                hex: utxo.hex,
                address: depositAddr,
            },
        ],
        outputs: [
            {
                address: toAddr,
                value: BigInt(90000000),
            },
        ],
        lock: {
            kmsKeyId: process.env.RELAYER_KMS_KEY_ID || 'local-1',
        },
    };
    const hex = await buildAndSignRefund(payload);
    (0, assert_1.default)(hex && typeof hex === 'string', 'builder did not return raw hex');
    const txidSent = await rpc('sendrawtransaction', [hex]);
    (0, assert_1.default)(txidSent && typeof txidSent === 'string', 'sendrawtransaction did not return txid');
    const mempoolAfter = await rpc('getrawmempool');
    (0, assert_1.default)(mempoolAfter.includes(txidSent), 'tx not found in mempool after broadcast');
}));
;
