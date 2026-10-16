'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function BestPracticesPage() {
  return (
    <DocLayout
      title="Best Practices"
      description="Guidelines for building robust cross-VM applications"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Follow these best practices to build secure, efficient, and maintainable 
        applications on X3 Atlas Sphere.
      </p>

      <h2>Security</h2>

      <h3>Authorization First</h3>
      <CodeBlock language="typescript">
{`// Always verify authorization before constructing Comits
const isAuthorized = await api.rpc.atlasKernel.isAuthorized(accountId);
if (!isAuthorized) {
  throw new Error('Account not authorized for Comit submission');
}

// Then proceed with Comit construction
const comit = api.tx.atlasKernel.submitComit(...);`}
      </CodeBlock>

      <h3>Validate Inputs</h3>
      <CodeBlock language="solidity">
{`// GOOD: Validate all inputs
function processSwap(
    uint256 amountIn,
    uint256 minAmountOut,
    address recipient
) external {
    require(amountIn > 0, "Zero amount");
    require(minAmountOut > 0, "Zero min output");
    require(recipient != address(0), "Zero address");
    require(recipient != address(this), "Self-transfer");
    
    // ... proceed with swap
}

// BAD: Missing validation
function unsafeSwap(uint256 amount, address recipient) external {
    // Direct processing without checks - vulnerable!
    token.transfer(recipient, amount);
}`}
      </CodeBlock>

      <h3>Reentrancy Protection</h3>
      <CodeBlock language="solidity">
{`import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract SafeProtocol is ReentrancyGuard {
    mapping(address => uint256) public balances;
    
    function withdraw(uint256 amount) external nonReentrant {
        require(balances[msg.sender] >= amount, "Insufficient");
        
        // Update state BEFORE external call
        balances[msg.sender] -= amount;
        
        // External call last
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "Transfer failed");
    }
}`}
      </CodeBlock>

      <h2>Performance</h2>

      <h3>Minimize Comit Payload Size</h3>
      <CodeBlock language="typescript">
{`// GOOD: Focused Comit with minimal data
const evmPayload = contract.interface.encodeFunctionData(
  'swap',
  [amountIn, minOut, deadline]
);

// BAD: Bloated payload with unnecessary data
const badPayload = contract.interface.encodeFunctionData(
  'swapWithMetadata',
  [amountIn, minOut, deadline, userPreferences, analytics, logs]
);`}
      </CodeBlock>

      <h3>Batch Operations</h3>
      <CodeBlock language="typescript">
{`// GOOD: Single Comit for multiple operations
const multicall = contract.interface.encodeFunctionData('multicall', [
  [
    contract.interface.encodeFunctionData('approve', [spender, amount]),
    contract.interface.encodeFunctionData('deposit', [amount]),
    contract.interface.encodeFunctionData('stake', [amount]),
  ]
]);

// BAD: Multiple separate Comits
// Comit 1: approve
// Comit 2: deposit  
// Comit 3: stake
// More expensive and not atomic between Comits!`}
      </CodeBlock>

      <h3>Gas Estimation</h3>
      <CodeBlock language="typescript">
{`// Always estimate before submission
const estimate = await api.rpc.atlasKernel.estimateComitFee(
  evmPayload,
  svmPayload
);

// Add buffer for safety
const fee = estimate.muln(120).divn(100); // 20% buffer

// Check user balance
const balance = await api.query.system.account(account.address);
if (balance.data.free.lt(fee)) {
  throw new Error('Insufficient balance for Comit fee');
}`}
      </CodeBlock>

      <h2>Reliability</h2>

      <h3>Idempotent Operations</h3>
      <CodeBlock language="solidity">
{`// GOOD: Idempotent - safe for retry
mapping(bytes32 => bool) public processedRequests;

function processRequest(bytes32 requestId, uint256 amount) external {
    require(!processedRequests[requestId], "Already processed");
    processedRequests[requestId] = true;
    
    // Process...
}

// BAD: Not idempotent - retry causes double processing
function unsafeProcess(uint256 amount) external {
    balances[msg.sender] += amount; // Will double on retry
}`}
      </CodeBlock>

      <h3>Event Emission</h3>
      <CodeBlock language="solidity">
{`// Emit comprehensive events for indexing and debugging
event SwapExecuted(
    bytes32 indexed comitId,
    address indexed user,
    address tokenIn,
    address tokenOut,
    uint256 amountIn,
    uint256 amountOut,
    uint256 timestamp
);

function swap(...) external {
    // ... swap logic ...
    
    emit SwapExecuted(
        comitId,
        msg.sender,
        tokenIn,
        tokenOut,
        amountIn,
        amountOut,
        block.timestamp
    );
}`}
      </CodeBlock>

      <h3>Monitoring</h3>
      <CodeBlock language="typescript">
{`// Monitor Comit success rate
const metrics = {
  submitted: 0,
  succeeded: 0,
  failed: 0,
  avgGasUsed: 0,
};

api.query.system.events((events) => {
  events.forEach(({ event }) => {
    if (event.section === 'atlasKernel') {
      if (event.method === 'ComitSubmitted') metrics.submitted++;
      if (event.method === 'ComitFinalized') metrics.succeeded++;
      if (event.method === 'ComitFailed') metrics.failed++;
    }
  });
  
  // Report to monitoring
  reportMetrics(metrics);
});`}
      </CodeBlock>

      <h2>Testing</h2>

      <h3>Test Both VMs</h3>
      <CodeBlock language="typescript">
{`describe('Cross-VM Swap', () => {
  it('should atomically swap tokens', async () => {
    // Setup: Fund accounts on both VMs
    await fundEvmAccount(user, evmToken, amount);
    await fundSvmAccount(user, svmMint, amount);
    
    // Execute Comit
    const result = await submitSwapComit(user, amount);
    
    // Verify: Check both VMs
    const evmBalance = await evmToken.balanceOf(user);
    const svmBalance = await getSplBalance(user, svmMint);
    
    expect(evmBalance).to.equal(expectedEvmBalance);
    expect(svmBalance).to.equal(expectedSvmBalance);
  });
  
  it('should rollback both VMs on failure', async () => {
    const beforeEvm = await evmToken.balanceOf(user);
    const beforeSvm = await getSplBalance(user, svmMint);
    
    // Force failure
    await expect(submitBadComit()).to.be.rejected;
    
    // Verify: No state change on either VM
    expect(await evmToken.balanceOf(user)).to.equal(beforeEvm);
    expect(await getSplBalance(user, svmMint)).to.equal(beforeSvm);
  });
});`}
      </CodeBlock>

      <h2>Checklist</h2>
      <ul className="space-y-2">
        <li>✅ Authorization check before Comit submission</li>
        <li>✅ Input validation in all contracts/programs</li>
        <li>✅ Reentrancy guards where applicable</li>
        <li>✅ Gas estimation with buffer</li>
        <li>✅ Comprehensive event emission</li>
        <li>✅ Idempotent operation design</li>
        <li>✅ Cross-VM integration tests</li>
        <li>✅ Failure/rollback testing</li>
        <li>✅ Monitoring and alerting setup</li>
        <li>✅ Security audit for production</li>
      </ul>
    </DocLayout>
  );
}
