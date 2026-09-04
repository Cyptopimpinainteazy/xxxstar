const API = 'http://127.0.0.1:8765/api';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  return res.json();
}

export interface NetworkStatus {
  peers: number; syncing: boolean; bestBlock: number; chain: string;
  tokenSymbol: string; ss58Format: number; finalizedHead: string; rpcUrl: string;
}

export interface Block {
  number: number; hash: string; timestamp: string; txCount: number; producer: string;
}

export interface Transaction {
  hash: string; blockNumber: number; from: string; to: string; value: string;
  status: string; timestamp: string;
}

export interface Account {
  address: string; publicKey: string | null; keyType: string; balance: string;
  nonce: number; label: string | null; network: string; createdAt: string;
}

export interface Contract {
  address: string; name: string | null; owner: string | null; verified: boolean;
  compiler: string | null; sourcePath: string | null; txHash: string | null;
  deployedAt: string;
}

export interface FileEntry {
  name: string; path: string; type: 'file' | 'dir'; size: number;
}

export interface Template {
  name: string; filename: string; path: string; description: string;
  size: number; lines: number;
}

export interface ABIInfo {
  name: string; path: string; methods: { name: string; type: string; stateMutability: string }[];
  hasBytecode: boolean; abiCount: number;
}

export interface Project {
  id: number; name: string; path: string; template: string | null; createdAt: string;
}

export interface SearchResults {
  blocks: { number: number; hash: string }[];
  transactions: { hash: string; blockNumber: number }[];
  accounts: { address: string; label: string | null }[];
  contracts: { address: string; name: string | null }[];
}

export const api = {
  health: () => fetchJson<{ status: string }>(`${API}/health`),
  networkStatus: () => fetchJson<NetworkStatus>(`${API}/network/status`),

  rpc: (method: string, params: unknown[] = []) =>
    fetchJson<{ jsonrpc: string; id: number; result?: unknown; error?: { message: string } }>(
      `${API}/rpc`,
      { method: 'POST', body: JSON.stringify({ jsonrpc: '2.0', method, params, id: Date.now() }) }
    ),

  files: (path = '.') => fetchJson<FileEntry[]>(`${API}/files?path=${encodeURIComponent(path)}`),
  readFile: (path: string) => fetchJson<{ path: string; content: string; size: number }>(`${API}/files/read?path=${encodeURIComponent(path)}`),
  writeFile: (path: string, content: string) =>
    fetchJson<{ path: string; written: number }>(`${API}/files/write`, { method: 'POST', body: JSON.stringify({ path, content }) }),

  templates: () => fetchJson<Template[]>(`${API}/templates`),
  template: (name: string) => fetchJson<{ name: string; content: string; path: string }>(`${API}/templates/${encodeURIComponent(name)}`),
  scaffold: (template: string, projectName: string) =>
    fetchJson<{ name: string; path: string; template: string; files: string[] }>(
      `${API}/templates/scaffold`,
      { method: 'POST', body: JSON.stringify({ template, project_name: projectName }) }
    ),

  abis: () => fetchJson<ABIInfo[]>(`${API}/abis`),
  abi: (name: string) => fetchJson<{ name: string; abi: unknown[]; bytecode: unknown; deployedBytecode: unknown }>(
    `${API}/abis/${encodeURIComponent(name)}`
  ),

  projects: () => fetchJson<Project[]>(`${API}/projects`),
  compile: (code: string, language = 'x3') =>
    fetchJson<{ success: boolean; output: string; errors: string; warnings: string }>(
      `${API}/compile`,
      { method: 'POST', body: JSON.stringify({ code, language }) }
    ),

  generateKey: (keyType = 'ed25519', label = '') =>
    fetchJson<{ address: string; publicKey: string; label: string; keyType: string; seed: string }>(
      `${API}/keys/generate`,
      { method: 'POST', body: JSON.stringify({ key_type: keyType, label }) }
    ),

  deploy: (name: string, bytecode: string, fromAddress: string, abi = '[]') =>
    fetchJson<{ address: string; txHash: string; name: string; from: string }>(
      `${API}/contracts/deploy`,
      { method: 'POST', body: JSON.stringify({ name, bytecode, from_address: fromAddress, abi }) }
    ),

  buildTx: (from: string, to = '', value = '0', data = '0x') =>
    fetchJson<{ unsigned: Record<string, string>; rlp: string; hash: string }>(
      `${API}/tx/build`,
      { method: 'POST', body: JSON.stringify({ from_address: from, to, value, data }) }
    ),

  estimateGas: (from: string, to = '', data = '0x', value = '0') =>
    fetchJson<{ gasEstimate: number; error?: string }>(
      `${API}/tx/estimate`,
      { method: 'POST', body: JSON.stringify({ from_address: from, to, data, value }) }
    ),

  inspectBalance: (address: string) =>
    fetchJson<{ address: string; balance: string; balanceHex: string }>(
      `${API}/inspect/balance?address=${encodeURIComponent(address)}`
    ),
  inspectCode: (address: string) =>
    fetchJson<{ address: string; code: string; hasCode: boolean }>(
      `${API}/inspect/code?address=${encodeURIComponent(address)}`
    ),
  inspectStorage: (address: string, slot = '0x0') =>
    fetchJson<{ address: string; slot: string; value: string }>(
      `${API}/inspect/storage?address=${encodeURIComponent(address)}&slot=${encodeURIComponent(slot)}`
    ),

  getEvents: (address?: string, fromBlock = '0x0', toBlock = 'latest', topics: string[] = []) =>
    fetchJson<unknown[]>(
      `${API}/events`,
      { method: 'POST', body: JSON.stringify({ address: address || undefined, fromBlock, toBlock, topics }) }
    ),

  blocks: (limit = 20, offset = 0) =>
    fetchJson<Block[]>(`${API}/explorer/blocks?limit=${limit}&offset=${offset}`),
  block: (number: number) => fetchJson<Block>(`${API}/explorer/blocks/${number}`),
  transactions: (limit = 20, offset = 0) =>
    fetchJson<Transaction[]>(`${API}/explorer/transactions?limit=${limit}&offset=${offset}`),
  transaction: (hash: string) => fetchJson<Transaction>(`${API}/explorer/transactions/${hash}`),
  accounts: () => fetchJson<Account[]>(`${API}/accounts`),
  account: (address: string) => fetchJson<Account>(`${API}/accounts/${address}`),
  contracts: () => fetchJson<Contract[]>(`${API}/contracts`),
  swarmHealth: () => fetchJson<Record<string, unknown>>(`${API}/swarm/health`),
  search: (q: string) => fetchJson<SearchResults>(`${API}/search?q=${encodeURIComponent(q)}`),
};
