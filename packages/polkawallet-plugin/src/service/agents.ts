/**
 * Agent Accounts service — AI agent management
 *
 * Maps to pallet: agent-accounts
 * Extrinsics: register_agent, update_operator, update_permissions,
 *             suspend_agent, record_consumption, update_reputation
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getAgent(agentId: string) {
  const api = getApi();
  return api.query.agentAccounts.agents(agentId);
}

async function getAgentPermissions(agentId: string) {
  const api = getApi();
  return api.query.agentAccounts.permissions(agentId);
}

async function getAgentReputation(agentId: string) {
  const api = getApi();
  return api.query.agentAccounts.reputation(agentId);
}

async function getAllAgents() {
  const api = getApi();
  const entries = await api.query.agentAccounts.agents.entries();
  return entries.map(([key, val]: [any, any]) => ({
    agentId: key.args[0].toHuman(),
    ...val.toJSON(),
  }));
}

async function getMyAgents(operator: string) {
  const all = await getAllAgents();
  return all.filter((a: any) => a.operator === operator);
}

/* ─── Extrinsics ─── */

function registerAgent(
  name: string,
  permissions: {
    canTrade: boolean;
    canSubmitComit: boolean;
    canExecuteX3: boolean;
    canGovern: boolean;
    maxSpendPerBlock: string;
  }
) {
  const api = getApi();
  return api.tx.agentAccounts.registerAgent(name, permissions);
}

function updateOperator(agentId: string, newOperator: string) {
  const api = getApi();
  return api.tx.agentAccounts.updateOperator(agentId, newOperator);
}

function updatePermissions(
  agentId: string,
  permissions: {
    canTrade: boolean;
    canSubmitComit: boolean;
    canExecuteX3: boolean;
    canGovern: boolean;
    maxSpendPerBlock: string;
  }
) {
  const api = getApi();
  return api.tx.agentAccounts.updatePermissions(agentId, permissions);
}

function suspendAgent(agentId: string, reason: string) {
  const api = getApi();
  return api.tx.agentAccounts.suspendAgent(agentId, reason);
}

function recordConsumption(agentId: string, amount: string) {
  const api = getApi();
  return api.tx.agentAccounts.recordConsumption(agentId, amount);
}

function updateReputation(agentId: string, delta: number) {
  const api = getApi();
  return api.tx.agentAccounts.updateReputation(agentId, delta);
}

export default {
  getAgent,
  getAgentPermissions,
  getAgentReputation,
  getAllAgents,
  getMyAgents,
  registerAgent,
  updateOperator,
  updatePermissions,
  suspendAgent,
  recordConsumption,
  updateReputation,
};
