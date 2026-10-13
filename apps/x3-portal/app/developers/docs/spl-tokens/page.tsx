'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function SplTokensPage() {
  return (
    <DocLayout
      title="SPL Token Standard"
      description="Create and manage SPL tokens on X3 Atlas Sphere's SVM"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        SPL tokens are the standard for fungible and non-fungible tokens on SVM.
        Create tokens that can interact with both SVM programs and EVM contracts via Comits.
      </p>

      <h2>Create a Token</h2>
      <CodeBlock language="bash">
{`# Install SPL Token CLI
cargo install spl-token-cli

# Create a new token mint
spl-token create-token

# Output:
# Creating token Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr
# Signature: ...

# Create token with specific decimals
spl-token create-token --decimals 6`}
      </CodeBlock>

      <h2>Token Operations</h2>
      <CodeBlock language="bash">
{`# Create token account
spl-token create-account TOKEN_MINT_ADDRESS

# Mint tokens
spl-token mint TOKEN_MINT_ADDRESS 1000000

# Check balance
spl-token balance TOKEN_MINT_ADDRESS

# Transfer tokens
spl-token transfer TOKEN_MINT_ADDRESS 100 RECIPIENT_ADDRESS

# Burn tokens
spl-token burn TOKEN_MINT_ADDRESS 50`}
      </CodeBlock>

      <h2>Programmatic Token Creation</h2>
      <CodeBlock language="typescript" title="create-token.ts">
{`import {
  Connection,
  Keypair,
  Transaction,
  SystemProgram,
} from '@solana/frontend/web3.js';
import {
  TOKEN_PROGRAM_ID,
  createInitializeMintInstruction,
  getMinimumBalanceForRentExemptMint,
  MINT_SIZE,
  createMint,
  getMint,
} from '@solana/spl-token';

const connection = new Connection('https://rpc.testnet.atlas-sphere.io');
const payer = Keypair.fromSecretKey(/* your secret key */);

// Simple way using helper
const mint = await createMint(
  connection,
  payer,           // Payer
  payer.publicKey, // Mint authority
  payer.publicKey, // Freeze authority (null for no freeze)
  9                // Decimals
);

console.log('Token mint address:', mint.toBase58());

// Get mint info
const mintInfo = await getMint(connection, mint);
console.log('Supply:', mintInfo.supply);
console.log('Decimals:', mintInfo.decimals);`}
      </CodeBlock>

      <h2>Token Accounts</h2>
      <CodeBlock language="typescript">
{`import {
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddress,
  getAccount,
  createAccount,
} from '@solana/spl-token';

// Get associated token address (deterministic)
const ata = await getAssociatedTokenAddress(
  mintAddress,
  ownerAddress
);

// Create associated token account
const createAtaIx = createAssociatedTokenAccountInstruction(
  payer.publicKey,    // Payer
  ata,                // ATA address
  ownerAddress,       // Owner
  mintAddress         // Mint
);

// Get token account info
const tokenAccount = await getAccount(connection, ata);
console.log('Balance:', tokenAccount.amount);`}
      </CodeBlock>

      <h2>Minting Tokens</h2>
      <CodeBlock language="typescript">
{`import { mintTo, mintToChecked } from '@solana/spl-token';

// Mint tokens
await mintTo(
  connection,
  payer,                  // Payer
  mintAddress,            // Mint
  destinationAta,         // Destination token account
  mintAuthority,          // Mint authority
  1_000_000_000n          // Amount (with decimals)
);

// Safer: mintToChecked verifies decimals
await mintToChecked(
  connection,
  payer,
  mintAddress,
  destinationAta,
  mintAuthority,
  1_000_000_000n,
  9                       // Expected decimals
);`}
      </CodeBlock>

      <h2>Transfers</h2>
      <CodeBlock language="typescript">
{`import { transfer, transferChecked } from '@solana/spl-token';

// Transfer tokens
await transfer(
  connection,
  payer,
  sourceAta,
  destinationAta,
  owner,
  100_000_000n
);

// Safer: transferChecked
await transferChecked(
  connection,
  payer,
  sourceAta,
  mintAddress,
  destinationAta,
  owner,
  100_000_000n,
  9                       // Decimals
);`}
      </CodeBlock>

      <h2>Token Metadata</h2>
      <CodeBlock language="typescript">
{`import { Metaplex } from '@metaplex-foundation/js';

const metaplex = new Metaplex(connection);

// Create NFT with metadata
const { nft } = await metaplex.nfts().create({
  uri: 'https://arweave.net/metadata.json',
  name: 'My X3 NFT',
  sellerFeeBasisPoints: 500, // 5% royalty
  symbol: 'X3NFT',
});

// Update metadata
await metaplex.nfts().update({
  nftOrSft: nft,
  name: 'Updated Name',
});`}
      </CodeBlock>

      <Callout type="info" title="Cross-VM Tokens">
        SPL tokens can be bridged to the Canonical Ledger, making them accessible 
        from EVM contracts. See <a href="/developers/docs/cross-vm-assets" className="text-orange-400 hover:text-orange-300">Cross-VM Assets</a> for details.
      </Callout>

      <h2>Token Program in Anchor</h2>
      <CodeBlock language="rust">
{`use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
    let cpi_accounts = Transfer {
        from: ctx.accounts.from.to_account_info(),
        to: ctx.accounts.to.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    
    token::transfer(cpi_ctx, amount)
}`}
      </CodeBlock>

      <h2>Token-2022 (Token Extensions)</h2>
      <p>
        X3 also supports Token-2022 with advanced features:
      </p>
      <ul>
        <li><strong>Transfer fees</strong> - Automatic fee collection</li>
        <li><strong>Interest-bearing</strong> - Automatic rebasing</li>
        <li><strong>Non-transferable</strong> - Soulbound tokens</li>
        <li><strong>Confidential transfers</strong> - Privacy features</li>
        <li><strong>Permanent delegate</strong> - Third-party control</li>
      </ul>
    </DocLayout>
  );
}
