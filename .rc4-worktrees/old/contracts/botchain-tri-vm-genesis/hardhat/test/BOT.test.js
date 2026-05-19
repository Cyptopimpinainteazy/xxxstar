const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("BOT Token", function () {
  let bot;
  let owner;
  let alice;
  let bob;

  beforeEach(async function () {
    [owner, alice, bob] = await ethers.getSigners();

    const BOT = await ethers.getContractFactory("contracts/BOT.sol:BOT");
    bot = await BOT.deploy();
    await bot.waitForDeployment();
  });

  describe("Deployment", function () {
    it("Should set the correct name and symbol", async function () {
      expect(await bot.name()).to.equal("Botchain Token");
      expect(await bot.symbol()).to.equal("BOT");
    });

    it("Should mint initial supply to deployer", async function () {
      const initialSupply = ethers.parseEther("1000000");
      expect(await bot.balanceOf(owner.address)).to.equal(initialSupply);
    });

    it("Should set correct max supply", async function () {
      const maxSupply = ethers.parseEther("1000000000"); // 1 billion
      expect(await bot.MAX_SUPPLY()).to.equal(maxSupply);
    });
  });

  describe("Faucet", function () {
    it("Should allow faucet minting on local network", async function () {
      const amount = ethers.parseEther("1000");
      await bot.faucet(alice.address, amount);
      expect(await bot.balanceOf(alice.address)).to.equal(amount);
    });

    it("Should emit FaucetMint event", async function () {
      const amount = ethers.parseEther("500");
      await expect(bot.faucet(alice.address, amount))
        .to.emit(bot, "FaucetMint")
        .withArgs(alice.address, amount);
    });

    it("Should reject faucet mint exceeding max supply", async function () {
      const maxSupply = await bot.MAX_SUPPLY();
      const currentSupply = await bot.totalSupply();
      const exceedAmount = maxSupply - currentSupply + 1n;

      await expect(bot.faucet(alice.address, exceedAmount)).to.be.revertedWith(
        "BOT: max supply exceeded"
      );
    });

    it("Should reject faucet to zero address", async function () {
      const amount = ethers.parseEther("100");
      await expect(bot.faucet(ethers.ZeroAddress, amount)).to.be.revertedWith(
        "BOT: zero address"
      );
    });
  });

  describe("Owner Mint", function () {
    it("Should allow owner to mint", async function () {
      const amount = ethers.parseEther("10000");
      await bot.mint(alice.address, amount);
      expect(await bot.balanceOf(alice.address)).to.equal(amount);
    });

    it("Should reject non-owner mint", async function () {
      const amount = ethers.parseEther("10000");
      await expect(
        bot.connect(alice).mint(bob.address, amount)
      ).to.be.revertedWithCustomError(bot, "OwnableUnauthorizedAccount");
    });

    it("Should reject mint exceeding max supply", async function () {
      const maxSupply = await bot.MAX_SUPPLY();
      const currentSupply = await bot.totalSupply();
      const exceedAmount = maxSupply - currentSupply + 1n;

      await expect(bot.mint(alice.address, exceedAmount)).to.be.revertedWith(
        "BOT: max supply exceeded"
      );
    });
  });

  describe("Transfers", function () {
    it("Should transfer tokens between accounts", async function () {
      const amount = ethers.parseEther("1000");
      await bot.transfer(alice.address, amount);
      expect(await bot.balanceOf(alice.address)).to.equal(amount);
    });

    it("Should fail transfer if sender has insufficient balance", async function () {
      const amount = ethers.parseEther("1000");
      await expect(
        bot.connect(alice).transfer(bob.address, amount)
      ).to.be.revertedWithCustomError(bot, "ERC20InsufficientBalance");
    });
  });

  describe("Faucet Toggle", function () {
    it("Should allow owner to disable faucet", async function () {
      await bot.setFaucetEnabled(false);
      expect(await bot.faucetEnabled()).to.equal(false);
    });

    it("Should reject faucet when disabled", async function () {
      await bot.setFaucetEnabled(false);
      const amount = ethers.parseEther("100");
      await expect(bot.faucet(alice.address, amount)).to.be.revertedWith(
        "BOT: faucet disabled"
      );
    });
  });
});
