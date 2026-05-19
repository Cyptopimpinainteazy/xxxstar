import { ApiPromise } from '@polkadot/api';
import * as _polkadot_types_types from '@polkadot/types/types';
import { Signer, RegistryTypes } from '@polkadot/types/types';
import { EventEmitter } from 'eventemitter3';
import { C as ConnectionState, X as X3ChainConfig, a as X3Network, S as SignerAccount, b as ComitParams, T as TxStatusCallback, c as ComitResult, d as X3Balance, e as X3Account, R as RegisterExecutorParams, f as ComitEvent, g as SubmitJobParams, h as SubmitReceiptParams, J as JobInfo, i as TreasuryProposalParams, j as RecurringPaymentParams, Y as YieldStrategyParams, k as SvmCreateAccountParams, l as SvmDeployProgramParams, m as SvmTransferParams } from './tx-helper-BUR0DrYk.js';
import { SettlementService } from './settlement/index.js';
import { AtomicTradeService } from './trades/index.js';
import { DomainService } from './domains/index.js';
import { GovernanceService } from './governance/index.js';
import { X3VmClient } from './x3vm/index.js';
import '@polkadot/keyring/types';

/**
 * Setting service — network connection utilities
 */

declare function subscribeMessage(method: any, params: any[], msgChannel: string, transform?: (data: any) => any): Promise<any>;
declare function getNetworkConst(api: ApiPromise): Promise<{
    x3chain: {
        ss58Prefix: number;
        chainId: number;
        blockTime: number;
    };
}>;
declare function getNetworkProperties(api: ApiPromise): Promise<{
    tokenDecimals: number[];
    tokenSymbol: string[];
    ss58Format: number;
    chainId: number;
}>;

interface ApiEvents {
    connected: (state: ConnectionState) => void;
    disconnected: () => void;
    error: (error: Error) => void;
    ready: (api: ApiPromise) => void;
    reconnecting: (attempt: number, delay: number) => void;
    reconnected: (state: ConnectionState) => void;
}
/**
 * Enhanced X3Chain API with automatic reconnection and retry logic
 */
declare class X3ChainApi extends EventEmitter<ApiEvents> {
    private _api;
    private _provider;
    private _config;
    private _connectionState;
    private _reconnectAttempts;
    private _reconnectTimer;
    private _isDisconnecting;
    constructor(config?: X3ChainConfig);
    /** Get the underlying Polkadot API instance */
    get api(): ApiPromise;
    /** Current connection state */
    get state(): ConnectionState | null;
    /** Whether the API is connected */
    get isConnected(): boolean;
    /** Get current network */
    get network(): X3Network;
    /**
     * Connect to the x3chain node
     */
    connect(): Promise<ApiPromise>;
    /**
     * Handle disconnection with automatic reconnection
     */
    private _handleDisconnect;
    /**
     * Attempt to reconnect to the node
     */
    private _reconnect;
    /**
     * Disconnect from the node
     */
    disconnect(): Promise<void>;
    /**
     * Set a signer (for Polkawallet mobile extension bridge)
     */
    setSigner(signer: _polkadot_types_types.Signer): void;
    /**
     * Get available account addresses from the connected signer/extension
     */
    getAccounts(): Promise<string[]>;
    /**
     * Execute a query with retry logic
     */
    executeWithRetry<T>(fn: () => Promise<T>, maxRetries?: number, delay?: number): Promise<T>;
    /**
     * Check if the API is connected and ready
     */
    ensureConnected(): Promise<void>;
}
/**
 * Convenience factory to create and connect an API instance
 */
declare function createX3Api(config?: X3ChainConfig): Promise<X3ChainApi>;
/**
 * Create API instance from environment configuration
 */
declare function createX3ApiFromEnv(): Promise<X3ChainApi>;

/**
 * X3 Kernel Service — submit_comit, submit_comit_v2, account/asset management
 */

declare class KernelService {
    private api;
    constructor(api: ApiPromise);
    /**
     * Submit a Comit (dual-VM: EVM + SVM)
     */
    submitComit(account: SignerAccount, params: Omit<ComitParams, 'x3Payload'>, statusCb?: TxStatusCallback): Promise<ComitResult>;
    /**
     * Submit a Comit v2 (tri-VM: EVM + SVM + X3)
     */
    submitComitV2(account: SignerAccount, params: ComitParams, statusCb?: TxStatusCallback): Promise<ComitResult>;
    /** Get canonical balance for account + asset */
    getBalance(account: string, assetId: number): Promise<bigint>;
    /** Get all balances for an account across all registered assets */
    getAllBalances(account: string): Promise<X3Balance[]>;
    /** Get account info */
    getAccount(address: string): Promise<X3Account>;
    /** Get next comit nonce for account */
    getNonce(address: string): Promise<bigint>;
    /** Get asset metadata */
    getAssetMetadata(assetId: number): Promise<{
        symbol: string;
        decimals: number;
    } | null>;
    /** Get the current authority set */
    getAuthorities(): Promise<string[]>;
    /** Check if an account is authorized */
    isAuthorized(address: string): Promise<boolean>;
    /** Estimate fee for a comit v2 submission */
    estimateComitFee(senderAddress: string, params: ComitParams): Promise<bigint>;
    private _parseComitResult;
}

declare class VerifierService {
    private api;
    constructor(api: ApiPromise);
    /** Register as an x3vm executor (staking required) */
    registerExecutor(account: SignerAccount, params: RegisterExecutorParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Deactivate executor registration */
    deactivateExecutor(account: SignerAccount, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit a job for x3vm execution */
    submitJob(account: SignerAccount, params: SubmitJobParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit an execution receipt (proof of computation) */
    submitReceipt(account: SignerAccount, params: SubmitReceiptParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Dispute a receipt */
    disputeReceipt(account: SignerAccount, jobId: string, reason: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get job info by ID */
    getJob(jobId: string): Promise<JobInfo | null>;
    /** Get executor info */
    getExecutor(address: string): Promise<any>;
    /** Get verified state root for a job */
    getVerifiedStateRoot(jobId: string): Promise<string | null>;
    /** Query if verification is globally enabled */
    isVerificationEnabled(): Promise<boolean>;
    /** Get protocol treasury balance */
    getProtocolTreasury(): Promise<bigint>;
    /** Get verifier stats */
    getStats(): Promise<{
        totalSubmitted: number;
        totalVerified: number;
    }>;
}

declare class TreasuryService {
    private api;
    constructor(api: ApiPromise);
    /** Submit a treasury spending proposal */
    submitProposal(account: SignerAccount, params: TreasuryProposalParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Approve a spending proposal (requires multi-sig signer) */
    approveProposal(account: SignerAccount, proposalId: number, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Execute an approved proposal */
    executeProposal(account: SignerAccount, proposalId: number, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Deposit funds into the treasury */
    deposit(account: SignerAccount, amount: bigint, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Create a recurring payment schedule */
    createRecurringPayment(account: SignerAccount, params: RecurringPaymentParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Cancel a recurring payment */
    cancelRecurringPayment(account: SignerAccount, paymentId: number, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Register a yield strategy */
    registerYieldStrategy(account: SignerAccount, params: YieldStrategyParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Execute a yield strategy (deploy capital) */
    executeYieldStrategy(account: SignerAccount, strategyId: number, amount: bigint, expectedReturn: bigint, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Report yield return (return capital + profit) */
    reportYieldReturn(account: SignerAccount, strategyId: number, returnedAmount: bigint, originalAmount: bigint, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Pause the treasury */
    pause(account: SignerAccount, reason: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Unpause the treasury */
    unpause(account: SignerAccount, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get proposal info */
    getProposal(proposalId: number): Promise<any>;
    /** Get current signers */
    getSigners(): Promise<string[]>;
    /** Get recurring payment info */
    getRecurringPayment(paymentId: number): Promise<any>;
    /** Get yield strategy info */
    getYieldStrategy(strategyId: number): Promise<any>;
    /** Is the treasury paused? */
    isPaused(): Promise<boolean>;
    /** Get treasury stats */
    getStats(): Promise<any>;
}

declare class SvmService {
    private api;
    constructor(api: ApiPromise);
    /** Create an SVM account */
    createAccount(account: SignerAccount, params: SvmCreateAccountParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Deploy an SVM program (BPF bytecode) */
    deployProgram(account: SignerAccount, params: SvmDeployProgramParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Transfer lamports between SVM accounts */
    transfer(account: SignerAccount, params: SvmTransferParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Close an SVM account (recover lamports) */
    closeAccount(account: SignerAccount, pubkey: Uint8Array, recipient: Uint8Array, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Fund an SVM account from Substrate balance */
    fundAccount(account: SignerAccount, svmPubkey: Uint8Array, amount: bigint, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get SVM account info */
    getAccount(pubkey: Uint8Array): Promise<any>;
    /** Get SVM account data */
    getAccountData(pubkey: Uint8Array): Promise<Uint8Array | null>;
    /** Get SVM program info */
    getProgram(programId: Uint8Array): Promise<any>;
    /** Get current SVM slot */
    getCurrentSlot(): Promise<number>;
    /** Get total SVM lamports in system */
    getTotalLamports(): Promise<bigint>;
}

declare class AtlasX3Plugin {
    private _x3Api;
    private _initialized;
    private _kernel?;
    private _settlement?;
    private _trades?;
    private _domains?;
    private _verifier?;
    private _governance?;
    private _treasury?;
    private _svm?;
    private _x3vm?;
    constructor(config: X3ChainConfig);
    /** Connect to the x3chain node and initialize all services */
    init(): Promise<void>;
    /** Disconnect and clean up */
    dispose(): Promise<void>;
    /** Set signer for Polkawallet mobile integration */
    setSigner(signer: Signer): void;
    /** Get connection state */
    get connectionState(): ConnectionState | null;
    /** Whether the plugin is initialized and connected */
    get isReady(): boolean;
    /** The raw Polkadot API instance (for advanced use) */
    get rawApi(): ApiPromise;
    /** X3 Kernel — Comit submission, balances, account management */
    get kernel(): KernelService;
    /** X3 Settlement Engine — cross-chain atomic settlement, BTC proofs, bonds */
    get settlement(): SettlementService;
    /** Atomic Trade Engine — multi-leg cross-VM trade batches, AMM routing, TWAP */
    get trades(): AtomicTradeService;
    /** X3 Domain Registry — .x3 domain registration and DNS */
    get domains(): DomainService;
    /** X3 Verifier — executor registration, job verification, state root proofs */
    get verifier(): VerifierService;
    /** Governance — proposals, voting, delegation, AI governance, kill switch */
    get governance(): GovernanceService;
    /** Treasury — multi-sig spending, recurring payments, yield strategies */
    get treasury(): TreasuryService;
    /** SVM Runtime — Solana VM accounts, programs, transfers */
    get svm(): SvmService;
    /** X3VM — compile x3 lang, deploy contracts, call functions, flash loans */
    get x3vm(): X3VmClient;
    on<K extends keyof ApiEvents>(event: K, handler: ApiEvents[K]): this;
    off<K extends keyof ApiEvents>(event: K, handler: ApiEvents[K]): this;
    private _initServices;
    private _ensureReady;
}
/**
 * Create a plugin connected to a local dev node
 */
declare function createLocalPlugin(): AtlasX3Plugin;
/**
 * Create a plugin connected to the X3 testnet
 */
declare function createTestnetPlugin(): AtlasX3Plugin;
/**
 * Create a plugin connected to the X3 mainnet
 */
declare function createMainnetPlugin(): AtlasX3Plugin;

/**
 * X3 Chain x3chain Runtime Type Definitions
 *
 * Complete type registry for Polkawallet integration — mirrors all runtime
 * pallets: x3-kernel, x3-settlement-engine, x3-domain-registry,
 * x3-verifier, atomic-trade-engine, governance, treasury, svm-runtime.
 */

declare const X3ChainCustomTypes: RegistryTypes;
/**
 * Runtime RPC methods for x3-chain-specific calls
 */
declare const X3ChainRpc: {
    x3: {
        getCanonicalBalance: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getAssetMetadata: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getNonce: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        isAuthorized: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getAuthorities: {
            description: string;
            params: {
                name: string;
                type: string;
                isOptional: boolean;
            }[];
            type: string;
        };
    };
    x3Settlement: {
        getIntent: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getIntentState: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getBond: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getBtcBestHeight: {
            description: string;
            params: {
                name: string;
                type: string;
                isOptional: boolean;
            }[];
            type: string;
        };
    };
    atomicTrade: {
        getBatch: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getTwap: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
        getAmmAdapter: {
            description: string;
            params: ({
                name: string;
                type: string;
                isOptional?: undefined;
            } | {
                name: string;
                type: string;
                isOptional: boolean;
            })[];
            type: string;
        };
    };
};
/**
 * Signed extensions for the x3chain runtime
 */
declare const X3ChainSignedExtensions: {
    ChargeTransactionPayment: {
        extrinsic: {
            tip: string;
        };
        payload: {};
    };
};

/**
 * X3 Chain x3chain — Polkawallet JS API entry point
 *
 * This is the @polkadot/api wrapper that gets bundled and loaded inside
 * the Polkawallet Flutter app's hidden WebView. It exposes:
 *
 *  window.settings   — connect/disconnect, network props
 *  window.keyring    — standard substrate keyring (from parent js_api)
 *  window.account    — balance queries, identity
 *  window.x3chain    — x3chain-specific: kernel, atomic trades, x3vm, domains, governance
 *  window.x3vm       — x3 bytecode submission & execution
 *  window.atomicTrade— atomic trade batches
 *  window.x3domains  — .x3 domain registration & management
 *  window.governance — proposals, voting, AI governance
 *  window.evolution  — evolution engine & agent management
 *  window.settlement — intent-based settlement, escrow, bonds
 *  window.agents     — agent account management
 *  window.flashloan  — flashloan intent creation via settlement
 */

declare function connect(nodes: string[]): Promise<unknown>;
declare function connectLocal(): Promise<unknown>;
declare function connectTestnet(): Promise<unknown>;
declare function connectMainnet(): Promise<unknown>;
declare function disconnect(): Promise<void>;
declare const settings: {
    connect: typeof connect;
    connectLocal: typeof connectLocal;
    connectTestnet: typeof connectTestnet;
    connectMainnet: typeof connectMainnet;
    disconnect: typeof disconnect;
    subscribeMessage: typeof subscribeMessage;
    getNetworkConst: typeof getNetworkConst;
    getNetworkProperties: typeof getNetworkProperties;
};

export { AtlasX3Plugin, AtomicTradeService, DomainService, GovernanceService, KernelService, SettlementService, SvmService, TreasuryService, VerifierService, X3ChainCustomTypes, X3ChainRpc, X3ChainSignedExtensions, X3VmClient, createLocalPlugin, createMainnetPlugin, createTestnetPlugin, createX3Api, createX3ApiFromEnv, settings as default };
