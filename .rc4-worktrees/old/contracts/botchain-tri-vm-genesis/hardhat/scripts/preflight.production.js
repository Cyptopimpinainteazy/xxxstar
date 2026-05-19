const hre = require("hardhat");

function requireEnv(name) {
  const value = process.env[name];
  if (!value || value.trim() === "") {
    throw new Error(`Missing required environment variable: ${name}`);
  }
  return value.trim();
}

function optionalEnv(name, fallback) {
  const value = process.env[name];
  return value && value.trim() !== "" ? value.trim() : fallback;
}

function parseBool(value, fallback = false) {
  if (value === undefined) return fallback;
  return ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function assertAddress(label, value) {
  if (!hre.ethers.isAddress(value)) {
    throw new Error(`Invalid address for ${label}: ${value}`);
  }
}

async function main() {
  console.log("=== Botchain Production Deployment Preflight ===\n");

  const networkName = hre.network.name;
  if (networkName === "hardhat") {
    throw new Error(
      "Refusing production preflight on the in-process hardhat network"
    );
  }

  const [deployer] = await hre.ethers.getSigners();
  const providerNetwork = await hre.ethers.provider.getNetwork();
  const deployerBalance = await hre.ethers.provider.getBalance(deployer.address);

  const expectedChainId = optionalEnv("PRODUCTION_CHAIN_ID", "");
  const compilerVerifier = requireEnv("COMPILER_VERIFIER_ADDRESS");
  const checkerVerifier = requireEnv("CHECKER_VERIFIER_ADDRESS");
  const dexQuoteToken = requireEnv("DEX_QUOTE_TOKEN_ADDRESS");
  const marriageFeeBot = optionalEnv("MARRIAGE_FEE_BOT", "100");
  const bootstrapDex = parseBool(process.env.BOOTSTRAP_DEX, false);
  const dexBotLiquidity = optionalEnv("DEX_BOT_LIQUIDITY", "0");
  const dexQuoteLiquidity = optionalEnv("DEX_QUOTE_LIQUIDITY", "0");
  const pendingMultisig = optionalEnv("PENDING_MULTISIG_ADDRESS", "");
  const transferOwnershipTo = optionalEnv("TRANSFER_OWNERSHIP_TO", "");
  const verifyContracts = parseBool(process.env.VERIFY_CONTRACTS, false);
  const failOnVerifyError = parseBool(process.env.FAIL_ON_VERIFY_ERROR, false);

  assertAddress("COMPILER_VERIFIER_ADDRESS", compilerVerifier);
  assertAddress("CHECKER_VERIFIER_ADDRESS", checkerVerifier);
  assertAddress("DEX_QUOTE_TOKEN_ADDRESS", dexQuoteToken);
  if (pendingMultisig) assertAddress("PENDING_MULTISIG_ADDRESS", pendingMultisig);
  if (transferOwnershipTo) assertAddress("TRANSFER_OWNERSHIP_TO", transferOwnershipTo);

  if (expectedChainId && providerNetwork.chainId.toString() !== expectedChainId) {
    throw new Error(
      `Connected chainId ${providerNetwork.chainId} does not match PRODUCTION_CHAIN_ID ${expectedChainId}`
    );
  }

  const quoteTokenCode = await hre.ethers.provider.getCode(dexQuoteToken);
  if (quoteTokenCode === "0x") {
    throw new Error(
      `DEX_QUOTE_TOKEN_ADDRESS has no contract code: ${dexQuoteToken}`
    );
  }

  const quoteToken = await hre.ethers.getContractAt("IERC20", dexQuoteToken);
  const quoteBalance = await quoteToken.balanceOf(deployer.address);

  if (bootstrapDex) {
    const botLiquidity = hre.ethers.parseEther(dexBotLiquidity);
    const quoteLiquidity = hre.ethers.parseEther(dexQuoteLiquidity);
    if (botLiquidity === 0n || quoteLiquidity === 0n) {
      throw new Error(
        "BOOTSTRAP_DEX requires non-zero DEX_BOT_LIQUIDITY and DEX_QUOTE_LIQUIDITY"
      );
    }
    if (quoteBalance < quoteLiquidity) {
      throw new Error(
        `Insufficient deployer quote token balance for bootstrap: have ${quoteBalance}, need ${quoteLiquidity}`
      );
    }
  }

  if (verifyContracts && !process.env.ETHERSCAN_API_KEY) {
    throw new Error(
      "VERIFY_CONTRACTS=true requires ETHERSCAN_API_KEY to be set"
    );
  }

  const report = {
    network: networkName,
    chainId: providerNetwork.chainId.toString(),
    deployer: deployer.address,
    deployerNativeBalance: hre.ethers.formatEther(deployerBalance),
    compilerVerifier,
    checkerVerifier,
    dexQuoteToken,
    deployerQuoteTokenBalance: quoteBalance.toString(),
    marriageFeeBot,
    bootstrapDex,
    dexBotLiquidity,
    dexQuoteLiquidity,
    pendingMultisig: pendingMultisig || null,
    transferOwnershipTo: transferOwnershipTo || null,
    verifyContracts,
    failOnVerifyError,
  };

  console.log(JSON.stringify(report, null, 2));
  console.log("\nPreflight checks passed. No transactions were sent.");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
