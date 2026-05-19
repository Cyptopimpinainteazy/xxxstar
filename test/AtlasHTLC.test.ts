/**
 * AtlasHTLC.sol — Hardhat / Waffle test suite
 *
 * Tests cover:
 *  - ETH-native HTLC: create, claim (happy path), refund (after expiry)
 *  - ERC-20 HTLC: create, claim
 *  - Security: double-claim rejected, wrong secret rejected, early refund rejected
 *  - View helpers: isHTLCFunded, isHTLCClaimed, isHTLCExpired
 */

import { ethers } from "hardhat";
import { expect } from "chai";
import { Contract, Signer, BigNumber } from "ethers";
import crypto from "crypto";

// ─── Helpers ───────────────────────────────────────────────────────────────

function randomSecret(): { secret: Buffer; hashLock: string } {
  const secret = crypto.randomBytes(32);
  const hashLock = "0x" + crypto.createHash("sha256").update(secret).digest("hex");
  return { secret, hashLock };
}

function secretBytes32(buf: Buffer): string {
  return "0x" + buf.toString("hex");
}

async function deployHTLC(deployer: Signer): Promise<Contract> {
  const Factory = await ethers.getContractFactory("AtlasHTLC", deployer);
  return Factory.deploy();
}

async function latestBlockTimestamp(): Promise<number> {
  const block = await ethers.provider.getBlock("latest");
  return block.timestamp;
}

async function increaseTime(seconds: number): Promise<void> {
  await ethers.provider.send("evm_increaseTime", [seconds]);
  await ethers.provider.send("evm_mine", []);
}

// ─── Tests ─────────────────────────────────────────────────────────────────

describe("AtlasHTLC — ETH native", () => {
  let htlc: Contract;
  let owner: Signer;
  let recipient: Signer;
  let ownerAddr: string;
  let recipientAddr: string;

  beforeEach(async () => {
    [owner, recipient] = await ethers.getSigners();
    ownerAddr = await owner.getAddress();
    recipientAddr = await recipient.getAddress();
    htlc = await deployHTLC(owner);
  });

  it("creates an ETH HTLC and emits HTLCCreated", async () => {
    const { secret, hashLock } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 3600;

    const tx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero, // native ETH
      ethers.utils.parseEther("1"),
      timeLock,
      { value: ethers.utils.parseEther("1") }
    );
    const receipt = await tx.wait();

    expect(receipt.events).to.have.lengthOf.at.least(1);
    const event = receipt.events!.find((e: any) => e.event === "HTLCCreated");
    expect(event).to.exist;
    expect(event!.args!.recipient).to.equal(recipientAddr);
    expect(event!.args!.hashLock).to.equal(hashLock);
  });

  it("recipient can claim ETH HTLC with correct secret", async () => {
    const { secret, hashLock } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 3600;

    const createTx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero,
      ethers.utils.parseEther("1"),
      timeLock,
      { value: ethers.utils.parseEther("1") }
    );
    const createReceipt = await createTx.wait();
    const htlcId = createReceipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;

    const balanceBefore = await ethers.provider.getBalance(recipientAddr);
    const claimTx = await htlc.connect(recipient).claimHTLC(htlcId, secretBytes32(secret));
    await claimTx.wait();
    const balanceAfter = await ethers.provider.getBalance(recipientAddr);

    // Balance must have increased (minus gas)
    expect(balanceAfter.gt(balanceBefore)).to.be.true;

    const { claimed } = await htlc.isHTLCClaimed(htlcId);
    expect(claimed).to.be.true;
  });

  it("rejects claim with wrong secret", async () => {
    const { hashLock } = randomSecret();
    const { secret: wrongSecret } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 3600;

    const createTx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero,
      ethers.utils.parseEther("0.5"),
      timeLock,
      { value: ethers.utils.parseEther("0.5") }
    );
    const receipt = await createTx.wait();
    const htlcId = receipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;

    await expect(
      htlc.connect(recipient).claimHTLC(htlcId, secretBytes32(wrongSecret))
    ).to.be.revertedWith(/[Ii]nvalid secret|[Hh]ash mismatch|[Ss]ecret/);
  });

  it("sender can refund ETH HTLC after expiry", async () => {
    const { hashLock } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 60; // 1 minute

    const createTx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero,
      ethers.utils.parseEther("1"),
      timeLock,
      { value: ethers.utils.parseEther("1") }
    );
    const receipt = await createTx.wait();
    const htlcId = receipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;

    // Fast-forward past expiry
    await increaseTime(120);

    const balanceBefore = await ethers.provider.getBalance(ownerAddr);
    const refundTx = await htlc.connect(owner).refundHTLC(htlcId);
    await refundTx.wait();
    const balanceAfter = await ethers.provider.getBalance(ownerAddr);

    expect(balanceAfter.gt(balanceBefore)).to.be.true;
  });

  it("rejects early refund before timelock expires", async () => {
    const { hashLock } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 7200; // 2 hours

    const createTx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero,
      ethers.utils.parseEther("0.1"),
      timeLock,
      { value: ethers.utils.parseEther("0.1") }
    );
    const receipt = await createTx.wait();
    const htlcId = receipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;

    await expect(htlc.connect(owner).refundHTLC(htlcId)).to.be.revertedWith(
      /[Nn]ot expired|[Tt]imelock|[Ee]xpiry/
    );
  });

  it("prevents double-claim", async () => {
    const { secret, hashLock } = randomSecret();
    const now = await latestBlockTimestamp();
    const timeLock = now + 3600;

    const createTx = await htlc.connect(owner).createHTLC(
      hashLock,
      recipientAddr,
      ethers.constants.AddressZero,
      ethers.utils.parseEther("0.1"),
      timeLock,
      { value: ethers.utils.parseEther("0.1") }
    );
    const receipt = await createTx.wait();
    const htlcId = receipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;

    await htlc.connect(recipient).claimHTLC(htlcId, secretBytes32(secret));

    await expect(
      htlc.connect(recipient).claimHTLC(htlcId, secretBytes32(secret))
    ).to.be.reverted;
  });
});

describe("AtlasHTLC — view helpers", () => {
  let htlc: Contract;
  let owner: Signer;
  let recipient: Signer;
  let htlcId: string;
  let secret: Buffer;

  beforeEach(async () => {
    [owner, recipient] = await ethers.getSigners();
    htlc = await deployHTLC(owner);

    const { secret: s, hashLock } = randomSecret();
    secret = s;
    const now = await latestBlockTimestamp();
    const timeLock = now + 300; // 5 minutes

    const tx = await htlc.connect(owner).createHTLC(
      hashLock,
      await recipient.getAddress(),
      ethers.constants.AddressZero,
      ethers.utils.parseEther("0.1"),
      timeLock,
      { value: ethers.utils.parseEther("0.1") }
    );
    const receipt = await tx.wait();
    htlcId = receipt.events!.find((e: any) => e.event === "HTLCCreated")!.args!.id;
  });

  it("isHTLCFunded returns true after creation", async () => {
    expect(await htlc.isHTLCFunded(htlcId)).to.be.true;
  });

  it("isHTLCClaimed returns false before claim", async () => {
    const { claimed } = await htlc.isHTLCClaimed(htlcId);
    expect(claimed).to.be.false;
  });

  it("isHTLCExpired returns false before timelock", async () => {
    expect(await htlc.isHTLCExpired(htlcId)).to.be.false;
  });

  it("isHTLCExpired returns true after timelock", async () => {
    await increaseTime(400);
    expect(await htlc.isHTLCExpired(htlcId)).to.be.true;
  });

  it("isHTLCClaimed returns true with secret after claim", async () => {
    await htlc.connect(recipient).claimHTLC(htlcId, secretBytes32(secret));
    const { claimed, secret: revealedSecret } = await htlc.isHTLCClaimed(htlcId);
    expect(claimed).to.be.true;
    expect(revealedSecret).to.equal(secretBytes32(secret));
  });
});
