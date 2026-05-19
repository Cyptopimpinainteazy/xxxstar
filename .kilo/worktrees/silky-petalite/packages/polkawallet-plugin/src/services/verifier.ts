/**
 * X3 Verifier Service — off-chain execution verification,
 * executor registration, job submission, and receipt verification
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  RegisterExecutorParams,
  SubmitJobParams,
  SubmitReceiptParams,
  JobInfo,
  JobStatus,
  TxStatusCallback,
} from '../types/interfaces';
import { hexToU8a } from '@polkadot/util';

export class VerifierService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------

  /** Register as an x3vm executor (staking required) */
  async registerExecutor(
    account: SignerAccount,
    params: RegisterExecutorParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3Verifier.registerExecutor(params.stake);
    return signAndSend(tx, account, statusCb);
  }

  /** Deactivate executor registration */
  async deactivateExecutor(
    account: SignerAccount,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3Verifier.deactivateExecutor();
    return signAndSend(tx, account, statusCb);
  }

  /** Submit a job for x3vm execution */
  async submitJob(
    account: SignerAccount,
    params: SubmitJobParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3Verifier.submitJob(
      params.bytecodeHash,
      params.inputHash,
      params.gasLimit,
      params.reward,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Submit an execution receipt (proof of computation) */
  async submitReceipt(
    account: SignerAccount,
    params: SubmitReceiptParams,
    statusCb?: TxStatusCallback,
  ) {
    const receipt = {
      job_id: params.jobId,
      executor: typeof account === 'string' ? account : account.address,
      input_hash: params.inputHash,
      output_hash: params.outputHash,
      state_root_before: params.stateRootBefore,
      state_root_after: params.stateRootAfter,
      gas_used: params.gasUsed,
      timestamp: params.timestamp,
      output_data:
        typeof params.outputData === 'string'
          ? hexToU8a(params.outputData)
          : params.outputData,
      state_changes: params.stateChanges,
      merkle_proof: params.merkleProof,
      signature:
        typeof params.signature === 'string'
          ? hexToU8a(params.signature)
          : params.signature,
    };

    const tx = this.api.tx.x3Verifier.submitReceipt(receipt);
    return signAndSend(tx, account, statusCb);
  }

  /** Dispute a receipt */
  async disputeReceipt(
    account: SignerAccount,
    jobId: string,
    reason: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.x3Verifier.disputeReceipt(
      jobId,
      new TextEncoder().encode(reason),
    );
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get job info by ID */
  async getJob(jobId: string): Promise<JobInfo | null> {
    const job = await this.api.query.x3Verifier.jobs(jobId);
    const json = (job as any).toJSON?.();
    if (!json) return null;

    return {
      jobId,
      submitter: json.submitter,
      bytecodeHash: json.bytecode_hash,
      inputHash: json.input_hash,
      gasLimit: BigInt(json.gas_limit ?? '0'),
      reward: BigInt(json.reward ?? '0'),
      executor: json.executor ?? undefined,
      status: json.status as JobStatus,
      createdAt: json.created_at,
    };
  }

  /** Get executor info */
  async getExecutor(address: string) {
    const exec = await this.api.query.x3Verifier.executors(address);
    return (exec as any).toJSON?.() ?? null;
  }

  /** Get verified state root for a job */
  async getVerifiedStateRoot(jobId: string): Promise<string | null> {
    const root = await this.api.query.x3Verifier.verifiedStateRoots(jobId);
    const hex = (root as any).toHex?.();
    return hex && hex !== '0x' + '00'.repeat(32) ? hex : null;
  }

  /** Query if verification is globally enabled */
  async isVerificationEnabled(): Promise<boolean> {
    const enabled = await this.api.query.x3Verifier.verificationEnabled();
    return (enabled as any).isTrue ?? true;
  }

  /** Get protocol treasury balance */
  async getProtocolTreasury(): Promise<bigint> {
    const treasury = await this.api.query.x3Verifier.protocolTreasury();
    return (treasury as any).toBigInt?.() ?? 0n;
  }

  /** Get verifier stats */
  async getStats(): Promise<{ totalSubmitted: number; totalVerified: number }> {
    const [submitted, verified] = await Promise.all([
      this.api.query.x3Verifier.totalJobsSubmitted(),
      this.api.query.x3Verifier.totalJobsVerified(),
    ]);

    return {
      totalSubmitted: (submitted as any).toNumber?.() ?? 0,
      totalVerified: (verified as any).toNumber?.() ?? 0,
    };
  }
}
