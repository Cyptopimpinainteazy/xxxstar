// src/core/tx-helper.ts
async function signAndSend(tx, account, statusCallback) {
  return new Promise((resolve, reject) => {
    const unsubPromise = tx.signAndSend(account, (result) => {
      const status = { status: "pending" };
      if (result.status.isInBlock) {
        status.status = "inBlock";
        status.blockHash = result.status.asInBlock.toHex();
        status.txHash = result.txHash.toHex();
        statusCallback?.(status);
      }
      if (result.status.isFinalized) {
        const blockHash = result.status.asFinalized.toHex();
        const events = result.events.map((record) => ({
          type: `${record.event.section}.${record.event.method}`,
          data: record.event.data.toJSON()
        }));
        const dispatchError = result.events.find(
          ({ event }) => event.section === "system" && event.method === "ExtrinsicFailed"
        );
        if (dispatchError) {
          const errorStatus = {
            status: "error",
            blockHash,
            txHash: result.txHash.toHex(),
            error: "ExtrinsicFailed",
            events
          };
          statusCallback?.(errorStatus);
          reject(new Error(`Extrinsic failed in block ${blockHash}`));
          return;
        }
        const finalStatus = {
          status: "finalized",
          blockHash,
          txHash: result.txHash.toHex(),
          events
        };
        statusCallback?.(finalStatus);
        resolve({
          blockHash,
          blockNumber: 0,
          // populated by caller if needed
          txHash: result.txHash.toHex(),
          events
        });
      }
      if (result.isError) {
        const errorStatus = {
          status: "error",
          error: "Transaction error"
        };
        statusCallback?.(errorStatus);
        reject(new Error("Transaction error"));
      }
    });
    unsubPromise.catch((err) => {
      statusCallback?.({ status: "error", error: err.message });
      reject(err);
    });
  });
}

// src/services/governance.ts
var GovernanceService = class {
  constructor(api) {
    this.api = api;
  }
  // ---------------------------------------------------------------------------
  // Standard Governance
  // ---------------------------------------------------------------------------
  /** Submit a governance proposal */
  async submitProposal(account, params, statusCb) {
    const tx = this.api.tx.governance.submitProposal(
      params.call,
      new TextEncoder().encode(params.title),
      new TextEncoder().encode(params.description)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Vote on a proposal */
  async vote(account, params, statusCb) {
    const tx = this.api.tx.governance.vote(
      params.proposalId,
      params.direction,
      params.balance,
      params.conviction
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Delegate voting power */
  async delegate(account, params, statusCb) {
    const tx = this.api.tx.governance.delegate(params.target, params.conviction);
    return signAndSend(tx, account, statusCb);
  }
  /** Remove delegation */
  async undelegate(account, statusCb) {
    const tx = this.api.tx.governance.undelegate();
    return signAndSend(tx, account, statusCb);
  }
  /** Finalize a proposal after voting period ends */
  async finalizeProposal(account, proposalId, statusCb) {
    const tx = this.api.tx.governance.finalizeProposal(proposalId);
    return signAndSend(tx, account, statusCb);
  }
  /** Unlock tokens after conviction lock expires */
  async unlock(account, targetAccount, statusCb) {
    const tx = this.api.tx.governance.unlock(targetAccount);
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // AI Governance
  // ---------------------------------------------------------------------------
  /** Submit an AI governance proposal */
  async submitAIProposal(account, params, statusCb) {
    const tx = this.api.tx.governance.submitAiProposal(
      params.proposalType,
      typeof params.payload === "string" ? new TextEncoder().encode(params.payload) : params.payload,
      {
        risk_score: params.impactAssessment.riskScore,
        affected_pallets: params.impactAssessment.affectedPallets.map(
          (p) => new TextEncoder().encode(p)
        ),
        reversible: params.impactAssessment.reversible,
        estimated_gas: params.impactAssessment.estimatedGas
      },
      {
        min_simulation_blocks: params.simulationRequirements.minSimulationBlocks,
        required_coverage_percent: params.simulationRequirements.requiredCoveragePercent,
        max_state_changes: params.simulationRequirements.maxStateChanges
      }
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Activate the kill switch (emergency) */
  async activateKillSwitch(account, level, reason, statusCb) {
    const tx = this.api.tx.governance.activateKillSwitch(
      level,
      new TextEncoder().encode(reason)
    );
    return signAndSend(tx, account, statusCb);
  }
  /** Deactivate the kill switch */
  async deactivateKillSwitch(account, statusCb) {
    const tx = this.api.tx.governance.deactivateKillSwitch();
    return signAndSend(tx, account, statusCb);
  }
  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------
  /** Get proposal info */
  async getProposal(proposalId) {
    const proposal = await this.api.query.governance.proposals(proposalId);
    return proposal.toJSON?.() ?? null;
  }
  /** Get proposal tally (aye/nay/abstain counts) */
  async getProposalTally(proposalId) {
    const tally = await this.api.query.governance.proposalVotes(proposalId);
    return tally.toJSON?.() ?? null;
  }
  /** Get all active proposals */
  async getActiveProposals() {
    const count = await this.api.query.governance.proposalCount();
    const total = count.toNumber?.() ?? 0;
    const active = [];
    for (let i = 0; i < total; i++) {
      const proposal = await this.api.query.governance.proposals(i);
      const json = proposal.toJSON?.();
      if (json?.status === "Voting") {
        active.push(i);
      }
    }
    return active;
  }
  /** Get delegation info for an account */
  async getDelegation(account) {
    const delegation = await this.api.query.governance.delegations(account);
    return delegation.toJSON?.() ?? null;
  }
  /** Get current kill switch level */
  async getKillSwitchLevel() {
    const level = await this.api.query.governance.killSwitchLevelStorage();
    return level.toString();
  }
  /** Get AI proposal by ID */
  async getAIProposal(proposalId) {
    const proposal = await this.api.query.governance.aIProposals(proposalId);
    return proposal.toJSON?.() ?? null;
  }
  /** Get governance config */
  async getConfig() {
    const config = await this.api.query.governance.governanceConfig();
    return config.toJSON?.() ?? null;
  }
};

export { GovernanceService };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map