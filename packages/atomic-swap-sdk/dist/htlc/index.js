"use strict";
/**
 * HTLC Module — Re-exports all HTLC adapters and utilities.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SubstrateHTLCAdapter = exports.BitcoinHTLCAdapter = exports.SolanaHTLCAdapter = exports.EvmHTLCAdapter = exports.calculateTimeLocks = exports.hexToBytes = exports.bytesToHex = exports.sha256FromHex = exports.sha256Hex = exports.generateSecret = void 0;
exports.createHTLCAdapter = createHTLCAdapter;
var base_1 = require("./base");
Object.defineProperty(exports, "generateSecret", { enumerable: true, get: function () { return base_1.generateSecret; } });
Object.defineProperty(exports, "sha256Hex", { enumerable: true, get: function () { return base_1.sha256Hex; } });
Object.defineProperty(exports, "sha256FromHex", { enumerable: true, get: function () { return base_1.sha256FromHex; } });
Object.defineProperty(exports, "bytesToHex", { enumerable: true, get: function () { return base_1.bytesToHex; } });
Object.defineProperty(exports, "hexToBytes", { enumerable: true, get: function () { return base_1.hexToBytes; } });
Object.defineProperty(exports, "calculateTimeLocks", { enumerable: true, get: function () { return base_1.calculateTimeLocks; } });
var evm_1 = require("./evm");
Object.defineProperty(exports, "EvmHTLCAdapter", { enumerable: true, get: function () { return evm_1.EvmHTLCAdapter; } });
var solana_1 = require("./solana");
Object.defineProperty(exports, "SolanaHTLCAdapter", { enumerable: true, get: function () { return solana_1.SolanaHTLCAdapter; } });
var bitcoin_1 = require("./bitcoin");
Object.defineProperty(exports, "BitcoinHTLCAdapter", { enumerable: true, get: function () { return bitcoin_1.BitcoinHTLCAdapter; } });
var substrate_1 = require("./substrate");
Object.defineProperty(exports, "SubstrateHTLCAdapter", { enumerable: true, get: function () { return substrate_1.SubstrateHTLCAdapter; } });
const evm_2 = require("./evm");
const solana_2 = require("./solana");
const bitcoin_2 = require("./bitcoin");
const substrate_2 = require("./substrate");
/** EVM chain IDs that use the EvmHTLCAdapter */
const EVM_CHAINS = new Set([
    "ethereum", "ethereum-sepolia", "ethereum-holesky",
    "polygon", "polygon-amoy",
    "bsc", "bsc-testnet",
    "arbitrum", "arbitrum-sepolia",
    "optimism", "optimism-sepolia",
    "base", "base-sepolia",
    "avalanche", "avalanche-fuji",
    "fantom", "zksync", "linea", "scroll", "celo", "gnosis", "moonbeam",
]);
const SOLANA_CHAINS = new Set(["solana", "solana-devnet", "solana-testnet"]);
const BITCOIN_CHAINS = new Set(["bitcoin", "bitcoin-testnet", "bitcoin-signet"]);
const SUBSTRATE_CHAINS = new Set(["x3-substrate", "polkadot", "kusama"]);
/**
 * Factory: create the right HTLC adapter for a given chain.
 */
function createHTLCAdapter(config) {
    if (EVM_CHAINS.has(config.chainId)) {
        if (!config.htlcContractAddress) {
            throw new Error(`HTLC contract address required for EVM chain ${config.chainId}`);
        }
        return new evm_2.EvmHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
    }
    if (SOLANA_CHAINS.has(config.chainId)) {
        if (!config.htlcContractAddress) {
            throw new Error(`HTLC program ID required for Solana chain ${config.chainId}`);
        }
        return new solana_2.SolanaHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
    }
    if (BITCOIN_CHAINS.has(config.chainId)) {
        const network = config.chainId === "bitcoin" ? "mainnet" :
            config.chainId === "bitcoin-signet" ? "signet" : "testnet";
        return new bitcoin_2.BitcoinHTLCAdapter(config.chainId, config.rpcEndpoint, network);
    }
    if (SUBSTRATE_CHAINS.has(config.chainId)) {
        return new substrate_2.SubstrateHTLCAdapter(config.chainId, config.rpcEndpoint, config.wsEndpoint);
    }
    // Default: try EVM adapter if contract address provided
    if (config.htlcContractAddress) {
        return new evm_2.EvmHTLCAdapter(config.chainId, config.rpcEndpoint, config.htlcContractAddress);
    }
    throw new Error(`No HTLC adapter available for chain ${config.chainId}`);
}
//# sourceMappingURL=index.js.map