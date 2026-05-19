const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("SimpleDEX", function () {
  let dex;
  let tokenA; // BOT
  let tokenB; // Mock WETH
  let owner;
  let alice;
  let bob;

  beforeEach(async function () {
    [owner, alice, bob] = await ethers.getSigners();

    // Deploy tokens
    const BOT = await ethers.getContractFactory("contracts/BOT.sol:BOT");
    tokenA = await BOT.deploy();
    await tokenA.waitForDeployment();

    tokenB = await BOT.deploy(); // Reuse BOT as mock token B
    await tokenB.waitForDeployment();

    // Deploy DEX
    const SimpleDEX = await ethers.getContractFactory(
      "contracts/SimpleDEX.sol:SimpleDEX"
    );
    dex = await SimpleDEX.deploy(
      await tokenA.getAddress(),
      await tokenB.getAddress()
    );
    await dex.waitForDeployment();

    // Fund accounts
    await tokenA.faucet(owner.address, ethers.parseEther("1000000"));
    await tokenB.faucet(owner.address, ethers.parseEther("1000000"));
    await tokenA.faucet(alice.address, ethers.parseEther("100000"));
    await tokenB.faucet(alice.address, ethers.parseEther("100000"));
    await tokenA.faucet(bob.address, ethers.parseEther("100000"));
    await tokenB.faucet(bob.address, ethers.parseEther("100000"));

    // Approve DEX
    const maxApproval = ethers.parseEther("1000000");
    await tokenA.approve(await dex.getAddress(), maxApproval);
    await tokenB.approve(await dex.getAddress(), maxApproval);
    await tokenA.connect(alice).approve(await dex.getAddress(), maxApproval);
    await tokenB.connect(alice).approve(await dex.getAddress(), maxApproval);
    await tokenA.connect(bob).approve(await dex.getAddress(), maxApproval);
    await tokenB.connect(bob).approve(await dex.getAddress(), maxApproval);
  });

  describe("Add Liquidity", function () {
    it("Should add initial liquidity", async function () {
      const amountA = ethers.parseEther("10000");
      const amountB = ethers.parseEther("10000");

      await expect(dex.addLiquidity(amountA, amountB, 0)).to.emit(
        dex,
        "LiquidityAdded"
      );

      expect(await dex.reserveA()).to.equal(amountA);
      expect(await dex.reserveB()).to.equal(amountB);

      // Check LP tokens (minus MINIMUM_LIQUIDITY)
      const lpBalance = await dex.balanceOf(owner.address);
      expect(lpBalance).to.be.gt(0);
    });

    it("Should add proportional liquidity", async function () {
      // First add
      await dex.addLiquidity(
        ethers.parseEther("10000"),
        ethers.parseEther("10000"),
        0
      );

      // Second add by alice
      const lpBefore = await dex.balanceOf(alice.address);
      await dex
        .connect(alice)
        .addLiquidity(ethers.parseEther("5000"), ethers.parseEther("5000"), 0);
      const lpAfter = await dex.balanceOf(alice.address);

      expect(lpAfter).to.be.gt(lpBefore);
    });

    it("Should reject zero amounts", async function () {
      await expect(
        dex.addLiquidity(0, ethers.parseEther("100"), 0)
      ).to.be.revertedWith("Amounts must be positive");
    });

    it("Should support very large initial liquidity without overflow", async function () {
      const largeAmount = 2n ** 128n;
      const HighSupplyToken = await ethers.getContractFactory(
        "contracts/HighSupplyToken.sol:HighSupplyToken"
      );
      const largeTokenA = await HighSupplyToken.deploy(
        "Large Token A",
        "LTA",
        largeAmount
      );
      await largeTokenA.waitForDeployment();

      const largeTokenB = await HighSupplyToken.deploy(
        "Large Token B",
        "LTB",
        largeAmount
      );
      await largeTokenB.waitForDeployment();

      const LargeDex = await ethers.getContractFactory(
        "contracts/SimpleDEX.sol:SimpleDEX"
      );
      const largeDex = await LargeDex.deploy(
        await largeTokenA.getAddress(),
        await largeTokenB.getAddress()
      );
      await largeDex.waitForDeployment();

      await largeTokenA.approve(await largeDex.getAddress(), largeAmount);
      await largeTokenB.approve(await largeDex.getAddress(), largeAmount);

      await expect(largeDex.addLiquidity(largeAmount, largeAmount, 0)).to.emit(
        largeDex,
        "LiquidityAdded"
      );

      expect(await largeDex.reserveA()).to.equal(largeAmount);
      expect(await largeDex.reserveB()).to.equal(largeAmount);
    });
  });

  describe("Remove Liquidity", function () {
    beforeEach(async function () {
      await dex.addLiquidity(
        ethers.parseEther("10000"),
        ethers.parseEther("10000"),
        0
      );
    });

    it("Should remove liquidity", async function () {
      const lpBalance = await dex.balanceOf(owner.address);
      const halfLP = lpBalance / 2n;

      await expect(dex.removeLiquidity(halfLP, 0, 0)).to.emit(
        dex,
        "LiquidityRemoved"
      );

      const newBalance = await dex.balanceOf(owner.address);
      expect(newBalance).to.be.lt(lpBalance);
    });

    it("Should reject insufficient LP tokens", async function () {
      const tooManyLP = ethers.parseEther("999999999");
      await expect(dex.removeLiquidity(tooManyLP, 0, 0)).to.be.revertedWith(
        "Insufficient LP tokens"
      );
    });
  });

  describe("Swap", function () {
    beforeEach(async function () {
      await dex.addLiquidity(
        ethers.parseEther("10000"),
        ethers.parseEther("10000"),
        0
      );
    });

    it("Should swap A for B", async function () {
      const amountIn = ethers.parseEther("100");
      const expectedOut = await dex.getAmountOutAToB(amountIn);

      const balanceBefore = await tokenB.balanceOf(alice.address);

      await expect(dex.connect(alice).swapAForB(amountIn, 0)).to.emit(
        dex,
        "Swap"
      );

      const balanceAfter = await tokenB.balanceOf(alice.address);
      expect(balanceAfter - balanceBefore).to.be.gte(expectedOut);
    });

    it("Should swap B for A", async function () {
      const amountIn = ethers.parseEther("100");
      const expectedOut = await dex.getAmountOutBToA(amountIn);

      const balanceBefore = await tokenA.balanceOf(alice.address);

      await dex.connect(alice).swapBForA(amountIn, 0);

      const balanceAfter = await tokenA.balanceOf(alice.address);
      expect(balanceAfter - balanceBefore).to.be.gte(expectedOut);
    });

    it("Should apply 0.3% fee", async function () {
      const amountIn = ethers.parseEther("1000");

      // Without fee, output would be ~1000 * 10000 / 11000 ≈ 909
      // With 0.3% fee, it should be slightly less
      const amountOut = await dex.getAmountOutAToB(amountIn);

      // Fee reduces output by ~0.3%
      const idealOut =
        (amountIn * ethers.parseEther("10000")) /
        (ethers.parseEther("10000") + amountIn);

      expect(amountOut).to.be.lt(idealOut);
    });

    it("Should reject insufficient output", async function () {
      const amountIn = ethers.parseEther("100");
      const impossibleMinOut = ethers.parseEther("1000");

      await expect(
        dex.connect(alice).swapAForB(amountIn, impossibleMinOut)
      ).to.be.revertedWith("Insufficient output");
    });

    it("Should update reserves after swap", async function () {
      const reserveABefore = await dex.reserveA();
      const reserveBBefore = await dex.reserveB();

      const amountIn = ethers.parseEther("100");
      await dex.connect(alice).swapAForB(amountIn, 0);

      const reserveAAfter = await dex.reserveA();
      const reserveBAfter = await dex.reserveB();

      expect(reserveAAfter).to.be.gt(reserveABefore);
      expect(reserveBAfter).to.be.lt(reserveBBefore);
    });
  });

  describe("Price Functions", function () {
    beforeEach(async function () {
      await dex.addLiquidity(
        ethers.parseEther("10000"),
        ethers.parseEther("20000"),
        0
      );
    });

    it("Should return correct price A in B", async function () {
      const price = await dex.getPriceAInB();
      // reserveB / reserveA = 20000 / 10000 = 2
      expect(price).to.equal(ethers.parseEther("2"));
    });

    it("Should return correct price B in A", async function () {
      const price = await dex.getPriceBInA();
      // reserveA / reserveB = 10000 / 20000 = 0.5
      expect(price).to.equal(ethers.parseEther("0.5"));
    });
  });

  describe("Constant Product", function () {
    it("Should maintain k after swaps", async function () {
      await dex.addLiquidity(
        ethers.parseEther("10000"),
        ethers.parseEther("10000"),
        0
      );

      const [reserveA1, reserveB1] = await dex.getReserves();
      const k1 = reserveA1 * reserveB1;

      // Do a swap
      await dex.connect(alice).swapAForB(ethers.parseEther("100"), 0);

      const [reserveA2, reserveB2] = await dex.getReserves();
      const k2 = reserveA2 * reserveB2;

      // k should stay same or increase (due to fees)
      expect(k2).to.be.gte(k1);
    });
  });
});
