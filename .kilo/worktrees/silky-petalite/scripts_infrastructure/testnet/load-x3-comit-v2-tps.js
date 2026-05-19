#!/usr/bin/env node
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady, blake2AsU8a } = require('@polkadot/util-crypto');

function envInt(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const n = Number(raw);
  return Number.isFinite(n) && n > 0 ? Math.floor(n) : fallback;
}

function envBigInt(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  try {
    return BigInt(raw);
  } catch (_) {
    return fallback;
  }
}

function envBool(name, fallback = false) {
  const raw = process.env[name];
  if (!raw) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(raw.toLowerCase());
}

function uniqueSigners(signers) {
  const seen = new Set();
  const out = [];
  for (const signer of signers) {
    if (seen.has(signer.address)) continue;
    seen.add(signer.address);
    out.push(signer);
  }
  return out;
}

function u32le(value) {
  const out = Buffer.alloc(4);
  out.writeUInt32LE(value >>> 0, 0);
  return out;
}

function u16le(value) {
  const out = Buffer.alloc(2);
  out.writeUInt16LE(value >>> 0, 0);
  return out;
}

function u64le(value) {
  let n = BigInt(value);
  const out = Buffer.alloc(8);
  for (let i = 0; i < 8; i += 1) {
    out[i] = Number(n & 0xffn);
    n >>= 8n;
  }
  return out;
}

function u128le(value) {
  let n = BigInt(value);
  const out = Buffer.alloc(16);
  for (let i = 0; i < 16; i += 1) {
    out[i] = Number(n & 0xffn);
    n >>= 8n;
  }
  return out;
}

function hexOf(bytes) {
  return `0x${Buffer.from(bytes).toString('hex')}`;
}

function assembleSimpleX3Module() {
  const out = [];
  out.push(Buffer.from('X3BC', 'ascii'));
  out.push(u32le(1));
  out.push(u32le(0));
  out.push(u32le(0));
  out.push(u32le(1));
  out.push(u32le(0));

  out.push(u32le(2));
  out.push(Buffer.from([0]));
  out.push(u64le(42));
  out.push(Buffer.from([0]));
  out.push(u64le(7));

  const code = [];
  code.push(Buffer.from([0x10, 0x00]));
  code.push(u32le(0));
  code.push(Buffer.from([0x10, 0x01]));
  code.push(u32le(1));
  code.push(Buffer.from([0x20, 0x02, 0x00, 0x01]));
  code.push(Buffer.from([0x05, 0x02]));
  const codeBuf = Buffer.concat(code);

  const funcName = Buffer.from('add_test', 'utf8');
  out.push(u32le(1));
  out.push(u16le(funcName.length));
  out.push(funcName);
  out.push(u32le(0));
  out.push(Buffer.from([0]));
  out.push(u16le(3));
  out.push(u16le(4));
  out.push(Buffer.from([1]));

  out.push(u32le(0));
  out.push(u32le(codeBuf.length));
  out.push(codeBuf);
  out.push(Buffer.from([0, 0]));
  return Buffer.concat(out);
}

function computePrepareRootV2(comitId, evmPayload, svmPayload, x3Payload, nonce, fee) {
  const data = Buffer.concat([
    Buffer.from(comitId),
    Buffer.from(evmPayload),
    Buffer.from(svmPayload),
    Buffer.from(x3Payload),
    u64le(nonce),
    u128le(fee),
  ]);
  return blake2AsU8a(data, 256);
}

function computeComitId(accountId, nonce, sequence) {
  return blake2AsU8a(
    Buffer.concat([
      Buffer.from(accountId),
      u64le(nonce),
      u64le(sequence),
      u64le(Date.now()),
    ]),
    256
  );
}

function buildReportBase(wsUrl, senderMode, signers, senderOffset, senderCount, preFundAmount, feePlanck) {
  return {
    timestamp_utc: new Date().toISOString(),
    chain: 'x3',
    rpc_ws: wsUrl,
    tx_type: 'x3Kernel.submitComitV2',
    payload_profile: 'real_x3_only',
    sender_mode: senderMode,
    senders: signers.length,
    sender_offset: senderOffset,
    sender_count_requested: senderCount,
    prefund_amount_planck: preFundAmount.toString(),
    declared_fee_planck: feePlanck.toString(),
  };
}

async function signAndSubmitAndWait(api, signer, call, nonce) {
  return new Promise((resolve, reject) => {
    let unsub = null;
    call
      .signAndSend(signer, { nonce }, (result) => {
        if (result.dispatchError) {
          const error = result.dispatchError.isModule
            ? (() => {
                const decoded = api.registry.findMetaError(result.dispatchError.asModule);
                return `${decoded.section}.${decoded.name}`;
              })()
            : result.dispatchError.toString();
          if (unsub) unsub();
          reject(new Error(error));
          return;
        }
        if (result.status.isFinalized) {
          if (unsub) unsub();
          resolve(result.status.asFinalized.toHex());
        }
      })
      .then((u) => {
        unsub = u;
      })
      .catch(reject);
  });
}

async function isAuthorized(provider, signer) {
  try {
    return await provider.send('x3_isAuthorized', [hexOf(signer.publicKey)]);
  } catch (_) {
    return false;
  }
}

async function ensureAuthorizedAccounts(api, provider, adminSigner, signers) {
  const missing = [];
  for (const signer of signers) {
    // eslint-disable-next-line no-await-in-loop
    if (!(await isAuthorized(provider, signer))) {
      missing.push(signer);
    }
  }

  if (missing.length === 0) {
    return { attempted: 0, authorized_now: signers.length, used_sudo: false };
  }

  if (!api.tx.sudo?.sudo || !api.tx.x3Kernel?.authorizeAccount) {
    throw new Error(
      `Missing authorization path for ${missing.length} accounts; authorize them on-chain or enable sudo`
    );
  }

  const batchSize = envInt('AUTH_BATCH_SIZE', 64);
  const adminInfo = await api.query.system.account(adminSigner.address);
  let adminNonce = Number(adminInfo.nonce.toString());
  for (let i = 0; i < missing.length; i += batchSize) {
    const chunk = missing.slice(i, i + batchSize);
    const calls = chunk.map((signer) => api.tx.x3Kernel.authorizeAccount(signer.address));
    const batched = api.tx.utility?.batchAll ? api.tx.utility.batchAll(calls) : calls[0];
    // eslint-disable-next-line no-await-in-loop
    await signAndSubmitAndWait(api, adminSigner, api.tx.sudo.sudo(batched), adminNonce);
    adminNonce += 1;
    if (!api.tx.utility?.batchAll) {
      for (let j = 1; j < calls.length; j += 1) {
        // eslint-disable-next-line no-await-in-loop
        await signAndSubmitAndWait(api, adminSigner, api.tx.sudo.sudo(calls[j]), adminNonce);
        adminNonce += 1;
      }
    }
  }

  return { attempted: missing.length, authorized_now: signers.length, used_sudo: true };
}

async function samplePendingExtrinsics(provider) {
  try {
    const pending = await provider.send('author_pendingExtrinsics', []);
    return Array.isArray(pending) ? pending.length : null;
  } catch (_) {
    return null;
  }
}

function summarizeSamples(samples) {
  const numeric = samples.filter((value) => typeof value === 'number');
  if (numeric.length === 0) {
    return { supported: false };
  }
  const sum = numeric.reduce((acc, value) => acc + value, 0);
  return {
    supported: true,
    samples: numeric.length,
    min: Math.min(...numeric),
    max: Math.max(...numeric),
    avg: Number((sum / numeric.length).toFixed(3)),
    last: numeric[numeric.length - 1],
  };
}

async function main() {
  const wsUrl = process.env.RPC_WS || 'ws://127.0.0.1:9944';
  const durationSec = envInt('DURATION_SEC', 180);
  const concurrency = envInt('CONCURRENCY', 256);
  const finalityWaitSec = envInt('FINALITY_WAIT_SEC', 45);
  const ss58Prefix = envInt('SS58_PREFIX', 42);
  const senderMode = (process.env.SENDER_MODE || 'derived').toLowerCase();
  const senderCount = envInt('SENDER_COUNT', 120);
  const senderOffset = envInt('SENDER_OFFSET', 0);
  const preFund = envBool('PRE_FUND', false);
  const onlyPrepare = envBool('ONLY_PREPARE', envBool('ONLY_PREFUND', false));
  const preFundAmount = envBigInt('PREFUND_AMOUNT_PLANCK', 1_000_000_000_000n);
  const derivationBase = process.env.DERIVATION_BASE || '//Alice//load';
  const funderSuri = process.env.FUNDER_SURI || '//Alice';
  const prefundSettleSec = envInt('PREFUND_SETTLE_SEC', 20);
  const requireBaseline = envBool('REQUIRE_BASELINE', false);
  const minDurationSec = envInt('MIN_DURATION_SEC', 180);
  const minFinalizedTps = Number(process.env.MIN_FINALIZED_TPS || '0');
  const maxErrorRateRaw = Number(process.env.MAX_ERROR_RATE || '0.01');
  const maxErrorRate = Number.isFinite(maxErrorRateRaw) ? maxErrorRateRaw : 0.01;
  const declaredFeePlanck = envBigInt('DECLARED_FEE_PLANCK', 10_000n);
  const x3Payload = assembleSimpleX3Module();

  await cryptoWaitReady();
  const provider = new WsProvider(wsUrl);
  const api = await ApiPromise.create({ provider });
  const keyring = new Keyring({ type: 'sr25519', ss58Format: ss58Prefix });
  const funder = keyring.addFromUri(funderSuri);

  let signers = [];
  if (senderMode === 'derived') {
    for (let i = 0; i < senderCount; i += 1) {
      const idx = senderOffset + i;
      signers.push(keyring.addFromUri(`${derivationBase}//${idx}`));
    }
  } else {
    const devSeeds = ['//Alice', '//Bob', '//Charlie', '//Dave', '//Eve', '//Ferdie', '//One'];
    const picked = devSeeds.slice(senderOffset, senderOffset + senderCount);
    signers = picked.map((suri) => keyring.addFromUri(suri));
  }
  signers = uniqueSigners(signers);
  if (signers.length === 0) {
    throw new Error('No signers selected; check SENDER_MODE/SENDER_COUNT/SENDER_OFFSET');
  }

  if (preFund) {
    const funderInfo = await api.query.system.account(funder.address);
    let funderNonce = BigInt(funderInfo.nonce.toString());
    const targets = [];
    for (const signer of signers) {
      if (signer.address === funder.address) continue;
      // eslint-disable-next-line no-await-in-loop
      const acct = await api.query.system.account(signer.address);
      const free = BigInt(acct.data.free.toString());
      if (free >= preFundAmount) continue;
      const delta = preFundAmount - free;
      try {
        // eslint-disable-next-line no-await-in-loop
        const tx = await api.tx.balances
          .transferKeepAlive(signer.address, delta.toString())
          .signAsync(funder, { nonce: Number(funderNonce) });
        // eslint-disable-next-line no-await-in-loop
        await api.rpc.author.submitExtrinsic(tx);
      } catch (err) {
        throw new Error(`prefund failed for ${signer.address}: ${err.message || err}`);
      }
      targets.push(signer.address);
      funderNonce += 1n;
    }

    if (targets.length > 0) {
      await new Promise((resolve) => setTimeout(resolve, prefundSettleSec * 1000));
      for (const address of targets) {
        // eslint-disable-next-line no-await-in-loop
        const acct = await api.query.system.account(address);
        const free = BigInt(acct.data.free.toString());
        if (free < preFundAmount) {
          throw new Error(`prefund verification failed for ${address}: ${free} < ${preFundAmount}`);
        }
      }
    }
  }

  const authReport = await ensureAuthorizedAccounts(api, provider, funder, signers);

  if (onlyPrepare) {
    const report = {
      ...buildReportBase(wsUrl, senderMode, signers, senderOffset, senderCount, preFundAmount, declaredFeePlanck),
      prefund_enabled: preFund,
      prepare_only: true,
      authorization: authReport,
    };
    console.log(JSON.stringify(report, null, 2));
    await api.disconnect();
    return;
  }

  let sent = 0;
  let accepted = 0;
  let failed = 0;
  let active = 0;
  let signerIdx = 0;
  const startMs = Date.now();
  const endSubmitMs = startMs + durationSec * 1000;
  const failureReasons = {};
  const txpoolSamples = [];
  const finalizedBlocks = [];
  const benchmarkAddresses = new Set(signers.map((signer) => signer.address));

  const signerStates = [];
  for (const signer of signers) {
    // eslint-disable-next-line no-await-in-loop
    const info = await api.query.system.account(signer.address);
    signerStates.push({ signer, nonce: BigInt(info.nonce.toString()), busy: false });
  }

  const startNonces = await Promise.all(
    signers.map(async (signer) => {
      const info = await api.query.system.account(signer.address);
      return BigInt(info.nonce.toString());
    })
  );

  const unsubscribeFinalizedHeads = await api.rpc.chain.subscribeFinalizedHeads(async (header) => {
    try {
      const blockHash = header.hash.toHex();
      const signedBlock = await api.rpc.chain.getBlock(header.hash);
      let signedExtrinsics = 0;
      let benchmarkSignedExtrinsics = 0;
      for (const extrinsic of signedBlock.block.extrinsics) {
        if (!extrinsic.isSigned) continue;
        signedExtrinsics += 1;
        if (benchmarkAddresses.has(extrinsic.signer.toString())) {
          benchmarkSignedExtrinsics += 1;
        }
      }
      finalizedBlocks.push({
        number: header.number.toNumber(),
        hash: blockHash,
        observed_at_ms: Date.now(),
        extrinsics_total: signedBlock.block.extrinsics.length,
        signed_extrinsics: signedExtrinsics,
        benchmark_signed_extrinsics: benchmarkSignedExtrinsics,
      });
    } catch (_) {
      // Ignore metrics-only failures; submission path must continue.
    }
  });

  const txpoolInterval = setInterval(async () => {
    txpoolSamples.push(await samplePendingExtrinsics(provider));
  }, 1000);

  function recordFailure(reason) {
    const key = reason || 'unknown';
    failureReasons[key] = (failureReasons[key] || 0) + 1;
  }

  function submitOne(sequence, state) {
    return new Promise((resolve) => {
      const nonce = state.nonce;
      state.busy = true;
      const comitId = computeComitId(state.signer.publicKey, nonce, sequence);
      const prepareRoot = computePrepareRootV2(comitId, Buffer.alloc(0), Buffer.alloc(0), x3Payload, nonce, declaredFeePlanck);

      api.tx.x3Kernel
        .submitComitV2(
          hexOf(comitId),
          '0x',
          '0x',
          hexOf(x3Payload),
          Number(nonce),
          declaredFeePlanck.toString(),
          hexOf(prepareRoot)
        )
        .signAsync(state.signer, { nonce: Number(nonce) })
        .then((tx) => api.rpc.author.submitExtrinsic(tx))
        .then(() => {
          state.nonce += 1n;
          accepted += 1;
          state.busy = false;
          resolve();
        })
        .catch((err) => {
          failed += 1;
          recordFailure(err?.message || String(err));
          state.busy = false;
          resolve();
        });
    });
  }

  async function runLoad() {
    return new Promise((resolve) => {
      const pump = () => {
        while (Date.now() < endSubmitMs && active < concurrency) {
          let state = null;
          for (let i = 0; i < signerStates.length; i += 1) {
            const candidate = signerStates[(signerIdx + i) % signerStates.length];
            if (!candidate.busy) {
              state = candidate;
              signerIdx = (signerIdx + i + 1) % signerStates.length;
              break;
            }
          }
          if (!state) break;
          const sequence = sent + 1;
          sent += 1;
          active += 1;
          submitOne(sequence, state).finally(() => {
            active -= 1;
            if (Date.now() >= endSubmitMs && active === 0) {
              resolve();
              return;
            }
            pump();
          });
        }
        if (Date.now() >= endSubmitMs && active === 0) {
          resolve();
        }
      };
      pump();
    });
  }

  await runLoad();
  await new Promise((resolve) => setTimeout(resolve, finalityWaitSec * 1000));

  clearInterval(txpoolInterval);
  unsubscribeFinalizedHeads();
  txpoolSamples.push(await samplePendingExtrinsics(provider));

  const endNonces = await Promise.all(
    signers.map(async (signer) => {
      const info = await api.query.system.account(signer.address);
      return BigInt(info.nonce.toString());
    })
  );

  let finalizedByNonce = 0n;
  for (let i = 0; i < signers.length; i += 1) {
    const delta = endNonces[i] - startNonces[i];
    if (delta > 0n) finalizedByNonce += delta;
  }

  const metricEndMs = Date.now();
  const wallSec = (metricEndMs - startMs) / 1000;
  const finalized = Number(finalizedByNonce);
  const finalizedTpsWall = wallSec > 0 ? finalized / wallSec : 0;
  const acceptedSubmitTps = durationSec > 0 ? accepted / durationSec : 0;
  const sendTps = durationSec > 0 ? sent / durationSec : 0;
  const finalizedTpsSubmitWindow = durationSec > 0 ? finalized / durationSec : 0;

  const benchmarkBlocks = finalizedBlocks
    .filter((block) => block.observed_at_ms >= startMs && block.observed_at_ms <= metricEndMs)
    .sort((a, b) => a.number - b.number);
  let observedBlockSpanMs = 0;
  for (let i = 1; i < benchmarkBlocks.length; i += 1) {
    observedBlockSpanMs += Math.max(0, benchmarkBlocks[i].observed_at_ms - benchmarkBlocks[i - 1].observed_at_ms);
  }
  const avgBlockTimeMs = benchmarkBlocks.length > 1 ? observedBlockSpanMs / (benchmarkBlocks.length - 1) : null;
  const benchmarkSignedInBlocks = benchmarkBlocks.reduce((acc, block) => acc + block.benchmark_signed_extrinsics, 0);
  const signedInBlocks = benchmarkBlocks.reduce((acc, block) => acc + block.signed_extrinsics, 0);
  const signedPerBlock = benchmarkBlocks.length > 0 ? benchmarkSignedInBlocks / benchmarkBlocks.length : 0;
  const inBlockTps = observedBlockSpanMs > 0 ? benchmarkSignedInBlocks / (observedBlockSpanMs / 1000) : 0;

  const report = {
    ...buildReportBase(wsUrl, senderMode, signers, senderOffset, senderCount, preFundAmount, declaredFeePlanck),
    authorization: authReport,
    prefund_enabled: preFund,
    duration_sec: durationSec,
    concurrency,
    finality_wait_sec: finalityWaitSec,
    sent,
    accepted,
    failed,
    submit_rejections: failed,
    finalized,
    finalized_method: 'account_nonce_delta',
    send_tps: Number(sendTps.toFixed(3)),
    accepted_submit_tps: Number(acceptedSubmitTps.toFixed(3)),
    finalized_tps_submit_window: Number(finalizedTpsSubmitWindow.toFixed(3)),
    finalized_tps_wall: Number(finalizedTpsWall.toFixed(3)),
    wall_sec_including_finality_wait: Number(wallSec.toFixed(3)),
    error_rate: sent > 0 ? Number((failed / sent).toFixed(4)) : 0,
    avg_block_time_ms: avgBlockTimeMs === null ? null : Number(avgBlockTimeMs.toFixed(3)),
    finalized_blocks_observed: benchmarkBlocks.length,
    benchmark_signed_extrinsics_in_observed_blocks: benchmarkSignedInBlocks,
    signed_extrinsics_in_observed_blocks: signedInBlocks,
    signed_extrinsics_per_block: Number(signedPerBlock.toFixed(3)),
    in_block_tps: Number(inBlockTps.toFixed(3)),
    txpool_depth: summarizeSamples(txpoolSamples),
    failure_reasons: failureReasons,
  };

  const baselineOk =
    durationSec >= minDurationSec &&
    (minFinalizedTps <= 0 || finalizedTpsSubmitWindow >= minFinalizedTps) &&
    report.error_rate <= maxErrorRate;

  report.baseline_requirements = {
    require_baseline: requireBaseline,
    min_duration_sec: minDurationSec,
    min_finalized_tps: minFinalizedTps,
    max_error_rate: maxErrorRate,
    baseline_ok: baselineOk,
  };

  console.log(JSON.stringify(report, null, 2));
  if (requireBaseline && !baselineOk) {
    process.exit(1);
  }
  await api.disconnect();
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
