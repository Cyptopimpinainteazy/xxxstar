#!/usr/bin/env node
// Standard Transactions Per Second (sTPS) for X3.
//
// Method after paritytech/polkadot-stps (Unlicense), docs/introduction.md:
//   * a transaction is a keep-alive balance transfer between two pre-existing accounts;
//   * neither account has been touched in the benchmark (one transfer per sender, nonce 0,
//     every sender and receiver distinct) - the worst case for Substrate storage access;
//   * no account cleanup and no account creation (receivers are endowed at genesis).
// sTPS = successful transfers in finalized blocks / on-chain time those blocks span
// (pallet_timestamp of the block before the first one containing a transfer, to the last one).
// Submitted or accepted counts are reported separately and are never called sTPS.
//
//   node stps.js spec  <base-plain-spec.json> <out.json> <N>   endow N senders + N receivers
//   node stps.js run   [ws://127.0.0.1:9944] <N> [timeout-seconds]
//
// Accounts are //x3-stps/sender/<i> and //x3-stps/receiver/<i> (sr25519, public dev
// derivations: TEST NETWORKS ONLY).
'use strict';
const fs = require('fs');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');
const { cryptoWaitReady } = require('@polkadot/util-crypto');
const { u8aToHex } = require('@polkadot/util');

const ENDOWMENT = 10n ** 21n; // far above any existential deposit
const AMOUNT = 10n ** 15n;

function pairs(n) {
  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const senders = [];
  const receivers = [];
  for (let i = 0; i < n; i += 1) {
    senders.push(keyring.addFromUri(`//x3-stps/sender/${i}`));
    receivers.push(keyring.addFromUri(`//x3-stps/receiver/${i}`));
  }
  return { senders, receivers };
}

async function spec(base, out, n) {
  await cryptoWaitReady();
  // Edit the text, not a parsed copy: JSON.parse turns u128 balances such as
  // 1000000000000000000000000 into doubles, which the node then cannot read back.
  const text = fs.readFileSync(base, 'utf8');
  const anchor = /"balances"\s*:\s*\{\s*"balances"\s*:\s*\[/;
  const match = anchor.exec(text);
  if (!match) throw new Error('expected a plain spec with genesis.runtimeGenesis.config.balances.balances');
  const { senders, receivers } = pairs(n);
  const entries = [...senders, ...receivers]
    .filter((p) => !text.includes(`"${p.address}"`))
    .map((p) => `["${p.address}", ${ENDOWMENT}]`);
  const at = match.index + match[0].length;
  const tail = text.slice(at).trimStart();
  const sep = entries.length && !tail.startsWith(']') ? ',' : '';
  fs.writeFileSync(out, text.slice(0, at) + entries.join(',') + sep + text.slice(at));
  console.log(JSON.stringify({ spec: out, endowed: entries.length }));
}

async function run(ws, n, timeoutSec) {
  await cryptoWaitReady();
  // Derive every key before connecting: derivation is synchronous and slow (20,000 keys take
  // minutes), and an event loop blocked that long cannot answer the server's WebSocket pings,
  // so the node drops the connection (1006) before a single transfer is submitted.
  const { senders, receivers } = pairs(n);
  const api = await ApiPromise.create({ provider: new WsProvider(ws), noInitWarn: true });
  // Compare public keys, not SS58 strings: the chain's SS58 prefix need not be 42.
  const senderSet = new Set(senders.map((s) => u8aToHex(s.publicKey)));

  // Pre-sign everything first, so signing time is not part of the measured window.
  const signed = [];
  for (let i = 0; i < n; i += 1) {
    const tx = api.tx.balances.transferKeepAlive(receivers[i].address, AMOUNT);
    signed.push(await tx.signAsync(senders[i], { nonce: 0, era: 0 }));
    // signAsync resolves without leaving the microtask queue; yield so the socket can answer
    // the server's pings, or a long signing phase gets the connection dropped (1006).
    if (i % 100 === 99) await new Promise((r) => setImmediate(r));
    if (process.env.STPS_DEBUG && i % 2000 === 1999) console.error(new Date().toISOString(), 'signed', i + 1);
  }

  let counted = 0;
  const blocks = []; // { number, ts, transfers }
  let prevTs = null;
  let firstTs = null;
  // GRANDPA finalizes several blocks at once and the subscription reports only the newest
  // finalized head, so every block number in between is fetched and counted, in order.
  let collectorErrors = 0;
  let nextBlock = null;
  let queue = Promise.resolve();
  const countBlock = async (number) => {
    const hash = await api.rpc.chain.getBlockHash(number);
    const at = await api.at(hash);
    const [events, now] = await Promise.all([at.query.system.events(), at.query.timestamp.now()]);
    let transfers = 0;
    for (const { event } of events) {
      if (api.events.balances.Transfer.is(event) && senderSet.has(event.data[0].toHex())) transfers += 1;
    }
    const ts = now.toBigInt();
    if (transfers > 0) {
      if (firstTs === null) firstTs = prevTs ?? ts;
      counted += transfers;
      blocks.push({ number, ts: Number(ts), transfers });
    }
    prevTs = ts;
  };
  const done = new Promise((resolve) => {
    api.rpc.chain.subscribeFinalizedHeads((header) => {
      const head = header.number.toNumber();
      queue = queue.then(async () => {
        if (nextBlock === null) nextBlock = head;
        for (; nextBlock <= head; nextBlock += 1) {
          try {
            await countBlock(nextBlock);
          } catch (err) {
            collectorErrors += 1;
            if (collectorErrors <= 3) console.error('collector:', nextBlock, err.message);
          }
        }
        if (counted >= n) resolve();
      });
    });
  });

  const submitStart = Date.now();
  // Submit in bounded chunks: one socket carrying thousands of in-flight calls overflows the
  // node's per-connection RPC buffer and the server drops the connection.
  const chunk = Number(process.env.STPS_SUBMIT_CHUNK || 500);
  const results = [];
  for (let i = 0; i < signed.length; i += chunk) {
    results.push(...(await Promise.allSettled(signed.slice(i, i + chunk).map((tx) => api.rpc.author.submitExtrinsic(tx)))));
    if (process.env.STPS_DEBUG) console.error(new Date().toISOString(), 'submitted', Math.min(i + chunk, signed.length), 'ok so far', results.filter((r) => r.status === 'fulfilled').length);
  }
  const submitMs = Date.now() - submitStart;
  const accepted = results.filter((r) => r.status === 'fulfilled').length;
  const rejectReasons = {};
  for (const r of results) {
    if (r.status === 'rejected') {
      const reason = String(r.reason && r.reason.message ? r.reason.message : r.reason).slice(0, 120);
      rejectReasons[reason] = (rejectReasons[reason] || 0) + 1;
    }
  }

  await Promise.race([done, new Promise((r) => setTimeout(r, timeoutSec * 1000))]);
  const lastTs = blocks.length ? BigInt(blocks[blocks.length - 1].ts) : null;
  const spanMs = lastTs !== null && firstTs !== null ? Number(lastTs - firstTs) : 0;
  const stps = spanMs > 0 ? counted / (spanMs / 1000) : 0;
  const header = await api.rpc.chain.getHeader();
  const version = api.runtimeVersion;
  console.log(JSON.stringify({
    method: 'sTPS (paritytech/polkadot-stps): keep-alive transfers between untouched endowed accounts, counted in finalized blocks over on-chain time',
    chain: (await api.rpc.system.chain()).toString(),
    spec_version: version.specVersion.toNumber(),
    best_block: header.number.toNumber(),
    transfers_requested: n,
    submitted_accepted: accepted,
    rejected: n - accepted,
    reject_reasons: rejectReasons,
    submit_wall_ms: submitMs,
    finalized_transfers: counted,
    blocks_with_transfers: blocks.length,
    max_transfers_in_a_block: blocks.reduce((m, b) => Math.max(m, b.transfers), 0),
    onchain_span_ms: spanMs,
    stps: Number(stps.toFixed(2)),
    complete: counted >= n,
    collector_errors: collectorErrors,
    blocks,
  }, null, 1));
  await api.disconnect();
  process.exit(counted >= n ? 0 : 1);
}

const [cmd, ...args] = process.argv.slice(2);
if (cmd === 'spec') {
  spec(args[0], args[1], Number(args[2])).catch((e) => { console.error(e); process.exit(2); });
} else if (cmd === 'run') {
  const ws = args[0] && args[0].startsWith('ws') ? args.shift() : 'ws://127.0.0.1:9944';
  run(ws, Number(args[0]), Number(args[1] || 600)).catch((e) => { console.error(e); process.exit(2); });
} else {
  console.error('usage: stps.js spec <base.json> <out.json> <N> | run [ws] <N> [timeout]');
  process.exit(2);
}
