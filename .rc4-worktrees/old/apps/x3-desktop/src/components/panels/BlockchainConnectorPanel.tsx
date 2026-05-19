/**
 * BlockchainConnectorPanel — main panel for the X3 Desktop window manager.
 *
 * Provides a tabbed UI for:
 *   1. Networks  — browse all supported chains
 *   2. Connectors — create & manage live connections
 *   3. Test Bench — run test profiles & benchmarks
 *   4. Results   — view test reports
 *   5. Billing   — plan & usage
 */
import React, { useState, useMemo, useCallback } from "react";

// ─── Inline types (mirrored from @x3-chain/blockchain-connector) ────────

type ChainFamily = "evm" | "bitcoin" | "solana" | "cosmos" | "substrate" | "near" | "other";
type NetworkType = "mainnet" | "testnet" | "devnet" | "regtest" | "local";
type ConnectorStatus = "connecting" | "connected" | "syncing" | "degraded" | "disconnected" | "error";
type TestStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

interface ChainDescriptor {
  id: string;
  name: string;
  family: ChainFamily;
  network: NetworkType;
  nativeCurrency: { name: string; symbol: string; decimals: number };
  chainId: number | string;
  defaultRpcUrls: string[];
  defaultWsUrls: string[];
  explorerUrl?: string;
  available: boolean;
  avgBlockTimeSeconds: number;
  consensus: string;
  signatureAlgorithm: string;
  hashAlgorithm: string;
  gpuAccelerated: boolean;
  icon?: string;
}

interface ConnectorInstance {
  id: string;
  chain: ChainDescriptor;
  status: ConnectorStatus;
  metrics: {
    blockHeight: number;
    tps: number;
    latencyMs: number;
    peerCount: number;
    totalRequests: number;
    totalErrors: number;
    uptimeSeconds: number;
    gasPrice?: string;
    hashRate?: string;
  };
  createdAt: string;
  error?: string;
}

interface TestRun {
  id: string;
  connectorId: string;
  profileId: string;
  status: TestStatus;
  startedAt: string;
  completedAt?: string;
  results: TestResult[];
  summary?: {
    totalTests: number;
    passed: number;
    failed: number;
    totalDurationMs: number;
    overallScore: number;
    grade: string;
  };
  error?: string;
}

interface TestResult {
  testId: string;
  testName: string;
  passed: boolean;
  durationMs: number;
  metrics: Record<string, number | string | boolean | undefined>;
  error?: string;
}

// ─── Chain Registry (inline, matches packages/blockchain-connector) ──────────

const CHAINS: ChainDescriptor[] = [
  { id: "ethereum", name: "Ethereum Mainnet", family: "evm", network: "mainnet", nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 }, chainId: 1, defaultRpcUrls: ["https://eth.llamarpc.com"], defaultWsUrls: ["wss://eth.llamarpc.com"], explorerUrl: "https://etherscan.io", available: true, avgBlockTimeSeconds: 12, consensus: "Proof of Stake", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "ethereum" },
  { id: "ethereum-sepolia", name: "Ethereum Sepolia", family: "evm", network: "testnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 11155111, defaultRpcUrls: ["https://rpc.sepolia.org"], defaultWsUrls: [], explorerUrl: "https://sepolia.etherscan.io", available: true, avgBlockTimeSeconds: 12, consensus: "Proof of Stake", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "ethereum" },
  { id: "ethereum-holesky", name: "Ethereum Holesky", family: "evm", network: "testnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 17000, defaultRpcUrls: ["https://rpc.holesky.ethpandaops.io"], defaultWsUrls: [], explorerUrl: "https://holesky.etherscan.io", available: true, avgBlockTimeSeconds: 12, consensus: "Proof of Stake", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "ethereum" },
  { id: "bitcoin", name: "Bitcoin Mainnet", family: "bitcoin", network: "mainnet", nativeCurrency: { name: "Bitcoin", symbol: "BTC", decimals: 8 }, chainId: "bitcoin-mainnet", defaultRpcUrls: ["https://blockstream.info/api"], defaultWsUrls: [], explorerUrl: "https://blockstream.info", available: true, avgBlockTimeSeconds: 600, consensus: "Proof of Work", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "bitcoin" },
  { id: "bitcoin-testnet", name: "Bitcoin Testnet3", family: "bitcoin", network: "testnet", nativeCurrency: { name: "tBTC", symbol: "tBTC", decimals: 8 }, chainId: "bitcoin-testnet3", defaultRpcUrls: ["https://blockstream.info/testnet/api"], defaultWsUrls: [], explorerUrl: "https://blockstream.info/testnet", available: true, avgBlockTimeSeconds: 600, consensus: "Proof of Work", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "bitcoin" },
  { id: "bitcoin-signet", name: "Bitcoin Signet", family: "bitcoin", network: "testnet", nativeCurrency: { name: "sBTC", symbol: "sBTC", decimals: 8 }, chainId: "bitcoin-signet", defaultRpcUrls: ["https://mempool.space/signet/api"], defaultWsUrls: [], explorerUrl: "https://mempool.space/signet", available: true, avgBlockTimeSeconds: 600, consensus: "Proof of Work", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "bitcoin" },
  { id: "solana", name: "Solana Mainnet", family: "solana", network: "mainnet", nativeCurrency: { name: "Solana", symbol: "SOL", decimals: 9 }, chainId: "solana-mainnet-beta", defaultRpcUrls: ["https://api.mainnet-beta.solana.com"], defaultWsUrls: ["wss://api.mainnet-beta.solana.com"], explorerUrl: "https://solscan.io", available: true, avgBlockTimeSeconds: 0.4, consensus: "PoH + PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "solana" },
  { id: "solana-devnet", name: "Solana Devnet", family: "solana", network: "devnet", nativeCurrency: { name: "SOL", symbol: "SOL", decimals: 9 }, chainId: "solana-devnet", defaultRpcUrls: ["https://api.devnet.solana.com"], defaultWsUrls: ["wss://api.devnet.solana.com"], explorerUrl: "https://solscan.io/?cluster=devnet", available: true, avgBlockTimeSeconds: 0.4, consensus: "PoH + PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "solana" },
  { id: "solana-testnet", name: "Solana Testnet", family: "solana", network: "testnet", nativeCurrency: { name: "SOL", symbol: "SOL", decimals: 9 }, chainId: "solana-testnet", defaultRpcUrls: ["https://api.testnet.solana.com"], defaultWsUrls: [], explorerUrl: "https://solscan.io/?cluster=testnet", available: true, avgBlockTimeSeconds: 0.4, consensus: "PoH + PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "solana" },
  { id: "polygon", name: "Polygon PoS", family: "evm", network: "mainnet", nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 }, chainId: 137, defaultRpcUrls: ["https://polygon-rpc.com"], defaultWsUrls: [], explorerUrl: "https://polygonscan.com", available: true, avgBlockTimeSeconds: 2, consensus: "Proof of Stake", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "polygon" },
  { id: "polygon-amoy", name: "Polygon Amoy", family: "evm", network: "testnet", nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 }, chainId: 80002, defaultRpcUrls: ["https://rpc-amoy.polygon.technology"], defaultWsUrls: [], explorerUrl: "https://amoy.polygonscan.com", available: true, avgBlockTimeSeconds: 2, consensus: "Proof of Stake", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "polygon" },
  { id: "bsc", name: "BNB Smart Chain", family: "evm", network: "mainnet", nativeCurrency: { name: "BNB", symbol: "BNB", decimals: 18 }, chainId: 56, defaultRpcUrls: ["https://bsc-dataseed.binance.org"], defaultWsUrls: [], explorerUrl: "https://bscscan.com", available: true, avgBlockTimeSeconds: 3, consensus: "PoSA", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "bsc" },
  { id: "bsc-testnet", name: "BSC Testnet", family: "evm", network: "testnet", nativeCurrency: { name: "tBNB", symbol: "tBNB", decimals: 18 }, chainId: 97, defaultRpcUrls: ["https://data-seed-prebsc-1-s1.binance.org:8545"], defaultWsUrls: [], explorerUrl: "https://testnet.bscscan.com", available: true, avgBlockTimeSeconds: 3, consensus: "PoSA", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "bsc" },
  { id: "avalanche", name: "Avalanche C-Chain", family: "evm", network: "mainnet", nativeCurrency: { name: "AVAX", symbol: "AVAX", decimals: 18 }, chainId: 43114, defaultRpcUrls: ["https://api.avax.network/ext/bc/C/rpc"], defaultWsUrls: [], explorerUrl: "https://snowtrace.io", available: true, avgBlockTimeSeconds: 2, consensus: "Snowball", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "avalanche" },
  { id: "avalanche-fuji", name: "Avalanche Fuji", family: "evm", network: "testnet", nativeCurrency: { name: "AVAX", symbol: "AVAX", decimals: 18 }, chainId: 43113, defaultRpcUrls: ["https://api.avax-test.network/ext/bc/C/rpc"], defaultWsUrls: [], explorerUrl: "https://testnet.snowtrace.io", available: true, avgBlockTimeSeconds: 2, consensus: "Snowball", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "avalanche" },
  { id: "arbitrum", name: "Arbitrum One", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 42161, defaultRpcUrls: ["https://arb1.arbitrum.io/rpc"], defaultWsUrls: [], explorerUrl: "https://arbiscan.io", available: true, avgBlockTimeSeconds: 0.25, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "arbitrum" },
  { id: "arbitrum-sepolia", name: "Arbitrum Sepolia", family: "evm", network: "testnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 421614, defaultRpcUrls: ["https://sepolia-rollup.arbitrum.io/rpc"], defaultWsUrls: [], explorerUrl: "https://sepolia.arbiscan.io", available: true, avgBlockTimeSeconds: 0.25, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "arbitrum" },
  { id: "optimism", name: "Optimism", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 10, defaultRpcUrls: ["https://mainnet.optimism.io"], defaultWsUrls: [], explorerUrl: "https://optimistic.etherscan.io", available: true, avgBlockTimeSeconds: 2, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "optimism" },
  { id: "optimism-sepolia", name: "Optimism Sepolia", family: "evm", network: "testnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 11155420, defaultRpcUrls: ["https://sepolia.optimism.io"], defaultWsUrls: [], explorerUrl: "https://sepolia-optimism.etherscan.io", available: true, avgBlockTimeSeconds: 2, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "optimism" },
  { id: "base", name: "Base", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 8453, defaultRpcUrls: ["https://mainnet.base.org"], defaultWsUrls: [], explorerUrl: "https://basescan.org", available: true, avgBlockTimeSeconds: 2, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "base" },
  { id: "base-sepolia", name: "Base Sepolia", family: "evm", network: "testnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 84532, defaultRpcUrls: ["https://sepolia.base.org"], defaultWsUrls: [], explorerUrl: "https://sepolia.basescan.org", available: true, avgBlockTimeSeconds: 2, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "base" },
  { id: "near", name: "NEAR Mainnet", family: "near", network: "mainnet", nativeCurrency: { name: "NEAR", symbol: "NEAR", decimals: 24 }, chainId: "near-mainnet", defaultRpcUrls: ["https://rpc.mainnet.near.org"], defaultWsUrls: [], explorerUrl: "https://nearblocks.io", available: true, avgBlockTimeSeconds: 1.2, consensus: "Nightshade PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "near" },
  { id: "near-testnet", name: "NEAR Testnet", family: "near", network: "testnet", nativeCurrency: { name: "NEAR", symbol: "NEAR", decimals: 24 }, chainId: "near-testnet", defaultRpcUrls: ["https://rpc.testnet.near.org"], defaultWsUrls: [], explorerUrl: "https://testnet.nearblocks.io", available: true, avgBlockTimeSeconds: 1.2, consensus: "Nightshade PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "near" },
  { id: "cosmos", name: "Cosmos Hub", family: "cosmos", network: "mainnet", nativeCurrency: { name: "ATOM", symbol: "ATOM", decimals: 6 }, chainId: "cosmoshub-4", defaultRpcUrls: ["https://cosmos-rpc.publicnode.com:443"], defaultWsUrls: [], explorerUrl: "https://www.mintscan.io/cosmos", available: true, avgBlockTimeSeconds: 6, consensus: "Tendermint BFT", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "cosmos" },
  { id: "osmosis", name: "Osmosis", family: "cosmos", network: "mainnet", nativeCurrency: { name: "OSMO", symbol: "OSMO", decimals: 6 }, chainId: "osmosis-1", defaultRpcUrls: ["https://osmosis-rpc.publicnode.com:443"], defaultWsUrls: [], explorerUrl: "https://www.mintscan.io/osmosis", available: true, avgBlockTimeSeconds: 6, consensus: "Tendermint BFT", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: true, icon: "osmosis" },
  { id: "polkadot", name: "Polkadot", family: "substrate", network: "mainnet", nativeCurrency: { name: "DOT", symbol: "DOT", decimals: 10 }, chainId: "polkadot", defaultRpcUrls: ["https://rpc.polkadot.io"], defaultWsUrls: ["wss://rpc.polkadot.io"], explorerUrl: "https://polkadot.subscan.io", available: true, avgBlockTimeSeconds: 6, consensus: "NPoS (GRANDPA)", signatureAlgorithm: "sr25519", hashAlgorithm: "blake2b", gpuAccelerated: false, icon: "polkadot" },
  { id: "kusama", name: "Kusama", family: "substrate", network: "mainnet", nativeCurrency: { name: "KSM", symbol: "KSM", decimals: 12 }, chainId: "kusama", defaultRpcUrls: ["https://kusama-rpc.dwellir.com"], defaultWsUrls: [], explorerUrl: "https://kusama.subscan.io", available: true, avgBlockTimeSeconds: 6, consensus: "NPoS (GRANDPA)", signatureAlgorithm: "sr25519", hashAlgorithm: "blake2b", gpuAccelerated: false, icon: "kusama" },
  { id: "fantom", name: "Fantom Opera", family: "evm", network: "mainnet", nativeCurrency: { name: "FTM", symbol: "FTM", decimals: 18 }, chainId: 250, defaultRpcUrls: ["https://rpc.ftm.tools"], defaultWsUrls: [], explorerUrl: "https://ftmscan.com", available: true, avgBlockTimeSeconds: 1, consensus: "Lachesis aBFT", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "fantom" },
  { id: "zksync", name: "zkSync Era", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 324, defaultRpcUrls: ["https://mainnet.era.zksync.io"], defaultWsUrls: [], explorerUrl: "https://explorer.zksync.io", available: true, avgBlockTimeSeconds: 1, consensus: "ZK Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "zksync" },
  { id: "linea", name: "Linea", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 59144, defaultRpcUrls: ["https://rpc.linea.build"], defaultWsUrls: [], explorerUrl: "https://lineascan.build", available: true, avgBlockTimeSeconds: 2, consensus: "ZK Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "linea" },
  { id: "scroll", name: "Scroll", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 534352, defaultRpcUrls: ["https://rpc.scroll.io"], defaultWsUrls: [], explorerUrl: "https://scrollscan.com", available: true, avgBlockTimeSeconds: 3, consensus: "ZK Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "scroll" },
  { id: "celo", name: "Celo", family: "evm", network: "mainnet", nativeCurrency: { name: "CELO", symbol: "CELO", decimals: 18 }, chainId: 42220, defaultRpcUrls: ["https://forno.celo.org"], defaultWsUrls: [], explorerUrl: "https://celoscan.io", available: true, avgBlockTimeSeconds: 5, consensus: "PBFT PoS", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "celo" },
  { id: "gnosis", name: "Gnosis Chain", family: "evm", network: "mainnet", nativeCurrency: { name: "xDAI", symbol: "xDAI", decimals: 18 }, chainId: 100, defaultRpcUrls: ["https://rpc.gnosischain.com"], defaultWsUrls: [], explorerUrl: "https://gnosisscan.io", available: true, avgBlockTimeSeconds: 5, consensus: "AuRa PoS", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "gnosis" },
  { id: "moonbeam", name: "Moonbeam", family: "evm", network: "mainnet", nativeCurrency: { name: "GLMR", symbol: "GLMR", decimals: 18 }, chainId: 1284, defaultRpcUrls: ["https://rpc.api.moonbeam.network"], defaultWsUrls: [], explorerUrl: "https://moonbeam.moonscan.io", available: true, avgBlockTimeSeconds: 12, consensus: "Parachain PoS", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "moonbeam" },
  { id: "manta", name: "Manta Pacific", family: "evm", network: "mainnet", nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 }, chainId: 169, defaultRpcUrls: ["https://pacific-rpc.manta.network/http"], defaultWsUrls: [], explorerUrl: "https://pacific-explorer.manta.network", available: true, avgBlockTimeSeconds: 2, consensus: "Optimistic Rollup", signatureAlgorithm: "secp256k1", hashAlgorithm: "keccak256", gpuAccelerated: true, icon: "manta" },
  { id: "sui", name: "Sui", family: "other", network: "mainnet", nativeCurrency: { name: "SUI", symbol: "SUI", decimals: 9 }, chainId: "sui-mainnet", defaultRpcUrls: ["https://fullnode.mainnet.sui.io:443"], defaultWsUrls: [], explorerUrl: "https://suiscan.xyz", available: true, avgBlockTimeSeconds: 0.5, consensus: "Narwhal/Bullshark", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "sui" },
  { id: "aptos", name: "Aptos", family: "other", network: "mainnet", nativeCurrency: { name: "APT", symbol: "APT", decimals: 8 }, chainId: "aptos-mainnet", defaultRpcUrls: ["https://fullnode.mainnet.aptoslabs.com/v1"], defaultWsUrls: [], explorerUrl: "https://aptoscan.com", available: true, avgBlockTimeSeconds: 1, consensus: "AptosBFT", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "aptos" },
  { id: "ton", name: "TON", family: "other", network: "mainnet", nativeCurrency: { name: "TON", symbol: "TON", decimals: 9 }, chainId: "ton-mainnet", defaultRpcUrls: ["https://toncenter.com/api/v2/jsonRPC"], defaultWsUrls: [], explorerUrl: "https://tonviewer.com", available: true, avgBlockTimeSeconds: 5, consensus: "BFT PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "ton" },
  { id: "cardano", name: "Cardano", family: "other", network: "mainnet", nativeCurrency: { name: "ADA", symbol: "ADA", decimals: 6 }, chainId: "cardano-mainnet", defaultRpcUrls: ["https://cardano-mainnet.blockfrost.io/api/v0"], defaultWsUrls: [], explorerUrl: "https://cardanoscan.io", available: true, avgBlockTimeSeconds: 20, consensus: "Ouroboros PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "blake2b", gpuAccelerated: false, icon: "cardano" },
  { id: "tezos", name: "Tezos", family: "other", network: "mainnet", nativeCurrency: { name: "XTZ", symbol: "XTZ", decimals: 6 }, chainId: "tezos-mainnet", defaultRpcUrls: ["https://mainnet.api.tez.ie"], defaultWsUrls: [], explorerUrl: "https://tzstats.com", available: true, avgBlockTimeSeconds: 15, consensus: "LPoS", signatureAlgorithm: "ed25519", hashAlgorithm: "blake2b", gpuAccelerated: false, icon: "tezos" },
  { id: "algorand", name: "Algorand", family: "other", network: "mainnet", nativeCurrency: { name: "ALGO", symbol: "ALGO", decimals: 6 }, chainId: "algorand-mainnet", defaultRpcUrls: ["https://mainnet-api.algonode.cloud"], defaultWsUrls: [], explorerUrl: "https://algoexplorer.io", available: true, avgBlockTimeSeconds: 3.3, consensus: "Pure PoS", signatureAlgorithm: "ed25519", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "algorand" },
  { id: "xrp", name: "XRP Ledger", family: "other", network: "mainnet", nativeCurrency: { name: "XRP", symbol: "XRP", decimals: 6 }, chainId: "xrpl-mainnet", defaultRpcUrls: ["https://xrplcluster.com"], defaultWsUrls: [], explorerUrl: "https://xrpscan.com", available: true, avgBlockTimeSeconds: 4, consensus: "FBA", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "xrp" },
  { id: "flow", name: "Flow", family: "other", network: "mainnet", nativeCurrency: { name: "FLOW", symbol: "FLOW", decimals: 8 }, chainId: "flow-mainnet", defaultRpcUrls: ["https://rest-mainnet.onflow.org"], defaultWsUrls: [], explorerUrl: "https://www.flowdiver.io", available: true, avgBlockTimeSeconds: 2.5, consensus: "HotStuff PoS", signatureAlgorithm: "secp256k1", hashAlgorithm: "sha256", gpuAccelerated: false, icon: "flow" },
];

// ─── Test Profiles ─────────────────────────────────────────────────────────────

const TEST_PROFILES = [
  { id: "latency", name: "Latency Test", description: "1K RPC requests — p50/p90/p99", category: "performance", duration: 60 },
  { id: "throughput", name: "Throughput Test", description: "Sustained 500 TPS for 60s", category: "performance", duration: 120 },
  { id: "reorg-simulation", name: "Reorg Simulation", description: "1-3 block reorgs, verify events", category: "reliability", duration: 30 },
  { id: "edge-cases", name: "Edge Cases", description: "Malformed tx, sig errors, nonce mismatches", category: "functional", duration: 30 },
  { id: "validator-health", name: "Validator Health", description: "Validator uptime, stake, liveness", category: "functional", duration: 30 },
  { id: "gpu-benchmark", name: "GPU Benchmark", description: "SHA-256, Keccak, secp256k1, Ed25519 GPU kernels", category: "performance", duration: 60 },
  { id: "pool-performance", name: "Pool Performance", description: "Pool connectivity, hashrate, rewards", category: "performance", duration: 45 },
  { id: "full-suite", name: "Full Suite", description: "All tests sequentially", category: "functional", duration: 300 },
];

// ─── Family Colors & Icons ──────────────────────────────────────────────────────

const FAMILY_COLORS: Record<string, string> = {
  evm: "#627eea",
  bitcoin: "#f7931a",
  solana: "#9945ff",
  cosmos: "#2e3148",
  substrate: "#e6007a",
  near: "#00c1de",
  other: "#666",
};

const FAMILY_LABELS: Record<string, string> = {
  evm: "EVM",
  bitcoin: "Bitcoin",
  solana: "Solana",
  cosmos: "Cosmos",
  substrate: "Substrate",
  near: "NEAR",
  other: "Other L1s",
};

const STATUS_COLORS: Record<string, string> = {
  connected: "#00d4aa",
  connecting: "#ffaa00",
  syncing: "#4488ff",
  degraded: "#ffaa00",
  disconnected: "#666",
  error: "#ff4444",
  completed: "#00d4aa",
  running: "#4488ff",
  failed: "#ff4444",
  queued: "#666",
};

// ─── Sub-Components ─────────────────────────────────────────────────────────────

function Badge({ children, color }: { children: React.ReactNode; color: string }) {
  return (
    <span
      style={{
        background: `${color}22`,
        color,
        border: `1px solid ${color}44`,
        padding: "2px 8px",
        borderRadius: 4,
        fontSize: 11,
        fontFamily: "monospace",
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.05em",
      }}
    >
      {children}
    </span>
  );
}

function StatCard({ label, value, sub, color = "#e0e0e0" }: { label: string; value: string | number; sub?: string; color?: string }) {
  return (
    <div
      style={{
        background: "#111113",
        border: "1px solid #2a2a2e",
        borderRadius: 6,
        padding: "12px 16px",
        flex: "1 1 120px",
        minWidth: 120,
      }}
    >
      <div style={{ fontSize: 11, color: "#8a8a8e", marginBottom: 4, textTransform: "uppercase", letterSpacing: "0.05em" }}>{label}</div>
      <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "monospace", color }}>{value}</div>
      {sub && <div style={{ fontSize: 11, color: "#555", marginTop: 2 }}>{sub}</div>}
    </div>
  );
}

function TabBar({ tabs, active, onSelect }: { tabs: { id: string; label: string; count?: number }[]; active: string; onSelect: (id: string) => void }) {
  return (
    <div style={{ display: "flex", gap: 0, borderBottom: "1px solid #2a2a2e", marginBottom: 16 }}>
      {tabs.map((t) => (
        <button
          key={t.id}
          onClick={() => onSelect(t.id)}
          style={{
            background: "none",
            border: "none",
            borderBottom: active === t.id ? "2px solid #ff6b35" : "2px solid transparent",
            color: active === t.id ? "#e0e0e0" : "#8a8a8e",
            padding: "8px 16px",
            fontSize: 13,
            fontWeight: active === t.id ? 600 : 400,
            cursor: "pointer",
            fontFamily: "inherit",
            transition: "all 0.15s",
          }}
        >
          {t.label}
          {t.count !== undefined && (
            <span style={{ marginLeft: 6, fontSize: 11, opacity: 0.6 }}>({t.count})</span>
          )}
        </button>
      ))}
    </div>
  );
}

// ─── Networks Tab ─────────────────────────────────────────────────────────

function NetworksTab({ onConnect }: { onConnect: (chain: ChainDescriptor) => void }) {
  const [familyFilter, setFamilyFilter] = useState<string>("all");
  const [networkFilter, setNetworkFilter] = useState<string>("all");
  const [search, setSearch] = useState("");

  const families = useMemo(() => {
    const counts: Record<string, number> = {};
    CHAINS.forEach((c) => (counts[c.family] = (counts[c.family] || 0) + 1));
    return counts;
  }, []);

  const filtered = useMemo(() => {
    return CHAINS.filter((c) => {
      if (familyFilter !== "all" && c.family !== familyFilter) return false;
      if (networkFilter !== "all" && c.network !== networkFilter) return false;
      if (search && !c.name.toLowerCase().includes(search.toLowerCase()) && !c.id.includes(search.toLowerCase())) return false;
      return true;
    });
  }, [familyFilter, networkFilter, search]);

  return (
    <div>
      {/* Summary cards */}
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 16 }}>
        <StatCard label="Total Chains" value={CHAINS.length} color="#ff6b35" />
        <StatCard label="Mainnets" value={CHAINS.filter((c) => c.network === "mainnet").length} color="#00d4aa" />
        <StatCard label="Testnets" value={CHAINS.filter((c) => c.network !== "mainnet").length} color="#ffaa00" />
        <StatCard label="GPU Accelerated" value={CHAINS.filter((c) => c.gpuAccelerated).length} color="#9945ff" />
        <StatCard label="Families" value={Object.keys(families).length} color="#4488ff" />
      </div>

      {/* Filters */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
        <input
          type="text"
          placeholder="Search chains..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{
            background: "#111113",
            border: "1px solid #2a2a2e",
            borderRadius: 4,
            padding: "6px 12px",
            color: "#e0e0e0",
            fontSize: 13,
            width: 200,
            fontFamily: "inherit",
          }}
        />
        <select
          value={familyFilter}
          onChange={(e) => setFamilyFilter(e.target.value)}
          style={{ background: "#111113", border: "1px solid #2a2a2e", borderRadius: 4, padding: "6px 8px", color: "#e0e0e0", fontSize: 12 }}
        >
          <option value="all">All Families</option>
          {Object.entries(families).map(([f, count]) => (
            <option key={f} value={f}>{FAMILY_LABELS[f] || f} ({count})</option>
          ))}
        </select>
        <select
          value={networkFilter}
          onChange={(e) => setNetworkFilter(e.target.value)}
          style={{ background: "#111113", border: "1px solid #2a2a2e", borderRadius: 4, padding: "6px 8px", color: "#e0e0e0", fontSize: 12 }}
        >
          <option value="all">All Networks</option>
          <option value="mainnet">Mainnet</option>
          <option value="testnet">Testnet</option>
          <option value="devnet">Devnet</option>
        </select>
        <span style={{ fontSize: 12, color: "#8a8a8e" }}>{filtered.length} chains</span>
      </div>

      {/* Chain Grid */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 8 }}>
        {filtered.map((chain) => (
          <div
            key={chain.id}
            style={{
              background: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: 6,
              padding: "12px 14px",
              display: "flex",
              flexDirection: "column",
              gap: 6,
              transition: "border-color 0.15s",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.borderColor = FAMILY_COLORS[chain.family] || "#444")}
            onMouseLeave={(e) => (e.currentTarget.style.borderColor = "#2a2a2e")}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div
                  style={{
                    width: 28,
                    height: 28,
                    borderRadius: 14,
                    background: `${FAMILY_COLORS[chain.family]}22`,
                    border: `1px solid ${FAMILY_COLORS[chain.family]}44`,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    fontSize: 11,
                    fontWeight: 700,
                    color: FAMILY_COLORS[chain.family],
                  }}
                >
                  {chain.nativeCurrency.symbol.slice(0, 3)}
                </div>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>{chain.name}</div>
                  <div style={{ fontSize: 11, color: "#8a8a8e", fontFamily: "monospace" }}>
                    {chain.nativeCurrency.symbol} · {chain.consensus}
                  </div>
                </div>
              </div>
              <Badge color={chain.network === "mainnet" ? "#00d4aa" : "#ffaa00"}>
                {chain.network}
              </Badge>
            </div>

            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", fontSize: 10 }}>
              <Badge color={FAMILY_COLORS[chain.family]}>{chain.family}</Badge>
              <Badge color="#8a8a8e">{chain.signatureAlgorithm}</Badge>
              <Badge color="#8a8a8e">{chain.hashAlgorithm}</Badge>
              {chain.gpuAccelerated && <Badge color="#9945ff">GPU</Badge>}
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 4 }}>
              <span style={{ fontSize: 11, color: "#555", fontFamily: "monospace" }}>
                ~{chain.avgBlockTimeSeconds}s blocks · ID: {String(chain.chainId).slice(0, 12)}
              </span>
              <button
                onClick={() => onConnect(chain)}
                style={{
                  background: "#ff6b35",
                  color: "#fff",
                  border: "none",
                  borderRadius: 4,
                  padding: "4px 12px",
                  fontSize: 11,
                  fontWeight: 600,
                  cursor: "pointer",
                  transition: "opacity 0.15s",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.85")}
                onMouseLeave={(e) => (e.currentTarget.style.opacity = "1")}
              >
                Connect
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Connectors Tab ────────────────────────────────────────────

function ConnectorsTab({
  connectors,
  onTest,
  onDisconnect,
  onRefresh,
}: {
  connectors: ConnectorInstance[];
  onTest: (connId: string) => void;
  onDisconnect: (connId: string) => void;
  onRefresh: (connId: string) => void;
}) {
  if (connectors.length === 0) {
    return (
      <div style={{ textAlign: "center", padding: "60px 0", color: "#555" }}>
        <div style={{ fontSize: 32, marginBottom: 12 }}>&#x26D3;</div>
        <div style={{ fontSize: 14, marginBottom: 8 }}>No active connectors</div>
        <div style={{ fontSize: 12 }}>Go to Networks tab and click Connect on any chain</div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 16 }}>
        <StatCard label="Active" value={connectors.filter((c) => c.status === "connected").length} color="#00d4aa" />
        <StatCard label="Total" value={connectors.length} />
        <StatCard
          label="Avg Latency"
          value={`${Math.round(connectors.reduce((a, c) => a + c.metrics.latencyMs, 0) / connectors.length)}ms`}
          color="#4488ff"
        />
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {connectors.map((conn) => (
          <div
            key={conn.id}
            style={{
              background: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: 6,
              padding: "14px 16px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <div
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: 4,
                    background: STATUS_COLORS[conn.status] || "#666",
                    boxShadow: `0 0 6px ${STATUS_COLORS[conn.status] || "#666"}66`,
                  }}
                />
                <span style={{ fontWeight: 600, fontSize: 14 }}>{conn.chain.name}</span>
                <span style={{ fontSize: 11, fontFamily: "monospace", color: "#8a8a8e" }}>{conn.id}</span>
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <Badge color={STATUS_COLORS[conn.status] || "#666"}>{conn.status}</Badge>
                <Badge color={FAMILY_COLORS[conn.chain.family]}>{conn.chain.family}</Badge>
              </div>
            </div>

            {/* Metrics grid */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(120px, 1fr))", gap: 8, marginBottom: 10 }}>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>Block: </span>
                <span style={{ fontFamily: "monospace", color: "#00d4aa" }}>{conn.metrics.blockHeight.toLocaleString()}</span>
              </div>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>TPS: </span>
                <span style={{ fontFamily: "monospace" }}>{conn.metrics.tps.toLocaleString()}</span>
              </div>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>Latency: </span>
                <span style={{ fontFamily: "monospace", color: conn.metrics.latencyMs < 200 ? "#00d4aa" : "#ffaa00" }}>
                  {conn.metrics.latencyMs}ms
                </span>
              </div>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>Peers: </span>
                <span style={{ fontFamily: "monospace" }}>{conn.metrics.peerCount}</span>
              </div>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>Requests: </span>
                <span style={{ fontFamily: "monospace" }}>{conn.metrics.totalRequests.toLocaleString()}</span>
              </div>
              <div style={{ fontSize: 11 }}>
                <span style={{ color: "#8a8a8e" }}>Uptime: </span>
                <span style={{ fontFamily: "monospace" }}>{conn.metrics.uptimeSeconds}s</span>
              </div>
              {conn.metrics.gasPrice && (
                <div style={{ fontSize: 11 }}>
                  <span style={{ color: "#8a8a8e" }}>Gas: </span>
                  <span style={{ fontFamily: "monospace" }}>{conn.metrics.gasPrice}</span>
                </div>
              )}
              {conn.metrics.hashRate && (
                <div style={{ fontSize: 11 }}>
                  <span style={{ color: "#8a8a8e" }}>Hash Rate: </span>
                  <span style={{ fontFamily: "monospace" }}>{conn.metrics.hashRate}</span>
                </div>
              )}
            </div>

            {conn.error && (
              <div style={{ fontSize: 11, color: "#ff4444", marginBottom: 8, fontFamily: "monospace" }}>
                Error: {conn.error}
              </div>
            )}

            <div style={{ display: "flex", gap: 6 }}>
              <button
                onClick={() => onRefresh(conn.id)}
                style={{ background: "#1a1a1d", border: "1px solid #2a2a2e", borderRadius: 4, padding: "4px 10px", color: "#8a8a8e", fontSize: 11, cursor: "pointer" }}
              >
                Refresh
              </button>
              <button
                onClick={() => onTest(conn.id)}
                style={{ background: "#4488ff22", border: "1px solid #4488ff44", borderRadius: 4, padding: "4px 10px", color: "#4488ff", fontSize: 11, fontWeight: 600, cursor: "pointer" }}
              >
                Run Tests
              </button>
              {conn.chain.explorerUrl && (
                <a
                  href={conn.chain.explorerUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ background: "#1a1a1d", border: "1px solid #2a2a2e", borderRadius: 4, padding: "4px 10px", color: "#8a8a8e", fontSize: 11, textDecoration: "none", cursor: "pointer" }}
                >
                  Explorer
                </a>
              )}
              <button
                onClick={() => onDisconnect(conn.id)}
                style={{ background: "#ff444422", border: "1px solid #ff444444", borderRadius: 4, padding: "4px 10px", color: "#ff4444", fontSize: 11, cursor: "pointer", marginLeft: "auto" }}
              >
                Disconnect
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Test Bench Tab ────────────────────────────────────────────

function TestBenchTab({
  connectors,
  testRuns,
  onRunTest,
}: {
  connectors: ConnectorInstance[];
  testRuns: TestRun[];
  onRunTest: (connectorId: string, profileId: string) => void;
}) {
  const [selectedConnector, setSelectedConnector] = useState("");
  const [selectedProfile, setSelectedProfile] = useState("latency");
  const [customEndpoint, setCustomEndpoint] = useState("");

  return (
    <div>
      {/* Test bench controls */}
      <div style={{ background: "#111113", border: "1px solid #2a2a2e", borderRadius: 6, padding: 16, marginBottom: 16 }}>
        <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Run Test</div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
          <div>
            <label style={{ fontSize: 11, color: "#8a8a8e", display: "block", marginBottom: 4 }}>Connector</label>
            <select
              value={selectedConnector}
              onChange={(e) => setSelectedConnector(e.target.value)}
              style={{ background: "#0a0a0b", border: "1px solid #2a2a2e", borderRadius: 4, padding: "6px 8px", color: "#e0e0e0", fontSize: 12, minWidth: 200 }}
            >
              <option value="">Select connector...</option>
              {connectors.map((c) => (
                <option key={c.id} value={c.id}>{c.chain.name} ({c.id})</option>
              ))}
            </select>
          </div>

          <div>
            <label style={{ fontSize: 11, color: "#8a8a8e", display: "block", marginBottom: 4 }}>Test Profile</label>
            <select
              value={selectedProfile}
              onChange={(e) => setSelectedProfile(e.target.value)}
              style={{ background: "#0a0a0b", border: "1px solid #2a2a2e", borderRadius: 4, padding: "6px 8px", color: "#e0e0e0", fontSize: 12, minWidth: 200 }}
            >
              {TEST_PROFILES.map((p) => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          </div>

          <div style={{ display: "flex", alignItems: "flex-end" }}>
            <button
              onClick={() => selectedConnector && onRunTest(selectedConnector, selectedProfile)}
              disabled={!selectedConnector}
              style={{
                background: selectedConnector ? "#ff6b35" : "#333",
                color: "#fff",
                border: "none",
                borderRadius: 4,
                padding: "6px 20px",
                fontSize: 12,
                fontWeight: 600,
                cursor: selectedConnector ? "pointer" : "not-allowed",
                opacity: selectedConnector ? 1 : 0.5,
              }}
            >
              Run Test
            </button>
          </div>
        </div>

        {/* Custom endpoint for testing your own chain */}
        <div style={{ borderTop: "1px solid #2a2a2e", paddingTop: 12, marginTop: 4 }}>
          <div style={{ fontSize: 12, fontWeight: 600, color: "#ffaa00", marginBottom: 8 }}>
            Test Your Own Chain / Pool / GPU Farm
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <input
              type="text"
              placeholder="RPC endpoint (e.g. https://your-node:8545)"
              value={customEndpoint}
              onChange={(e) => setCustomEndpoint(e.target.value)}
              style={{
                background: "#0a0a0b",
                border: "1px solid #2a2a2e",
                borderRadius: 4,
                padding: "6px 12px",
                color: "#e0e0e0",
                fontSize: 12,
                fontFamily: "monospace",
                flex: 1,
              }}
            />
            <button
              style={{
                background: "#ffaa0022",
                border: "1px solid #ffaa0044",
                borderRadius: 4,
                padding: "6px 14px",
                color: "#ffaa00",
                fontSize: 11,
                fontWeight: 600,
                cursor: "pointer",
                whiteSpace: "nowrap",
              }}
            >
              Add Custom
            </button>
          </div>
          <div style={{ fontSize: 10, color: "#555", marginTop: 4 }}>
            Connect to your own node, validator, mining pool, or GPU farm for testing
          </div>
        </div>
      </div>

      {/* Test profile cards */}
      <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Available Test Profiles</div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 8, marginBottom: 16 }}>
        {TEST_PROFILES.map((p) => (
          <div
            key={p.id}
            style={{
              background: selectedProfile === p.id ? "#ff6b3511" : "#111113",
              border: `1px solid ${selectedProfile === p.id ? "#ff6b3544" : "#2a2a2e"}`,
              borderRadius: 6,
              padding: "10px 14px",
              cursor: "pointer",
            }}
            onClick={() => setSelectedProfile(p.id)}
          >
            <div style={{ fontWeight: 600, fontSize: 12, marginBottom: 4 }}>{p.name}</div>
            <div style={{ fontSize: 11, color: "#8a8a8e", marginBottom: 6 }}>{p.description}</div>
            <div style={{ display: "flex", gap: 6 }}>
              <Badge color="#4488ff">{p.category}</Badge>
              <Badge color="#8a8a8e">~{p.duration}s</Badge>
            </div>
          </div>
        ))}
      </div>

      {/* Recent test runs */}
      {testRuns.length > 0 && (
        <>
          <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Recent Activity</div>
          {testRuns.slice(-5).reverse().map((run) => (
            <div
              key={run.id}
              style={{ background: "#111113", border: "1px solid #2a2a2e", borderRadius: 6, padding: "10px 14px", marginBottom: 6 }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <Badge color={STATUS_COLORS[run.status] || "#666"}>{run.status}</Badge>
                  <span style={{ fontSize: 12, fontWeight: 600 }}>{run.profileId}</span>
                  <span style={{ fontSize: 11, color: "#555", fontFamily: "monospace" }}>{run.connectorId}</span>
                </div>
                {run.summary && (
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <span style={{ fontSize: 18, fontWeight: 700, color: run.summary.grade.startsWith("A") ? "#00d4aa" : run.summary.grade === "B" ? "#4488ff" : "#ffaa00" }}>
                      {run.summary.grade}
                    </span>
                    <span style={{ fontSize: 11, color: "#8a8a8e" }}>
                      {run.summary.passed}/{run.summary.totalTests} passed
                    </span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

// ─── Results Tab ───────────────────────────────────────────────

function ResultsTab({ testRuns }: { testRuns: TestRun[] }) {
  const [expanded, setExpanded] = useState<string | null>(null);

  if (testRuns.length === 0) {
    return (
      <div style={{ textAlign: "center", padding: "60px 0", color: "#555" }}>
        <div style={{ fontSize: 32, marginBottom: 12 }}>&#x1F4CA;</div>
        <div style={{ fontSize: 14 }}>No test results yet</div>
        <div style={{ fontSize: 12, marginTop: 4 }}>Run a test from the Test Bench tab</div>
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 16 }}>
        <StatCard label="Total Runs" value={testRuns.length} />
        <StatCard label="Passed" value={testRuns.filter((r) => r.status === "completed").length} color="#00d4aa" />
        <StatCard label="Failed" value={testRuns.filter((r) => r.status === "failed").length} color="#ff4444" />
        <StatCard
          label="Avg Score"
          value={Math.round(testRuns.filter((r) => r.summary).reduce((a, r) => a + (r.summary?.overallScore ?? 0), 0) / Math.max(testRuns.filter((r) => r.summary).length, 1))}
          color="#4488ff"
        />
      </div>

      {testRuns.slice().reverse().map((run) => (
        <div
          key={run.id}
          style={{ background: "#111113", border: "1px solid #2a2a2e", borderRadius: 6, marginBottom: 8, overflow: "hidden" }}
        >
          <div
            style={{ padding: "12px 16px", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center" }}
            onClick={() => setExpanded(expanded === run.id ? null : run.id)}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 11, fontFamily: "monospace", color: "#555" }}>{run.id}</span>
              <Badge color={STATUS_COLORS[run.status]}>{run.status}</Badge>
              <span style={{ fontSize: 13, fontWeight: 600 }}>{run.profileId}</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              {run.summary && (
                <>
                  <span
                    style={{
                      fontSize: 22,
                      fontWeight: 800,
                      fontFamily: "monospace",
                      color: run.summary.grade.startsWith("A") ? "#00d4aa" : run.summary.grade === "B" ? "#4488ff" : "#ffaa00",
                    }}
                  >
                    {run.summary.grade}
                  </span>
                  <span style={{ fontSize: 11, color: "#8a8a8e" }}>
                    {run.summary.passed}/{run.summary.totalTests} · {run.summary.totalDurationMs}ms
                  </span>
                </>
              )}
              <span style={{ color: "#555", transform: expanded === run.id ? "rotate(180deg)" : "none", transition: "transform 0.2s" }}>
                &#9662;
              </span>
            </div>
          </div>

          {expanded === run.id && (
            <div style={{ borderTop: "1px solid #2a2a2e", padding: "12px 16px" }}>
              {run.results.map((r) => (
                <div
                  key={r.testId}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "6px 0",
                    borderBottom: "1px solid #1a1a1d",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ color: r.passed ? "#00d4aa" : "#ff4444", fontWeight: 700, fontSize: 12 }}>
                      {r.passed ? "PASS" : "FAIL"}
                    </span>
                    <span style={{ fontSize: 12 }}>{r.testName}</span>
                  </div>
                  <div style={{ display: "flex", gap: 12, fontSize: 11, fontFamily: "monospace", color: "#8a8a8e" }}>
                    {r.metrics.p50Ms !== undefined && <span>p50: {r.metrics.p50Ms}ms</span>}
                    {r.metrics.p90Ms !== undefined && <span>p90: {r.metrics.p90Ms}ms</span>}
                    {r.metrics.requestsPerSecond !== undefined && <span>{r.metrics.requestsPerSecond} req/s</span>}
                    {r.metrics.gpuOpsPerSecond !== undefined && <span>{Number(r.metrics.gpuOpsPerSecond).toLocaleString()} ops/s</span>}
                    <span>{r.durationMs}ms</span>
                  </div>
                </div>
              ))}
              {run.error && (
                <div style={{ marginTop: 8, padding: 8, background: "#ff444411", borderRadius: 4, fontSize: 12, color: "#ff4444", fontFamily: "monospace" }}>
                  {run.error}
                </div>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

// ─── Billing Tab ───────────────────────────────────────────────

function BillingTab() {
  const plans = [
    { tier: "free", name: "Free", price: 0, connectors: 2, requests: 10_000, ws: 5, sla: 99, features: ["2 connectors", "10K req/mo", "5 WS connections", "Community support"] },
    { tier: "bronze", name: "Bronze", price: 29, connectors: 10, requests: 100_000, ws: 50, sla: 99.5, features: ["10 connectors", "100K req/mo", "50 WS connections", "Email support", "Benchmark reports"] },
    { tier: "silver", name: "Silver", price: 99, connectors: 50, requests: 1_000_000, ws: 500, sla: 99.9, features: ["50 connectors", "1M req/mo", "500 WS connections", "Priority support", "Custom tests", "SLA 99.9%"] },
    { tier: "gold", name: "Gold", price: 299, connectors: 200, requests: 10_000_000, ws: 5000, sla: 99.95, features: ["200 connectors", "10M req/mo", "5K WS", "Dedicated support", "GPU Access", "Custom SLA 99.95%"] },
    { tier: "enterprise", name: "Enterprise", price: -1, connectors: -1, requests: -1, ws: -1, sla: 99.99, features: ["Unlimited connectors", "Unlimited requests", "Premium GPU", "Dedicated infra", "On-chain billing", "SOC 2 Type II"] },
  ];

  return (
    <div>
      <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 4 }}>Pricing & Plans</div>
      <div style={{ fontSize: 12, color: "#8a8a8e", marginBottom: 16 }}>
        Choose a plan for your team. Upgrade any time.
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 10 }}>
        {plans.map((plan) => (
          <div
            key={plan.tier}
            style={{
              background: plan.tier === "gold" ? "#ff6b3508" : "#111113",
              border: `1px solid ${plan.tier === "gold" ? "#ff6b3544" : "#2a2a2e"}`,
              borderRadius: 6,
              padding: 16,
              display: "flex",
              flexDirection: "column",
            }}
          >
            <div style={{ fontWeight: 700, fontSize: 14, marginBottom: 4 }}>{plan.name}</div>
            <div style={{ fontSize: 24, fontWeight: 800, fontFamily: "monospace", color: "#ff6b35", marginBottom: 12 }}>
              {plan.price === -1 ? "Custom" : plan.price === 0 ? "Free" : `$${plan.price}`}
              {plan.price > 0 && <span style={{ fontSize: 12, color: "#8a8a8e", fontWeight: 400 }}>/mo</span>}
            </div>
            <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4, marginBottom: 12 }}>
              {plan.features.map((f) => (
                <div key={f} style={{ fontSize: 11, color: "#8a8a8e", display: "flex", alignItems: "center", gap: 6 }}>
                  <span style={{ color: "#00d4aa" }}>&#10003;</span> {f}
                </div>
              ))}
            </div>
            <button
              style={{
                background: plan.tier === "free" ? "#1a1a1d" : "#ff6b35",
                color: plan.tier === "free" ? "#8a8a8e" : "#fff",
                border: plan.tier === "free" ? "1px solid #2a2a2e" : "none",
                borderRadius: 4,
                padding: "8px 14px",
                fontSize: 12,
                fontWeight: 600,
                cursor: "pointer",
                width: "100%",
              }}
            >
              {plan.tier === "free" ? "Current Plan" : plan.tier === "enterprise" ? "Contact Sales" : "Upgrade"}
            </button>
          </div>
        ))}
      </div>

      {/* Usage */}
      <div style={{ marginTop: 24 }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Current Usage (Free Tier)</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <StatCard label="Requests" value="847 / 10K" sub="8.5% used" color="#00d4aa" />
          <StatCard label="Connectors" value="0 / 2" sub="0% used" color="#4488ff" />
          <StatCard label="WS Minutes" value="0 / 500" sub="0% used" color="#ffaa00" />
        </div>
      </div>
    </div>
  );
}

// ─── Main Panel ────────────────────────────────────────────────

export default function BlockchainConnectorPanel() {
  const [activeTab, setActiveTab] = useState("networks");
  const [connectors, setConnectors] = useState<ConnectorInstance[]>([]);
  const [testRuns, setTestRuns] = useState<TestRun[]>([]);
  const [connectingChain, setConnectingChain] = useState<string | null>(null);

  // Simulated connect (in production, calls ConnectorManager)
  const handleConnect = useCallback((chain: ChainDescriptor) => {
    setConnectingChain(chain.id);

    const newConn: ConnectorInstance = {
      id: `conn_${Math.random().toString(36).slice(2, 8)}`,
      chain,
      status: "connecting",
      metrics: {
        blockHeight: 0,
        tps: 0,
        latencyMs: 0,
        peerCount: 0,
        totalRequests: 0,
        totalErrors: 0,
        uptimeSeconds: 0,
      },
      createdAt: new Date().toISOString(),
    };

    setConnectors((prev) => [...prev, newConn]);
    setActiveTab("connectors");

    // Simulate connection + metric fetch
    setTimeout(() => {
      setConnectors((prev) =>
        prev.map((c) =>
          c.id === newConn.id
            ? {
                ...c,
                status: "connected" as ConnectorStatus,
                metrics: {
                  blockHeight: Math.floor(Math.random() * 20_000_000) + 1_000_000,
                  tps: Math.floor(Math.random() * 5000) + 10,
                  latencyMs: Math.floor(Math.random() * 150) + 30,
                  peerCount: Math.floor(Math.random() * 100) + 5,
                  totalRequests: 1,
                  totalErrors: 0,
                  uptimeSeconds: 1,
                  gasPrice: chain.family === "evm" ? `${Math.floor(Math.random() * 50) + 5} Gwei` : undefined,
                  hashRate: chain.family === "bitcoin" ? `${Math.floor(Math.random() * 500) + 100} EH/s` : undefined,
                },
              }
            : c,
        ),
      );
      setConnectingChain(null);
    }, 1200);
  }, []);

  const handleDisconnect = useCallback((connId: string) => {
    setConnectors((prev) => prev.filter((c) => c.id !== connId));
  }, []);

  const handleRefresh = useCallback((connId: string) => {
    setConnectors((prev) =>
      prev.map((c) =>
        c.id === connId
          ? {
              ...c,
              metrics: {
                ...c.metrics,
                blockHeight: c.metrics.blockHeight + Math.floor(Math.random() * 10) + 1,
                tps: Math.floor(Math.random() * 5000) + 10,
                latencyMs: Math.floor(Math.random() * 150) + 30,
                totalRequests: c.metrics.totalRequests + 1,
                uptimeSeconds: c.metrics.uptimeSeconds + Math.floor(Math.random() * 60),
              },
              updatedAt: new Date().toISOString(),
            }
          : c,
      ),
    );
  }, []);

  const handleRunTest = useCallback((connectorId: string, profileId: string) => {
    const profile = TEST_PROFILES.find((p) => p.id === profileId);
    if (!profile) return;

    const runId = `run_${Math.random().toString(36).slice(2, 8)}`;
    const run: TestRun = {
      id: runId,
      connectorId,
      profileId,
      status: "running",
      startedAt: new Date().toISOString(),
      results: [],
    };

    setTestRuns((prev) => [...prev, run]);
    setActiveTab("results");

    // Simulate test execution
    setTimeout(() => {
      const numTests = Math.floor(Math.random() * 6) + 3;
      const results: TestResult[] = Array.from({ length: numTests }, (_, i) => ({
        testId: `test_${i}`,
        testName: `${profile.name} - Test ${i + 1}`,
        passed: Math.random() > 0.15,
        durationMs: Math.floor(Math.random() * 3000) + 100,
        metrics: {
          p50Ms: Math.floor(Math.random() * 100) + 20,
          p90Ms: Math.floor(Math.random() * 200) + 50,
          p99Ms: Math.floor(Math.random() * 500) + 100,
          requestsPerSecond: Math.floor(Math.random() * 500) + 50,
          successRate: Math.floor(Math.random() * 20) + 80,
          gpuOpsPerSecond: profileId === "gpu-benchmark" ? Math.floor(Math.random() * 10_000_000) + 50_000 : undefined,
        },
      }));

      const passed = results.filter((r) => r.passed).length;
      const score = Math.round((passed / numTests) * 100);

      setTestRuns((prev) =>
        prev.map((r) =>
          r.id === runId
            ? {
                ...r,
                status: "completed",
                completedAt: new Date().toISOString(),
                results,
                summary: {
                  totalTests: numTests,
                  passed,
                  failed: numTests - passed,
                  totalDurationMs: results.reduce((a, r) => a + r.durationMs, 0),
                  overallScore: score,
                  grade: score >= 98 ? "A+" : score >= 90 ? "A" : score >= 75 ? "B" : score >= 60 ? "C" : "D",
                },
              }
            : r,
        ),
      );
    }, 2500);
  }, []);

  const handleStartTest = useCallback(
    (_connId: string) => {
      setActiveTab("testbench");
    },
    [],
  );

  const tabs = [
    { id: "networks", label: "Networks", count: CHAINS.length },
    { id: "connectors", label: "Connectors", count: connectors.length },
    { id: "testbench", label: "Test Bench" },
    { id: "results", label: "Results", count: testRuns.length },
    { id: "billing", label: "Billing" },
  ];

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "#0a0a0f",
        color: "#e0e0e0",
        fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, sans-serif",
        fontSize: 14,
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div style={{ padding: "12px 16px 0", borderBottom: "none" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 16, fontWeight: 700 }}>Blockchain Connector</span>
            <Badge color="#ff6b35">v0.1.0</Badge>
          </div>
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            {connectingChain && (
              <span style={{ fontSize: 11, color: "#ffaa00", fontFamily: "monospace" }}>
                Connecting to {connectingChain}...
              </span>
            )}
            <Badge color="#00d4aa">{connectors.filter((c) => c.status === "connected").length} active</Badge>
          </div>
        </div>
        <TabBar tabs={tabs} active={activeTab} onSelect={setActiveTab} />
      </div>

      {/* Content */}
      <div style={{ flex: 1, padding: "0 16px 16px", overflowY: "auto" }}>
        {activeTab === "networks" && <NetworksTab onConnect={handleConnect} />}
        {activeTab === "connectors" && (
          <ConnectorsTab
            connectors={connectors}
            onTest={handleStartTest}
            onDisconnect={handleDisconnect}
            onRefresh={handleRefresh}
          />
        )}
        {activeTab === "testbench" && (
          <TestBenchTab connectors={connectors} testRuns={testRuns} onRunTest={handleRunTest} />
        )}
        {activeTab === "results" && <ResultsTab testRuns={testRuns} />}
        {activeTab === "billing" && <BillingTab />}
      </div>
    </div>
  );
}
