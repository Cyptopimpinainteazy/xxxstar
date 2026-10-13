'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function AtomicExecutionPage() {
  return (
    <DocLayout
      title="Atomic Cross-VM Execution"
      description="How X3 ensures atomic execution across EVM and SVM"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        X3 Atlas Sphere guarantees atomic execution across both VMs. This guide explains 
        how atomicity is achieved and what guarantees your applications can rely on.
      </p>

      <h2>Atomicity Guarantees</h2>
      <p>
        When a Comit executes, you have these guarantees:
      </p>
      <ul>
        <li><strong>All-or-nothing</strong> - Both EVM and SVM succeed, or both rollback</li>
        <li><strong>No intermediate states</strong> - External observers never see partial execution</li>
        <li><strong>Deterministic ordering</strong> - Comits within a block execute in defined order</li>
        <li><strong>State consistency</strong> - Canonical ledger reflects final atomic state</li>
      </ul>

      <h2>Execution Flow</h2>
      <CodeBlock language="text">
{`┌─────────────────────────────────────────────────────────────┐
│                    Comit Execution                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. VALIDATION                                              │
│     ├── Check nonce                                         │
│     ├── Verify authorization                                │
│     ├── Validate payload sizes                              │
│     └── Reserve fee                                         │
│                                                             │
│  2. EVM EXECUTION                                           │
│     ├── Create EVM context                                  │
│     ├── Execute payload                                     │
│     ├── Collect gas used                                    │
│     └── Store state changes (pending)                       │
│                                                             │
│  3. SVM EXECUTION                                           │
│     ├── Create SVM context                                  │
│     ├── Execute instructions                                │
│     ├── Collect compute units                               │
│     └── Store state changes (pending)                       │
│                                                             │
│  4. VERIFICATION                                            │
│     └── Verify prepare_root matches inputs                  │
│                                                             │
│  5. COMMIT OR ROLLBACK                                      │
│     ├── If success: Apply all state changes                 │
│     └── If failure: Discard all state changes               │
│                                                             │
└─────────────────────────────────────────────────────────────┘`}
      </CodeBlock>

      <h2>Rollback Scenarios</h2>
      <p>
        Comits rollback entirely in these cases:
      </p>

      <h3>EVM Failure</h3>
      <CodeBlock language="solidity">
{`// This causes full Comit rollback
contract Example {
    function riskyOperation() external {
        // Some state changes...
        
        // This revert rolls back EVERYTHING
        // Including the SVM portion of the Comit
        require(condition, "Failed");
    }
}`}
      </CodeBlock>

      <h3>SVM Failure</h3>
      <CodeBlock language="rust">
{`// SVM program error
pub fn process_instruction(...) -> ProgramResult {
    // Some operations...
    
    // This error rolls back EVERYTHING
    // Including the EVM portion of the Comit
    return Err(ProgramError::Custom(1));
}`}
      </CodeBlock>

      <h3>Verification Failure</h3>
      <CodeBlock language="text">
{`Even if both EVM and SVM execute successfully, the Comit
can still fail if the prepare_root doesn't match.

This ensures the Comit hasn't been tampered with between
submission and execution.`}
      </CodeBlock>

      <h2>State Isolation</h2>
      <Callout type="info" title="Important">
        During Comit execution, state changes are isolated. Neither the EVM nor SVM 
        can observe the other's pending changes until the Comit finalizes.
      </Callout>

      <CodeBlock language="typescript">
{`// Example: Token swap across VMs

// EVM side: Lock tokens
// SVM side: Mint wrapped tokens

// During execution:
// - EVM can't see SVM's minted tokens
// - SVM can't see EVM's locked balance change

// After finalization:
// - Both changes are visible atomically
// - No window where one side is visible without the other`}
      </CodeBlock>

      <h2>Handling Partial Success</h2>
      <p>
        Design your contracts to handle rollbacks gracefully:
      </p>
      <CodeBlock language="solidity">
{`// GOOD: Check conditions upfront
contract SafeSwap {
    function swapForSVM(uint256 amount) external {
        // Check balance first
        require(token.balanceOf(msg.sender) >= amount, "Insufficient");
        
        // All checks pass, do the transfer
        token.transferFrom(msg.sender, address(this), amount);
        
        // If SVM side fails, this entire function rolls back
        // User keeps their tokens
    }
}

// BAD: Side effects before checks
contract UnsafeSwap {
    function badSwap(uint256 amount) external {
        // External call first (bad pattern anyway)
        externalContract.notify(amount);
        
        // Check after - if this fails, notify already happened
        // But wait - Comit rollback will revert notify too!
        require(token.balanceOf(msg.sender) >= amount);
    }
}`}
      </CodeBlock>

      <h2>Cross-VM Communication</h2>
      <p>
        While EVM and SVM can't directly call each other, they communicate through:
      </p>
      <ul>
        <li><strong>Canonical Ledger</strong> - Shared balance state</li>
        <li><strong>Event logs</strong> - Both VMs emit events</li>
        <li><strong>Comit results</strong> - Success/failure propagates to both</li>
      </ul>

      <h2>Ordering Guarantees</h2>
      <CodeBlock language="text">
{`Block N:
  Comit A (submitted first)
    └── EVM executes, then SVM executes
  Comit B (submitted second)  
    └── EVM executes, then SVM executes
  
Within a block:
  - Comits execute in submission order
  - EVM always executes before SVM within a Comit
  - No interleaving between Comits`}
      </CodeBlock>

      <h2>Best Practices for Atomicity</h2>
      <ul>
        <li><strong>Validate both sides</strong> - Ensure both payloads will succeed</li>
        <li><strong>Minimize computation</strong> - Keep Comits focused and efficient</li>
        <li><strong>Test rollback paths</strong> - Verify your app handles failures</li>
        <li><strong>Use events</strong> - Emit events for off-chain tracking</li>
        <li><strong>Avoid external calls</strong> - Keep Comits self-contained</li>
      </ul>
    </DocLayout>
  );
}
