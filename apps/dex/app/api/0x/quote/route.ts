import {
  BASE_CHAIN_ID,
  BASE_TOKENS,
  isEvmAddress,
  isRecord,
  normalizeZeroXQuote,
  zeroXFailure,
} from '../../../lib/base-swap';
import {
  createQuoteRateLimiters,
  enforceQuoteRateLimit,
  getTrustedClientIp,
} from '../../../lib/quote-rate-limit';

export const dynamic = 'force-dynamic';

const ZEROX_QUOTE_URL = 'https://api.0x.org/swap/allowance-holder/quote';
const MAX_UINT256 = (1n << 256n) - 1n;

interface QuoteRequest {
  sellToken: string;
  buyToken: string;
  sellAmount: string;
  taker: string;
  slippageBps: number;
}

function jsonError(error: string, status: number, headers: HeadersInit = {}) {
  return Response.json(
    { error },
    { status, headers: { 'Cache-Control': 'no-store', ...headers } },
  );
}

function parseRequest(body: unknown): QuoteRequest | string {
  if (!isRecord(body)) {
    return 'Request body must be a JSON object.';
  }
  const { sellToken, buyToken, sellAmount, taker, slippageBps } = body;
  if (
    typeof sellToken !== 'string' ||
    typeof buyToken !== 'string' ||
    typeof sellAmount !== 'string' ||
    !isEvmAddress(taker)
  ) {
    return 'Token addresses, sell amount, or wallet address are invalid.';
  }
  if (
    !BASE_TOKENS.some((token) => token.address.toLowerCase() === sellToken.toLowerCase()) ||
    !BASE_TOKENS.some((token) => token.address.toLowerCase() === buyToken.toLowerCase())
  ) {
    return 'Only the configured Base WETH and USDC tokens are supported.';
  }
  if (sellToken.toLowerCase() === buyToken.toLowerCase()) {
    return 'Sell and buy tokens must be different.';
  }
  if (!/^[1-9]\d{0,77}$/.test(sellAmount)) {
    return 'Sell amount must be a positive integer in token base units.';
  }
  const amount = BigInt(sellAmount);
  if (amount > MAX_UINT256) {
    return 'Sell amount exceeds the ERC-20 uint256 limit.';
  }
  if (
    typeof slippageBps !== 'number' ||
    !Number.isInteger(slippageBps) ||
    slippageBps < 1 ||
    slippageBps > 500
  ) {
    return 'Slippage must be a whole number from 1 to 500 basis points.';
  }

  const canonicalSellToken = BASE_TOKENS.find(
    (token) => token.address.toLowerCase() === sellToken.toLowerCase(),
  );
  const canonicalBuyToken = BASE_TOKENS.find(
    (token) => token.address.toLowerCase() === buyToken.toLowerCase(),
  );
  if (!canonicalSellToken || !canonicalBuyToken) {
    return 'Only the configured Base WETH and USDC tokens are supported.';
  }

  return {
    sellToken: canonicalSellToken.address,
    buyToken: canonicalBuyToken.address,
    sellAmount,
    taker,
    slippageBps,
  };
}

export async function POST(request: Request) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return jsonError('Request body is not valid JSON.', 400);
  }

  const parsed = parseRequest(body);
  if (typeof parsed === 'string') {
    return jsonError(parsed, 400);
  }

  const apiKey = process.env.ZEROX_API_KEY?.trim();
  if (!apiKey) {
    return jsonError('The server has not been configured with ZEROX_API_KEY.', 503);
  }

  const clientIp = getTrustedClientIp(request.headers);
  if (!clientIp) {
    return jsonError('Request did not include a trusted client IP address.', 400);
  }

  let rateLimiters: ReturnType<typeof createQuoteRateLimiters>;
  try {
    rateLimiters = createQuoteRateLimiters();
  } catch {
    return jsonError('The quote service rate limiter is not configured.', 503);
  }

  let rateLimit: Awaited<ReturnType<typeof enforceQuoteRateLimit>>;
  try {
    rateLimit = await enforceQuoteRateLimit(clientIp, parsed.taker, rateLimiters);
  } catch {
    return jsonError('The quote service rate limiter is unavailable.', 503);
  }
  if (!rateLimit.success) {
    return jsonError(
      'Quote request rate limit exceeded. Wait before requesting another quote.',
      429,
      { 'Retry-After': rateLimit.retryAfterSeconds.toString() },
    );
  }

  const url = new URL(ZEROX_QUOTE_URL);
  url.searchParams.set('chainId', BASE_CHAIN_ID.toString());
  url.searchParams.set('sellToken', parsed.sellToken);
  url.searchParams.set('buyToken', parsed.buyToken);
  url.searchParams.set('sellAmount', parsed.sellAmount);
  url.searchParams.set('taker', parsed.taker);
  url.searchParams.set('slippagePpm', (parsed.slippageBps * 100).toString());

  let upstream: Response;
  try {
    upstream = await fetch(url, {
      headers: {
        '0x-api-key': apiKey,
        '0x-version': 'v2',
      },
      cache: 'no-store',
      signal: AbortSignal.timeout(10_000),
    });
  } catch (error) {
    const timedOut = error instanceof Error && error.name === 'TimeoutError';
    return jsonError(
      timedOut ? '0x quote request timed out.' : 'Unable to reach the 0x quote service.',
      502,
    );
  }

  if (!upstream.ok) {
    const failure = zeroXFailure(upstream.status);
    return jsonError(failure.message, failure.status);
  }

  let payload: unknown;
  try {
    payload = await upstream.json();
  } catch {
    return jsonError('0x returned a response that was not valid JSON.', 502);
  }

  const result = normalizeZeroXQuote(payload, parsed);
  if ('error' in result) {
    return jsonError(result.error, 422);
  }

  return Response.json(result, {
    headers: { 'Cache-Control': 'no-store' },
  });
}
