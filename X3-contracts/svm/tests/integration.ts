import * as anchor from "@coral-xyz/anchor";
import { Program, web3, BN } from "@coral-xyz/anchor";
import { assert, expect } from "chai";

describe("x3-svm", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const payer = provider.wallet as anchor.Wallet;

  // Program instances
  const x3VmErc20 = anchor.workspace.X3VmErc20 as Program;
  const x3ExternalGateway = anchor.workspace.X3ExternalGateway as Program;
  const x3KernelBridge = anchor.workspace.X3KernelBridge as Program;
  const x3ReceiptVerifier = anchor.workspace.X3ReceiptVerifier as Program;
  const x3Core = anchor.workspace.X3Core as Program;

  describe("Receipt Verifier", () => {
    const verifierSeeds = [Buffer.from("verifier")];

    it("initializes with validators", async () => {
      const [verifierPda] = web3.PublicKey.findProgramAddressSync(
        verifierSeeds,
        x3ReceiptVerifier.programId
      );

      const validators: Array<Array<number>> = [];
      for (let i = 0; i < 3; i++) {
        const kp = web3.Keypair.generate();
        validators.push(Array.from(kp.publicKey.toBytes()));
      }

      await x3ReceiptVerifier.methods
        .initialize(validators, new BN(2))
        .accounts({
          verifier: verifierPda,
          authority: payer.publicKey,
          instructionsSysvar: web3.SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();

      const verifier = await x3ReceiptVerifier.account.verifier.fetch(
        verifierPda
      );
      assert.equal(verifier.validatorCount.toNumber(), 3);
      assert.equal(verifier.quorumThreshold.toNumber(), 2);
      assert.equal(verifier.verifierSetId.toNumber(), 1);
    });
  });

  describe("Token Adapter (x3_vm_erc20)", () => {
    const ASSET_ID = Array.from(
      web3.Keypair.generate().publicKey.toBytes()
    ) as Array<number>;
    const MINT_DECIMALS = 9;

    let mintPda: web3.PublicKey;
    let adapterConfigPda: web3.PublicKey;
    let mintAuthorityPda: web3.PublicKey;
    let userTokenAccount: web3.PublicKey;
    const user = web3.Keypair.generate();

    it("initializes adapter with SPL mint", async () => {
      const [config] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("adapter"), Buffer.from(ASSET_ID)],
        x3VmErc20.programId
      );
      const [mint] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("mint_authority"), Buffer.from(ASSET_ID)],
        x3VmErc20.programId
      );
      const [mintAuth] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("mint_authority"), Buffer.from(ASSET_ID)],
        x3VmErc20.programId
      );
      adapterConfigPda = config;
      mintPda = mint;
      mintAuthorityPda = mintAuth;

      // Airdrop to payer
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
          payer.publicKey,
          10 * web3.LAMPORTS_PER_SOL
        )
      );

      await x3VmErc20.methods
        .initialize(ASSET_ID, MINT_DECIMALS, payer.publicKey)
        .accounts({
          adapterConfig: adapterConfigPda,
          mint: mintPda,
          mintAuthority: mintAuthorityPda,
          payer: payer.publicKey,
          systemProgram: web3.SystemProgram.programId,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
          rent: web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const configAcc = await x3VmErc20.account.adapterConfig.fetch(
        adapterConfigPda
      );
      assert.isTrue(configAcc.initialized);
      assert.equal(
        configAcc.mint.toBase58(),
        mintPda.toBase58()
      );

      // Create user token account and mint some tokens
      const ata = await anchor.utils.token.associatedAddress({
        mint: mintPda,
        owner: user.publicKey,
      });
      userTokenAccount = ata;
    });
  });

  describe("Kernel Bridge", () => {
    const KERNEL_AUTH = web3.Keypair.generate();

    it("initializes", async () => {
      const [bridgePda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("kernel_bridge")],
        x3KernelBridge.programId
      );

      await x3KernelBridge.methods
        .initialize(KERNEL_AUTH.publicKey)
        .accounts({
          bridge: bridgePda,
          payer: payer.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();
    });
  });

  describe("External Gateway", () => {
    const CHAIN_ID = new BN(1);
    const X3_CHAIN_ID = new BN(42);
    const MIN_CONFIRMATIONS = new BN(1);

    it("initializes", async () => {
      const [gatewayPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("gateway")],
        x3ExternalGateway.programId
      );

      await x3ExternalGateway.methods
        .initialize(CHAIN_ID, X3_CHAIN_ID, MIN_CONFIRMATIONS)
        .accounts({
          gateway: gatewayPda,
          owner: payer.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();
    });
  });

  describe("Flashloan core", () => {
    it("pure fee math matches EVM", () => {
      // The unit tests in x3_core cover this, but verify via JS as well
      const fee = x3Core.simulateFlashloan(
        new BN("100000000000000000000"),
        9,
        { honest: {} }
      );
      assert.isTrue(fee.ok);
      assert.equal(fee.poolDelta.toString(), "90000000000000000");
    });
  });
});
