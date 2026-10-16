/**
 * Substrate Query Functions for Atlas Sphere
 * 
 * Provides typed query functions for interacting with the Atlas Sphere blockchain.
 */

import { getApi } from './client';
import type { Header, SignedBlock } from '@polkadot/types/interfaces';
import type { Codec } from '@polkadot/types/types';

// ============================================================================
// Types
// ============================================================================

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

export interface ComitInfo {
  comitId: string;
  origin: string;
  evmPayloadSize: number;
  svmPayloadSize: number;
  nonce: number;
  fee: string;
  prepareRoot: string;
  status: 'submitted' | 'executed' | 'finalized' | 'failed';
  blockNumber?: number;
  timestamp?: number;
  gasUsed?: number;
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

// ============================================================================
// Network Queries
// ============================================================================

/**
 * Get comprehensive network statistics
 */
export async function getNetworkStats(): Promise<NetworkStats> {
  const api = await getApi();
  
  const [
    chain,
    nodeName,
    nodeVersion,
    header,
    health,
    authorities,
  ] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version(),
    api.rpc.chain.getHeader(),
    api.rpc.system.health(),
    api.query.atlasKernel?.authorities?.() || api.query.aura?.authorities?.(),
  ]);

  // Get timestamp from the latest block
  const block = await api.rpc.chain.getBlock(header.hash);
  const timestamp = extractTimestamp(block);

  return {
    chain: chain.toString(),
    nodeName: nodeName.toString(),
    nodeVersion: nodeVersion.toString(),
    blockNumber: header.number.toNumber(),
    blockHash: header.hash.toHex(),
    timestamp,
    peerCount: health.peers.toNumber(),
    isSyncing: health.isSyncing.isTrue,
    authorityCount: (authorities as unknown as { length?: number })?.length || 0,
  };
}

/**
 * Get the latest finalized block number
 */
export async function getFinalizedBlockNumber(): Promise<number> {
  const api = await getApi();
  const hash = await api.rpc.chain.getFinalizedHead();
  const header = await api.rpc.chain.getHeader(hash);
  return header.number.toNumber();
}

// ============================================================================
// Block Queries
// ============================================================================

/**
 * Get block information by number or hash
 */
export async function getBlock(blockId: number | string): Promise<BlockInfo | null> {
  const api = await getApi();
  
  let hash: string;
  if (typeof blockId === 'number') {
    const blockHash = await api.rpc.chain.getBlockHash(blockId);
    hash = blockHash.toHex();
  } else {
    hash = blockId;
  }

  const [signedBlock, header] = await Promise.all([
    api.rpc.chain.getBlock(hash),
    api.rpc.chain.getHeader(hash),
  ]);

  if (!signedBlock || !header) return null;

  const timestamp = extractTimestamp(signedBlock);
  const author = extractAuthor(header);

  return {
    number: header.number.toNumber(),
    hash: header.hash.toHex(),
    parentHash: header.parentHash.toHex(),
    stateRoot: header.stateRoot.toHex(),
    extrinsicsRoot: header.extrinsicsRoot.toHex(),
    timestamp,
    author,
    extrinsicsCount: signedBlock.block.extrinsics.length,
  };
}

/**
 * Get recent blocks
 */
export async function getRecentBlocks(count: number = 10): Promise<BlockInfo[]> {
  const api = await getApi();
  const header = await api.rpc.chain.getHeader();
  const currentBlock = header.number.toNumber();
  
  const blocks: BlockInfo[] = [];
  const startBlock = Math.max(0, currentBlock - count + 1);

  for (let i = currentBlock; i >= startBlock; i--) {
    const block = await getBlock(i);
    if (block) blocks.push(block);
  }

  return blocks;
}

/**
 * Subscribe to new block headers
 */
export async function subscribeNewHeads(
  callback: (header: Header) => void
): Promise<() => void> {
  const api = await getApi();
  const unsub = await api.rpc.chain.subscribeNewHeads(callback);
  return unsub;
}

// ============================================================================
// Extrinsic Queries
// ============================================================================

/**
 * Get extrinsics from a specific block
 */
export async function getBlockExtrinsics(blockId: number | string): Promise<ExtrinsicInfo[]> {
  const api = await getApi();
  
  let hash: string;
  let blockNumber: number;
  
  if (typeof blockId === 'number') {
    const blockHash = await api.rpc.chain.getBlockHash(blockId);
    hash = blockHash.toHex();
    blockNumber = blockId;
  } else {
    hash = blockId;
    const header = await api.rpc.chain.getHeader(hash);
    blockNumber = header.number.toNumber();
  }

  const signedBlock = await api.rpc.chain.getBlock(hash);
  const timestamp = extractTimestamp(signedBlock);
  const events = await api.query.system.events.at(hash);

  const extrinsics: ExtrinsicInfo[] = signedBlock.block.extrinsics.map((ext, index) => {
    const { method, section } = ext.method;
    const extrinsicEvents = (events as unknown as Array<{ phase: { asApplyExtrinsic?: { toNumber: () => number } } }>)
      .filter((e) => e.phase.asApplyExtrinsic?.toNumber() === index);
    
    const success = !extrinsicEvents.some(
      (e) => (e as unknown as { event: { section: string; method: string } }).event?.section === 'system' && 
             (e as unknown as { event: { section: string; method: string } }).event?.method === 'ExtrinsicFailed'
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

  return extrinsics;
}

/**
 * Get recent extrinsics across multiple blocks
 */
export async function getRecentExtrinsics(count: number = 20): Promise<ExtrinsicInfo[]> {
  const api = await getApi();
  const header = await api.rpc.chain.getHeader();
  const currentBlock = header.number.toNumber();

  const extrinsics: ExtrinsicInfo[] = [];
  let blocksChecked = 0;
  const maxBlocks = 50; // Limit search depth

  while (extrinsics.length < count && blocksChecked < maxBlocks) {
    const blockNumber = currentBlock - blocksChecked;
    if (blockNumber < 0) break;

    const blockExts = await getBlockExtrinsics(blockNumber);
    // Filter out timestamp inherents and unsigned system extrinsics
    const filtered = blockExts.filter(
      (ext) => ext.signer || (ext.section !== 'timestamp' && ext.section !== 'paraInherent')
    );
    extrinsics.push(...filtered);
    blocksChecked++;
  }

  return extrinsics.slice(0, count);
}

// ============================================================================
// Atlas Kernel Queries
// ============================================================================

/**
 * Get Comit information by ID
 */
export async function getComit(comitId: string): Promise<ComitInfo | null> {
  const api = await getApi();
  
  // Query SubmittedComits storage to check if comit exists
  const submittedAt = await api.query.atlasKernel?.submittedComits?.(comitId);
  
  if (!submittedAt || (submittedAt as unknown as { isNone?: boolean }).isNone) {
    return null;
  }

  // For now, return basic info from storage
  // Full comit details would require indexer or event scanning
  const blockNumber = (submittedAt as unknown as { unwrap?: () => { toNumber: () => number } }).unwrap?.().toNumber();

  return {
    comitId,
    origin: '', // Would need event data
    evmPayloadSize: 0,
    svmPayloadSize: 0,
    nonce: 0,
    fee: '0',
    prepareRoot: '',
    status: 'submitted',
    blockNumber,
  };
}

/**
 * Get authorized accounts for Comit submission
 */
export async function getAuthorizedAccounts(): Promise<string[]> {
  const api = await getApi();
  
  // Use RPC method if available
  try {
    const accounts = await (api.rpc as unknown as { atlasKernel: { getAuthorizedAccounts: () => Promise<Codec> } }).atlasKernel.getAuthorizedAccounts();
    return (accounts as unknown as { map: (fn: (a: Codec) => string) => string[] }).map((a: Codec) => a.toString());
  } catch {
    // Fallback: iterate storage (less efficient)
    const entries = await api.query.atlasKernel?.authorizedAccounts?.entries?.();
    return (entries as unknown as Array<[{ args: Codec[] }]>)?.map(([key]) => key.args[0].toString()) || [];
  }
}

/**
 * Get current authority set
 */
export async function getAuthorities(): Promise<ValidatorInfo[]> {
  const api = await getApi();
  
  // Try Atlas Kernel authorities first
  let authorities = await api.query.atlasKernel?.authorities?.();
  
  // Fallback to Aura authorities
  if (!authorities || (authorities as unknown as { length?: number }).length === 0) {
    authorities = await api.query.aura?.authorities?.();
  }

  if (!authorities) return [];

  return (authorities as unknown as Codec[]).map((auth) => ({
    address: auth.toString(),
    isActive: true,
  }));
}

/**
 * Check if an account is authorized
 */
export async function isAccountAuthorized(address: string): Promise<boolean> {
  const api = await getApi();
  
  try {
    const result = await (api.rpc as unknown as { atlasKernel: { isAuthorized: (addr: string) => Promise<Codec> } }).atlasKernel.isAuthorized(address);
    return (result as unknown as { isTrue?: boolean }).isTrue ?? false;
  } catch {
    // Fallback: check storage directly
    const entry = await api.query.atlasKernel?.authorizedAccounts?.(address);
    return !!(entry as unknown as { isSome?: boolean })?.isSome;
  }
}

/**
 * Get canonical balance for an account and asset
 */
export async function getCanonicalBalance(
  account: string,
  assetId: number
): Promise<string> {
  const api = await getApi();
  
  try {
    const balance = await (api.rpc as unknown as { atlasKernel: { getCanonicalBalance: (acc: string, asset: number) => Promise<Codec> } }).atlasKernel.getCanonicalBalance(account, assetId);
    return balance.toString();
  } catch {
    // Fallback: query storage directly
    const balance = await api.query.atlasKernel?.canonicalLedger?.(account, assetId);
    return balance?.toString() || '0';
  }
}

// ============================================================================
// Account Queries
// ============================================================================

/**
 * Get account information
 */
export async function getAccountInfo(address: string): Promise<AccountInfo | null> {
  const api = await getApi();
  
  try {
    const accountData = await api.query.system.account(address);
    const isAuthorized = await isAccountAuthorized(address);

    const data = (accountData as unknown as {
      nonce: { toNumber: () => number };
      consumers: { toNumber: () => number };
      providers: { toNumber: () => number };
      sufficients: { toNumber: () => number };
      data: {
        free: { toString: () => string };
        reserved: { toString: () => string };
        frozen: { toString: () => string };
      };
    });

    return {
      address,
      nonce: data.nonce.toNumber(),
      free: data.data.free.toString(),
      reserved: data.data.reserved.toString(),
      frozen: data.data.frozen.toString(),
      isAuthorized,
      consumers: data.consumers.toNumber(),
      providers: data.providers.toNumber(),
      sufficients: data.sufficients.toNumber(),
    };
  } catch (error) {
    console.error('Error fetching account info:', error);
    return null;
  }
}

/**
 * Get Comit nonce for an account
 */
export async function getAccountComitNonce(address: string): Promise<number> {
  const api = await getApi();
  const nonce = await api.query.atlasKernel?.nonces?.(address);
  return (nonce as unknown as { toNumber?: () => number })?.toNumber?.() || 0;
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Extract timestamp from a signed block
 */
function extractTimestamp(signedBlock: SignedBlock): number {
  for (const ext of signedBlock.block.extrinsics) {
    if (ext.method.section === 'timestamp' && ext.method.method === 'set') {
      const arg = ext.method.args[0];
      return Number((arg as unknown as { toBigInt?: () => bigint }).toBigInt?.() || arg.toString());
    }
  }
  return Date.now();
}

/**
 * Extract block author from header
 */
function extractAuthor(header: Header): string | null {
  // Aura consensus includes author in digest
  for (const log of header.digest.logs) {
    const logHuman = log.toHuman() as { PreRuntime?: [string, string] } | null;
    if (logHuman?.PreRuntime) {
      const [engine, data] = logHuman.PreRuntime;
      if (engine === 'aura') {
        // The data contains the authority index, would need to look up in authorities
        return `Authority-${parseInt(data, 16)}`;
      }
    }
  }
  return null;
}
