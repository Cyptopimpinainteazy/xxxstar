#!/usr/bin/env node
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

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
  for (const s of signers) {
    if (seen.has(s.address)) continue;
    seen.add(s.address);
    out.push(s);
  }
  return out;
}

async function main() {
  const wsUrl = process.env.RPC_WS || 'ws://127.0.0.1:9944';
  const durationSec = envInt('DURATION_SEC', 30);
  const concurrency = envInt('CONCURRENCY', 32);
  const finalityWaitSec = envInt('FINALITY_WAIT_SEC', 18);
  const ss58Prefix = envInt('SS58_PREFIX', 42);
  const senderMode = (process.env.SENDER_MODE || 'dev').toLowerCase();
  const senderCount = envInt('SENDER_COUNT', 6);
  const senderOffset = envInt('SENDER_OFFSET', 0);
  const preFund = envBool('PRE_FUND', false);
  const onlyPrefund = envBool('ONLY_PREFUND', false);
  const preFundAmount = envBigInt('PREFUND_AMOUNT_PLANCK', 1_000_000_000_000n);
  const derivationBase = process.env.DERIVATION_BASE || '//Alice//load';
  const funderSuri = process.env.FUNDER_SURI || '//Alice';
  const prefundSettleSec = envInt('PREFUND_SETTLE_SEC', 20);
  const requireBaseline = envBool('REQUIRE_BASELINE', false);
  const minDurationSec = envInt('MIN_DURATION_SEC', 1200);
  const minFinalizedTps = envInt('MIN_FINALIZED_TPS', 0);
  const maxErrorRateRaw = Number(process.env.MAX_ERROR_RATE || '0.01');
  const maxErrorRate = Number.isFinite(maxErrorRateRaw) ? maxErrorRateRaw : 0.01;

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
      const acct = await api.query.system.account(signer.address);
      const free = BigInt(acct.data.free.toString());
      if (free >= preFundAmount) continue;
      const delta = preFundAmount - free;
      try {
        const tx = await api.tx.balances
          .transferKeepAlive(signer.address, delta.toString())
          .signAsync(funder, { nonce: Number(funderNonce) });
        await api.rpc.author.submitExtrinsic(tx);
      } catch (err) {
        throw new Error(`prefund failed for ${signer.address}: ${err.message || err}`);
      }
      targets.push(signer.address);
      funderNonce += 1n;
    }

    if (targets.length > 0) {
      await new Promise((r) => setTimeout(r, prefundSettleSec * 1000));
      for (const address of targets) {
        const acct = await api.query.system.account(address);
        const free = BigInt(acct.data.free.toString());
        if (free < preFundAmount) {
          throw new Error(`prefund verification failed for ${address}: ${free} < ${preFundAmount}`);
        }
      }
    }
  }

  if (onlyPrefund) {
    const report = {
      timestamp_utc: new Date().toISOString(),
      chain: 'x3',
      rpc_ws: wsUrl,
      tx_type: 'system.remark',
      sender_mode: senderMode,
      senders: signers.length,
      sender_offset: senderOffset,
      sender_count_requested: senderCount,
      prefund_enabled: preFund,
      prefund_only: true,
      prefund_amount_planck: preFundAmount.toString(),
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

  const signerStates = [];
  for (const signer of signers) {
    const info = await api.query.system.account(signer.address);
    signerStates.push({ signer, nonce: BigInt(info.nonce.toString()), busy: false });
  }
  const startNonces = await Promise.all(
    signers.map(async (s) => {
      const info = await api.query.system.account(s.address);
      return BigInt(info.nonce.toString());
    })
  );

  function submitOne(id, state) {
    return new Promise((resolve) => {
      const nonce = state.nonce;
      state.busy = true;

      api.tx.system
        .remark(`x3-load-${id}-${Date.now()}`)
        .signAsync(state.signer, { nonce: Number(nonce) })
        .then((tx) => api.rpc.author.submitExtrinsic(tx))
        .then(() => {
          state.nonce += 1n;
          accepted += 1;
          state.busy = false;
          resolve();
        })
        .catch(() => {
          failed += 1;
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
          const id = sent + 1;
          sent += 1;
          active += 1;
          submitOne(id, state).finally(() => {
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
  await new Promise((r) => setTimeout(r, finalityWaitSec * 1000));

  const endNonces = await Promise.all(
    signers.map(async (s) => {
      const info = await api.query.system.account(s.address);
      return BigInt(info.nonce.toString());
    })
  );

  let finalizedByNonce = 0n;
  for (let i = 0; i < signers.length; i += 1) {
    const delta = endNonces[i] - startNonces[i];
    if (delta > 0n) finalizedByNonce += delta;
  }

  const endMs = Date.now();
  const wallSec = (endMs - startMs) / 1000;
  const finalized = Number(finalizedByNonce);
  const finalizedTpsWall = wallSec > 0 ? finalized / wallSec : 0;
  const acceptedSubmitTps = durationSec > 0 ? accepted / durationSec : 0;
  const sendTps = durationSec > 0 ? sent / durationSec : 0;
  const finalizedTpsSubmitWindow = durationSec > 0 ? finalized / durationSec : 0;

  const report = {
    timestamp_utc: new Date().toISOString(),
    chain: 'x3',
    rpc_ws: wsUrl,
    tx_type: 'system.remark',
    sender_mode: senderMode,
    senders: signers.length,
    sender_offset: senderOffset,
    sender_count_requested: senderCount,
    prefund_enabled: preFund,
    prefund_amount_planck: preFundAmount.toString(),
    duration_sec: durationSec,
    concurrency,
    finality_wait_sec: finalityWaitSec,
    sent,
    accepted,
    failed,
    finalized,
    finalized_method: 'account_nonce_delta',
    send_tps: Number(sendTps.toFixed(3)),
    accepted_submit_tps: Number(acceptedSubmitTps.toFixed(3)),
    finalized_tps_submit_window: Number(finalizedTpsSubmitWindow.toFixed(3)),
    finalized_tps_wall: Number(finalizedTpsWall.toFixed(3)),
    wall_sec_including_finality_wait: Number(wallSec.toFixed(3)),
    error_rate: sent > 0 ? Number((failed / sent).toFixed(4)) : 0,
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
