/**
 * Evolution Core service — genetic algorithm engine, mutations, AI agents
 *
 * Maps to pallet: evolution-core
 * Extrinsics: propose_mutation, approve_mutation, record_metrics,
 *             register_ai_agent, emergency_stop, rollback_mutation
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getEvolutionStatus() {
  const api = getApi();
  return (api.rpc as any).evolutionCore.getEvolutionStatus();
}

async function getBlockMetrics(blockNumber: number) {
  const api = getApi();
  return (api.rpc as any).evolutionCore.getBlockMetrics(blockNumber);
}

async function getMutation(mutationId: number) {
  const api = getApi();
  return api.query.evolutionCore.mutations(mutationId);
}

async function getAllMutations() {
  const api = getApi();
  const entries = await api.query.evolutionCore.mutations.entries();
  return entries.map(([key, val]: [any, any]) => ({
    id: key.args[0].toString(),
    ...val.toJSON(),
  }));
}

async function getActiveMutations() {
  const all = await getAllMutations();
  return all.filter((m: any) => m.status === 'Proposed' || m.status === 'Approved');
}

async function getRegisteredAgents() {
  const api = getApi();
  const entries = await api.query.evolutionCore.aiAgents.entries();
  return entries.map(([key, val]: [any, any]) => ({
    agentId: key.args[0].toHuman(),
    ...val.toJSON(),
  }));
}

/* ─── Extrinsics ─── */

function proposeMutation(description: string, parameters: string) {
  const api = getApi();
  return api.tx.evolutionCore.proposeMutation(description, parameters);
}

function approveMutation(mutationId: number) {
  const api = getApi();
  return api.tx.evolutionCore.approveMutation(mutationId);
}

function recordMetrics(metricsData: string) {
  const api = getApi();
  return api.tx.evolutionCore.recordMetrics(metricsData);
}

function registerAiAgent(name: string, capabilities: string[], operator: string) {
  const api = getApi();
  return api.tx.evolutionCore.registerAiAgent(name, capabilities, operator);
}

function emergencyStop(reason: string) {
  const api = getApi();
  return api.tx.evolutionCore.emergencyStop(reason);
}

function rollbackMutation(mutationId: number, reason: string) {
  const api = getApi();
  return api.tx.evolutionCore.rollbackMutation(mutationId, reason);
}

export default {
  getEvolutionStatus,
  getBlockMetrics,
  getMutation,
  getAllMutations,
  getActiveMutations,
  getRegisteredAgents,
  proposeMutation,
  approveMutation,
  recordMetrics,
  registerAiAgent,
  emergencyStop,
  rollbackMutation,
};
