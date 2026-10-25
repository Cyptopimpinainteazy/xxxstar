import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import ts from 'typescript';

const source = await readFile(new URL('../app/lib/base-swap.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
}).outputText;
const baseSwap = await import(
  `data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`
);

const {
  BASE_CHAIN_ID,
  BASE_TOKENS,
  ZEROX_ALLOWANCE_HOLDER,
  assertWalletContext,
  buildApprovalData,
  checkQuoteRateLimit,
  formatUnits,
  formatError,
  isTransactionHash,
  normalizeZeroXQuote,
  parseQuantity,
  parseUnits,
  sumTokenTransfers,
  trustedClientIp,
  verifyTransactionRequest,
  waitForTransactionReceipt,
  zeroXFailure,
} = baseSwap;

const taker = '0x1111111111111111111111111111111111111111';
const transactionTarget = '0x2222222222222222222222222222222222222222';
const transactionHash = `0x${'a'.repeat(64)}`;
const TRANSFER_EVENT =
  '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef';
const input = {
  sellToken: BASE_TOKENS[0].address,
  buyToken: BASE_TOKENS[1].address,
  sellAmount: '1000000000000000000',
  taker,
  slippageBps: 100,
};

function makeQuote() {
  return {
    liquidityAvailable: true,
    sellToken: input.sellToken,
    buyToken: input.buyToken,
    sellAmount: input.sellAmount,
    buyAmount: '2000000000',
    minBuyAmount: '1980000000',
    blockNumber: '12345',
    allowanceTarget: ZEROX_ALLOWANCE_HOLDER,
    transaction: {
      from: taker,
      to: transactionTarget,
      data: '0x12345678',
      value: '0',
    },
    issues: {
      simulationIncomplete: false,
      invalidSourcesPassed: [],
      allowance: {
        spender: ZEROX_ALLOWANCE_HOLDER,
      },
    },
  };
}

test('parses and formats base-unit token amounts exactly', () => {
  assert.equal(parseUnits('1.234567', 6), 1_234_567n);
  assert.equal(formatUnits('1234567', 6), '1.234567');
  assert.throws(() => parseUnits('1.0000001', 6), /decimal places/);
  assert.throws(() => parseUnits('0', 18), /greater than zero/);
  assert.throws(() => parseUnits('1e3', 18), /valid positive/);
});

test('parses valid EVM quantities and rejects malformed quantities', () => {
  assert.equal(parseQuantity('0x2105'), 8453n);
  assert.equal(parseQuantity('8453'), 8453n);
  assert.equal(parseQuantity('0x'), null);
  assert.equal(parseQuantity('-1'), null);
});

test('normalizes a valid exact-input 0x ERC-20 quote', () => {
  const result = normalizeZeroXQuote(makeQuote(), input);

  assert.ok('quote' in result);
  assert.equal(result.quote.chainId, BASE_CHAIN_ID);
  assert.equal(result.quote.allowanceTarget, ZEROX_ALLOWANCE_HOLDER);
  assert.equal(result.quote.transaction.value, '0');
  assert.equal(result.quote.transaction.to, transactionTarget);
});

test('rejects unavailable liquidity and altered trade parameters', () => {
  const unavailable = normalizeZeroXQuote({ liquidityAvailable: false }, input);
  assert.ok('error' in unavailable);
  assert.match(unavailable.error, /no available liquidity/i);

  const wrongAmount = makeQuote();
  wrongAmount.sellAmount = '2';
  assert.ok('error' in normalizeZeroXQuote(wrongAmount, input));

  const wrongToken = makeQuote();
  wrongToken.buyToken = input.sellToken;
  assert.ok('error' in normalizeZeroXQuote(wrongToken, input));
});

test('maps upstream provider errors without disguising them as successful quotes', () => {
  assert.deepEqual(zeroXFailure(429), {
    status: 429,
    message: '0x rate limit reached. Wait before requesting another quote.',
  });
  assert.deepEqual(zeroXFailure(400), {
    status: 502,
    message: '0x quote request failed with status 400.',
  });
  assert.deepEqual(zeroXFailure(503), {
    status: 502,
    message: '0x quote request failed with status 503.',
  });
});

test('rate limits both trusted client IP and wallet and computes retry-after', async () => {
  const headers = new Headers({
    'x-real-ip': '203.0.113.8',
    'x-forwarded-for': '198.51.100.1',
  });
  assert.equal(
    trustedClientIp(headers, (value) => (value === '203.0.113.8' ? 4 : 0)),
    '203.0.113.8',
  );
  assert.equal(
    trustedClientIp(new Headers({ 'x-forwarded-for': '203.0.113.8' }), () => 0),
    null,
  );

  const identifiers = [];
  const limiters = {
    byIp: {
      async limit(identifier) {
        identifiers.push(['ip', identifier]);
        return { success: true, reset: 20_000 };
      },
    },
    byWallet: {
      async limit(identifier) {
        identifiers.push(['wallet', identifier]);
        return { success: false, reset: 12_500 };
      },
    },
  };
  const result = await checkQuoteRateLimit(
    '203.0.113.8',
    taker.toUpperCase(),
    limiters,
    (identifier) => `hashed:${identifier}`,
    10_250,
  );
  assert.deepEqual(result, { success: false, retryAfterSeconds: 3 });
  assert.deepEqual(identifiers, [
    ['ip', 'hashed:203.0.113.8'],
    ['wallet', taker],
  ]);
});

test('requires Base chain and the expected connected wallet account', async () => {
  const provider = (chainId, accounts) => ({
    async request({ method }) {
      if (method === 'eth_chainId') return chainId;
      if (method === 'eth_accounts') return accounts;
      throw new Error(`Unexpected RPC method: ${method}`);
    },
  });

  await assertWalletContext(provider('0x2105', [taker]), taker);
  await assert.rejects(
    assertWalletContext(provider('0x1', [taker]), taker),
    /Base Mainnet/,
  );
  await assert.rejects(
    assertWalletContext(provider('0x2105', [transactionTarget]), taker),
    /account changed/,
  );
});

test('encodes exact token approval and surfaces wallet rejection', () => {
  assert.equal(
    buildApprovalData(ZEROX_ALLOWANCE_HOLDER, 123n),
    `0x095ea7b3${ZEROX_ALLOWANCE_HOLDER.slice(2).toLowerCase().padStart(64, '0')}${'7b'.padStart(64, '0')}`,
  );
  assert.equal(formatError({ code: 4001 }), 'The wallet request was rejected.');
  assert.equal(isTransactionHash(transactionHash), true);
  assert.equal(isTransactionHash('0x1234'), false);
});

test('validates the submitted wallet transaction matches the reviewed call', async () => {
  const expected = {
    from: taker,
    to: transactionTarget,
    data: '0x095ea7b3',
  };
  const provider = {
    async request() {
      return {
        hash: transactionHash,
        from: expected.from,
        to: expected.to,
        input: expected.data,
        value: '0x0',
      };
    },
  };
  await verifyTransactionRequest(provider, transactionHash, expected);

  await assert.rejects(
    verifyTransactionRequest(
      { async request() { return { ...await provider.request(), to: taker }; } },
      transactionHash,
      expected,
    ),
    /do not match/,
  );
});

test('waits for successful receipt confirmations and rejects reverted swaps', async () => {
  const receipt = {
    transactionHash,
    blockNumber: '0x10',
    status: '0x1',
    from: taker,
    to: transactionTarget,
    logs: [],
  };
  const provider = {
    async request({ method }) {
      if (method === 'eth_chainId') return '0x2105';
      if (method === 'eth_getTransactionReceipt') return receipt;
      if (method === 'eth_blockNumber') return '0x12';
      throw new Error(`Unexpected RPC method: ${method}`);
    },
  };
  const statusUpdates = [];
  const confirmed = await waitForTransactionReceipt(
    provider,
    transactionHash,
    3,
    (message) => statusUpdates.push(message),
    100,
    0,
  );
  assert.equal(confirmed.status, 1n);
  assert.equal(confirmed.blockNumber, 16n);

  await assert.rejects(
    waitForTransactionReceipt(
      {
        async request(args) {
          if (args.method === 'eth_getTransactionReceipt') {
            return { ...receipt, status: '0x0' };
          }
          return provider.request(args);
        },
      },
      transactionHash,
      1,
      () => {},
      100,
      0,
    ),
    /reverted on Base/,
  );
});

test('verifies input and output transfers from the confirmed receipt logs', () => {
  const accountTopic = `0x${taker.slice(2).toLowerCase().padStart(64, '0')}`;
  const receipt = {
    transactionHash,
    blockNumber: 1n,
    status: 1n,
    from: taker,
    to: transactionTarget,
    logs: [
      {
        address: BASE_TOKENS[0].address,
        topics: [TRANSFER_EVENT, accountTopic, `0x${'3'.repeat(64)}`],
        data: `0x${100n.toString(16).padStart(64, '0')}`,
      },
      {
        address: BASE_TOKENS[1].address,
        topics: [TRANSFER_EVENT, `0x${'3'.repeat(64)}`, accountTopic],
        data: `0x${200n.toString(16).padStart(64, '0')}`,
      },
    ],
  };
  assert.equal(sumTokenTransfers(receipt, BASE_TOKENS[0], taker, 'from'), 100n);
  assert.equal(sumTokenTransfers(receipt, BASE_TOKENS[1], taker, 'to'), 200n);
});

test('rejects unsafe spender, transaction value, and wallet binding', () => {
  const wrongSpender = makeQuote();
  wrongSpender.issues.allowance.spender = transactionTarget;
  assert.ok('error' in normalizeZeroXQuote(wrongSpender, input));

  const nonzeroValue = makeQuote();
  nonzeroValue.transaction.value = '0x1';
  assert.ok('error' in normalizeZeroXQuote(nonzeroValue, input));

  const wrongFrom = makeQuote();
  wrongFrom.transaction.from = transactionTarget;
  assert.ok('error' in normalizeZeroXQuote(wrongFrom, input));
});

test('rejects a quote whose minimum exceeds its expected output', () => {
  const invalidMinimum = makeQuote();
  invalidMinimum.minBuyAmount = '2000000001';
  assert.ok('error' in normalizeZeroXQuote(invalidMinimum, input));
});


test('rejects incomplete or failed upstream execution simulations', () => {
  for (const simulationIncomplete of [true, undefined, 'false']) {
    const payload = makeQuote();
    payload.issues.simulationIncomplete = simulationIncomplete;
    assert.ok('error' in normalizeZeroXQuote(payload, input));
  }
  const missingIssues = makeQuote();
  delete missingIssues.issues;
  assert.ok('error' in normalizeZeroXQuote(missingIssues, input));
});

test('rejects output quantities exceeding the ERC-20 uint256 range', () => {
  for (const field of ['buyAmount', 'minBuyAmount']) {
    const payload = makeQuote();
    payload.buyAmount = ((1n << 256n) + 1n).toString();
    payload[field] = (1n << 256n).toString();
    assert.ok('error' in normalizeZeroXQuote(payload, input));
  }
});

test('accepts the largest valid uint256 output without precision loss', () => {
  const payload = makeQuote();
  payload.buyAmount = ((1n << 256n) - 1n).toString();
  payload.minBuyAmount = payload.buyAmount;
  const result = normalizeZeroXQuote(payload, input);
  assert.ok('quote' in result);
  assert.equal(result.quote.buyAmount, payload.buyAmount);
});


test('rejects a provider minimum that exceeds the authorized slippage', () => {
  const payload = makeQuote();
  payload.minBuyAmount = '1';
  assert.ok('error' in normalizeZeroXQuote(payload, input));
});

test('rounds the minimum output upward and rejects invalid slippage policies', () => {
  const payload = makeQuote();
  payload.buyAmount = '101';
  payload.minBuyAmount = '99';
  assert.ok('error' in normalizeZeroXQuote(payload, input));
  payload.minBuyAmount = '100';
  assert.ok('quote' in normalizeZeroXQuote(payload, input));
  for (const slippageBps of [undefined, NaN, -1, 0, 501, 1.5]) {
    assert.ok('error' in normalizeZeroXQuote(payload, { ...input, slippageBps }));
  }
});
