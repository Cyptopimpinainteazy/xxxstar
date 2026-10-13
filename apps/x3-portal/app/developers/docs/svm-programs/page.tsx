'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function SvmProgramsPage() {
  return (
    <DocLayout
      title="SVM Programs Overview"
      description="Build Solana-compatible programs on X3 Atlas Sphere's SVM environment"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        X3 Atlas Sphere includes a native SVM (Solana Virtual Machine) that runs alongside
        the EVM, enabling Solana-style programs to execute with the same tooling and patterns.
      </p>

      <h2>SVM on X3 Atlas Sphere</h2>
      <p>
        Our SVM implementation provides:
      </p>
      <ul>
        <li><strong>Account model</strong> - Solana's data/code separation</li>
        <li><strong>Parallel execution</strong> - Sealevel runtime for concurrent transactions</li>
        <li><strong>BPF programs</strong> - Deploy Solana-compatible bytecode</li>
        <li><strong>Cross-VM calls</strong> - Interact with EVM contracts via Comits</li>
      </ul>

      <h2>Program Architecture</h2>
      <p>
        SVM programs follow Solana's architecture:
      </p>
      <CodeBlock language="rust" title="programs/counter/src/lib.rs">
{`use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// Define program state
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Counter {
    pub count: u64,
}

// Instruction types
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    Initialize,
    Increment,
    Decrement,
}

// Entry point
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = CounterInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    match instruction {
        CounterInstruction::Initialize => {
            msg!("Initializing counter");
            let counter = Counter { count: 0 };
            counter.serialize(&mut *counter_account.data.borrow_mut())?;
        }
        CounterInstruction::Increment => {
            msg!("Incrementing counter");
            let mut counter = Counter::try_from_slice(&counter_account.data.borrow())?;
            counter.count = counter.count.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
            counter.serialize(&mut *counter_account.data.borrow_mut())?;
        }
        CounterInstruction::Decrement => {
            msg!("Decrementing counter");
            let mut counter = Counter::try_from_slice(&counter_account.data.borrow())?;
            counter.count = counter.count.checked_sub(1).ok_or(ProgramError::ArithmeticOverflow)?;
            counter.serialize(&mut *counter_account.data.borrow_mut())?;
        }
    }

    Ok(())
}`}
      </CodeBlock>

      <h2>Key Concepts</h2>
      
      <h3>1. Accounts</h3>
      <p>
        SVM separates data and code. Programs are stateless, and all state is stored in accounts:
      </p>
      <ul>
        <li><strong>Data accounts</strong> - Store program state</li>
        <li><strong>Program accounts</strong> - Store executable code</li>
        <li><strong>System accounts</strong> - Special accounts for rent, etc.</li>
      </ul>

      <h3>2. Program Derived Addresses (PDAs)</h3>
      <CodeBlock language="rust">
{`// Derive a PDA for storing user data
let (pda, bump) = Pubkey::find_program_address(
    &[b"user", user_pubkey.as_ref()],
    program_id,
);

// Use seeds for signing
let seeds = &[b"user", user_pubkey.as_ref(), &[bump]];`}
      </CodeBlock>

      <h3>3. Cross-Program Invocations (CPI)</h3>
      <CodeBlock language="rust">
{`use solana_program::program::invoke_signed;

// Call another program
invoke_signed(
    &token_instruction,
    &[token_account.clone(), authority.clone()],
    &[&[b"authority", &[authority_bump]]],
)?;`}
      </CodeBlock>

      <h2>Building Programs</h2>
      <CodeBlock language="bash">
{`# Install Solana CLI tools
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Create new program
cargo new --lib my_program
cd my_program

# Add dependencies to Cargo.toml
# [dependencies]
# solana-program = "1.18"
# borsh = "1.0"

# Build for BPF
cargo build-bpf

# The compiled program will be at target/deploy/my_program.so`}
      </CodeBlock>

      <Callout type="info" title="Anchor Recommended">
        For most projects, we recommend using the <a href="/developers/docs/anchor" className="text-orange-400 hover:text-orange-300">Anchor framework</a> which 
        provides a higher-level abstraction and reduces boilerplate significantly.
      </Callout>

      <h2>Differences from Solana</h2>
      <p>
        X3's SVM is compatible with Solana programs but has some differences:
      </p>
      <ul>
        <li><strong>Consensus</strong> - Uses Substrate's Aura/GRANDPA instead of PoH</li>
        <li><strong>Native token</strong> - ATLAS instead of SOL for gas</li>
        <li><strong>Cross-VM</strong> - Can interact with EVM via Comits</li>
        <li><strong>Rent</strong> - Follows X3's fee model</li>
      </ul>
    </DocLayout>
  );
}
