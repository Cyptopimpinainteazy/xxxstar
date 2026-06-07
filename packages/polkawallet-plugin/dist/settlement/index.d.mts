import { S as SignerAccount, t as CreateIntentParams, T as TxStatusCallback, f as ComitEvent, L as LockEscrowParams, E as ExternalChainId, B as BtcProofParams, u as BondParams, v as SettlementIntentInfo, I as IntentState } from '../tx-helper-BUR0DrYk.mjs';
export { w as AssetSpec, x as BtcBlockHeader } from '../tx-helper-BUR0DrYk.mjs';
import { ApiPromise } from '@polkadot/api';
import '@polkadot/types/types';
import '@polkadot/keyring/types';

declare class SettlementService {
    private api;
    constructor(api: ApiPromise);
    /** Create a cross-chain settlement intent (HTLC-based) */
    createIntent(account: SignerAccount, params: CreateIntentParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Lock escrow for a settlement leg */
    lockEscrow(account: SignerAccount, params: LockEscrowParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit proof from an external chain */
    submitProof(account: SignerAccount, intentId: string, chain: ExternalChainId, proof: unknown, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Claim settlement (reveal HTLC secret) */
    claimSettlement(account: SignerAccount, intentId: string, secret: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Refund expired settlement */
    refundSettlement(account: SignerAccount, intentId: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit BTC transaction proof (SPV) */
    submitBtcProof(account: SignerAccount, params: BtcProofParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit a BTC block header for the light-client */
    submitBtcHeader(account: SignerAccount, header: BtcProofParams['blockHeader'], statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Deposit a bond */
    depositBond(account: SignerAccount, params: BondParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Request bond withdrawal */
    requestBondWithdraw(account: SignerAccount, bondId: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Finalize bond withdrawal */
    finalizeBondWithdraw(account: SignerAccount, bondId: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Report an invariant violation */
    reportViolation(account: SignerAccount, intentId: string, violationType: string, evidence: Uint8Array | string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get settlement intent by ID */
    getIntent(intentId: string): Promise<SettlementIntentInfo | null>;
    /** Get intent state */
    getIntentState(intentId: string): Promise<IntentState>;
    /** Get bond info */
    getBond(bondId: string): Promise<any>;
    /** Get bonds owned by an account */
    getBondsByOwner(account: string): Promise<string[]>;
    /** Get BTC best known block height */
    getBtcBestHeight(): Promise<number>;
    /** Get protocol stats */
    getStats(): Promise<{
        totalIntents: number;
        totalSettledVolume: bigint;
        violations: number;
    }>;
    /** Subscribe to settlement events for a given intent */
    subscribeToIntent(intentId: string, callback: (state: IntentState) => void): Promise<() => void>;
    private _encodeAssetSpec;
    private _decodeAssetSpec;
}

export { BondParams, BtcProofParams, CreateIntentParams, ExternalChainId, IntentState, LockEscrowParams, SettlementIntentInfo, SettlementService };
