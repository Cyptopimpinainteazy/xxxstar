// SPDX-License-Identifier: MIT
// SSRF guard for outbound HTTP fetches initiated from request input.
// Rejects URLs whose host resolves to a private/loopback/link-local/ULA/multicast
// range, and any non-http(s) scheme (file:, gopher:, dict:, ftp:, etc.).
//
// Used by services that accept a user-supplied URL and fetch it server-side
// (RPC probes, TPS benchmarks, webhook callbacks). Fixes CodeQL alerts
// js/server-side-request-forgery and js/uncontrolled-data-in-path-expression.

'use strict';

const dns = require('dns').promises;
const net = require('net');
const { URL } = require('url');

// Hosts that are always considered safe regardless of DNS resolution.
// Use sparingly — these are intentional public destinations.
const ALLOW_HOSTS = new Set();

// Hosts that are always blocked, even if the user is on a private network.
const BLOCK_HOSTS = new Set();

function isPrivateIPv4(ip) {
  const parts = ip.split('.').map((p) => parseInt(p, 10));
  if (parts.length !== 4 || parts.some((p) => Number.isNaN(p))) return true;
  const [a, b] = parts;
  if (a === 10) return true;                                      // 10.0.0.0/8
  if (a === 127) return true;                                     // 127.0.0.0/8 loopback
  if (a === 0) return true;                                       // 0.0.0.0/8
  if (a === 169 && b === 254) return true;                        // 169.254.0.0/16 link-local (AWS IMDS, etc.)
  if (a === 172 && b >= 16 && b <= 31) return true;               // 172.16.0.0/12
  if (a === 192 && b === 168) return true;                        // 192.168.0.0/16
  if (a === 192 && b === 0) return true;                          // 192.0.0.0/24 IETF
  if (a === 192 && b === 0 && parts[2] === 0) return true;         // 192.0.0.0/24
  if (a === 192 && b === 88 && parts[2] === 99) return true;      // 192.88.99.0/24
  if (a === 198 && (b === 18 || b === 19)) return true;            // 198.18.0.0/15 benchmark
  if (a === 198 && b === 51 && parts[2] === 100) return true;     // 198.51.100.0/24 TEST-NET-2
  if (a === 203 && b === 0 && parts[2] === 113) return true;      // 203.0.113.0/24 TEST-NET-3
  if (a >= 224 && a <= 239) return true;                          // 224.0.0.0/4 multicast
  if (a >= 240) return true;                                      // 240.0.0.0/4 reserved/broadcast
  return false;
}

function isPrivateIPv6(ip) {
  const lower = ip.toLowerCase();
  if (lower === '::1' || lower === '::') return true;             // loopback / unspecified
  if (lower.startsWith('fe80:') || lower.startsWith('fe80::')) return true; // link-local
  if (lower.startsWith('fc') || lower.startsWith('fd')) return true;          // fc00::/7 ULA
  if (lower.startsWith('ff')) return true;                                    // ff00::/8 multicast
  // IPv4-mapped IPv6 (::ffff:a.b.c.d)
  const m = lower.match(/^::ffff:([0-9.]+)$/);
  if (m) return isPrivateIPv4(m[1]);
  // ::/96 deprecated
  if (lower.startsWith('::')) return true;
  return false;
}

function isPrivateHostLiteral(host) {
  if (!host) return true;
  if (BLOCK_HOSTS.has(host)) return true;
  if (ALLOW_HOSTS.has(host)) return false;
  // If it's a literal IP, evaluate directly.
  const family = net.isIP(host);
  if (family === 4) return isPrivateIPv4(host);
  if (family === 6) return isPrivateIPv6(host);
  return false; // hostname — need DNS resolution
}

async function isPrivateHost(host) {
  if (isPrivateHostLiteral(host)) return true;
  let addrs = [];
  try {
    addrs = await dns.lookup(host, { all: true });
  } catch (e) {
    // DNS failure: treat as unsafe (do not fetch).
    return true;
  }
  for (const a of addrs) {
    if (a.family === 4 && isPrivateIPv4(a.address)) return true;
    if (a.family === 6 && isPrivateIPv6(a.address)) return true;
  }
  return false;
}

const ALLOWED_SCHEMES = new Set(['http:', 'https:']);

/**
 * Validate a user-supplied URL for safe outbound fetch.
 * @param {string} rawUrl
 * @returns {Promise<{ ok: true, url: URL } | { ok: false, reason: string }>}
 */
async function validateOutboundUrl(rawUrl) {
  if (typeof rawUrl !== 'string' || rawUrl.length === 0) {
    return { ok: false, reason: 'empty_url' };
  }
  let parsed;
  try {
    parsed = new URL(rawUrl);
  } catch (e) {
    return { ok: false, reason: 'invalid_url' };
  }
  if (!ALLOWED_SCHEMES.has(parsed.protocol)) {
    return { ok: false, reason: 'blocked_scheme:' + parsed.protocol };
  }
  if (parsed.username || parsed.password) {
    return { ok: false, reason: 'embedded_credentials' };
  }
  if (await isPrivateHost(parsed.hostname)) {
    return { ok: false, reason: 'private_or_blocked_host:' + parsed.hostname };
  }
  return { ok: true, url: parsed };
}

module.exports = {
  validateOutboundUrl,
  isPrivateIPv4,
  isPrivateIPv6,
  isPrivateHostLiteral,
  isPrivateHost,
  // Test hooks
  _ALLOW_HOSTS: ALLOW_HOSTS,
  _BLOCK_HOSTS: BLOCK_HOSTS,
};
