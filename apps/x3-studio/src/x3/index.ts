export const X3_KEYWORDS = [
  'intent', 'chain', 'vm', 'route', 'lock', 'claim', 'refund',
  'finality', 'oracle', 'proof', 'solver', 'adapter', 'relayer',
  'quorum', 'timeout', 'slashing', 'scoreboard', 'require', 'emit',
  'asset', 'address', 'amount', 'deadline', 'validator', 'bridge',
  'swap', 'proof_ledger', 'htlc', 'secret_hash', 'preimage', 'settlement',
  'if', 'else', 'for', 'while', 'return', 'let', 'mut', 'fn',
  'struct', 'enum', 'match', 'import', 'from', 'as', 'true', 'false',
];

export const X3_TYPES = [
  'u8', 'u16', 'u32', 'u64', 'u128', 'i8', 'i16', 'i32', 'i64', 'i128',
  'f32', 'f64', 'bool', 'string', 'Address', 'Amount', 'Asset', 'Hash',
  'Signature', 'BlockNumber', 'IntentId', 'ChainId', 'VmId',
];

export const X3_SNIPPETS: Record<string, string[]> = {
  'cross-chain intent': [
    'intent swap {',
    '  chain "ethereum" -> "solana"',
    '  asset 0x... -> ...',
    '  amount 1000',
    '  deadline block+100',
    '  solver any',
    '}',
  ],
  'htlc': [
    'htlc {',
    '  sender: 0x...',
    '  receiver: 0x...',
    '  secret_hash: 0x...',
    '  timeout: block + 100',
    '  amount: 1000',
    '}',
    '',
    'fn claim(preimage: Hash) {',
    '  require(sha256(preimage) == secret_hash)',
    '  transfer(amount, receiver)',
    '  emit Claimed(receiver, amount)',
    '}',
  ],
  'solver order': [
    'solver_order {',
    '  id: "order_001"',
    '  source_chain: "ethereum"',
    '  dest_chain: "solana"',
    '  source_asset: 0x...',
    '  dest_asset: 0x...',
    '  min_return: 950',
    '  deadline: block + 50',
    '  fee_tier: 1',
    '}',
  ],
  'proof ledger write': [
    'proof write {',
    '  proof_hash: 0x...',
    '  command: "cargo check"',
    '  exit_code: 0',
    '  timestamp: block.timestamp',
    '  validator: msg.sender',
    '}',
  ],
  'relayer config': [
    'relayer_config {',
    '  name: "x3-relayer-1"',
    '  rpc_endpoint: "http://localhost:9944"',
    '  poll_interval: 6',
    '  confirmations: 12',
    '  max_batch: 10',
    '}',
  ],
  'validator task': [
    'validator_task {',
    '  id: "val_001"',
    '  chain: "x3"',
    '  check: "block_finality"',
    '  interval: 6',
    '  alert_on_failure: true',
    '}',
  ],
  'adapter definition': [
    'adapter "evm" {',
    '  chains: ["ethereum", "base", "arbitrum"]',
    '  lock: true',
    '  claim: true',
    '  refund: true',
    '  finality: 12',
    '  proof: true',
    '}',
  ],
  'rpc quorum': [
    'rpc_quorum {',
    '  chain: "ethereum"',
    '  endpoints: [',
    '    "https://eth-mainnet.alchemy.com/...",',
    '    "https://eth-mainnet.infura.io/...",',
    '  ]',
    '  quorum: 2',
    '  timeout: 5000',
    '}',
  ],
};

export function tokenizeX3(source: string): { type: string; value: string; line: number; col: number }[] {
  const tokens: { type: string; value: string; line: number; col: number }[] = [];
  const lines = source.split('\n');

  for (let lineNum = 0; lineNum < lines.length; lineNum++) {
    const line = lines[lineNum];
    let col = 0;
    while (col < line.length) {
      if (line[col] === ' ' || line[col] === '\t') { col++; continue; }
      if (line[col] === '/' && line[col + 1] === '/') { break; }
      if (line[col] === '/' && line[col + 1] === '*') {
        let end = line.indexOf('*/', col + 2);
        while (end === -1 && lineNum < lines.length - 1) {
          lineNum++; end = lines[lineNum].indexOf('*/');
        }
        tokens.push({ type: 'comment', value: source.substring(col, end + 2), line: lineNum + 1, col });
        col = end + 2;
        continue;
      }
      if (line[col] === '"') {
        const end = line.indexOf('"', col + 1);
        tokens.push({ type: 'string', value: line.substring(col, end + 1), line: lineNum + 1, col });
        col = end + 1;
        continue;
      }
      if (/[0-9]/.test(line[col])) {
        let num = '';
        while (col < line.length && /[0-9a-fA-Fx.]/.test(line[col])) { num += line[col]; col++; }
        tokens.push({ type: 'number', value: num, line: lineNum + 1, col });
        continue;
      }
      if (/[a-zA-Z_]/.test(line[col])) {
        let word = '';
        while (col < line.length && /[a-zA-Z0-9_]/.test(line[col])) { word += line[col]; col++; }
        const type = X3_KEYWORDS.includes(word) ? 'keyword' : X3_TYPES.includes(word) ? 'type' : 'identifier';
        tokens.push({ type, value: word, line: lineNum + 1, col });
        continue;
      }
      if (/[{}()\[\];,:]/.test(line[col])) {
        tokens.push({ type: 'punctuation', value: line[col], line: lineNum + 1, col });
        col++;
        continue;
      }
      if (/[+\-*/%=<>!&|^~]/.test(line[col])) {
        let op = line[col];
        if (col + 1 < line.length && /[=]/.test(line[col + 1])) { op += line[col + 1]; col++; }
        tokens.push({ type: 'operator', value: op, line: lineNum + 1, col });
        col++;
        continue;
      }
      col++;
    }
  }
  return tokens;
}

export function validateX3(source: string): { line: number; message: string; severity: 'error' | 'warning' }[] {
  const errors: { line: number; message: string; severity: 'error' | 'warning' }[] = [];
  const tokens = tokenizeX3(source);
  const lines = source.split('\n');

  // Check unmatched braces
  let braceCount = 0;
  for (const t of tokens) {
    if (t.value === '{') braceCount++;
    if (t.value === '}') braceCount--;
  }
  if (braceCount > 0) errors.push({ line: lines.length, message: 'Unmatched opening brace', severity: 'error' });
  if (braceCount < 0) errors.push({ line: 1, message: 'Unmatched closing brace', severity: 'error' });

  // Check for require with condition
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.startsWith('require(') && !line.endsWith(')')) {
      errors.push({ line: i + 1, message: 'require() call spans multiple lines', severity: 'warning' });
    }
  }

  return errors;
}
