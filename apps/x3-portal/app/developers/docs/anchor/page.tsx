'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function AnchorPage() {
  return (
    <DocLayout
      title="Anchor Framework"
      description="Build SVM programs faster with the Anchor framework"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Anchor is the recommended framework for building SVM programs on X3 Atlas Sphere.
        It provides safety checks, boilerplate reduction, and a powerful IDL system.
      </p>

      <h2>Installation</h2>
      <CodeBlock language="bash">
{`# Install Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest

# Verify installation
anchor --version`}
      </CodeBlock>

      <h2>Create New Project</h2>
      <CodeBlock language="bash">
{`# Initialize new Anchor project
anchor init my_x3_program
cd my_x3_program

# Project structure:
# ├── Anchor.toml       # Project configuration
# ├── Cargo.toml        # Rust dependencies
# ├── programs/         # Your programs
# │   └── my_x3_program/
# │       └── src/
# │           └── lib.rs
# ├── tests/            # TypeScript tests
# └── migrations/       # Deploy scripts`}
      </CodeBlock>

      <h2>Counter Program Example</h2>
      <CodeBlock language="rust" title="programs/counter/src/lib.rs">
{`use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
        counter.authority = ctx.accounts.authority.key();
        msg!("Counter initialized!");
        Ok(())
    }

    pub fn increment(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = counter.count.checked_add(1).unwrap();
        msg!("Counter incremented to: {}", counter.count);
        Ok(())
    }

    pub fn decrement(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(counter.count > 0, ErrorCode::CounterUnderflow);
        counter.count = counter.count.checked_sub(1).unwrap();
        msg!("Counter decremented to: {}", counter.count);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Counter::INIT_SPACE
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(
        mut,
        has_one = authority
    )]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Counter {
    pub count: u64,
    pub authority: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Cannot decrement below zero")]
    CounterUnderflow,
}`}
      </CodeBlock>

      <h2>Key Anchor Features</h2>

      <h3>Account Validation</h3>
      <CodeBlock language="rust">
{`#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, Vault>,
    
    #[account(mut, constraint = from.owner == authority.key())]
    pub from: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}`}
      </CodeBlock>

      <h3>Error Handling</h3>
      <CodeBlock language="rust">
{`#[error_code]
pub enum MyError {
    #[msg("Insufficient funds for transfer")]
    InsufficientFunds,
    
    #[msg("Invalid authority")]
    InvalidAuthority,
    
    #[msg("Amount exceeds maximum: {0}")]
    AmountExceedsMax(u64),
}

// Usage
require!(amount <= MAX_AMOUNT, MyError::AmountExceedsMax(MAX_AMOUNT));`}
      </CodeBlock>

      <h3>Events</h3>
      <CodeBlock language="rust">
{`#[event]
pub struct TransferEvent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

// Emit in instruction
emit!(TransferEvent {
    from: ctx.accounts.from.key(),
    to: ctx.accounts.to.key(),
    amount,
});`}
      </CodeBlock>

      <h2>Writing Tests</h2>
      <CodeBlock language="typescript" title="tests/counter.ts">
{`import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Counter } from "../target/types/counter";
import { expect } from "chai";

describe("counter", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Counter as Program<Counter>;
  const counter = anchor.web3.Keypair.generate();

  it("Initializes the counter", async () => {
    await program.methods
      .initialize()
      .accounts({
        counter: counter.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([counter])
      .rpc();

    const account = await program.account.counter.fetch(counter.publicKey);
    expect(account.count.toNumber()).to.equal(0);
  });

  it("Increments the counter", async () => {
    await program.methods
      .increment()
      .accounts({
        counter: counter.publicKey,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const account = await program.account.counter.fetch(counter.publicKey);
    expect(account.count.toNumber()).to.equal(1);
  });
});`}
      </CodeBlock>

      <h2>Building and Testing</h2>
      <CodeBlock language="bash">
{`# Build program
anchor build

# Run tests
anchor test

# Deploy to devnet/localnet
anchor deploy

# Generate IDL
anchor idl init --filepath target/idl/counter.json PROGRAM_ID`}
      </CodeBlock>

      <Callout type="info" title="X3 Configuration">
        Update <code>Anchor.toml</code> to use X3's RPC endpoint for deployment:
        <pre className="mt-2 text-xs">
{`[provider]
cluster = "https://rpc.testnet.atlas-sphere.io"`}
        </pre>
      </Callout>
    </DocLayout>
  );
}
