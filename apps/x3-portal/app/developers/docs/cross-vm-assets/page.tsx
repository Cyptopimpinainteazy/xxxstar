'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function CrossVmAssetsPage() {
  return (
    <DocLayout
      title="Cross-VM Assets"
      description="Managing assets that span both EVM and SVM environments"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Assets on X3 Atlas Sphere exist in the Canonical Ledger, accessible from both 
        EVM and SVM. Learn how to register, transfer, and manage cross-VM assets.
      </p>

      <h2>The Canonical Ledger</h2>
      <p>
        The Canonical Ledger is the single source of truth for all assets on X3:
      </p>
      <CodeBlock language="rust">
{`// Canonical Ledger structure
CanonicalLedger: Map<(AccountId, AssetId), Balance>

// Single balance per (account, asset) pair
// - No wrapped tokens
// - No bridging needed
// - Accessible from both VMs`}
      </CodeBlock>

      <Callout type="info" title="Key Concept">
        Unlike other cross-chain solutions, X3 doesn't use wrapped tokens. Each asset 
        exists once in the Canonical Ledger and can be operated on from either VM.
      </Callout>

      <h2>Registering Assets</h2>
      <CodeBlock language="typescript">
{`import { ApiPromise } from '@polkadot/api';

// Register a new asset (governance only)
const registerAsset = api.tx.atlasKernel.registerAsset(
  assetId,      // Unique identifier
  'MyToken',    // Symbol
  18            // Decimals
);

// Submit via governance
await api.tx.sudo.sudo(registerAsset).signAndSend(sudoKey);

// Query asset metadata
const metadata = await api.query.atlasKernel.assetMetadata(assetId);
console.log('Symbol:', metadata.symbol.toHuman());
console.log('Decimals:', metadata.decimals.toNumber());`}
      </CodeBlock>

      <h2>Querying Balances</h2>
      <CodeBlock language="typescript">
{`// Via Substrate RPC
const balance = await api.rpc.atlasKernel.getCanonicalBalance(
  accountId,
  assetId
);

// Via EVM (contract call)
const evmProvider = new ethers.JsonRpcProvider('https://rpc.testnet.atlas-sphere.io');
const ledgerContract = new ethers.Contract(
  CANONICAL_LEDGER_PRECOMPILE,
  ['function balanceOf(address account, bytes32 assetId) view returns (uint256)'],
  evmProvider
);
const evmBalance = await ledgerContract.balanceOf(evmAddress, assetId);

// Via SVM (account lookup)
const svmBalance = await connection.getAccountInfo(balanceAccountPda);`}
      </CodeBlock>

      <h2>Cross-VM Transfers</h2>
      <p>
        Move assets between VM contexts using Comits:
      </p>
      <CodeBlock language="typescript" title="cross-vm-transfer.ts">
{`// Transfer from EVM-controlled to SVM-controlled account

// EVM payload: Release tokens from EVM contract
const evmPayload = escrowContract.interface.encodeFunctionData(
  'release',
  [assetId, amount, recipientSubstrateAccount]
);

// SVM payload: Credit the SVM account
const svmPayload = buildCreditInstruction(
  recipientSvmAccount,
  assetId,
  amount
);

// Submit Comit - atomic transfer across VMs
const comit = api.tx.atlasKernel.submitComit(
  evmPayload,
  svmPayload,
  fee
);

await comit.signAndSend(account);`}
      </CodeBlock>

      <h2>EVM Integration</h2>
      <CodeBlock language="solidity" title="CanonicalLedgerAccess.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ICanonicalLedger {
    function balanceOf(address account, bytes32 assetId) external view returns (uint256);
    function transfer(bytes32 assetId, address to, uint256 amount) external returns (bool);
    function transferFrom(bytes32 assetId, address from, address to, uint256 amount) external returns (bool);
}

contract DeFiProtocol {
    ICanonicalLedger public constant ledger = ICanonicalLedger(0x000000000000000000000000000000000000C0DE);
    bytes32 public constant ATLAS_ID = bytes32(0);
    
    function getAtlasBalance(address user) external view returns (uint256) {
        return ledger.balanceOf(user, ATLAS_ID);
    }
    
    function deposit(bytes32 assetId, uint256 amount) external {
        // Transfer from user to this contract
        ledger.transferFrom(assetId, msg.sender, address(this), amount);
        // ... rest of deposit logic
    }
}`}
      </CodeBlock>

      <h2>SVM Integration</h2>
      <CodeBlock language="rust" title="programs/defi/src/lib.rs">
{`use anchor_lang::prelude::*;

#[program]
pub mod defi {
    use super::*;

    pub fn deposit(ctx: Context<Deposit>, asset_id: [u8; 32], amount: u64) -> Result<()> {
        // Read from canonical ledger PDA
        let ledger_balance = ctx.accounts.canonical_balance.amount;
        require!(ledger_balance >= amount, ErrorCode::InsufficientBalance);
        
        // Update user position
        let position = &mut ctx.accounts.user_position;
        position.deposited = position.deposited.checked_add(amount).unwrap();
        
        // Emit event for indexers
        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            asset_id,
            amount,
        });
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        seeds = [b"canonical", asset_id.as_ref(), user.key().as_ref()],
        bump
    )]
    pub canonical_balance: Account<'info, CanonicalBalance>,
    
    #[account(mut)]
    pub user_position: Account<'info, UserPosition>,
    
    pub user: Signer<'info>,
}`}
      </CodeBlock>

      <h2>Asset ID Standards</h2>
      <p>
        Asset IDs follow a consistent format:
      </p>
      <CodeBlock language="typescript">
{`// Native ATLAS token
const ATLAS_ASSET_ID = '0x0000000000000000000000000000000000000000000000000000000000000000';

// ERC20 tokens (hash of contract address)
const erc20AssetId = keccak256(
  solidityPacked(['uint8', 'address'], [1, tokenAddress])
);

// SPL tokens (hash of mint address)
const splAssetId = keccak256(
  solidityPacked(['uint8', 'bytes32'], [2, mintPubkey.toBytes()])
);`}
      </CodeBlock>

      <h2>Best Practices</h2>
      <ul>
        <li><strong>Use canonical ledger</strong> - Don't create wrapped tokens</li>
        <li><strong>Check balances atomically</strong> - Use Comits for cross-VM checks</li>
        <li><strong>Handle decimals</strong> - EVM uses 18, some SPL use 6 or 9</li>
        <li><strong>Emit events</strong> - For indexer compatibility on both VMs</li>
        <li><strong>Test cross-VM flows</strong> - Verify atomicity in integration tests</li>
      </ul>
    </DocLayout>
  );
}
