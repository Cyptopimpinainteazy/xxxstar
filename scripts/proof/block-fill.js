// How full are the blocks, and what would the per-block limits allow?
//
// Every throughput figure in this repository comes from a load generator that
// measures *its own* transactions reaching finality. That is a statement about
// the harness: a generator that cannot sign and submit faster than the chain can
// include will report the generator's rate and call it the chain's. The chain's
// own side of the answer is the block content, which is what this reads.
//
// It reports three things the audit's throughput question needs:
//
//   * the block limits the runtime is configured with (weight and length);
//   * the weight of one signed `system.remark`, i.e. the unit the load generator
//     sends, and how many of them fit in a block by weight;
//   * what live blocks actually contain — extrinsics, weight fill, byte fill and
//     the resulting inclusion rate in extrinsics per second.
//
// The gap between "how many fit" and "how many are there" decides whether a
// measured TPS number is about the chain or about the client feeding it.
//
// Usage:
//   NODE_PATH=<dir with @polkadot/api> RPC_WS=ws://127.0.0.1:9944 \
//     node scripts/proof/block-fill.js [--samples 25] [--interval-ms 400]
//
// Exit code 0 when the sample completed, 1 when the endpoint did not answer.
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/keyring');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

// ref_time is picoseconds.
const PICOS_PER_MS = 1e9;

function arg(name, fallback) {
  const index = process.argv.indexOf(`--${name}`);
  if (index === -1 || index + 1 >= process.argv.length) return fallback;
  return Number(process.argv[index + 1]);
}

(async () => {
  await cryptoWaitReady();
  const api = await ApiPromise.create({
    provider: new WsProvider(process.env.RPC_WS || 'ws://127.0.0.1:9944'),
  });

  const limits = api.consts.system.blockWeights;
  const maxRefTime = limits.maxBlock.refTime.toBigInt();
  const maxProof = limits.maxBlock.proofSize.toBigInt();
  // `blockLength` is `{ max: { normal, operational, mandatory } }`: the
  // operational limit is the hard cap, `normal` is what ordinary extrinsics get.
  const lengthLimits = api.consts.system.blockLength.max;
  const maxBytes = lengthLimits.operational.toString();
  const normalBytes = lengthLimits.normal.toString();

  console.log('block limits configured in the runtime');
  console.log(
    `  weight:     ${Number(maxRefTime) / PICOS_PER_MS} ms of ref_time, proof size ${maxProof}`,
  );
  console.log(`  length:     ${maxBytes} bytes hard cap, ${normalBytes} bytes normal`);
  console.log('');

  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const alice = keyring.addFromUri('//Alice');
  const payment = await api.tx.system.remark('measure').paymentInfo(alice.address);
  const remarkWeight = payment.weight.refTime.toBigInt();
  const byWeight = Number(maxRefTime) / Number(remarkWeight);
  console.log('one signed system.remark, the unit the load generator sends');
  console.log(
    `  weight:     ${Number(remarkWeight) / PICOS_PER_MS} ms (${remarkWeight} ps), class ${payment.class.toString()}`,
  );
  console.log(`  by weight:  ${Math.floor(byWeight)} fit in one block`);
  console.log('');

  const sampleCount = arg('samples', 25);
  const intervalMs = arg('interval-ms', 400);
  const blocks = [];
  const started = Date.now();
  let previousHash = null;
  for (let i = 0; i < sampleCount; i += 1) {
    const hash = await api.rpc.chain.getBlockHash();
    const block = await api.rpc.chain.getBlock(hash);
    const weight = await api.query.system.blockWeight.at(hash);
    // Only count a block once: the head does not move between every poll.
    if (hash.toString() !== previousHash) {
      blocks.push({
        number: block.block.header.number.toNumber(),
        extrinsics: block.block.extrinsics.length,
        refTime: weight.normal.refTime.toBigInt(),
        proofSize: weight.normal.proofSize.toBigInt(),
        bytes: block.block.toHex().length / 2,
      });
      previousHash = hash.toString();
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  const elapsed = (Date.now() - started) / 1000;

  if (blocks.length === 0) {
    console.error('error: no blocks sampled');
    process.exit(1);
  }

  const extrinsics = blocks.map((b) => b.extrinsics);
  const refTimes = blocks.map((b) => b.refTime);
  const bytes = blocks.map((b) => b.bytes);
  const sum = (values) => values.reduce((a, b) => a + b, 0);
  const mean = (values) => sum(values) / values.length;
  const peak = (values) => values.reduce((a, b) => (a > b ? a : b));

  // The polls do not catch every block (they are slower than the 200 ms
  // cadence), so the *rate* has to come from how far the height moved, not from
  // how many blocks happened to be sampled. Summing only the sampled blocks and
  // dividing by the window would undercount by the number of missed blocks.
  const heightMoved = blocks[blocks.length - 1].number - blocks[0].number;
  const blockInterval = heightMoved > 0 ? elapsed / heightMoved : elapsed / blocks.length;
  const inclusionRate = sum(extrinsics) / (blocks.length * blockInterval);
  const meanRef = mean(refTimes.map(Number));

  console.log(
    `live blocks (${blocks.length} sampled over ${elapsed.toFixed(1)}s; the head advanced ` +
      `${heightMoved} blocks, so polls miss blocks faster than the poll interval)`,
  );
  console.log(`  height:      ${blocks[0].number} .. ${blocks[blocks.length - 1].number}`);
  console.log(
    `  extrinsics:  min ${Math.min(...extrinsics)}, max ${peak(extrinsics)}, mean ${mean(
      extrinsics,
    ).toFixed(2)} per block`,
  );
  console.log(
    `  weight:      mean ${(meanRef / PICOS_PER_MS).toFixed(3)} ms, max ${(
      Number(peak(refTimes)) / PICOS_PER_MS
    ).toFixed(3)} ms`,
  );
  console.log(
    `  weight fill: ${((meanRef / Number(maxRefTime)) * 100).toFixed(1)}% of the block budget`,
  );
  console.log(
    `  bytes:       mean ${Math.round(mean(bytes))}, max ${peak(bytes)} (${(
      (peak(bytes) / Number(normalBytes)) *
      100
    ).toFixed(2)}% of the normal limit)`,
  );
  console.log('');
  console.log(`inclusion rate over the window: ${inclusionRate.toFixed(1)} extrinsics per second`);
  console.log(
    `  estimated as ${mean(extrinsics).toFixed(2)} extrinsics per sampled block at the measured ` +
      `${blockInterval.toFixed(3)} s block interval`,
  );
  console.log(
    `  ${heightMoved} blocks over ${elapsed.toFixed(1)}s = ${(1 / blockInterval).toFixed(2)} blocks per second`,
  );
  console.log('  this is what the chain took, independent of what any client sent');
  console.log('');
  console.log("the chain's ceiling is not this number either: it is");
  console.log(
    `  by weight:     ${Math.floor(byWeight)} extrinsics per block -> ${Math.floor(
      byWeight / blockInterval,
    )} per second`,
  );
  console.log('  by wall clock: the measured cost of executing each extrinsic, reported by');
  console.log('                 scripts/proof/runtime-attribution.py');
  process.exit(0);
})().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
