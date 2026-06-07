import { S as SignerAccount, y as SubmitProposalParams, T as TxStatusCallback, f as ComitEvent, z as VoteParams, D as DelegateParams, F as AIProposalParams, K as KillSwitchLevel } from '../tx-helper-BUR0DrYk.js';
export { G as AIProposalType, H as ConvictionLevel, M as ImpactAssessment, N as SimulationRequirements, O as VoteDirection } from '../tx-helper-BUR0DrYk.js';
import { ApiPromise } from '@polkadot/api';
import '@polkadot/types/types';
import '@polkadot/keyring/types';

declare class GovernanceService {
    private api;
    constructor(api: ApiPromise);
    /** Submit a governance proposal */
    submitProposal(account: SignerAccount, params: SubmitProposalParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Vote on a proposal */
    vote(account: SignerAccount, params: VoteParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Delegate voting power */
    delegate(account: SignerAccount, params: DelegateParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Remove delegation */
    undelegate(account: SignerAccount, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Finalize a proposal after voting period ends */
    finalizeProposal(account: SignerAccount, proposalId: number, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Unlock tokens after conviction lock expires */
    unlock(account: SignerAccount, targetAccount: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Submit an AI governance proposal */
    submitAIProposal(account: SignerAccount, params: AIProposalParams, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Activate the kill switch (emergency) */
    activateKillSwitch(account: SignerAccount, level: KillSwitchLevel, reason: string, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Deactivate the kill switch */
    deactivateKillSwitch(account: SignerAccount, statusCb?: TxStatusCallback): Promise<{
        blockHash: string;
        blockNumber: number;
        txHash: string;
        events: ComitEvent[];
    }>;
    /** Get proposal info */
    getProposal(proposalId: number): Promise<any>;
    /** Get proposal tally (aye/nay/abstain counts) */
    getProposalTally(proposalId: number): Promise<any>;
    /** Get all active proposals */
    getActiveProposals(): Promise<number[]>;
    /** Get delegation info for an account */
    getDelegation(account: string): Promise<any>;
    /** Get current kill switch level */
    getKillSwitchLevel(): Promise<KillSwitchLevel>;
    /** Get AI proposal by ID */
    getAIProposal(proposalId: number): Promise<any>;
    /** Get governance config */
    getConfig(): Promise<any>;
}

export { AIProposalParams, DelegateParams, GovernanceService, KillSwitchLevel, SubmitProposalParams, VoteParams };
