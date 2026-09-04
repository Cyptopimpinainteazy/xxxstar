/**
 * Treasury Service — multi-sig spending proposals, recurring payments,
 * yield strategies, and emergency pause
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  TreasuryProposalParams,
  RecurringPaymentParams,
  YieldStrategyParams,
  RiskLevel,
  TxStatusCallback,
} from '../types/interfaces';

export class TreasuryService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Spending Proposals
  // ---------------------------------------------------------------------------

  /** Submit a treasury spending proposal */
  async submitProposal(
    account: SignerAccount,
    params: TreasuryProposalParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.submitProposal(
      params.beneficiary,
      params.amount,
      new TextEncoder().encode(params.description),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Approve a spending proposal (requires multi-sig signer) */
  async approveProposal(
    account: SignerAccount,
    proposalId: number,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.approveProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }

  /** Execute an approved proposal */
  async executeProposal(
    account: SignerAccount,
    proposalId: number,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.executeProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }

  /** Deposit funds into the treasury */
  async deposit(
    account: SignerAccount,
    amount: bigint,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.deposit(amount);
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Recurring Payments
  // ---------------------------------------------------------------------------

  /** Create a recurring payment schedule */
  async createRecurringPayment(
    account: SignerAccount,
    params: RecurringPaymentParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.createRecurringPayment(
      params.beneficiary,
      params.amount,
      params.interval,
      params.totalPayments ?? null,
      new TextEncoder().encode(params.description),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Cancel a recurring payment */
  async cancelRecurringPayment(
    account: SignerAccount,
    paymentId: number,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.cancelRecurringPayment(paymentId);
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Yield Strategies
  // ---------------------------------------------------------------------------

  /** Register a yield strategy */
  async registerYieldStrategy(
    account: SignerAccount,
    params: YieldStrategyParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.registerYieldStrategy(
      params.agent,
      params.maxAllocation,
      params.minExpectedReturn,
      params.riskLevel,
      new TextEncoder().encode(params.description),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Execute a yield strategy (deploy capital) */
  async executeYieldStrategy(
    account: SignerAccount,
    strategyId: number,
    amount: bigint,
    expectedReturn: bigint,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.executeYieldStrategy(
      strategyId,
      amount,
      expectedReturn,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Report yield return (return capital + profit) */
  async reportYieldReturn(
    account: SignerAccount,
    strategyId: number,
    returnedAmount: bigint,
    originalAmount: bigint,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.reportYieldReturn(
      strategyId,
      returnedAmount,
      originalAmount,
    );
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Emergency Controls
  // ---------------------------------------------------------------------------

  /** Pause the treasury */
  async pause(
    account: SignerAccount,
    reason: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.treasury.pause(new TextEncoder().encode(reason));
    return signAndSend(tx, account, statusCb);
  }

  /** Unpause the treasury */
  async unpause(account: SignerAccount, statusCb?: TxStatusCallback) {
    const tx = this.api.tx.treasury.unpause();
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get proposal info */
  async getProposal(proposalId: number) {
    const proposal = await this.api.query.treasury.proposals(proposalId);
    return (proposal as any).toJSON?.() ?? null;
  }

  /** Get current signers */
  async getSigners(): Promise<string[]> {
    const signers = await this.api.query.treasury.signers();
    return (signers as any).toJSON?.() ?? [];
  }

  /** Get recurring payment info */
  async getRecurringPayment(paymentId: number) {
    const payment = await this.api.query.treasury.recurringPayments(paymentId);
    return (payment as any).toJSON?.() ?? null;
  }

  /** Get yield strategy info */
  async getYieldStrategy(strategyId: number) {
    const strategy = await this.api.query.treasury.yieldStrategies(strategyId);
    return (strategy as any).toJSON?.() ?? null;
  }

  /** Is the treasury paused? */
  async isPaused(): Promise<boolean> {
    const paused = await this.api.query.treasury.isPaused();
    return (paused as any).isTrue ?? false;
  }

  /** Get treasury stats */
  async getStats() {
    const stats = await this.api.query.treasury.stats();
    return (stats as any).toJSON?.() ?? null;
  }
}
