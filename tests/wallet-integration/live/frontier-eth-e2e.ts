// Live check of eth_sendRawTransaction against a running `--features frontier` dev node.
//
// Run by scripts/frontier-eth-e2e.sh, which boots the node. Exits nonzero on any failure.
//
// The sender is Hardhat's well-known account #0, which the dev chain spec funds through its
// mapped Substrate account (`DEV_EVM_CALLERS` in node/src/chain_spec.rs). Balances are read from
// `System.Account` storage directly: `eth_getBalance` reports the kernel's `CanonicalLedger`,
// which is not the balance the EVM spends from.
import { Transaction, Wallet, getBytes, getCreateAddress, hexlify, keccak256, toUtf8Bytes } from "ethers";
import { blake2b } from "@noble/hashes/blake2b";
import { readFileSync, writeFileSync } from "fs";

const RPC = process.env.X3_RPC ?? "http://127.0.0.1:9944";
const HARDHAT_0 = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
// twox128("System") ++ twox128("Account")
const SYSTEM_ACCOUNT = "26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9";
const GAS_PRICE = 1_000_000_000n;

let id = 0;
async function rpc(method: string, params: unknown[] = []): Promise<any> {
  const res = await fetch(RPC, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: ++id, method, params }),
  });
  const body = (await res.json()) as { result?: unknown; error?: { message: string } };
  if (body.error) throw new Error(`${method}: ${body.error.message}`);
  return body.result;
}

/** Native free balance of an EVM address's mapped account (HashedAddressMapping<BlakeTwo256>). */
async function nativeFree(address: string): Promise<bigint> {
  const account = blake2b(new Uint8Array([...toUtf8Bytes("evm:"), ...getBytes(address)]), {
    dkLen: 32,
  });
  const key =
    "0x" + SYSTEM_ACCOUNT + hexlify(blake2b(account, { dkLen: 16 })).slice(2) + hexlify(account).slice(2);
  const raw: string | null = await rpc("state_getStorage", [key]);
  if (!raw) return 0n;
  // AccountInfo { nonce: u32, consumers: u32, providers: u32, sufficients: u32, data: { free: u128, .. } }
  const bytes = getBytes(raw).slice(16, 32);
  return bytes.reduceRight((acc, b) => (acc << 8n) | BigInt(b), 0n);
}

async function nonceOf(address: string): Promise<number> {
  return Number(await rpc("eth_getTransactionCount", [address, "latest"]));
}

async function expectRejected(label: string, raw: string): Promise<void> {
  try {
    await rpc("eth_sendRawTransaction", [raw]);
  } catch (e) {
    console.log(`  ok  ${label}: rejected (${(e as Error).message})`);
    return;
  }
  throw new Error(`${label}: was accepted`);
}

async function waitFor(label: string, check: () => Promise<boolean>, seconds = 90): Promise<void> {
  for (let i = 0; i < seconds; i++) {
    if (await check()) return;
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(`${label}: not observed within ${seconds}s`);
}

async function main(): Promise<void> {
  const chainId = BigInt(await rpc("eth_chainId"));
  const sender = new Wallet(HARDHAT_0);
  const recipient = Wallet.createRandom().address;
  const nonce0 = await nonceOf(sender.address);
  const senderFree0 = await nativeFree(sender.address);
  console.log(`chain ${chainId}, sender ${sender.address} nonce ${nonce0} free ${senderFree0}`);
  if (senderFree0 < 21_000n * GAS_PRICE) throw new Error("dev sender is not funded");

  // Above the existential deposit (100_000 base units): a smaller credit cannot create the account.
  const value = 1_000_000_000n;
  // Not 21_000: with GasLimitPovSizeRatio = 40 a transfer that creates the recipient account
  // measures ~21_600 gas here, so a 21_000 limit runs out of gas and moves no value.
  const gasLimit = 100_000n;
  const tx = { type: 0, chainId, nonce: nonce0, gasPrice: GAS_PRICE, gasLimit, to: recipient, value };
  const raw = await sender.signTransaction(tx);

  // Same signature over different contents recovers a different (unfunded) address.
  const tampered = Transaction.from(raw);
  tampered.value = value * 1_000n;
  await expectRejected("tampered value", tampered.serialized);
  await expectRejected("wrong chain id", await sender.signTransaction({ ...tx, chainId: 1n }));
  await expectRejected("garbage bytes", "0xdeadbeef");
  // The old [caller(20)][to(20)][value(16 LE)][len(4 LE)] payload naming the funded sender.
  const legacyPayload = new Uint8Array(60);
  legacyPayload.set(getBytes(sender.address), 0);
  legacyPayload.set(getBytes(recipient), 20);
  legacyPayload[40] = 1;
  await expectRejected("caller-named payload", hexlify(legacyPayload));
  if ((await nonceOf(sender.address)) !== nonce0) throw new Error("a rejected tx moved the nonce");
  if ((await nativeFree(recipient)) !== 0n) throw new Error("a rejected tx credited the recipient");

  const hash: string = await rpc("eth_sendRawTransaction", [raw]);
  if (hash !== keccak256(raw)) throw new Error(`returned hash ${hash} != keccak(raw) ${keccak256(raw)}`);
  console.log(`  ok  signed transfer accepted: ${hash}`);

  await waitFor("sender nonce advance", async () => (await nonceOf(sender.address)) === nonce0 + 1);
  await waitFor("recipient credit", async () => (await nativeFree(recipient)) === value);
  const debited = senderFree0 - (await nativeFree(sender.address));
  // Value plus gas used: at least the 21_000 intrinsic gas, at most the limit, in whole gas units.
  const fee = debited - value;
  if (fee < 21_000n * GAS_PRICE || fee > gasLimit * GAS_PRICE || fee % GAS_PRICE !== 0n) {
    throw new Error(`sender debited ${debited}: fee ${fee} is not value + gasUsed * gasPrice`);
  }
  console.log(`  ok  included: sender nonce ${nonce0 + 1}, recipient credited ${value}, gas used ${fee / GAS_PRICE}`);

  // Lookups come from the node's index (pallet-ethereum keeps only the latest block).
  let receipt: any = null;
  await waitFor("receipt", async () => (receipt = await rpc("eth_getTransactionReceipt", [hash])) !== null, 30);
  const gasUsed = fee / GAS_PRICE;
  const lower = (x: unknown) => String(x).toLowerCase();
  const expectEq = (label: string, got: unknown, want: unknown) => {
    if (lower(got) !== lower(want)) throw new Error(`${label}: got ${got}, want ${want}`);
  };
  expectEq("receipt.transactionHash", receipt.transactionHash, hash);
  expectEq("receipt.status", receipt.status, "0x1");
  expectEq("receipt.from", receipt.from, sender.address);
  expectEq("receipt.to", receipt.to, recipient);
  expectEq("receipt.gasUsed", BigInt(receipt.gasUsed), gasUsed);
  expectEq("receipt.effectiveGasPrice", BigInt(receipt.effectiveGasPrice), GAS_PRICE);
  const block = await rpc("chain_getBlockHash", [Number(receipt.blockNumber)]);
  expectEq("receipt.blockHash", receipt.blockHash, block);

  const fetched: any = await rpc("eth_getTransactionByHash", [hash]);
  if (!fetched) throw new Error("eth_getTransactionByHash returned null for an included tx");
  expectEq("tx.from", fetched.from, sender.address);
  expectEq("tx.to", fetched.to, recipient);
  expectEq("tx.value", BigInt(fetched.value), value);
  expectEq("tx.nonce", BigInt(fetched.nonce), BigInt(nonce0));
  expectEq("tx.blockHash", fetched.blockHash, receipt.blockHash);
  // Rebuilding the transaction from the RPC fields must reproduce the hash the wallet signed.
  const rebuilt = Transaction.from({
    type: 0, chainId, nonce: Number(fetched.nonce), gasPrice: BigInt(fetched.gasPrice),
    gasLimit: BigInt(fetched.gas), to: fetched.to, value: BigInt(fetched.value), data: fetched.input,
    signature: { r: fetched.r, s: fetched.s, v: Number(fetched.v) },
  });
  expectEq("tx rebuilt hash", rebuilt.hash, hash);
  const unknown = "0x" + "ab".repeat(32);
  if ((await rpc("eth_getTransactionReceipt", [unknown])) !== null) throw new Error("receipt for unknown hash");
  console.log(`  ok  lookups: receipt in block ${Number(receipt.blockNumber)}, tx fields rebuild the signed hash`);

  await expectRejected("replay", raw);
  const height = Number(await rpc("eth_blockNumber"));
  await waitFor("next block", async () => Number(await rpc("eth_blockNumber")) > height + 1);
  if ((await nativeFree(recipient)) !== value) throw new Error("replay credited the recipient again");
  if ((await nonceOf(sender.address)) !== nonce0 + 1) throw new Error("replay moved the nonce");
  console.log("  ok  replay had no effect");
  // A contract whose constructor emits one LOG1: PUSH32 topic, PUSH1 0, PUSH1 0, LOG1, STOP.
  const topic = keccak256(toUtf8Bytes("X3FrontierE2E()"));
  const deploy = await sender.signTransaction({
    type: 0, chainId, nonce: nonce0 + 1, gasPrice: GAS_PRICE, gasLimit: 200_000n, to: null,
    data: "0x7f" + topic.slice(2) + "60006000a100",
  });
  const deployHash: string = await rpc("eth_sendRawTransaction", [deploy]);
  let deployed: any = null;
  await waitFor("deploy receipt", async () => (deployed = await rpc("eth_getTransactionReceipt", [deployHash])) !== null, 60);
  const contract = getCreateAddress({ from: sender.address, nonce: nonce0 + 1 });
  expectEq("deploy.status", deployed.status, "0x1");
  expectEq("deploy.contractAddress", deployed.contractAddress, contract);
  expectEq("deploy.to", deployed.to, null);
  if (deployed.logs.length !== 1) throw new Error(`deploy emitted ${deployed.logs.length} logs, want 1`);
  expectEq("deploy log address", deployed.logs[0].address, contract);
  expectEq("deploy log topic", deployed.logs[0].topics[0], topic);

  const n = deployed.blockNumber;
  const logs: any[] = await rpc("eth_getLogs", [{ fromBlock: n, toBlock: n, address: contract, topics: [topic] }]);
  if (logs.length !== 1) throw new Error(`eth_getLogs by address+topic returned ${logs.length}, want 1`);
  expectEq("getLogs txHash", logs[0].transactionHash, deployHash);
  const byHash: any[] = await rpc("eth_getLogs", [{ blockHash: deployed.blockHash, topics: [[topic, keccak256("0x01")]] }]);
  if (byHash.length !== 1) throw new Error(`eth_getLogs by blockHash returned ${byHash.length}, want 1`);
  const wrongTopic: any[] = await rpc("eth_getLogs", [{ fromBlock: n, toBlock: n, topics: [keccak256("0x02")] }]);
  if (wrongTopic.length !== 0) throw new Error(`eth_getLogs with a non-matching topic returned ${wrongTopic.length}`);

  const deployBlock: any = await rpc("eth_getBlockByNumber", [n, false]);
  if (!deployBlock.transactions.map(lower).includes(lower(deployHash))) throw new Error("block does not list the deploy tx");
  if (BigInt(deployBlock.gasUsed) < BigInt(deployed.gasUsed)) throw new Error("block gasUsed below the deploy's gasUsed");
  if (/^0x0*$/.test(deployBlock.logsBloom) || deployBlock.logsBloom.length !== 2 + 512) throw new Error(`bad logsBloom ${deployBlock.logsBloom}`);
  const fullBlock: any = await rpc("eth_getBlockByHash", [deployed.blockHash, true]);
  const fullTx = fullBlock.transactions.find((t: any) => lower(t.hash) === lower(deployHash));
  if (!fullTx || fullTx.to !== null) throw new Error("full block does not carry the deploy as a create tx");
  console.log(`  ok  logs: contract ${contract} emitted 1 log; getLogs (range, blockHash, topic sets) and block fields agree`);

  // Hand the included transactions to the synced-node check (X3_SYNC_FILE, see the gate script).
  if (process.env.X3_SYNC_FILE) {
    writeFileSync(process.env.X3_SYNC_FILE, JSON.stringify({ hashes: [hash, deployHash], topic, block: Number(n) }));
  }
  console.log("PASS");
}

/** On a node that synced the chain, the same transactions, receipts and logs must be served. */
async function verifySynced(): Promise<void> {
  const want = JSON.parse(readFileSync(process.env.X3_SYNC_FILE!, "utf8"));
  const source = process.env.X3_SOURCE_RPC!;
  const at = async (url: string, method: string, params: unknown[]) => {
    const res = await fetch(url, {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
    });
    const body: any = await res.json();
    if (body.error) throw new Error(`${method} @ ${url}: ${body.error.message}`);
    return body.result;
  };
  // The synced node indexes on its catch-up timer; give it a few ticks.
  await waitFor("synced receipts", async () => {
    for (const h of want.hashes) if ((await at(RPC, "eth_getTransactionReceipt", [h])) === null) return false;
    return true;
  }, 120);
  for (const h of want.hashes) {
    const a = JSON.stringify(await at(source, "eth_getTransactionReceipt", [h]));
    const b = JSON.stringify(await at(RPC, "eth_getTransactionReceipt", [h]));
    if (a !== b) throw new Error(`receipt ${h} differs on the synced node:\n${a}\n${b}`);
    const ta = JSON.stringify(await at(source, "eth_getTransactionByHash", [h]));
    const tb = JSON.stringify(await at(RPC, "eth_getTransactionByHash", [h]));
    if (ta !== tb) throw new Error(`tx ${h} differs on the synced node`);
  }
  const filter = [{ fromBlock: "0x1", toBlock: "0x" + want.block.toString(16), topics: [want.topic] }];
  const la = JSON.stringify(await at(source, "eth_getLogs", filter));
  const lb = JSON.stringify(await at(RPC, "eth_getLogs", filter));
  if (la !== lb || JSON.parse(lb).length !== 1) throw new Error(`eth_getLogs differs on the synced node:\n${la}\n${lb}`);
  console.log(`  ok  synced node serves identical receipts, transactions and logs (blocks 1..${want.block})`);
  console.log("PASS");
}

(process.env.X3_MODE === "verify-synced" ? verifySynced() : main()).catch((e) => {
  console.error(`FAIL: ${(e as Error).message}`);
  process.exit(1);
});
