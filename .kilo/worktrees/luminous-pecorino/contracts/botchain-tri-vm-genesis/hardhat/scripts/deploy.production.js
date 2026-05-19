const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

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

function deploymentFilePath(networkName) {
  return path.join(
    __dirname,
    "..",
    "deployments",
    `${networkName}.production.json`
  );
}

async function verifyContract(address, constructorArguments) {
  await hre.run("verify:verify", {
    address,
    constructorArguments,
  });
}

async function maybeVerify(label, address, constructorArguments, deploymentRecord) {
  if (!deploymentRecord.configuration.verifyContracts) {
    return;
  }

  console.log(`\nVerifying ${label}...`);
  try {
    await verifyContract(address, constructorArguments);
    deploymentRecord.verification[label] = {
      address,
      status: "verified",
    };
    console.log(`   Verified ${label} at ${address}`);
  } catch (error) {
    const message = error && error.message ? error.message : String(error);
    deploymentRecord.verification[label] = {
      address,
      status: "failed",
      error: message,
    };
    console.log(`   Verification failed for ${label}: ${message}`);

    if (deploymentRecord.configuration.failOnVerifyError) {
      throw error;
    }
  }
}

async function main() {
  console.log("=== Botchain Production Contract Deployment ===\n");

  const networkName = hre.network.name;
  if (networkName === "hardhat") {
    throw new Error(
      "Refusing production deployment on the in-process hardhat network"
    );
  }

  const [deployer] = await hre.ethers.getSigners();
  const deployerBalance = await hre.ethers.provider.getBalance(deployer.address);

  console.log("Network:", networkName);
  console.log("Chain ID:", (await hre.ethers.provider.getNetwork()).chainId.toString());
  console.log("Deployer:", deployer.address);
  console.log("Balance:", hre.ethers.formatEther(deployerBalance), "ETH\n");

  const compilerVerifier = requireEnv("COMPILER_VERIFIER_ADDRESS");
  const checkerVerifier = requireEnv("CHECKER_VERIFIER_ADDRESS");
  const dexQuoteToken = requireEnv("DEX_QUOTE_TOKEN_ADDRESS");
  const marriageFee = hre.ethers.parseEther(
    optionalEnv("MARRIAGE_FEE_BOT", "100")
  );
  const bootstrapDex = parseBool(process.env.BOOTSTRAP_DEX, false);
  const dexBotLiquidity = hre.ethers.parseEther(
    optionalEnv("DEX_BOT_LIQUIDITY", "0")
  );
  const dexQuoteLiquidity = hre.ethers.parseEther(
    optionalEnv("DEX_QUOTE_LIQUIDITY", "0")
  );
  const pendingMultisig = optionalEnv("PENDING_MULTISIG_ADDRESS", "");
  const transferOwnershipTo = optionalEnv("TRANSFER_OWNERSHIP_TO", "");
  const verifyContracts = parseBool(process.env.VERIFY_CONTRACTS, false);
  const failOnVerifyError = parseBool(process.env.FAIL_ON_VERIFY_ERROR, false);

  assertAddress("COMPILER_VERIFIER_ADDRESS", compilerVerifier);
  assertAddress("CHECKER_VERIFIER_ADDRESS", checkerVerifier);
  assertAddress("DEX_QUOTE_TOKEN_ADDRESS", dexQuoteToken);
  if (pendingMultisig) assertAddress("PENDING_MULTISIG_ADDRESS", pendingMultisig);
  if (transferOwnershipTo) assertAddress("TRANSFER_OWNERSHIP_TO", transferOwnershipTo);

  if (bootstrapDex && (dexBotLiquidity === 0n || dexQuoteLiquidity === 0n)) {
    throw new Error(
      "BOOTSTRAP_DEX requires non-zero DEX_BOT_LIQUIDITY and DEX_QUOTE_LIQUIDITY"
    );
  }

  const addresses = {
    network: networkName,
    chainId: (await hre.ethers.provider.getNetwork()).chainId.toString(),
    deployer: deployer.address,
    deploymentMode: "production",
    configuration: {
      compilerVerifier,
      checkerVerifier,
      dexQuoteToken,
      marriageFeeBot: optionalEnv("MARRIAGE_FEE_BOT", "100"),
      bootstrapDex,
      dexBotLiquidity: optionalEnv("DEX_BOT_LIQUIDITY", "0"),
      dexQuoteLiquidity: optionalEnv("DEX_QUOTE_LIQUIDITY", "0"),
      pendingMultisig: pendingMultisig || null,
      transferOwnershipTo: transferOwnershipTo || null,
      verifyContracts,
      failOnVerifyError,
    },
    verification: {},
  };

  if (verifyContracts && !process.env.ETHERSCAN_API_KEY) {
    throw new Error(
      "VERIFY_CONTRACTS=true requires ETHERSCAN_API_KEY to be set"
    );
  }

  console.log("1. Deploying BOT...");
  const BOT = await hre.ethers.getContractFactory("BOT");
  const bot = await BOT.deploy();
  await bot.waitForDeployment();
  addresses.BOT = await bot.getAddress();
  console.log("   BOT:", addresses.BOT);
  await maybeVerify("BOT", addresses.BOT, [], addresses);

  console.log("\n2. Deploying MarriageLicense...");
  const MarriageLicense = await hre.ethers.getContractFactory("MarriageLicense");
  const marriageLicense = await MarriageLicense.deploy(
    addresses.BOT,
    compilerVerifier,
    checkerVerifier,
    marriageFee
  );
  await marriageLicense.waitForDeployment();
  addresses.MarriageLicense = await marriageLicense.getAddress();
  console.log("   MarriageLicense:", addresses.MarriageLicense);
  await maybeVerify(
    "MarriageLicense",
    addresses.MarriageLicense,
    [addresses.BOT, compilerVerifier, checkerVerifier, marriageFee],
    addresses
  );

  console.log("\n3. Deploying AtomicSwapAdapter...");
  const AtomicSwapAdapter = await hre.ethers.getContractFactory(
    "AtomicSwapAdapter"
  );
  const atomicSwap = await AtomicSwapAdapter.deploy();
  await atomicSwap.waitForDeployment();
  addresses.AtomicSwapAdapter = await atomicSwap.getAddress();
  console.log("   AtomicSwapAdapter:", addresses.AtomicSwapAdapter);
  await maybeVerify(
    "AtomicSwapAdapter",
    addresses.AtomicSwapAdapter,
    [],
    addresses
  );

  console.log("\n4. Deploying SimpleDEX...");
  const SimpleDEX = await hre.ethers.getContractFactory("SimpleDEX");
  const dex = await SimpleDEX.deploy(addresses.BOT, dexQuoteToken);
  await dex.waitForDeployment();
  addresses.SimpleDEX = await dex.getAddress();
  addresses.DEXQuoteToken = dexQuoteToken;
  console.log("   SimpleDEX:", addresses.SimpleDEX);
  console.log("   Quote token:", dexQuoteToken);
  await maybeVerify(
    "SimpleDEX",
    addresses.SimpleDEX,
    [addresses.BOT, dexQuoteToken],
    addresses
  );

  if (bootstrapDex) {
    console.log("\n5. Bootstrapping DEX liquidity...");
    const quoteToken = await hre.ethers.getContractAt("IERC20", dexQuoteToken);
    await bot.approve(addresses.SimpleDEX, dexBotLiquidity);
    await quoteToken.approve(addresses.SimpleDEX, dexQuoteLiquidity);
    await dex.addLiquidity(dexBotLiquidity, dexQuoteLiquidity, 0);
    console.log(
      "   Added initial liquidity:",
      hre.ethers.formatEther(dexBotLiquidity),
      "BOT /",
      hre.ethers.formatEther(dexQuoteLiquidity),
      "quote token"
    );
  } else {
    console.log("\n5. Skipping DEX bootstrap (BOOTSTRAP_DEX=false)");
  }

  if (pendingMultisig) {
    console.log("\n6. Scheduling MarriageLicense multisig update...");
    const tx = await marriageLicense.scheduleMultisigUpdate(pendingMultisig);
    await tx.wait();
    addresses.pendingMarriageLicenseMultisig = pendingMultisig;
    console.log("   Scheduled pending multisig:", pendingMultisig);
  }

  if (transferOwnershipTo) {
    console.log("\n7. Transferring contract ownership...\n   This affects BOT, MarriageLicense, and AtomicSwapAdapter.");
    await (await bot.transferOwnership(transferOwnershipTo)).wait();
    await (await marriageLicense.transferOwnership(transferOwnershipTo)).wait();
    await (await atomicSwap.transferOwnership(transferOwnershipTo)).wait();
    addresses.ownerTransferredTo = transferOwnershipTo;
    console.log("   Ownership transferred to:", transferOwnershipTo);
  }

  const outputPath = deploymentFilePath(networkName);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, JSON.stringify(addresses, null, 2));

  console.log("\n=== Production Deployment Summary ===");
  console.log(JSON.stringify(addresses, null, 2));
  console.log("\nSaved deployment record to:", outputPath);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
