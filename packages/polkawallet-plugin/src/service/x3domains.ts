/**
 * X3 Domain Registry service — .x3 domain names
 *
 * Maps to pallet: x3-domain-registry
 * Provides registration, resolution, and DNS record management for .x3 domains.
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getDomain(domain: string) {
  const api = getApi();
  return (api.rpc as any).x3Domains.getDomain(domain);
}

async function getRecords(domain: string) {
  const api = getApi();
  return (api.rpc as any).x3Domains.getRecords(domain);
}

async function listDomains() {
  const api = getApi();
  return (api.rpc as any).x3Domains.listDomains();
}

async function getOwnedDomains(account: string) {
  const api = getApi();
  const entries = await api.query.x3DomainRegistry.domains.entries();
  return entries
    .map(([key, val]: [any, any]) => ({
      name: key.args[0].toHuman(),
      ...val.toJSON(),
    }))
    .filter((d: any) => d.owner === account);
}

/**
 * Resolve a .x3 domain to its primary address (X3ADDR record).
 */
async function resolve(domain: string) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON() as any[];
  const x3addr = recordList.find((r: any) => r.record_type === 'X3ADDR');
  return x3addr ? x3addr.value : null;
}

/**
 * Resolve a .x3 domain to its EVM address.
 */
async function resolveEvm(domain: string) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON() as any[];
  const evmAddr = recordList.find((r: any) => r.record_type === 'EVMADDR');
  return evmAddr ? evmAddr.value : null;
}

/**
 * Resolve a .x3 domain to its SVM address.
 */
async function resolveSvm(domain: string) {
  const records = await getRecords(domain);
  if (!records) return null;
  const recordList = records.toJSON() as any[];
  const svmAddr = recordList.find((r: any) => r.record_type === 'SVMADDR');
  return svmAddr ? svmAddr.value : null;
}

/* ─── Extrinsics ─── */

/**
 * Register a .x3 domain.
 */
function registerDomain(domain: string, duration: number) {
  const api = getApi();
  return api.tx.x3DomainRegistry.registerDomain(domain, duration);
}

/**
 * Set a DNS record for a .x3 domain.
 */
function setRecord(domain: string, recordType: string, value: string) {
  const api = getApi();
  return api.tx.x3DomainRegistry.setRecord(domain, recordType, value);
}

/**
 * Remove a DNS record from a .x3 domain.
 */
function removeRecord(domain: string, recordType: string) {
  const api = getApi();
  return api.tx.x3DomainRegistry.removeRecord(domain, recordType);
}

/**
 * Transfer domain ownership.
 */
function transferDomain(domain: string, newOwner: string) {
  const api = getApi();
  return api.tx.x3DomainRegistry.transferDomain(domain, newOwner);
}

/**
 * Renew a .x3 domain registration.
 */
function renewDomain(domain: string, additionalDuration: number) {
  const api = getApi();
  return api.tx.x3DomainRegistry.renewDomain(domain, additionalDuration);
}

/**
 * Set the resolver for a .x3 domain.
 */
function setResolver(domain: string, resolver: string) {
  const api = getApi();
  return api.tx.x3DomainRegistry.setResolver(domain, resolver);
}

export default {
  getDomain,
  getRecords,
  listDomains,
  getOwnedDomains,
  resolve,
  resolveEvm,
  resolveSvm,
  registerDomain,
  setRecord,
  removeRecord,
  transferDomain,
  renewDomain,
  setResolver,
};
