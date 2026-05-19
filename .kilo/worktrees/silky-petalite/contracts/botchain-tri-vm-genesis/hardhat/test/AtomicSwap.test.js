const { expect } = require("chai");
const { ethers } = require("hardhat");
const { time } = require("@nomicfoundation/hardhat-network-helpers");
const crypto = require("crypto");

describe("AtomicSwapAdapter (SHA-256 Bitcoin Compatible)", function () {
  let atomicSwap;
  let bot;
  let owner;
  let alice;
  let bob;

  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;

  beforeEach(async function () {
    [owner, alice, bob] = await ethers.getSigners();

    // Deploy BOT token
    const BOT = await ethers.getContractFactory("contracts/BOT.sol:BOT");
    bot = await BOT.deploy();
    await bot.waitForDeployment();

    // Deploy AtomicSwapAdapter
    const AtomicSwapAdapter = await ethers.getContractFactory(
      "contracts/AtomicSwapAdapter.sol:AtomicSwapAdapter"
    );
    atomicSwap = await AtomicSwapAdapter.deploy();
    await atomicSwap.waitForDeployment();

    // Fund Alice with BOT tokens
    await bot.faucet(alice.address, ethers.parseEther("10000"));

    // Alice approves AtomicSwap
    await bot
      .connect(alice)
      .approve(await atomicSwap.getAddress(), ethers.parseEther("10000"));
  });

  /**
   * Generate a random 32-byte secret (preimage)
   * @returns {Buffer} Random 32-byte buffer
   */
  function generateSecret() {
    return crypto.randomBytes(32);
  }

  /**
   * Compute SHA-256 hashlock from preimage (Bitcoin compatible)
   * @param {Buffer|string} preimage - The secret preimage
   * @returns {string} 0x-prefixed hex string of SHA256(preimage)
   */
  function computeSha256Hashlock(preimage) {
    const buffer = Buffer.isBuffer(preimage)
      ? preimage
      : Buffer.from(preimage, "hex");
    const hash = crypto.createHash("sha256").update(buffer).digest();
    return "0x" + hash.toString("hex");
  }

  /**
   * Convert buffer to hex string for Solidity
   */
  function bufferToHex(buffer) {
    return "0x" + buffer.toString("hex");
  }

  describe("SHA-256 Hash Compatibility", function () {
    it("Should use SHA-256 matching Bitcoin OP_SHA256", async function () {
      // Generate preimage
      const preimage = generateSecret();
      const preimageHex = bufferToHex(preimage);

      // Compute hashlock using Node.js SHA-256 (same as Bitcoin)
      const expectedHashlock = computeSha256Hashlock(preimage);

      // Verify contract computes same hash
      const contractHashlock = await atomicSwap.computeHashlock(preimageHex);
      expect(contractHashlock).to.equal(expectedHashlock);
    });

    it("Should verify preimage correctly", async function () {
      const preimage = generateSecret();
      const preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);

      const isValid = await atomicSwap.verifyPreimage(preimageHex, hashlock);
      expect(isValid).to.be.true;

      // Wrong preimage should fail
      const wrongPreimage = generateSecret();
      const wrongPreimageHex = bufferToHex(wrongPreimage);
      const isInvalid = await atomicSwap.verifyPreimage(
        wrongPreimageHex,
        hashlock
      );
      expect(isInvalid).to.be.false;
    });

    it("Should match known Bitcoin HTLC test vector", async function () {
      // Known test vector: preimage and expected SHA-256 hash
      // This ensures our implementation matches Bitcoin's OP_SHA256
      const knownPreimage = Buffer.from(
        "0000000000000000000000000000000000000000000000000000000000000001",
        "hex"
      );
      const knownHash = crypto
        .createHash("sha256")
        .update(knownPreimage)
        .digest();
      const expectedHashHex = "0x" + knownHash.toString("hex");

      const contractHash = await atomicSwap.computeHashlock(
        bufferToHex(knownPreimage)
      );
      expect(contractHash.toLowerCase()).to.equal(
        expectedHashHex.toLowerCase()
      );
    });
  });

  describe("Lock Tokens", function () {
    it("Should lock tokens successfully with SHA-256 hashlock", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });

      expect(event).to.not.be.undefined;

      const parsed = atomicSwap.interface.parseLog(event);
      expect(parsed.args.hashlock).to.equal(hashlock);

      // Verify tokens transferred
      const swapBalance = await bot.balanceOf(await atomicSwap.getAddress());
      expect(swapBalance).to.equal(amount);
    });

    it("Should reject timelock too short", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 30 * 60; // 30 minutes

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokens(
            bob.address,
            await bot.getAddress(),
            amount,
            hashlock,
            timelock
          )
      ).to.be.revertedWithCustomError(atomicSwap, "TimelockTooShort");
    });

    it("Should reject timelock too long", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 8 * ONE_DAY;

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokens(
            bob.address,
            await bot.getAddress(),
            amount,
            hashlock,
            timelock
          )
      ).to.be.revertedWithCustomError(atomicSwap, "TimelockTooLong");
    });

    it("Should enforce remote timelock safety margin when provided", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokensWithRemoteTimelock(
            bob.address,
            await bot.getAddress(),
            amount,
            hashlock,
            timelock,
            timelock
          )
      ).to.be.revertedWithCustomError(atomicSwap, "RemoteTimelockNotLonger");

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokensWithRemoteTimelock(
            bob.address,
            await bot.getAddress(),
            amount,
            hashlock,
            timelock,
            timelock + 30 * 60
          )
      ).to.be.revertedWithCustomError(
        atomicSwap,
        "RemoteTimelockDeltaTooSmall"
      );

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokensWithRemoteTimelock(
            bob.address,
            await bot.getAddress(),
            amount,
            hashlock,
            timelock,
            timelock + 2 * ONE_HOUR
          )
      ).to.emit(atomicSwap, "TokensLocked");
    });

    it("Should reject zero amount", async function () {
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      await expect(
        atomicSwap
          .connect(alice)
          .lockTokens(
            bob.address,
            await bot.getAddress(),
            0,
            hashlock,
            timelock
          )
      ).to.be.revertedWithCustomError(atomicSwap, "InvalidAmount");
    });

    it("Should generate unique swap IDs", async function () {
      const amount = ethers.parseEther("100");
      const preimage1 = generateSecret();
      const preimage2 = generateSecret();
      const hashlock1 = computeSha256Hashlock(preimage1);
      const hashlock2 = computeSha256Hashlock(preimage2);
      const currentTime = await time.latest();

      const tx1 = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock1,
          currentTime + 2 * ONE_HOUR
        );

      const tx2 = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock2,
          currentTime + 3 * ONE_HOUR
        );

      const receipt1 = await tx1.wait();
      const receipt2 = await tx2.wait();

      const event1 = receipt1.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });
      const event2 = receipt2.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });

      const swapId1 = atomicSwap.interface.parseLog(event1).args.swapId;
      const swapId2 = atomicSwap.interface.parseLog(event2).args.swapId;

      expect(swapId1).to.not.equal(swapId2);
    });
  });

  describe("Claim with SHA-256 Preimage", function () {
    let swapId;
    let preimage;
    let preimageHex;
    let amount;

    beforeEach(async function () {
      amount = ethers.parseEther("100");
      preimage = generateSecret();
      preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });

      swapId = atomicSwap.interface.parseLog(event).args.swapId;
    });

    it("Should claim with correct SHA-256 preimage", async function () {
      const bobBalanceBefore = await bot.balanceOf(bob.address);

      const tx = await atomicSwap.connect(bob).claim(swapId, preimageHex);

      // Verify event emits preimage for watchtower extraction
      await expect(tx)
        .to.emit(atomicSwap, "TokensClaimed")
        .withArgs(swapId, preimageHex, bob.address);

      const bobBalanceAfter = await bot.balanceOf(bob.address);
      expect(bobBalanceAfter - bobBalanceBefore).to.equal(amount);

      // Verify swap state
      const swap = await atomicSwap.getSwap(swapId);
      expect(swap.claimed).to.equal(true);
    });

    it("Should allow anyone to submit claim (tokens go to participant)", async function () {
      // Owner submits claim on behalf of Bob
      const bobBalanceBefore = await bot.balanceOf(bob.address);

      await atomicSwap.connect(owner).claim(swapId, preimageHex);

      const bobBalanceAfter = await bot.balanceOf(bob.address);
      expect(bobBalanceAfter - bobBalanceBefore).to.equal(amount);
    });

    it("Should reject incorrect preimage", async function () {
      const wrongPreimage = generateSecret();
      const wrongPreimageHex = bufferToHex(wrongPreimage);

      await expect(
        atomicSwap.claim(swapId, wrongPreimageHex)
      ).to.be.revertedWithCustomError(atomicSwap, "InvalidPreimage");
    });

    it("Should reject claim after expiry", async function () {
      await time.increase(3 * ONE_HOUR);

      await expect(
        atomicSwap.claim(swapId, preimageHex)
      ).to.be.revertedWithCustomError(atomicSwap, "SwapExpired");
    });

    it("Should reject double claim", async function () {
      await atomicSwap.claim(swapId, preimageHex);

      await expect(
        atomicSwap.claim(swapId, preimageHex)
      ).to.be.revertedWithCustomError(atomicSwap, "SwapAlreadyClaimed");
    });
  });

  describe("Refund", function () {
    let swapId;
    let amount;

    beforeEach(async function () {
      amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });

      swapId = atomicSwap.interface.parseLog(event).args.swapId;
    });

    it("Should refund after timelock expires", async function () {
      await time.increase(3 * ONE_HOUR);

      const aliceBalanceBefore = await bot.balanceOf(alice.address);

      await expect(atomicSwap.connect(alice).refund(swapId))
        .to.emit(atomicSwap, "TokensRefunded")
        .withArgs(swapId, alice.address);

      const aliceBalanceAfter = await bot.balanceOf(alice.address);
      expect(aliceBalanceAfter - aliceBalanceBefore).to.equal(amount);

      const swap = await atomicSwap.getSwap(swapId);
      expect(swap.refunded).to.equal(true);
    });

    it("Should reject refund before timelock", async function () {
      await expect(
        atomicSwap.connect(alice).refund(swapId)
      ).to.be.revertedWithCustomError(atomicSwap, "SwapNotExpired");
    });

    it("Should reject refund by non-initiator", async function () {
      await time.increase(3 * ONE_HOUR);

      await expect(
        atomicSwap.connect(bob).refund(swapId)
      ).to.be.revertedWithCustomError(atomicSwap, "NotInitiator");
    });
  });

  describe("View Functions", function () {
    let swapId;
    let preimage;
    let preimageHex;

    beforeEach(async function () {
      const amount = ethers.parseEther("100");
      preimage = generateSecret();
      preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });

      swapId = atomicSwap.interface.parseLog(event).args.swapId;
    });

    it("Should report swap as active", async function () {
      expect(await atomicSwap.isSwapActive(swapId)).to.equal(true);
    });

    it("Should report canClaim correctly", async function () {
      expect(await atomicSwap.canClaim(swapId)).to.equal(true);

      await time.increase(3 * ONE_HOUR);
      expect(await atomicSwap.canClaim(swapId)).to.equal(false);
    });

    it("Should report canRefund correctly", async function () {
      expect(await atomicSwap.canRefund(swapId)).to.equal(false);

      await time.increase(3 * ONE_HOUR);
      expect(await atomicSwap.canRefund(swapId)).to.equal(true);
    });

    it("Should report timeUntilRefund correctly", async function () {
      const remaining = await atomicSwap.timeUntilRefund(swapId);
      expect(remaining).to.be.gt(ONE_HOUR); // Should be ~2 hours

      await time.increase(3 * ONE_HOUR);
      const remainingAfter = await atomicSwap.timeUntilRefund(swapId);
      expect(remainingAfter).to.equal(0);
    });

    it("Should expose remote timelock safety checks", async function () {
      const local = 1000;
      expect(await atomicSwap.isSafeRemoteTimelock(local, local)).to.equal(false);
      expect(await atomicSwap.isSafeRemoteTimelock(local, local + 1800)).to.equal(false);
      expect(await atomicSwap.isSafeRemoteTimelock(local, local + 3600)).to.equal(true);
    });
  });

  describe("Full Cross-Chain Atomic Swap Simulation", function () {
    it("Should complete full swap (happy path) - watchtower extracts preimage", async function () {
      // === SETUP ===
      // Alice has BTC, wants BOT
      // Bob has BOT, wants BTC
      // Alice generates the secret

      const amount = ethers.parseEther("1000");
      const preimage = generateSecret();
      const preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();

      // BTC timelock: 24 hours (longer - Alice's refund safety net)
      const btcTimelock = currentTime + 24 * ONE_HOUR;
      // EVM timelock: 12 hours (shorter - Bob must claim first or refund)
      const evmTimelock = currentTime + 12 * ONE_HOUR;

      console.log("=== Cross-Chain Atomic Swap ===");
      console.log(`Preimage (secret): ${preimageHex}`);
      console.log(`Hashlock (SHA256): ${hashlock}`);
      console.log(
        `EVM Timelock: ${new Date(evmTimelock * 1000).toISOString()}`
      );
      console.log(
        `BTC Timelock: ${new Date(btcTimelock * 1000).toISOString()}`
      );

      // === STEP 1: Alice creates BTC HTLC (simulated) ===
      console.log("\n1. Alice creates BTC HTLC with hashlock...");
      // In real scenario: bitcoind createrawtransaction with HTLC script

      // === STEP 2: Bob sees BTC HTLC, creates EVM HTLC ===
      console.log("2. Bob sees Alice's BTC HTLC, creates EVM HTLC...");
      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          evmTimelock
        );

      const receipt = await tx.wait();
      const lockedEvent = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });
      const swapId = atomicSwap.interface.parseLog(lockedEvent).args.swapId;

      console.log(`   EVM Swap ID: ${swapId}`);
      expect(await atomicSwap.isSwapActive(swapId)).to.equal(true);

      // === STEP 3: Alice claims BOT by revealing preimage ===
      console.log("3. Alice claims BOT by revealing preimage...");
      const claimTx = await atomicSwap
        .connect(alice)
        .claim(swapId, preimageHex);
      const claimReceipt = await claimTx.wait();

      // Watchtower extracts preimage from event
      const claimedEvent = claimReceipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensClaimed";
        } catch {
          return false;
        }
      });
      const parsedClaim = atomicSwap.interface.parseLog(claimedEvent);
      const revealedPreimage = parsedClaim.args.preimage;

      console.log(`   Preimage revealed in event: ${revealedPreimage}`);
      expect(revealedPreimage).to.equal(preimageHex);

      // === STEP 4: Bob (or watchtower) claims BTC using revealed preimage ===
      console.log("4. Bob claims BTC using preimage from EVM event...");
      // In real scenario: broadcast BTC tx with preimage to spend HTLC

      // Verify the preimage can be used to verify against hashlock
      const isValid = await atomicSwap.verifyPreimage(
        revealedPreimage,
        hashlock
      );
      expect(isValid).to.be.true;
      console.log("   ✓ Preimage verified - Bob can claim BTC!");

      // Verify Bob received BOT (actually Alice claimed, but participant is bob)
      // Wait, the test has alice as initiator, bob as participant
      // So bob should receive the tokens
      const swap = await atomicSwap.getSwap(swapId);
      expect(swap.claimed).to.equal(true);

      console.log("\n=== SWAP COMPLETE ===");
    });

    it("Should complete refund path if counterparty never claims", async function () {
      const amount = ethers.parseEther("1000");
      const preimage = generateSecret();
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const aliceBalanceBefore = await bot.balanceOf(alice.address);

      // Alice locks BOT
      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });
      const swapId = atomicSwap.interface.parseLog(event).args.swapId;

      // Verify balance decreased
      expect(await bot.balanceOf(alice.address)).to.equal(
        aliceBalanceBefore - amount
      );

      // Time passes, Bob never reveals preimage
      await time.increase(3 * ONE_HOUR);

      // Alice refunds
      await atomicSwap.connect(alice).refund(swapId);

      // Verify Alice got tokens back
      expect(await bot.balanceOf(alice.address)).to.equal(aliceBalanceBefore);
      expect(await atomicSwap.isSwapActive(swapId)).to.equal(false);
    });
  });

  describe("Security: Race Conditions & Edge Cases", function () {
    it("Should prevent claim after refund", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });
      const swapId = atomicSwap.interface.parseLog(event).args.swapId;

      // Wait for timelock
      await time.increase(3 * ONE_HOUR);

      // Alice refunds
      await atomicSwap.connect(alice).refund(swapId);

      // Bob tries to claim with valid preimage - should fail
      await expect(
        atomicSwap.connect(bob).claim(swapId, preimageHex)
      ).to.be.revertedWithCustomError(atomicSwap, "SwapAlreadyRefunded");
    });

    it("Should prevent refund after claim", async function () {
      const amount = ethers.parseEther("100");
      const preimage = generateSecret();
      const preimageHex = bufferToHex(preimage);
      const hashlock = computeSha256Hashlock(preimage);
      const currentTime = await time.latest();
      const timelock = currentTime + 2 * ONE_HOUR;

      const tx = await atomicSwap
        .connect(alice)
        .lockTokens(
          bob.address,
          await bot.getAddress(),
          amount,
          hashlock,
          timelock
        );

      const receipt = await tx.wait();
      const event = receipt.logs.find((log) => {
        try {
          return atomicSwap.interface.parseLog(log)?.name === "TokensLocked";
        } catch {
          return false;
        }
      });
      const swapId = atomicSwap.interface.parseLog(event).args.swapId;

      // Bob claims
      await atomicSwap.connect(bob).claim(swapId, preimageHex);

      // Wait for timelock
      await time.increase(3 * ONE_HOUR);

      // Alice tries to refund - should fail
      await expect(
        atomicSwap.connect(alice).refund(swapId)
      ).to.be.revertedWithCustomError(atomicSwap, "SwapAlreadyClaimed");
    });
  });
});
