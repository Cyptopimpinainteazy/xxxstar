/**
 * Governance Service — proposals, conviction voting, delegation,
 * AI governance, and kill switch
 */

import type { ApiPromise } from '@polkadot/api';
import { signAndSend } from '../core/tx-helper';
import type { SignerAccount } from '../core/tx-helper';
import type {
  SubmitProposalParams,
  VoteParams,
  DelegateParams,
  AIProposalParams,
  KillSwitchLevel,
  TxStatusCallback,
} from '../types/interfaces';

export class GovernanceService {
  constructor(private api: ApiPromise) {}

  // ---------------------------------------------------------------------------
  // Standard Governance
  // ---------------------------------------------------------------------------

  /** Submit a governance proposal */
  async submitProposal(
    account: SignerAccount,
    params: SubmitProposalParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.submitProposal(
      params.call,
      new TextEncoder().encode(params.title),
      new TextEncoder().encode(params.description),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Vote on a proposal */
  async vote(
    account: SignerAccount,
    params: VoteParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.vote(
      params.proposalId,
      params.direction,
      params.balance,
      params.conviction,
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Delegate voting power */
  async delegate(
    account: SignerAccount,
    params: DelegateParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.delegate(params.target, params.conviction);
    return signAndSend(tx, account, statusCb);
  }

  /** Remove delegation */
  async undelegate(account: SignerAccount, statusCb?: TxStatusCallback) {
    const tx = this.api.tx.governance.undelegate();
    return signAndSend(tx, account, statusCb);
  }

  /** Finalize a proposal after voting period ends */
  async finalizeProposal(
    account: SignerAccount,
    proposalId: number,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.finalizeProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }

  /** Unlock tokens after conviction lock expires */
  async unlock(
    account: SignerAccount,
    targetAccount: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.unlock(targetAccount);
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // AI Governance
  // ---------------------------------------------------------------------------

  /** Submit an AI governance proposal */
  async submitAIProposal(
    account: SignerAccount,
    params: AIProposalParams,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.submitAiProposal(
      params.proposalType,
      typeof params.payload === 'string'
        ? new TextEncoder().encode(params.payload)
        : params.payload,
      {
        risk_score: params.impactAssessment.riskScore,
        affected_pallets: params.impactAssessment.affectedPallets.map((p) =>
          new TextEncoder().encode(p),
        ),
        reversible: params.impactAssessment.reversible,
        estimated_gas: params.impactAssessment.estimatedGas,
      },
      {
        min_simulation_blocks: params.simulationRequirements.minSimulationBlocks,
        required_coverage_percent: params.simulationRequirements.requiredCoveragePercent,
        max_state_changes: params.simulationRequirements.maxStateChanges,
      },
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Activate the kill switch (emergency) */
  async activateKillSwitch(
    account: SignerAccount,
    level: KillSwitchLevel,
    reason: string,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.activateKillSwitch(
      level,
      new TextEncoder().encode(reason),
    );
    return signAndSend(tx, account, statusCb);
  }

  /** Deactivate the kill switch */
  async deactivateKillSwitch(
    account: SignerAccount,
    statusCb?: TxStatusCallback,
  ) {
    const tx = this.api.tx.governance.deactivateKillSwitch();
    return signAndSend(tx, account, statusCb);
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get proposal info */
  async getProposal(proposalId: number) {
    const proposal = await this.api.query.governance.proposals(proposalId);
    return (proposal as any).toJSON?.() ?? null;
  }

  /** Get proposal tally (aye/nay/abstain counts) */
  async getProposalTally(proposalId: number) {
    const tally = await this.api.query.governance.proposalVotes(proposalId);
    return (tally as any).toJSON?.() ?? null;
  }

  /** Get all active proposals */
  async getActiveProposals(): Promise<number[]> {
    const count = await this.api.query.governance.proposalCount();
    const total = (count as any).toNumber?.() ?? 0;
    const active: number[] = [];

    for (let i = 0; i < total; i++) {
      const proposal = await this.api.query.governance.proposals(i);
      const json = (proposal as any).toJSON?.();
      if (json?.status === 'Voting') {
        active.push(i);
      }
    }

    return active;
  }

  /** Get delegation info for an account */
  async getDelegation(account: string) {
    const delegation = await this.api.query.governance.delegations(account);
    return (delegation as any).toJSON?.() ?? null;
  }

  /** Get current kill switch level */
  async getKillSwitchLevel(): Promise<KillSwitchLevel> {
    const level = await this.api.query.governance.killSwitchLevelStorage();
    return (level as any).toString() as KillSwitchLevel;
  }

  /** Get AI proposal by ID */
  async getAIProposal(proposalId: number) {
    const proposal = await this.api.query.governance.aIProposals(proposalId);
    return (proposal as any).toJSON?.() ?? null;
  }

  /** Get governance config */
  async getConfig() {
    const config = await this.api.query.governance.governanceConfig();
    return (config as any).toJSON?.() ?? null;
  }
}
