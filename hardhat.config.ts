import { HardhatUserConfig } from "hardhat/config";
import "@nomiclabs/hardhat-waffle";
import "@nomiclabs/hardhat-ethers";

const config: HardhatUserConfig = {
  solidity: "0.8.24",
  networks: {
    // Add chain RPCs as needed
    chain1: { url: "<RPC_ENDPOINT_1>", accounts: ["<PRIVATE_KEY>"] },
    chain2: { url: "<RPC_ENDPOINT_2>", accounts: ["<PRIVATE_KEY>"] },
    // ...
    chain103: { url: "<RPC_ENDPOINT_103>", accounts: ["<PRIVATE_KEY>"] }
  },
};

export default config;
