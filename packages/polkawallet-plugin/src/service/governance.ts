/**
 * Governance service — proposals, voting, AI governance, kill switch
 *
 * Maps to pallet: governance
 * Extrinsics: submit_proposal, vote, delegate, fast_track,
 *             submit_ai_proposal, activate_kill_switch
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getProposal(proposalId: number) {
  const api = getApi();
  return api.query.governance.proposals(proposalId);
}

async function getAllProposals() {
  const api = getApi();
  const entries = await api.query.governance.proposals.entries();
  return entries.map(([key, val]: [any, any]) => ({
    id: key.args[0].toString(),
    ...val.toJSON(),
  }));
}

async function getActiveProposals() {
  const all = await getAllProposals();
  return all.filter((p: any) => p.status === 'Active');
}

async function getVotes(proposalId: number) {
  const api = getApi();
  const entries = await api.query.governance.votes.entries(proposalId);
  return entries.map(([key, val]: [any, any]) => ({
    voter: key.args[1].toHuman(),
    ...val.toJSON(),
  }));
}

async function getDelegations(account: string) {
  const api = getApi();
  return api.query.governance.delegations(account);
}

/* ─── Extrinsics ─── */

function submitProposal(callData: string, description: string) {
  const api = getApi();
  return api.tx.governance.submitProposal(callData, description);
}

function vote(proposalId: number, direction: 'Aye' | 'Nay' | 'Abstain', amount: string, conviction: number) {
  const api = getApi();
  return api.tx.governance.vote(proposalId, direction, amount, conviction);
}

function delegate(target: string, amount: string, conviction: number) {
  const api = getApi();
  return api.tx.governance.delegate(target, amount, conviction);
}

function fastTrack(proposalId: number) {
  const api = getApi();
  return api.tx.governance.fastTrack(proposalId);
}

/**
 * Submit an AI-generated governance proposal.
 */
function submitAiProposal(callData: string, description: string, aiModelId: string, confidence: number) {
  const api = getApi();
  return api.tx.governance.submitAiProposal(callData, description, aiModelId, confidence);
}

/**
 * Activate emergency kill switch (requires authority).
 */
function activateKillSwitch(reason: string) {
  const api = getApi();
  return api.tx.governance.activateKillSwitch(reason);
}

/**
 * Subscribe to proposal updates.
 */
async function subscribeProposal(proposalId: number, msgChannel: string) {
  const api = getApi();
  return api.query.governance.proposals(proposalId, (proposal: any) => {
    (window as any).send(msgChannel, {
      proposalId,
      ...proposal.toJSON(),
    });
  });
}

export default {
  getProposal,
  getAllProposals,
  getActiveProposals,
  getVotes,
  getDelegations,
  submitProposal,
  vote,
  delegate,
  fastTrack,
  submitAiProposal,
  activateKillSwitch,
  subscribeProposal,
};
