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
exports.LocalKmsProvider = void 0;
const bitcoin = __importStar(require("bitcoinjs-lib"));
// Local provider: for dev & testing only. Holds keys in memory.
class LocalKmsProvider {
    constructor(opts) {
        this.name = 'local-file-keystore';
        this.keys = new Map();
        // In future, load keys from opts.keyFilePath and decrypt
    }
    addKey(keyId, wif, network) {
        const net = network || (process.env.BITCOIN_NETWORK === 'mainnet' ? bitcoin.networks.bitcoin : bitcoin.networks.regtest);
        const ecc = require('tiny-secp256k1');
        const { ECPairFactory } = require('ecpair');
        const ECPair = ECPairFactory(ecc);
        const kp = ECPair.fromWIF(wif, net);
        this.keys.set(keyId, kp);
    }
    async signPsbt(psbt, keyId) {
        const key = this.keys.get(keyId);
        if (!key)
            throw new Error(`KMS key not found: ${keyId}`);
        // Sign all inputs with the key if they correspond to the key's pubkey
        for (let i = 0; i < psbt.inputCount; i++) {
            try {
                psbt.signInput(i, key);
            }
            catch (err) {
                // input may not be ours; continue
            }
        }
        // Note: do not finalize here - leave to caller to validate and finalize
    }
}
exports.LocalKmsProvider = LocalKmsProvider;
exports.default = LocalKmsProvider;
