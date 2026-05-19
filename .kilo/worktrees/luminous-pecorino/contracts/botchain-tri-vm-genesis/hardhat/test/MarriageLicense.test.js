const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("MarriageLicense", function () {
  let bot;
  let marriageLicense;
  let owner;
  let alice;
  let bob;
  let compilerSigner;
  let checkerSigner;

  const marriageFee = ethers.parseEther("100");

  beforeEach(async function () {
    [owner, alice, bob, compilerSigner, checkerSigner] =
      await ethers.getSigners();

    // Deploy BOT token
    const BOT = await ethers.getContractFactory("contracts/BOT.sol:BOT");
    bot = await BOT.deploy();
    await bot.waitForDeployment();

    // Deploy MarriageLicense
    const MarriageLicense = await ethers.getContractFactory(
      "contracts/MarriageLicense.sol:MarriageLicense"
    );
    marriageLicense = await MarriageLicense.deploy(
      await bot.getAddress(),
      compilerSigner.address,
      checkerSigner.address,
      marriageFee
    );
    await marriageLicense.waitForDeployment();

    // Fund Alice with BOT tokens
    await bot.faucet(alice.address, ethers.parseEther("10000"));

    // Alice approves MarriageLicense
    await bot
      .connect(alice)
      .approve(await marriageLicense.getAddress(), ethers.parseEther("10000"));
  });

  async function signCreateChild(
    artifactCID,
    parentA,
    parentB,
    creator,
    signer,
    contractInstance = marriageLicense
  ) {
    const network = await ethers.provider.getNetwork();
    const messageHash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["bytes32", "uint256", "uint256", "address", "uint256", "address"],
        [
          artifactCID,
          parentA,
          parentB,
          creator,
          network.chainId,
          await contractInstance.getAddress(),
        ]
      )
    );

    return signer.signMessage(ethers.getBytes(messageHash));
  }

  async function increaseTime(seconds) {
    await ethers.provider.send("evm_increaseTime", [seconds]);
    await ethers.provider.send("evm_mine", []);
  }

  describe("Deployment", function () {
    it("Should set the admin action delay", async function () {
      expect(await marriageLicense.ADMIN_ACTION_DELAY()).to.equal(3 * 24 * 60 * 60);
    });

    it("Should set correct token address", async function () {
      expect(await marriageLicense.botToken()).to.equal(await bot.getAddress());
    });

    it("Should set correct verifiers", async function () {
      expect(await marriageLicense.compilerVerifier()).to.equal(
        compilerSigner.address
      );
      expect(await marriageLicense.checkerVerifier()).to.equal(
        checkerSigner.address
      );
    });

    it("Should set correct marriage fee", async function () {
      expect(await marriageLicense.marriageFee()).to.equal(marriageFee);
    });

    it("Should start nextChildId at 1", async function () {
      expect(await marriageLicense.nextChildId()).to.equal(1);
    });
  });

  describe("Create Child", function () {
    it("Should create child with valid signatures", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-1")
      );

      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      await expect(
        marriageLicense.connect(alice).createChild(
          artifactCID,
          compilerSig,
          checkerSig,
          0, // parentA (genesis)
          0 // parentB (genesis)
        )
      )
        .to.emit(marriageLicense, "ChildCreated")
        .withArgs(1, alice.address, artifactCID, 0, 0);

      // Verify child data
      const child = await marriageLicense.getChild(1);
      expect(child.owner).to.equal(alice.address);
      expect(child.artifactCID).to.equal(artifactCID);
      expect(child.isActive).to.equal(true);
    });

    it("Should transfer fee on child creation", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-2")
      );
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      const aliceBalanceBefore = await bot.balanceOf(alice.address);

      await marriageLicense
        .connect(alice)
        .createChild(artifactCID, compilerSig, checkerSig, 0, 0);

      const aliceBalanceAfter = await bot.balanceOf(alice.address);
      expect(aliceBalanceBefore - aliceBalanceAfter).to.equal(marriageFee);
    });

    it("Should reject invalid compiler signature", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-3")
      );

      // Sign with wrong signer
      const badCompilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        bob
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      await expect(
        marriageLicense
          .connect(alice)
          .createChild(artifactCID, badCompilerSig, checkerSig, 0, 0)
      ).to.be.revertedWith("Invalid compiler signature");
    });

    it("Should reject duplicate manifest", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-4")
      );
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      // First creation succeeds
      await marriageLicense
        .connect(alice)
        .createChild(artifactCID, compilerSig, checkerSig, 0, 0);

      // Second creation with same CID fails
      await expect(
        marriageLicense
          .connect(alice)
          .createChild(artifactCID, compilerSig, checkerSig, 0, 0)
      ).to.be.revertedWith("Manifest already used");
    });

    it("Should reject replaying signatures for a different creator", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-replay")
      );
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      await expect(
        marriageLicense
          .connect(bob)
          .createChild(artifactCID, compilerSig, checkerSig, 0, 0)
      ).to.be.revertedWith("Invalid compiler signature");
    });

    it("Should mint NFT to creator", async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("test-artifact-5")
      );
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      await marriageLicense
        .connect(alice)
        .createChild(artifactCID, compilerSig, checkerSig, 0, 0);

      expect(await marriageLicense.ownerOf(1)).to.equal(alice.address);
    });
  });

  describe("Train Child", function () {
    let childId;
    let artifactCID;

    beforeEach(async function () {
      artifactCID = ethers.keccak256(ethers.toUtf8Bytes("training-test"));
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      const tx = await marriageLicense
        .connect(alice)
        .createChild(artifactCID, compilerSig, checkerSig, 0, 0);
      childId = 1;
    });

    it("Should record training data", async function () {
      const datasetCID = ethers.keccak256(ethers.toUtf8Bytes("dataset-1"));
      const modelCID = ethers.keccak256(ethers.toUtf8Bytes("model-1"));

      // Sign training
      const trainingHash = ethers.solidityPackedKeccak256(
        ["uint256", "bytes32", "bytes32"],
        [childId, datasetCID, modelCID]
      );
      const checkerSig = await checkerSigner.signMessage(
        ethers.getBytes(trainingHash)
      );

      await expect(
        marriageLicense
          .connect(alice)
          .trainChild(childId, datasetCID, modelCID, checkerSig)
      )
        .to.emit(marriageLicense, "ChildTrained")
        .withArgs(childId, datasetCID, modelCID);

      const child = await marriageLicense.getChild(childId);
      expect(child.datasetCID).to.equal(datasetCID);
      expect(child.modelCID).to.equal(modelCID);
      expect(child.trainedAt).to.be.gt(0);
    });

    it("Should reject training by non-owner", async function () {
      const datasetCID = ethers.keccak256(ethers.toUtf8Bytes("dataset-2"));
      const modelCID = ethers.keccak256(ethers.toUtf8Bytes("model-2"));

      const trainingHash = ethers.solidityPackedKeccak256(
        ["uint256", "bytes32", "bytes32"],
        [childId, datasetCID, modelCID]
      );
      const checkerSig = await checkerSigner.signMessage(
        ethers.getBytes(trainingHash)
      );

      await expect(
        marriageLicense
          .connect(bob)
          .trainChild(childId, datasetCID, modelCID, checkerSig)
      ).to.be.revertedWith("Not child owner");
    });
  });

  describe("Quarantine and Revoke", function () {
    let childId;

    beforeEach(async function () {
      const artifactCID = ethers.keccak256(
        ethers.toUtf8Bytes("quarantine-test")
      );
      const compilerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        compilerSigner
      );
      const checkerSig = await signCreateChild(
        artifactCID,
        0,
        0,
        alice.address,
        checkerSigner
      );

      await marriageLicense
        .connect(alice)
        .createChild(artifactCID, compilerSig, checkerSig, 0, 0);
      childId = 1;
    });

    it("Should schedule and execute quarantine after delay", async function () {
      const delay = await marriageLicense.ADMIN_ACTION_DELAY();

      await expect(
        marriageLicense.scheduleQuarantineChild(childId, "Safety concern")
      ).to.emit(marriageLicense, "QuarantineScheduled");

      await expect(
        marriageLicense.executeQuarantineChild(childId)
      ).to.be.revertedWith("Action not ready");

      await increaseTime(Number(delay));

      await expect(
        marriageLicense.executeQuarantineChild(childId)
      )
        .to.emit(marriageLicense, "ChildQuarantined")
        .withArgs(childId, "Safety concern");

      const child = await marriageLicense.getChild(childId);
      expect(child.isQuarantined).to.equal(true);
    });

    it("Should allow cancelling a scheduled quarantine", async function () {
      await marriageLicense.scheduleQuarantineChild(childId, "Safety concern");

      await expect(
        marriageLicense.cancelQuarantineChild(childId)
      )
        .to.emit(marriageLicense, "QuarantineCancelled")
        .withArgs(childId);

      await expect(
        marriageLicense.executeQuarantineChild(childId)
      ).to.be.revertedWith("Quarantine not scheduled");
    });

    it("Should schedule and execute revocation after delay", async function () {
      const delay = await marriageLicense.ADMIN_ACTION_DELAY();

      await expect(
        marriageLicense.scheduleRevokeChild(childId, "Policy violation")
      ).to.emit(marriageLicense, "RevocationScheduled");

      await expect(
        marriageLicense.executeRevokeChild(childId)
      ).to.be.revertedWith("Action not ready");

      await increaseTime(Number(delay));

      await expect(
        marriageLicense.executeRevokeChild(childId)
      )
        .to.emit(marriageLicense, "ChildRevoked")
        .withArgs(childId);

      const child = await marriageLicense.getChild(childId);
      expect(child.isActive).to.equal(false);
    });

    it("Should reject non-owner quarantine scheduling", async function () {
      await expect(
        marriageLicense.connect(bob).scheduleQuarantineChild(childId, "test")
      ).to.be.revertedWith("Not authorized");
    });
  });

  describe("Admin Functions", function () {
    it("Should update marriage fee", async function () {
      const newFee = ethers.parseEther("200");
      await marriageLicense.setMarriageFee(newFee);
      expect(await marriageLicense.marriageFee()).to.equal(newFee);
    });

    it("Should schedule and execute verifier updates after delay", async function () {
      const delay = await marriageLicense.ADMIN_ACTION_DELAY();

      await expect(
        marriageLicense.scheduleVerifierUpdate(alice.address, bob.address)
      ).to.emit(marriageLicense, "VerifierUpdateScheduled");

      await expect(
        marriageLicense.executeVerifierUpdate()
      ).to.be.revertedWith("Action not ready");

      await increaseTime(Number(delay));
      await marriageLicense.executeVerifierUpdate();

      expect(await marriageLicense.compilerVerifier()).to.equal(alice.address);
      expect(await marriageLicense.checkerVerifier()).to.equal(bob.address);
    });

    it("Should allow cancelling a verifier update", async function () {
      await marriageLicense.scheduleVerifierUpdate(alice.address, bob.address);
      await expect(marriageLicense.cancelVerifierUpdate()).to.emit(
        marriageLicense,
        "VerifierUpdateCancelled"
      );

      await expect(
        marriageLicense.executeVerifierUpdate()
      ).to.be.revertedWith("Verifier update not scheduled");
    });

    it("Should schedule and execute multisig updates after delay", async function () {
      const delay = await marriageLicense.ADMIN_ACTION_DELAY();

      await expect(
        marriageLicense.scheduleMultisigUpdate(alice.address)
      ).to.emit(marriageLicense, "MultisigUpdateScheduled");

      await increaseTime(Number(delay));
      await marriageLicense.executeMultisigUpdate();

      expect(await marriageLicense.multisig()).to.equal(alice.address);
    });
  });
});
