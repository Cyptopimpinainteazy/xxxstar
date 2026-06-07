/**
 * SVM Runtime Service — Solana VM layer on X3 Chain
 * Account creation, program deployment, transfers
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  SvmCreateAccountParams,
  SvmDeployProgramParams,
  SvmTransferParams,
  TxStatusCallback,
} from '../types/interfaces';

export class SvmService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Extrinsics
  // ---------------------------------------------------------------------------

  /** Create an SVM account */
  async createAccount(
    account: SignerAccount,
    params: SvmCreateAccountParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.svmRuntime.createAccount(
      Array.from(params.pubkey),
      params.lamports,
      params.space,
      Array.from(params.owner),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Deploy an SVM program (BPF bytecode) */
  async deployProgram(
    account: SignerAccount,
    params: SvmDeployProgramParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.svmRuntime.deployProgram(
      Array.from(params.programId),
      Array.from(params.bytecode),
      params.upgradeAuthority ? Array.from(params.upgradeAuthority) : null,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Transfer lamports between SVM accounts */
  async transfer(
    account: SignerAccount,
    params: SvmTransferParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.svmRuntime.transfer(
      Array.from(params.from),
      Array.from(params.to),
      params.amount,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Close an SVM account (recover lamports) */
  async closeAccount(
    account: SignerAccount,
    pubkey: Uint8Array,
    recipient: Uint8Array,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.svmRuntime.closeAccount(
      Array.from(pubkey),
      Array.from(recipient),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Fund an SVM account from Substrate balance */
  async fundAccount(
    account: SignerAccount,
    svmPubkey: Uint8Array,
    amount: bigint,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.svmRuntime.fundAccount(
      Array.from(svmPubkey),
      amount,
    );
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get SVM account info */
  async getAccount(pubkey: Uint8Array) {
    const info = await this.api.query.svmRuntime.accounts(Array.from(pubkey));
    return (info as any).toJSON?.() ?? null;
  }

  /** Get SVM account data */
  async getAccountData(pubkey: Uint8Array): Promise<Uint8Array | null> {
    const data = await this.api.query.svmRuntime.accountData(Array.from(pubkey));
    const hex = (data as any).toHex?.();
    if (!hex || hex === '0x') return null;
    return new Uint8Array(Buffer.from(hex.slice(2), 'hex'));
  }

  /** Get SVM program info */
  async getProgram(programId: Uint8Array) {
    const info = await this.api.query.svmRuntime.programs(Array.from(programId));
    return (info as any).toJSON?.() ?? null;
  }

  /** Get current SVM slot */
  async getCurrentSlot(): Promise<number> {
    const slot = await this.api.query.svmRuntime.currentSlot();
    return (slot as any).toNumber?.() ?? 0;
  }

  /** Get total SVM lamports in system */
  async getTotalLamports(): Promise<bigint> {
    const total = await this.api.query.svmRuntime.totalLamports();
    return (total as any).toBigInt?.() ?? 0n;
  }
}
