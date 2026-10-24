import { createHash } from 'node:crypto';
import { isIP } from 'node:net';
import { Ratelimit } from '@upstash/ratelimit';
import { Redis } from '@upstash/redis';
import { checkQuoteRateLimit, trustedClientIp } from './base-swap';
import type { QuoteRateLimiters } from './base-swap';

export function getTrustedClientIp(headers: Headers): string | null {
  return trustedClientIp(headers, isIP);
}

export function hashRateLimitIdentifier(identifier: string): string {
  return createHash('sha256').update(identifier).digest('hex');
}

export function createQuoteRateLimiters(): {
  byIp: QuoteRateLimiters['byIp'];
  byWallet: QuoteRateLimiters['byWallet'];
} {
  const url = process.env.UPSTASH_REDIS_REST_URL?.trim();
  const token = process.env.UPSTASH_REDIS_REST_TOKEN?.trim();
  if (!url || !token) {
    throw new Error('Upstash Redis rate-limit credentials are not configured.');
  }

  const redis = new Redis({ url, token });
  return {
    byIp: new Ratelimit({
      redis,
      limiter: Ratelimit.slidingWindow(60, '1 m'),
      prefix: 'x3:dex:0x-quote:ip',
      analytics: false,
    }),
    byWallet: new Ratelimit({
      redis,
      limiter: Ratelimit.slidingWindow(30, '1 m'),
      prefix: 'x3:dex:0x-quote:wallet',
      analytics: false,
    }),
  };
}

export function enforceQuoteRateLimit(
  clientIp: string,
  walletAddress: string,
  limiters: QuoteRateLimiters,
): Promise<{ success: boolean; retryAfterSeconds: number }> {
  return checkQuoteRateLimit(
    clientIp,
    walletAddress,
    limiters,
    hashRateLimitIdentifier,
  );
}
