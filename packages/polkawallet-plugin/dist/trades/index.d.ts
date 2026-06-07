import { S as SignerAccount, n as CreateTradeBatchParams, T as TxStatusCallback, f as ComitEvent, o as TradeResult, A as AmmProtocol, p as TradeBatchInfo } from '../tx-helper-BUR0DrYk.js';
export { q as TradeBatchStatus, r as TradeLegInput, s as TradeLegResult, V as VmType } from '../tx-helper-BUR0DrYk.js';
import { ApiPromise } from '@polkadot/api';
import '@polkadot/types/types';
import '@polkadot/keyring/types';

declare class AtomicTradeService {
    private api;
    constructor(api: ApiPromise);
    /** Create a multi-leg trade batch across EVM/SVM/X3 */
    createTradeBatch(account: SignerAccount, params: CreateTradeBatchParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Execute a pending trade batch */
    executeTradeBatch(account: SignerAccount, batchId: string, statusCb?: TxStatusCallback): Promise<TradeResult>;
    /** Execute trade batch through the Kernel ComitV2 path (tri-VM) */
    executeTradeBatchViaKernel(account: SignerAccount, batchId: string, comitId: string, statusCb?: TxStatusCallback): Promise<TradeResult>;
    /** Cancel a pending trade batch */
    cancelTradeBatch(account: SignerAccount, batchId: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Create a manual checkpoint for a batch */
    createCheckpoint(account: SignerAccount, batchId: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Create and immediately execute a trade batch */
    trade(account: SignerAccount, params: CreateTradeBatchParams, statusCb?: TxStatusCallback): Promise<TradeResult>;
    /**
     * Convenience: single-leg swap (the most common case)
     */
    swap(account: SignerAccount, opts: {
        ammProtocol: AmmProtocol;
        vmType: 'Evm' | 'Svm' | 'X3' | 'CrossVm';
        assetIn: string;
        assetOut: string;
        amountIn: bigint;
        minAmountOut: bigint;
        slippageBps?: number;
        deadline?: number;
        routeData?: Uint8Array | string;
    }, statusCb?: TxStatusCallback): Promise<TradeResult>;
    /** Get trade batch info by ID */
    getBatch(batchId: string): Promise<TradeBatchInfo | null>;
    /** Get all pending batch IDs for an account */
    getPendingBatches(account: string): Promise<string[]>;
    /** Get TWAP price for a token pair */
    getTwap(tokenA: string, tokenB: string): Promise<{
        cumulativePrice: bigint;
        lastUpdate: number;
    } | null>;
    /** Get AMM adapter configuration */
    getAmmAdapter(protocol: AmmProtocol): Promise<any>;
    /** Get protocol stats */
    getStats(): Promise<{
        completedBatches: any;
        failedBatches: any;
        totalVolume: any;
    }>;
    private _encodeTradeLeg;
    private _decodeTradeLeg;
    private _parseTradeResult;
}

export { AmmProtocol, AtomicTradeService, CreateTradeBatchParams, TradeBatchInfo, TradeResult };
