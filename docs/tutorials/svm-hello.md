# SVM Hello World Tutorial

This tutorial will guide you through deploying your first Solana program (SVM) on X3 Chain using Anchor. We'll create a simple counter program that demonstrates SVM compatibility.

## Prerequisites

- Rust 1.70+ installed
- Solana CLI tools installed (`solana-keygen`, `solana`)
- X3 Chain local node running (see [Getting Started](../getting-started.md))
- Basic understanding of Rust and Solana development

**Why this matters**: This tutorial demonstrates that you can deploy existing Solana programs on X3 Chain with minimal modifications, leveraging the full SVM compatibility.

## Step 1: Install Anchor and Set Up Environment

```bash
# Install Anchor (Solana's smart contract framework)
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force

# Install Anchor CLI
npm install -g @coral-xyz/anchor-cli

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.4/install)"

# Verify installations
anchor --version
solana --version
```

**Why this matters**: Anchor provides a familiar Rust-based development framework for Solana programs, making it easy to port existing Solana dApps to X3 Chain.

## Step 2: Configure Solana for X3 Chain

```bash
# Generate a new keypair for development
solana-keygen new --outfile ~/.config/solana/id.json

# Configure Solana to use X3 Chain
solana config set --url http://localhost:9934

# Verify configuration
solana config get
```

Expected output:
```
Config File: /home/user/.config/solana/cli/config.yml
RPC URL: http://localhost:9934
WebSocket URL: ws://localhost:9944
Keypair Path: /home/user/.config/solana/id.json
```

**Why this matters**: X3 Chain provides a Solana-compatible RPC endpoint on port 9934, allowing existing Solana tooling to work seamlessly.

## Step 3: Create Your Anchor Project

```bash
# Initialize new Anchor project
anchor init x3-svm-hello
cd x3-svm-hello

# Install dependencies
npm install
```

Update `Anchor.toml`:

```toml
[toolchain]

[features]
resolution = true
skip-lint = false

[programs.devnet]
x3_svm_hello = "YourProgramIDHere"

[programs.mainnet]
x3_svm_hello = "YourProgramIDHere"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

Update `programs/x3-svm-hello/src/lib.rs`:

```rust
use anchor_lang::prelude::*;

declare_id!("YourProgramIDHere");

#[program]
pub mod x3_svm_hello {
    use super::*;

    /// Initialize a new counter account
    pub fn initialize_counter(
        ctx: Context<InitializeCounter>,
        initial_value: u64,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.authority.key();
        counter.value = initial_value;
        counter.bump = ctx.bumps.counter;
        
        msg!("Counter initialized with value: {}", initial_value);
        emit!(CounterInitialized {
            authority: ctx.accounts.authority.key(),
            value: initial_value,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    /// Increment the counter
    pub fn increment_counter(ctx: Context<IncrementCounter>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let old_value = counter.value;
        counter.value = counter.value.checked_add(1).unwrap();
        
        msg!("Counter incremented from {} to {}", old_value, counter.value);
        emit!(CounterIncremented {
            authority: ctx.accounts.authority.key(),
            old_value,
            new_value: counter.value,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    /// Decrement the counter
    pub fn decrement_counter(ctx: Context<DecrementCounter>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let old_value = counter.value;
        
        // Prevent underflow
        require!(counter.value > 0, CounterError::CounterUnderflow);
        
        counter.value = counter.value.checked_sub(1).unwrap();
        
        msg!("Counter decremented from {} to {}", old_value, counter.value);
        emit!(CounterDecremented {
            authority: ctx.accounts.authority.key(),
            old_value,
            new_value: counter.value,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    /// Get the current counter value
    pub fn get_counter_value(ctx: Context<GetCounterValue>) -> Result<u64> {
        Ok(ctx.accounts.counter.value)
    }

    /// Reset counter to zero
    pub fn reset_counter(ctx: Context<ResetCounter>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let old_value = counter.value;
        counter.value = 0;
        
        msg!("Counter reset from {} to 0", old_value);
        emit!(CounterReset {
            authority: ctx.accounts.authority.key(),
            old_value,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }

    /// Transfer authority
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        let old_authority = counter.authority;
        counter.authority = new_authority;
        
        msg!("Authority transferred from {} to {}", old_authority, new_authority);
        emit!(AuthorityTransferred {
            old_authority,
            new_authority,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeCounter<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Counter::INIT_SPACE,
        seeds = [b"counter", authority.key().as_ref()],
        bump
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct IncrementCounter<'info> {
    #[account(
        mut,
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DecrementCounter<'info> {
    #[account(
        mut,
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetCounterValue<'info> {
    #[account(
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ResetCounter<'info> {
    #[account(
        mut,
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [b"counter", authority.key().as_ref()],
        bump = counter.bump,
        has_one = authority
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Counter {
    pub authority: Pubkey,
    pub value: u64,
    pub bump: u8,
}

#[error_code]
pub enum CounterError {
    #[msg("Counter cannot go below zero")]
    CounterUnderflow,
}

// Events
#[event]
pub struct CounterInitialized {
    pub authority: Pubkey,
    pub value: u64,
    pub timestamp: i64,
}

#[event]
pub struct CounterIncremented {
    pub authority: Pubkey,
    pub old_value: u64,
    pub new_value: u64,
    pub timestamp: i64,
}

#[event]
pub struct CounterDecremented {
    pub authority: Pubkey,
    pub old_value: u64,
    pub new_value: u64,
    pub timestamp: i64,
}

#[event]
pub struct CounterReset {
    pub authority: Pubkey,
    pub old_value: u64,
    pub timestamp: i64,
}

#[event]
pub struct AuthorityTransferred {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
    pub timestamp: i64,
}
```

**Why this matters**: This Anchor program follows standard Solana patterns and will work on both Solana and X3 Chain with identical behavior.

## Step 4: Create Client-Side Interaction

Update `programs/x3-svm-hello/src/client.ts`:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AtlasSvmHello } from "../target/types/x3_svm_hello";
import { PublicKey, SystemProgram } from "@solana/web3.js";

export async function initializeCounter(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  initialValue: number
) {
  const [counterPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("counter"), authority.toBuffer()],
    program.programId
  );

  const tx = await program.methods
    .initializeCounter(new anchor.BN(initialValue))
    .accounts({
      counter: counterPda,
      authority,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("✅ Counter initialized!");
  console.log("🔑 Counter PDA:", counterPda.toString());
  console.log("📊 Transaction signature:", tx);

  return counterPda;
}

export async function incrementCounter(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  counterPda: PublicKey
) {
  const tx = await program.methods
    .incrementCounter()
    .accounts({
      counter: counterPda,
      authority,
    })
    .rpc();

  console.log("➕ Counter incremented!");
  console.log("📊 Transaction signature:", tx);
}

export async function decrementCounter(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  counterPda: PublicKey
) {
  try {
    const tx = await program.methods
      .decrementCounter()
      .accounts({
        counter: counterPda,
        authority,
      })
      .rpc();

    console.log("➖ Counter decremented!");
    console.log("📊 Transaction signature:", tx);
  } catch (error) {
    console.log("❌ Decrement failed (expected if counter is 0):", error.message);
  }
}

export async function getCounterValue(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  counterPda: PublicKey
): Promise<number> {
  const value = await program.methods
    .getCounterValue()
    .accounts({
      counter: counterPda,
      authority,
    })
    .view();

  console.log("🔢 Current counter value:", value.toString());
  return value.toNumber();
}

export async function resetCounter(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  counterPda: PublicKey
) {
  const tx = await program.methods
    .resetCounter()
    .accounts({
      counter: counterPda,
      authority,
    })
    .rpc();

  console.log("🔄 Counter reset!");
  console.log("📊 Transaction signature:", tx);
}

export async function transferAuthority(
  program: Program<AtlasSvmHello>,
  authority: PublicKey,
  counterPda: PublicKey,
  newAuthority: PublicKey
) {
  const tx = await program.methods
    .transferAuthority(newAuthority)
    .accounts({
      counter: counterPda,
      authority,
    })
    .rpc();

  console.log("👤 Authority transferred!");
  console.log("📊 Transaction signature:", tx);
}
```

## Step 5: Create Deployment Script

Create `scripts/deploy.ts`:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AtlasSvmHello } from "../target/types/x3_svm_hello";
import { initializeCounter, incrementCounter, decrementCounter, getCounterValue, resetCounter } from "../programs/x3-svm-hello/src/client";

async function main() {
  console.log("🚀 Starting X3 Chain SVM Counter Demo...");
  
  // Configure Anchor for X3 Chain
  anchor.setProvider(anchor.AnchorProvider.env());
  
  const program = anchor.workspace.AtlasSvmHello as Program<AtlasSvmHello>;
  const authority = (anchor.workspace.AtlasSvmHello.provider as anchor.AnchorProvider).wallet.publicKey;
  
  console.log("🔑 Authority:", authority.toString());
  console.log("📡 Program ID:", program.programId.toString());
  
  try {
    // Initialize counter
    console.log("\n📋 Step 1: Initialize counter with value 10");
    const counterPda = await initializeCounter(program, authority, 10);
    
    // Get initial value
    console.log("\n📊 Step 2: Check initial value");
    let currentValue = await getCounterValue(program, authority, counterPda);
    
    // Increment counter
    console.log("\n➕ Step 3: Increment counter");
    await incrementCounter(program, authority, counterPda);
    currentValue = await getCounterValue(program, authority, counterPda);
    
    // Increment again
    console.log("\n➕ Step 4: Increment counter again");
    await incrementCounter(program, authority, counterPda);
    currentValue = await getCounterValue(program, authority, counterPda);
    
    // Decrement counter
    console.log("\n➖ Step 5: Decrement counter");
    await decrementCounter(program, authority, counterPda);
    currentValue = await getCounterValue(program, authority, counterPda);
    
    // Reset counter
    console.log("\n🔄 Step 6: Reset counter");
    await resetCounter(program, authority, counterPda);
    currentValue = await getCounterValue(program, authority, counterPda);
    
    // Test error case
    console.log("\n❌ Step 7: Test decrement below zero (should fail)");
    await decrementCounter(program, authority, counterPda);
    
    console.log("\n🎯 SVM Counter Demo completed successfully!");
    console.log("💡 Counter PDA for future use:", counterPda.toString());
    
  } catch (error) {
    console.error("❌ Demo failed:", error);
    throw error;
  }
}

// Run the demo
main()
  .then(() => {
    console.log("🎊 Demo completed!");
    process.exit(0);
  })
  .catch((error) => {
    console.error("💥 Demo failed:", error);
    process.exit(1);
  });
```

## Step 6: Create Tests

Update `tests/x3-svm-hello.ts`:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AtlasSvmHello } from "../target/types/x3_svm_hello";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("x3-svm-hello", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.AtlasSvmHello as Program<AtlasSvmHello>;

  beforeEach(async () => {
    // Airdrop SOL for testing
    await provider.connection.requestAirdrop(
      provider.wallet.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
  });

  it("Initialize counter", async () => {
    const authority = provider.wallet.publicKey;
    const [counterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("counter"), authority.toBuffer()],
      program.programId
    );

    await program.methods
      .initializeCounter(new anchor.BN(5))
      .accounts({
        counter: counterPda,
        authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const counterAccount = await program.account.counter.fetch(counterPda);
    expect(counterAccount.value.toNumber()).to.equal(5);
    expect(counterAccount.authority.toString()).to.equal(authority.toString());
  });

  it("Increment counter", async () => {
    const authority = provider.wallet.publicKey;
    const [counterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("counter"), authority.toBuffer()],
      program.programId
    );

    // Initialize
    await program.methods
      .initializeCounter(new anchor.BN(3))
      .accounts({
        counter: counterPda,
        authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Increment
    await program.methods
      .incrementCounter()
      .accounts({
        counter: counterPda,
        authority,
      })
      .rpc();

    const counterAccount = await program.account.counter.fetch(counterPda);
    expect(counterAccount.value.toNumber()).to.equal(4);
  });

  it("Decrement counter", async () => {
    const authority = provider.wallet.publicKey;
    const [counterPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("counter"), authority.toBuffer()],
      program.programId
    );

    // Initialize
    await program.methods
      .initializeCounter(new anchor.BN(10))
      .accounts({
        counter: counterPda,
        authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Decrement
    await program.methods
      .decrementCounter()
      .accounts({
        counter: counterPda,
        authority,
      })
      .rpc();

    const counterAccount = await program.account.counter.fetch(counterPda);
    expect(counterAccount.value.toNumber()).to.equal(9);
  });

  it("Prevent counter underflow", async () => {
    const authority = provider.wallet.publicKey;
    const [counterPda] = Public
