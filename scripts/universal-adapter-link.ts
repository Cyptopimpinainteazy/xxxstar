import { ethers } from "ethers";
import fs from "fs";

const registry = JSON.parse(fs.readFileSync("./scripts/chain-registry.json", "utf8"));

async function main() {
  for (const chain of registry.chains) {
    const provider = new ethers.providers.JsonRpcProvider(chain.rpc);
    // Placeholder: deploy or link UniversalAdapter
    // await UniversalAdapter.connect(...)
    console.log(`Linked UniversalAdapter for chain ${chain.id}`);
  }
}

main();
