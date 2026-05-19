#!/usr/bin/env node

/**
 * Authorize benchmark signer accounts for X3 submitComitV2 extrinsics.
 * 
 * Usage:
 *   node authorize-accounts.js \
 *     --wsEndpoint ws://127.0.0.1:9944 \
 *     --baseDerivation //Alice//load \
 *     --count 240 \
 *     --sudoSeed //Alice
 * 
 * Requirements:
 * - Root/sudo account available on chain (Alice by default)
 * - Chain running with X3 pallet and sudo pallet
 */

const { ApiPromise, WsProvider } = require("@polkadot/api");
const { Keyring } = require("@polkadot/keyring");
const { cryptoWaitReady } = require("@polkadot/util-crypto");

async function main() {
  const args = process.argv.slice(2).reduce((acc, arg, i, arr) => {
    if (arg.startsWith("--")) {
      const key = arg.slice(2);
      acc[key] = arr[i + 1];
    }
    return acc;
  }, {});

  const wsEndpoint = args.wsEndpoint || "ws://127.0.0.1:9944";
  const baseDerivation = args.baseDerivation || "//Alice//load";
  const count = parseInt(args.count) || 240;
  const sudoSeed = args.sudoSeed || "//Alice";
  const batchSize = parseInt(args.batchSize) || 10; // authorize in batches

  console.log(`Connecting to ${wsEndpoint}...`);
  const provider = new WsProvider(wsEndpoint);
  const api = await ApiPromise.create({ provider });

  await cryptoWaitReady();
  const keyring = new Keyring({ type: "sr25519" });

  // Setup sudo account
  const sudo = keyring.addFromUri(sudoSeed);
  console.log(`Using sudo account: ${sudo.address}`);

  // Get current nonce for sudo
  const sudoInfo = await api.query.system.account(sudo.address);
  let nonce = sudoInfo.nonce.toNumber();
  console.log(`Sudo nonce: ${nonce}`);

  // Generate signer addresses
  const signers = [];
  for (let i = 0; i < count; i++) {
    const derivation = `${baseDerivation}//${i}`;
    const account = keyring.addFromUri(derivation);
    signers.push(account);
  }

  console.log(`Generated ${signers.length} signer accounts`);

  // Runtime pallet naming changed from x3Kernel to atlasKernel in newer chains.
  const kernelTx = api.tx.x3Kernel || api.tx.atlasKernel;
  if (!kernelTx || typeof kernelTx.authorizeAccount !== "function") {
    throw new Error("Missing authorizeAccount extrinsic on x3Kernel/atlasKernel");
  }

  // Authorize in batches
  for (let batch = 0; batch < signers.length; batch += batchSize) {
    const batchAccounts = signers.slice(batch, Math.min(batch + batchSize, signers.length));
    console.log(`Authorizing batch ${Math.floor(batch / batchSize) + 1}/${Math.ceil(signers.length / batchSize)} (${batchAccounts.length} accounts)...`);

    const txs = batchAccounts.map(account =>
      kernelTx.authorizeAccount(account.address)
    );

    const batchTx = api.tx.utility.batch(txs);
    const sudoTx = api.tx.sudo.sudo(batchTx);

    try {
      await new Promise((resolve, reject) => {
        sudoTx.signAndSend(sudo, { nonce }, ({ status, events }) => {
          if (status.isInBlock) {
            console.log(`  ✓ Batch ${Math.floor(batch / batchSize) + 1} in block: ${status.asInBlock.toString()}`);
          }
          if (status.isFinalized) {
            console.log(`  ✓ Batch ${Math.floor(batch / batchSize) + 1} finalized`);
            resolve();
          }
          if (status.isBroadcast) {
            console.log(`  → Broadcast...`);
          }
        }).catch(reject);
      });
      nonce++;
    } catch (err) {
      console.error(`Error authorizing batch: ${err.message}`);
      process.exit(1);
    }
  }

  console.log(`✓ All ${signers.length} signer accounts authorized`);
  process.exit(0);
}

main().catch(err => {
  console.error(`Error: ${err.message}`);
  process.exit(1);
});
