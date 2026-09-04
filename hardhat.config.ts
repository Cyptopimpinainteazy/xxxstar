import { HardhatUserConfig } from "hardhat/config";
import "@nomiclabs/hardhat-waffle";
import "@nomiclabs/hardhat-ethers";

function envRpc(name: string, fallback: string): string {
  return process.env[`X3_RPC_${name}`] || fallback;
}
function envKey(name: string, fallback: string): string {
  return process.env[`X3_KEY_${name}`] || fallback;
}

const config: HardhatUserConfig = {
  solidity: "0.8.24",
  paths: {
    sources: "./X3-contracts/evm/contracts",
    cache: "./X3-contracts/evm/cache",
    artifacts: "./X3-contracts/evm/artifacts",
  },
  networks: {
    // Populate X3_RPC_<CHAIN> / X3_KEY_<CHAIN> env vars per network.
    // Fallback values below are placeholder templates — not deployable.
    x1: {
      url: envRpc("X1", "https://rpc.x1.example.com"),
      accounts: [envKey("X1", "0x0000000000000000000000000000000000000000000000000000000000000000")],
    },
    x2: {
      url: envRpc("X2", "https://rpc.x2.example.com"),
      accounts: [envKey("X2", "0x0000000000000000000000000000000000000000000000000000000000000000")],
    },
    x3: {
      url: envRpc("X3", "https://rpc.x3.example.com"),
      accounts: [envKey("X3", "0x0000000000000000000000000000000000000000000000000000000000000000")],
    },
  },
};

export default config;
