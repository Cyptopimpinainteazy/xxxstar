const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("Hardhat project hygiene", function () {
  it("resolves BOT from the primary contracts source without ambiguity after a clean build", async function () {
    const BOT = await ethers.getContractFactory("BOT");
    const bot = await BOT.deploy();
    await bot.waitForDeployment();

    expect(await bot.symbol()).to.equal("BOT");
  });
});
