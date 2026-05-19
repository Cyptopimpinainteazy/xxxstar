"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.buildAndSignRefund = buildAndSignRefund;
const bitcoin = __importStar(require("bitcoinjs-lib"));
async function buildAndSignRefund(payload) {
    // Expects payload.lock to contain: utxos: [{txid, vout, value, scriptPubKey}], refundTo: address, feeRate: sat/vB, privateKeyWIF
    const lock = payload.lock || {};
    const utxos = lock.utxos || [];
    const refundTo = lock.refundTo;
    const feeRate = lock.feeRate || 50; // sat/vB
    const wif = lock.privateKeyWIF;
    if (utxos.length === 0)
        throw new Error('No UTXOs provided');
    if (!refundTo)
        throw new Error('No refund output provided');
    if (!wif)
        throw new Error('No private key provided');
    const network = (process.env.BITCOIN_NETWORK === 'mainnet') ? bitcoin.networks.bitcoin : bitcoin.networks.regtest;
    // Use ECPairFactory for key handling (compatible across installs)
    const ecc = require('tiny-secp256k1');
    const { ECPairFactory } = require('ecpair');
    const ECPair = ECPairFactory(ecc);
    const keyPair = ECPair.fromWIF(wif, network);
    const psbt = new bitcoin.Psbt({ network });
    let inputValue = 0n;
    for (const utxo of utxos) {
        psbt.addInput({ hash: utxo.txid, index: utxo.vout, nonWitnessUtxo: Buffer.from(utxo.hex, 'hex') });
        inputValue += BigInt(utxo.value);
    }
    // Simple fee estimation: vsize approx inputs*148 + outputs*34 + 10
    const vsize = utxos.length * 148 + 1 * 34 + 10;
    const fee = BigInt(Math.max(1, Math.floor(Number(feeRate) * vsize)));
    const amountOut = inputValue - fee;
    if (amountOut <= 0n)
        throw new Error('Insufficient funds after fee');
    psbt.addOutput({ address: refundTo, value: amountOut });
    // Sign inputs: prefer KMS provider signing when available (key id from payload.lock.kmsKeyId or env)
    const keyId = lock.kmsKeyId || process.env.RELAYER_KMS_KEY_ID;
    // Try dynamic import to avoid CJS/ESM loader races
    let kms = null;
    if (keyId) {
        try {
            const kmsModule = await Promise.resolve().then(() => __importStar(require('../kms')));
            kms = kmsModule.getProvider ? kmsModule.getProvider() : null;
        }
        catch (err) {
            kms = null;
        }
    }
    if (kms && keyId) {
        // KMS provider will sign relevant inputs
        await kms.signPsbt(psbt, keyId);
        // Validate and finalize
        psbt.validateSignaturesOfAllInputs(() => true);
        psbt.finalizeAllInputs();
    }
    else {
        // Fallback to local WIF signing (development only)
        for (let i = 0; i < utxos.length; i++) {
            psbt.signInput(i, keyPair);
        }
        psbt.validateSignaturesOfAllInputs(() => true);
        psbt.finalizeAllInputs();
    }
    const tx = psbt.extractTransaction();
    return tx.toHex();
}
