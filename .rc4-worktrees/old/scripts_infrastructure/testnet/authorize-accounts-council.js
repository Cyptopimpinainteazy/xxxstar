#!/usr/bin/env node

/**
 * Authorize benchmark signer accounts for X3 submitComitV2 extrinsics, using
 * the Council collective (EnsureRootOrHalfCouncil) instead of Sudo.
 *
 * Works against runtimes built without pallet-sudo (e.g. non-`dev`-feature
 * builds), provided the dev/local chain spec seeds a single-member Council.
 * With one member and threshold=1, `Council::propose` executes the inner
 * call atomically in the same block (no separate vote/close needed).
 *
 * Usage:
 *   node authorize-accounts-council.js \
 *     --wsEndpoint ws://127.0.0.1:9933 \
 *     --baseDerivation //Alice//load \
 *     --count 240 \
 *     --councilSeed //Alice \
 *     --batchSize 10
 */

const { ApiPromise, WsProvider } = require("@polkadot/api");
const { Keyring } = require("@polkadot/keyring");
const { cryptoWaitReady } = require("@polkadot/util-crypto");

async function main() {
  const args = process.argv.slice(2).reduce((acc, arg, i, arr) => {
    if (arg.startsWith("--")) {
      acc[arg.slice(2)] = arr[i + 1];
    }
    return acc;
  }, {});

  const wsEndpoint = args.wsEndpoint || "ws://127.0.0.1:9933";
  const baseDerivation = args.baseDerivation || "//Alice//load";
  const count = parseInt(args.count) || 240;
  const councilSeed = args.councilSeed || "//Alice";
  const batchSize = parseInt(args.batchSize) || 10;

  console.log(`Connecting to ${wsEndpoint}...`);
  const provider = new WsProvider(wsEndpoint);
  const api = await ApiPromise.create({ provider });

  await cryptoWaitReady();
  const keyring = new Keyring({ type: "sr25519" });

  const council = keyring.addFromUri(councilSeed);
  console.log(`Using council member: ${council.address}`);

  // Sanity: ensure the signer is actually a council member.
  const members = (await api.query.council.members()).map((a) => a.toString());
  if (!members.includes(council.address)) {
    throw new Error(
      `Signer ${council.address} is not in council.members ${JSON.stringify(members)}`,
    );
  }
  if (typeof api.tx.council?.propose !== "function") {
    throw new Error("Missing council.propose extrinsic on this runtime");
  }

  const kernelTx = api.tx.x3Kernel || api.tx.atlasKernel;
  if (!kernelTx || typeof kernelTx.authorizeAccount !== "function") {
    throw new Error("Missing authorizeAccount on x3Kernel/atlasKernel");
  }
  if (typeof api.tx.utility?.batch !== "function") {
    throw new Error("Missing utility.batch on this runtime");
  }

  const acctInfo = await api.query.system.account(council.address);
  let nonce = acctInfo.nonce.toNumber();
  console.log(`Council nonce: ${nonce}`);

  // Generate signer addresses deterministically from the same scheme used by
  // the load generator.
  const signers = [];
  for (let i = 0; i < count; i++) {
    signers.push(keyring.addFromUri(`${baseDerivation}//${i}`));
  }
  console.log(`Generated ${signers.length} signer accounts`);

  const totalBatches = Math.ceil(signers.length / batchSize);
  for (let batch = 0; batch < signers.length; batch += batchSize) {
    const slice = signers.slice(batch, batch + batchSize);
    const idx = Math.floor(batch / batchSize) + 1;
    console.log(
      `Authorizing batch ${idx}/${totalBatches} (${slice.length} accounts)...`,
    );

    const inner = api.tx.utility.batch(
      slice.map((a) => kernelTx.authorizeAccount(a.address)),
    );

    // threshold=1 against a single-member council auto-executes the proposal.
    const lengthBound = inner.encodedLength;
    const proposeTx = api.tx.council.propose(1, inner, lengthBound);

    await new Promise((resolve, reject) => {
      proposeTx
        .signAndSend(council, { nonce }, ({ status, dispatchError, events }) => {
          if (status.isInBlock) {
            console.log(`  in block ${status.asInBlock.toString()}`);
            if (dispatchError) {
              const msg = dispatchError.isModule
                ? api.registry.findMetaError(dispatchError.asModule).docs.join(" ")
                : dispatchError.toString();
              return reject(new Error(`dispatchError: ${msg}`));
            }
            // Surface inner failures from utility.batch.
            for (const { event } of events) {
              if (api.events.utility?.BatchInterrupted?.is(event)) {
                return reject(
                  new Error(`utility.BatchInterrupted: ${event.data.toString()}`),
                );
              }
            }
            resolve();
          }
        })
        .catch(reject);
    });
    nonce++;
  }

  console.log(`OK: authorized ${signers.length} accounts via Council`);
  process.exit(0);
}

main().catch((err) => {
  console.error(`Error: ${err.message}`);
  process.exit(1);
});
