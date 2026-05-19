const hre = require("hardhat");
const fs = require("fs");
const path = require("path");

async function main() {
  console.log("=== Botchain Tri-VM Genesis Contract Deployment ===\n");

  const [deployer] = await hre.ethers.getSigners();
  console.log("Deployer address:", deployer.address);
  console.log(
    "Deployer balance:",
    hre.ethers.formatEther(
      await hre.ethers.provider.getBalance(deployer.address)
    ),
    "ETH\n"
  );

  // Deployment addresses storage
  const addresses = {};

  // 1. Deploy BOT Token
  console.log("1. Deploying BOT Token...");
  const BOT = await hre.ethers.getContractFactory("BOT");
  const bot = await BOT.deploy();
  await bot.waitForDeployment();
  const botAddress = await bot.getAddress();
  console.log("   BOT Token deployed to:", botAddress);
  addresses.BOT = botAddress;

  // 2. Deploy MarriageLicense
  console.log("\n2. Deploying MarriageLicense...");
  const marriageFee = hre.ethers.parseEther("100"); // 100 BOT
  const MarriageLicense = await hre.ethers.getContractFactory(
    "MarriageLicense"
  );
  const marriageLicense = await MarriageLicense.deploy(
    botAddress,
    deployer.address, // Compiler verifier (use deployer for testing)
    deployer.address, // Checker verifier (use deployer for testing)
    marriageFee
  );
  await marriageLicense.waitForDeployment();
  const marriageAddress = await marriageLicense.getAddress();
  console.log("   MarriageLicense deployed to:", marriageAddress);
  console.log("   Marriage fee:", hre.ethers.formatEther(marriageFee), "BOT");
  addresses.MarriageLicense = marriageAddress;

  // 3. Deploy AtomicSwapAdapter
  console.log("\n3. Deploying AtomicSwapAdapter...");
  const AtomicSwapAdapter = await hre.ethers.getContractFactory(
    "AtomicSwapAdapter"
  );
  const atomicSwap = await AtomicSwapAdapter.deploy();
  await atomicSwap.waitForDeployment();
  const atomicSwapAddress = await atomicSwap.getAddress();
  console.log("   AtomicSwapAdapter deployed to:", atomicSwapAddress);
  addresses.AtomicSwapAdapter = atomicSwapAddress;

  // 4. Deploy Mock WETH for DEX (in production, use actual WETH)
  console.log("\n4. Deploying Mock WETH for DEX testing...");
  const MockWETH = await hre.ethers.getContractFactory("BOT"); // Reuse BOT as mock
  const weth = await MockWETH.deploy();
  await weth.waitForDeployment();
  const wethAddress = await weth.getAddress();
  console.log("   Mock WETH deployed to:", wethAddress);
  addresses.MockWETH = wethAddress;

  // 5. Deploy SimpleDEX
  console.log("\n5. Deploying SimpleDEX (BOT/WETH pair)...");
  const SimpleDEX = await hre.ethers.getContractFactory("SimpleDEX");
  const dex = await SimpleDEX.deploy(botAddress, wethAddress);
  await dex.waitForDeployment();
  const dexAddress = await dex.getAddress();
  console.log("   SimpleDEX deployed to:", dexAddress);
  addresses.SimpleDEX = dexAddress;

  // 6. Setup initial state
  console.log("\n6. Setting up initial state...");

  // Approve MarriageLicense to spend BOT
  const approvalAmount = hre.ethers.parseEther("1000000");
  await bot.approve(marriageAddress, approvalAmount);
  console.log("   Approved MarriageLicense to spend BOT");

  // Add initial liquidity to DEX
  const liquidityBot = hre.ethers.parseEther("10000");
  const liquidityWeth = hre.ethers.parseEther("10000");

  await bot.approve(dexAddress, liquidityBot);
  await weth.approve(dexAddress, liquidityWeth);

  // Mint WETH to deployer
  await weth.faucet(deployer.address, liquidityWeth);

  await dex.addLiquidity(liquidityBot, liquidityWeth, 0);
  console.log("   Added initial DEX liquidity: 10000 BOT / 10000 WETH");

  // 7. Save addresses to file
  console.log("\n7. Saving deployment addresses...");
  const addressesPath = path.join(
    __dirname,
    "..",
    "deployments",
    "addresses.json"
  );
  fs.mkdirSync(path.dirname(addressesPath), { recursive: true });
  fs.writeFileSync(addressesPath, JSON.stringify(addresses, null, 2));
  console.log("   Addresses saved to:", addressesPath);

  // Summary
  console.log("\n=== Deployment Summary ===");
  console.log("BOT Token:        ", addresses.BOT);
  console.log("MarriageLicense:  ", addresses.MarriageLicense);
  console.log("AtomicSwapAdapter:", addresses.AtomicSwapAdapter);
  console.log("SimpleDEX:        ", addresses.SimpleDEX);
  console.log("Mock WETH:        ", addresses.MockWETH);
  console.log("\nDeployment complete! ✓");

  return addresses;
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
