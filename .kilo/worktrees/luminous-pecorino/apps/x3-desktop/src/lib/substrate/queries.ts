/**
 * Substrate Query Functions (ported from explorer)
 */

import { getApi } from './client';
import type { Header, SignedBlock } from '@polkadot/types/interfaces';
import type { Codec } from '@polkadot/types/types';

export interface BlockInfo {
  number: number;
  hash: string;
  parentHash: string;
  stateRoot: string;
  extrinsicsRoot: string;
  timestamp: number;
  author: string | null;
  extrinsicsCount: number;
}

export interface ExtrinsicInfo {
  hash: string;
  index: number;
  blockNumber: number;
  blockHash: string;
  section: string;
  method: string;
  args: Record<string, unknown>;
  signer: string | null;
  success: boolean;
  timestamp: number;
  fee?: string;
}

export interface NetworkStats {
  chain: string;
  nodeName: string;
  nodeVersion: string;
  blockNumber: number;
  blockHash: string;
  timestamp: number;
  peerCount: number;
  isSyncing: boolean;
  totalIssuance?: string;
  authorityCount: number;
}

export interface AccountInfo {
  address: string;
  nonce: number;
  free: string;
  reserved: string;
  frozen: string;
  isAuthorized: boolean;
  consumers: number;
  providers: number;
  sufficients: number;
}

export interface ValidatorInfo {
  address: string;
  isActive: boolean;
  isCurrentAuthor?: boolean;
  blocksProduced?: number;
}

/* ── Network ──────────────────────────────────────────────── */

export async function getNetworkStats(): Promise<NetworkStats> {
  const api = await getApi();
  const [chain, nodeName, nodeVersion, header, health, authorities] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version(),
    api.rpc.chain.getHeader(),
    api.rpc.system.health(),
    api.query.atlasKernel?.authorities?.() || api.query.aura?.authorities?.(),
  ]);
  const block = await api.rpc.chain.getBlock(header.hash);
  return {
    chain: chain.toString(),
    nodeName: nodeName.toString(),
    nodeVersion: nodeVersion.toString(),
    blockNumber: header.number.toNumber(),
    blockHash: header.hash.toHex(),
    timestamp: extractTimestamp(block),
    peerCount: health.peers.toNumber(),
    isSyncing: health.isSyncing.isTrue,
    authorityCount: (authorities as unknown as { length?: number })?.length || 0,
  };
}

/* ── Blocks ───────────────────────────────────────────────── */

export async function getBlock(blockId: number | string): Promise<BlockInfo | null> {
  const api = await getApi();
  let hash: string;
  if (typeof blockId === 'number') {
    const bh = await api.rpc.chain.getBlockHash(blockId);
    hash = bh.toHex();
  } else {
    hash = blockId;
  }
  const [signedBlock, header] = await Promise.all([
    api.rpc.chain.getBlock(hash),
    api.rpc.chain.getHeader(hash),
  ]);
  if (!signedBlock || !header) return null;
  return {
    number: header.number.toNumber(),
    hash: header.hash.toHex(),
    parentHash: header.parentHash.toHex(),
    stateRoot: header.stateRoot.toHex(),
    extrinsicsRoot: header.extrinsicsRoot.toHex(),
    timestamp: extractTimestamp(signedBlock),
    author: extractAuthor(header),
    extrinsicsCount: signedBlock.block.extrinsics.length,
  };
}

export async function getRecentBlocks(count = 10): Promise<BlockInfo[]> {
  const api = await getApi();
  const header = await api.rpc.chain.getHeader();
  const cur = header.number.toNumber();
  const blocks: BlockInfo[] = [];
  for (let i = cur; i >= Math.max(0, cur - count + 1); i--) {
    const b = await getBlock(i);
    if (b) blocks.push(b);
  }
  return blocks;
}

export async function subscribeNewHeads(callback: (h: Header) => void): Promise<() => void> {
  const api = await getApi();
  return api.rpc.chain.subscribeNewHeads(callback);
}

/* ── Extrinsics ───────────────────────────────────────────── */

export async function getBlockExtrinsics(blockId: number | string): Promise<ExtrinsicInfo[]> {
  const api = await getApi();
  let hash: string, blockNumber: number;
  if (typeof blockId === 'number') {
    const bh = await api.rpc.chain.getBlockHash(blockId);
    hash = bh.toHex();
    blockNumber = blockId;
  } else {
    hash = blockId;
    const h = await api.rpc.chain.getHeader(hash);
    blockNumber = h.number.toNumber();
  }
  const signedBlock = await api.rpc.chain.getBlock(hash);
  const timestamp = extractTimestamp(signedBlock);
  const events = await api.query.system.events.at(hash);
  return signedBlock.block.extrinsics.map((ext, index) => {
    const { method, section } = ext.method;
    const extrinsicEvents = (events as unknown as Array<{ phase: { asApplyExtrinsic?: { toNumber: () => number } } }>)
      .filter((e) => e.phase.asApplyExtrinsic?.toNumber() === index);
    const success = !extrinsicEvents.some(
      (e) =>
        (e as unknown as { event: { section: string; method: string } }).event?.section === 'system' &&
        (e as unknown as { event: { section: string; method: string } }).event?.method === 'ExtrinsicFailed',
    );
    return {
      hash: ext.hash.toHex(),
      index,
      blockNumber,
      blockHash: hash,
      section,
      method,
      args: ext.method.args.reduce((acc, arg, i) => {
        acc[`arg${i}`] = arg.toHuman();
        return acc;
      }, {} as Record<string, unknown>),
      signer: ext.signer?.toString() || null,
      success,
      timestamp,
    };
  });
}

export async function getRecentExtrinsics(count = 20): Promise<ExtrinsicInfo[]> {
  const api = await getApi();
  const header = await api.rpc.chain.getHeader();
  const cur = header.number.toNumber();
  const extrinsics: ExtrinsicInfo[] = [];
  let checked = 0;
  while (extrinsics.length < count && checked < 50) {
    const bn = cur - checked;
    if (bn < 0) break;
    const be = await getBlockExtrinsics(bn);
    extrinsics.push(...be.filter((e) => e.signer || (e.section !== 'timestamp' && e.section !== 'paraInherent')));
    checked++;
  }
  return extrinsics.slice(0, count);
}

/* ── X3 Kernel ─────────────────────────────────────────── */

export async function getAuthorizedAccounts(): Promise<string[]> {
  const api = await getApi();
  try {
    const accounts = await (api.rpc as any).atlasKernel.getAuthorizedAccounts();
    return (accounts as any).map((a: Codec) => a.toString());
  } catch {
    const entries = await api.query.atlasKernel?.authorizedAccounts?.entries?.();
    return (entries as any)?.map(([key]: any) => key.args[0].toString()) || [];
  }
}

export async function getAuthorities(): Promise<ValidatorInfo[]> {
  const api = await getApi();
  let authorities = await api.query.atlasKernel?.authorities?.();
  if (!authorities || (authorities as any).length === 0) {
    authorities = await api.query.aura?.authorities?.();
  }
  if (!authorities) return [];
  return (authorities as unknown as Codec[]).map((auth) => ({ address: auth.toString(), isActive: true }));
}

export async function isAccountAuthorized(address: string): Promise<boolean> {
  const api = await getApi();
  try {
    const result = await (api.rpc as any).atlasKernel.isAuthorized(address);
    return result.isTrue ?? false;
  } catch {
    const entry = await api.query.atlasKernel?.authorizedAccounts?.(address);
    return !!(entry as any)?.isSome;
  }
}

export async function getCanonicalBalance(account: string, assetId: number): Promise<string> {
  const api = await getApi();
  try {
    const balance = await (api.rpc as any).atlasKernel.getCanonicalBalance(account, assetId);
    return balance.toString();
  } catch {
    const balance = await api.query.atlasKernel?.canonicalLedger?.(account, assetId);
    return balance?.toString() || '0';
  }
}

export async function getAccountInfo(address: string): Promise<AccountInfo | null> {
  const api = await getApi();
  try {
    const accountData = await api.query.system.account(address);
    const authorized = await isAccountAuthorized(address);
    const data = accountData as any;
    return {
      address,
      nonce: data.nonce.toNumber(),
      free: data.data.free.toString(),
      reserved: data.data.reserved.toString(),
      frozen: data.data.frozen.toString(),
      isAuthorized: authorized,
      consumers: data.consumers.toNumber(),
      providers: data.providers.toNumber(),
      sufficients: data.sufficients.toNumber(),
    };
  } catch (e) {
    console.error('Error fetching account info:', e);
    return null;
  }
}

/* ── Helpers ──────────────────────────────────────────────── */

export interface RealRpcStats {
  total_requests: number;
  total_rejected: number;
  active_connections: number;
}

export async function fetchRpcStats(): Promise<RealRpcStats | null> {
  try {
    const api = await getApi();
    const data = await (api.rpc as any).x3Node.getRateLimitMetrics();
    return {
      total_requests: Number(data.total_requests.toString()),
      total_rejected: Number(data.total_rejected.toString()),
      active_connections: Number(data.active_connections.toString()),
    };
  } catch (e) {
    console.warn('Error fetching RPC stats (may not be supported on this node):', e);
    return null;
  }
}

/* ── Helpers ──────────────────────────────────────────────── */

function extractTimestamp(signedBlock: SignedBlock): number {
  for (const ext of signedBlock.block.extrinsics) {
    if (ext.method.section === 'timestamp' && ext.method.method === 'set') {
      const arg = ext.method.args[0];
      return Number((arg as any).toBigInt?.() || arg.toString());
    }
  }
  return Date.now();
}

function extractAuthor(header: Header): string | null {
  for (const log of header.digest.logs) {
    const logHuman = log.toHuman() as { PreRuntime?: [string, string] } | null;
    if (logHuman?.PreRuntime) {
      const [engine, data] = logHuman.PreRuntime;
      if (engine === 'aura') return `Authority-${parseInt(data, 16)}`;
    }
  }
  return null;
}

/* ── Governance ───────────────────────────────────────────── */

export interface GovernanceProposal {
  id: number;
  title: string;
  description: string;
  proposer: string;
  status: 'Active' | 'Passed' | 'Rejected' | 'Cancelled' | 'Enacted';
  votingStart: number;
  votingEnd: number;
  threshold: number;
  ayes: number;
  nays: number;
  weight: string;
  touchesInvariants: boolean;
  proofCommitment?: string;
  constitutionHash?: string;
}

export interface GovernanceSnapshot {
  proposalCount: number;
  activeProposals: number;
  totalVoters: number;
  totalDelegations: number;
  config: {
    quorum: number;
    threshold: number;
    votingPeriod: number;
    enactmentPeriod: number;
  };
  /** Total tokens currently staked / locked for governance */
  totalStaked?: string;
  /** Unique voters who have cast at least one vote */
  voterCount?: number;
  /** Average participation rate across recent proposals (0–100) */
  avgParticipation?: number;
}

export interface Delegation {
  target: string;
  conviction: 'None' | 'Locked1x' | 'Locked2x' | 'Locked3x' | 'Locked4x' | 'Locked5x' | 'Locked6x';
  balance: string;
}

export interface TreasuryProposal {
  id: number;
  proposer: string;
  beneficiary: string;
  amount: string;
  description: string;
  status: 'Pending' | 'Approved' | 'Rejected' | 'Executed' | 'Cancelled';
  track: 'Small' | 'Medium' | 'Large' | 'Critical';
  bond: string;
  signers: string[];
  approvals: number;
  createdAt: number;
}

export interface TreasuryAllocation {
  category: string;
  amount: string;
  percentage: number;
  recipient?: string;
}

export interface TreasurySnapshot {
  proposalCount: number;
  pendingProposals: TreasuryProposal[];
  signers: string[];
  totalBalance: string;
  isPaused: boolean;
  stats: {
    totalSpent: string;
    recurringTotal: string;
  };
  /** Alias kept for backwards-compat with panels that use .proposals */
  proposals?: TreasuryProposal[];
  /** Treasury allocations per category or wallet */
  allocations?: TreasuryAllocation[];
  /** Total funds already allocated (human-readable amount string) */
  totalAllocated?: string;
}

export interface TreasuryWallet {
  address: string;
  balance: string;
  name: string;
}

export async function getGovernanceSnapshot(): Promise<GovernanceSnapshot | null> {
  const api = await getApi();
  try {
    const [proposalCount, proposals, config] = await Promise.all([
      api.query.governance.proposalCount(),
      api.query.governance.proposals.entries(),
      api.query.governance.config(),
    ]);

    const activeProposals = proposals.filter(([, p]: any) => {
      const proposal = p.unwrap();
      return proposal.status === 'Active' || proposal.status === 'Voting';
    }).length;

    // Count unique voters from all proposals
    const voters = new Set<string>();
    for (const [key, value] of proposals) {
      const votes = await api.query.governance.proposalVotes((key.args[0] as any).toNumber()) as any;
      if (votes.ayes) {
        votes.ayes.forEach((v: any) => voters.add(v.toString()));
      }
      if (votes.nays) {
        votes.nays.forEach((v: any) => voters.add(v.toString()));
      }
    }

    const delegations = await api.query.governance.delegations.entries();
    const configData = (config as any).unwrap();

    return {
      proposalCount: (proposalCount as any).toNumber(),
      activeProposals: activeProposals,
      totalVoters: voters.size,
      totalDelegations: (delegations as any[]).length,
      config: {
        quorum: Number(configData.quorum.toString()) / 100,
        threshold: Number(configData.threshold.toString()) / 100,
        votingPeriod: Number(configData.votingPeriod.toString()),
        enactmentPeriod: Number(configData.enactmentPeriod.toString()),
      },
    };
  } catch (e) {
    console.error('Error fetching governance snapshot:', e);
    return null;
  }
}

export async function getProposalList(): Promise<GovernanceProposal[]> {
  const api = await getApi();
  try {
    const entries = await api.query.governance.proposals.entries();
    const proposals: GovernanceProposal[] = [];

    for (const [key, value] of entries) {
      const proposalId = (key.args[0] as any).toNumber();
      const proposal = (value as any).unwrap();
      const votes = await api.query.governance.proposalVotes(proposalId) as any;

      proposals.push({
        id: proposalId,
        title: proposal.title.toString(),
        description: proposal.description.toString(),
        proposer: proposal.proposer.toString(),
        status: proposal.status.toString() as GovernanceProposal['status'],
        votingStart: Number(proposal.voting_start.toString()),
        votingEnd: Number(proposal.voting_end.toString()),
        threshold: Number(proposal.threshold.toString()),
        ayes: Number(votes.ayes?.length || 0),
        nays: Number(votes.nays?.length || 0),
        weight: proposal.weight.toString(),
        touchesInvariants: proposal.touches_invariants,
        proofCommitment: proposal.proof_commitment?.toString(),
        constitutionHash: proposal.constitution_hash?.toString(),
      });
    }

    return proposals;
  } catch (e) {
    console.error('Error fetching proposal list:', e);
    return [];
  }
}

export async function getProposalTally(proposalId: number): Promise<{ ayes: number; nays: number; total: number } | null> {
  const api = await getApi();
  try {
    const votes = await api.query.governance.proposalVotes(proposalId) as any;
    const ayes = Number((votes as any).ayes?.length || 0);
    const nays = Number((votes as any).nays?.length || 0);
    return { ayes, nays, total: ayes + nays };
  } catch (e) {
    console.error(`Error fetching proposal ${proposalId} tally:`, e);
    return null;
  }
}

export async function getDelegation(address: string): Promise<Delegation | null> {
  const api = await getApi();
  try {
    const delegation = await api.query.governance.delegations(address) as any;
    if (!delegation.isSome) return null;

    const del = delegation.unwrap();
    return {
      target: del.target.toString(),
      conviction: del.conviction.toString() as Delegation['conviction'],
      balance: del.balance.toString(),
    };
  } catch (e) {
    console.error(`Error fetching delegation for ${address}:`, e);
    return null;
  }
}

export async function getTreasurySnapshot(): Promise<TreasurySnapshot | null> {
  const api = await getApi();
  try {
    const [proposals, signers, isPaused] = await Promise.all([
      api.query.treasury.proposals.entries(),
      api.query.treasury.signers() as unknown as any[],
      api.query.treasury.isPaused() as any,
    ]);

    const treasuryAccount = (await import('./client')).getTreasuryAccountId();

    const treasuryProposals: TreasuryProposal[] = [];
    for (const [key, value] of proposals) {
      const proposalId = (key.args[0] as any).toNumber();
      const proposal = (value as any).unwrap();

      treasuryProposals.push({
        id: proposalId,
        proposer: proposal.proposer.toString(),
        beneficiary: proposal.beneficiary.toString(),
        amount: proposal.amount.toString(),
        description: proposal.description.toString(),
        status: proposal.status.toString() as TreasuryProposal['status'],
        track: proposal.track.toString() as TreasuryProposal['track'],
        bond: proposal.bond.toString(),
        signers: proposal.signers.map((s: any) => s.toString()),
        approvals: proposal.approvals.toNumber(),
        createdAt: Number(proposal.created_at.toString()),
      });
    }

    return {
      proposalCount: proposals.length,
      pendingProposals: treasuryProposals.filter(p => p.status === 'Pending' || p.status === 'Approved'),
      signers: signers.map((s: any) => s.toString()),
      totalBalance: '0', // Will be fetched separately
      isPaused: isPaused.isTrue,
      stats: {
        totalSpent: '0',
        recurringTotal: '0',
      },
    };
  } catch (e) {
    console.error('Error fetching treasury snapshot:', e);
    return null;
  }
}

export async function getTreasuryBalance(): Promise<string> {
  const api = await getApi();
  try {
    const treasuryAccountId = (await import('./client')).getTreasuryAccountId();
    const accountData = await api.query.system.account(treasuryAccountId) as any;
    return accountData.data.free.toString();
  } catch (e) {
    console.error('Error fetching treasury balance:', e);
    return '0';
  }
}

export async function getTreasuryWallets(): Promise<TreasuryWallet[]> {
  const api = await getApi();
  try {
    const treasuryAccountId = (await import('./client')).getTreasuryAccountId();
    const accountData = await api.query.system.account(treasuryAccountId) as any;
    return [{
      address: treasuryAccountId,
      balance: accountData.data.free.toString(),
      name: 'Treasury Account',
    }];
  } catch (e) {
    console.error('Error fetching treasury wallets:', e);
    return [];
  }
}

export async function getAIProposals(): Promise<GovernanceProposal[]> {
  const api = await getApi();
  try {
    const entries = await api.query.governance.aiProposals.entries();
    const proposals: GovernanceProposal[] = [];

    for (const [key, value] of entries) {
      const proposalId = (key.args[0] as any).toNumber();
      const proposal = (value as any).unwrap();

      proposals.push({
        id: proposalId,
        title: proposal.title.toString(),
        description: proposal.description.toString(),
        proposer: proposal.proposer.toString(),
        status: proposal.status.toString() as GovernanceProposal['status'],
        votingStart: Number(proposal.voting_start.toString()),
        votingEnd: Number(proposal.voting_end.toString()),
        threshold: Number(proposal.threshold.toString()),
        ayes: 0,
        nays: 0,
        weight: proposal.weight.toString(),
        touchesInvariants: proposal.touches_invariants,
        proofCommitment: proposal.proof_commitment?.toString(),
        constitutionHash: proposal.constitution_hash?.toString(),
      });
    }

    return proposals;
  } catch (e) {
    console.error('Error fetching AI proposals:', e);
    return [];
  }
}

export async function getTopDelegates(count = 10): Promise<{ address: string; power: string }[]> {
  const api = await getApi();
  try {
    const delegations = await api.query.governance.delegations.entries();
    const powers: { [address: string]: string } = {};

    for (const [key, value] of delegations) {
      const delegator = key.args[0].toString();
      const del = (value as any).unwrap();
      const power = del.balance.toString();

      if (!powers[del.target.toString()]) {
        powers[del.target.toString()] = '0';
      }
      powers[del.target.toString()] = (BigInt(powers[del.target.toString()]) + BigInt(power)).toString();
    }

    return Object.entries(powers)
      .sort((a, b) => (BigInt(b[1]) > BigInt(a[1]) ? 1 : BigInt(b[1]) < BigInt(a[1]) ? -1 : 0))
      .slice(0, count)
      .map(([address, power]) => ({ address, power }));
  } catch (e) {
    console.error('Error fetching top delegates:', e);
    return [];
  }
}

/* ── Governance Extrinsics ─────────────────────────────────── */

export async function submitGovernanceProposal(
  signer: any,
  call: any,
  title: string,
  description: string,
  touchesInvariants: boolean = false,
  proofCommitment?: string,
  constitutionHash?: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const titleBytes = new TextEncoder().encode(title);
    const descriptionBytes = new TextEncoder().encode(description);

    const tx = api.tx.governance.submitProposal(
      call,
      titleBytes,
      descriptionBytes,
      touchesInvariants,
      proofCommitment ? new Uint8Array(Buffer.from(proofCommitment.replace('0x', ''), 'hex')) : null,
      constitutionHash ? new Uint8Array(Buffer.from(constitutionHash.replace('0x', ''), 'hex')) : null,
    );

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error submitting governance proposal:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function castVote(
  signer: any,
  proposalId: number,
  direction: 'Aye' | 'Nay',
  balance: string,
  conviction: 'None' | 'Locked1x' | 'Locked2x' | 'Locked3x' | 'Locked4x' | 'Locked5x' | 'Locked6x',
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.vote(
      proposalId,
      direction === 'Aye' ? { aye: balance } : { nay: balance },
      conviction,
    );

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error casting vote on proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function delegateVote(
  signer: any,
  target: string,
  conviction: 'None' | 'Locked1x' | 'Locked2x' | 'Locked3x' | 'Locked4x' | 'Locked5x' | 'Locked6x',
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.delegate(target, conviction);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error delegating vote:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function undelegateVote(signer: any): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.undelegate();

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error undelegating vote:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function fastTrackProposal(
  signer: any,
  proposalId: number,
  delay: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.fastTrack(proposalId, delay);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error fast-tracking proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function cancelProposal(
  signer: any,
  proposalId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.cancelProposal(proposalId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error cancelling proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function finalizeProposal(
  signer: any,
  proposalId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.finalizeProposal(proposalId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error finalizing proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function unlockVotes(signer: any, account: string): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.governance.unlock(account);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error unlocking votes for ${account}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

/* ── Treasury Extrinsics ───────────────────────────────────── */

export async function submitTreasuryProposal(
  signer: any,
  beneficiary: string,
  amount: string,
  description: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const descriptionBytes = new TextEncoder().encode(description);

    const tx = api.tx.treasury.submitProposal(
      beneficiary,
      amount,
      descriptionBytes,
    );

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error submitting treasury proposal:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function approveTreasuryProposal(
  signer: any,
  proposalId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.approveProposal(proposalId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error approving treasury proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function rejectTreasuryProposal(
  signer: any,
  proposalId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.rejectProposal(proposalId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error rejecting treasury proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function executeTreasuryProposal(
  signer: any,
  proposalId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.executeProposal(proposalId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error executing treasury proposal ${proposalId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function createRecurringPayment(
  signer: any,
  beneficiary: string,
  amount: string,
  interval: number,
  count?: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.createRecurringPayment(
      beneficiary,
      amount,
      interval,
      count || null,
    );

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error creating recurring payment:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function cancelRecurringPayment(
  signer: any,
  paymentId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.cancelRecurringPayment(paymentId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error cancelling recurring payment ${paymentId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function registerYieldStrategy(
  signer: any,
  agent: string,
  maxAllocation: string,
  profitShare: number,
  riskLevel: 'Low' | 'Medium' | 'High',
  description: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const descriptionBytes = new TextEncoder().encode(description);

    const tx = api.tx.treasury.registerYieldStrategy(
      agent,
      maxAllocation,
      profitShare,
      riskLevel,
      descriptionBytes,
    );

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error registering yield strategy:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function executeYieldStrategy(
  signer: any,
  strategyId: number,
  amount: string,
  maxIterations: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.executeYieldStrategy(strategyId, amount, maxIterations);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error executing yield strategy ${strategyId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function reportYieldReturn(
  signer: any,
  strategyId: number,
  totalReturn: string,
  principal: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.reportYieldReturn(strategyId, totalReturn, principal);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error reporting yield return for strategy ${strategyId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function deactivateYieldStrategy(
  signer: any,
  strategyId: number,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.deactivateYieldStrategy(strategyId);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error(`Error deactivating yield strategy ${strategyId}:`, e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function pauseTreasury(
  signer: any,
  reason: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const reasonBytes = new TextEncoder().encode(reason);

    const tx = api.tx.treasury.pause(reasonBytes);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error pausing treasury:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function unpauseTreasury(signer: any): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.unpause();

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error unpausing treasury:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function updateTreasurySigners(
  signer: any,
  signers: string[],
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.updateSigners(signers);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error updating treasury signers:', e);
    return { success: false, txHash: '', error: e.message };
  }
}

export async function depositToTreasury(
  signer: any,
  amount: string,
): Promise<{ success: boolean; txHash: string; error?: string }> {
  const api = await getApi();
  try {
    const tx = api.tx.treasury.deposit(amount);

    const result = await tx.signAndSend(signer, { nonce: -1 });
    return { success: true, txHash: result.hash.toString() };
  } catch (e: any) {
    console.error('Error depositing to treasury:', e);
    return { success: false, txHash: '', error: e.message };
  }
}
