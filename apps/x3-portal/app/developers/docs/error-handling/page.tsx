'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function ErrorHandlingPage() {
  return (
    <DocLayout
      title="Error Handling"
      description="Handle errors and failures in cross-VM operations"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Robust error handling is crucial for cross-VM applications. Learn how to detect,
        decode, and handle errors from Comit transactions.
      </p>

      <h2>Error Categories</h2>
      <p>
        Comit failures fall into these categories:
      </p>

      <h3>1. Validation Errors (Pre-execution)</h3>
      <CodeBlock language="typescript">
{`const ValidationErrors = {
  0x01: 'InvalidNonce',        // Nonce doesn't match expected
  0x02: 'InsufficientFee',     // Fee too low for operation
  0x03: 'NotAuthorized',       // Account not in AuthorizedAccounts
  0x04: 'PayloadTooLarge',     // EVM or SVM payload exceeds limit
  0x05: 'InvalidPayload',      // Malformed payload data
};`}
      </CodeBlock>

      <h3>2. Execution Errors</h3>
      <CodeBlock language="typescript">
{`const ExecutionErrors = {
  // EVM errors (0x10 prefix)
  0x10: 'EvmExecutionFailed',
  0x11: 'EvmOutOfGas',
  0x12: 'EvmRevert',
  0x13: 'EvmInvalidOpcode',
  
  // SVM errors (0x20 prefix)
  0x20: 'SvmExecutionFailed',
  0x21: 'SvmOutOfCompute',
  0x22: 'SvmProgramError',
  0x23: 'SvmAccountError',
};`}
      </CodeBlock>

      <h3>3. Verification Errors</h3>
      <CodeBlock language="typescript">
{`const VerificationErrors = {
  0x06: 'PrepareRootMismatch', // Input hash doesn't match
};`}
      </CodeBlock>

      <h2>Detecting Failures</h2>
      <CodeBlock language="typescript" title="error-handling.ts">
{`import { ApiPromise } from '@polkadot/api';

async function submitComitWithErrorHandling(
  api: ApiPromise,
  account: KeyringPair,
  evmPayload: Uint8Array,
  svmPayload: Uint8Array,
  fee: bigint
) {
  return new Promise((resolve, reject) => {
    api.tx.atlasKernel.submitComit(evmPayload, svmPayload, fee)
      .signAndSend(account, ({ status, events, dispatchError }) => {
        // Check for dispatch error (transaction-level failure)
        if (dispatchError) {
          if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            reject(new Error(\`\${decoded.section}.\${decoded.name}: \${decoded.docs}\`));
          } else {
            reject(new Error(dispatchError.toString()));
          }
          return;
        }

        if (status.isInBlock) {
          // Check for Comit-specific errors
          const failedEvent = events.find(({ event }) =>
            event.section === 'atlasKernel' && event.method === 'ComitFailed'
          );

          if (failedEvent) {
            const [comitId, errorCode] = failedEvent.event.data;
            reject(new ComitError(errorCode.toNumber(), comitId.toHex()));
            return;
          }

          // Check for success
          const successEvent = events.find(({ event }) =>
            event.section === 'atlasKernel' && event.method === 'ComitFinalized'
          );

          if (successEvent) {
            resolve(successEvent.event.data);
          }
        }
      });
  });
}`}
      </CodeBlock>

      <h2>Decoding EVM Errors</h2>
      <CodeBlock language="typescript">
{`import { ethers } from 'ethers';

function decodeEvmError(errorData: string, contractInterface: ethers.Interface) {
  // Check for standard revert string
  if (errorData.startsWith('0x08c379a0')) {
    const reason = ethers.AbiCoder.defaultAbiCoder().decode(
      ['string'],
      '0x' + errorData.slice(10)
    );
    return { type: 'revert', reason: reason[0] };
  }

  // Check for Panic(uint256)
  if (errorData.startsWith('0x4e487b71')) {
    const code = ethers.AbiCoder.defaultAbiCoder().decode(
      ['uint256'],
      '0x' + errorData.slice(10)
    );
    const panicCodes: Record<number, string> = {
      0x01: 'Assertion failed',
      0x11: 'Arithmetic overflow',
      0x12: 'Division by zero',
      0x21: 'Invalid enum value',
      0x31: 'Pop on empty array',
      0x32: 'Array index out of bounds',
    };
    return { type: 'panic', reason: panicCodes[Number(code[0])] || 'Unknown panic' };
  }

  // Try custom error decoding
  try {
    const parsed = contractInterface.parseError(errorData);
    if (parsed) {
      return { type: 'custom', name: parsed.name, args: parsed.args };
    }
  } catch {}

  return { type: 'unknown', data: errorData };
}`}
      </CodeBlock>

      <h2>Decoding SVM Errors</h2>
      <CodeBlock language="typescript">
{`import { AnchorError } from '@coral-xyz/anchor';

function decodeSvmError(errorData: Uint8Array, idl: Idl) {
  // Anchor custom errors start with specific prefix
  const errorCode = new DataView(errorData.buffer).getUint32(0, true);
  
  // Check IDL for custom error
  const customError = idl.errors?.find(e => e.code === errorCode);
  if (customError) {
    return {
      type: 'anchor',
      code: errorCode,
      name: customError.name,
      message: customError.msg,
    };
  }

  // Standard program errors
  const programErrors: Record<number, string> = {
    0: 'Success',
    1: 'NotEnoughAccountKeys',
    2: 'InvalidAccountData',
    3: 'AccountDataTooSmall',
    4: 'InsufficientFunds',
    5: 'IncorrectProgramId',
    6: 'MissingRequiredSignature',
    // ... etc
  };

  return {
    type: 'program',
    code: errorCode,
    message: programErrors[errorCode] || 'Unknown error',
  };
}`}
      </CodeBlock>

      <h2>Retry Strategies</h2>
      <CodeBlock language="typescript">
{`async function submitWithRetry(
  submitFn: () => Promise<void>,
  maxRetries = 3
) {
  let lastError: Error | null = null;
  
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      return await submitFn();
    } catch (error: any) {
      lastError = error;
      
      // Don't retry these errors
      if (error.code === 0x03) { // NotAuthorized
        throw error;
      }
      
      // Retry with fresh nonce
      if (error.code === 0x01) { // InvalidNonce
        continue;
      }
      
      // Retry with higher fee
      if (error.code === 0x02) { // InsufficientFee
        // Increase fee and retry
        continue;
      }
      
      // Exponential backoff for other errors
      await new Promise(r => setTimeout(r, 1000 * Math.pow(2, attempt)));
    }
  }
  
  throw lastError;
}`}
      </CodeBlock>

      <Callout type="warning" title="Idempotency">
        Design your Comits to be idempotent when possible. If a retry succeeds after 
        the original was included in a block, you don't want double-execution.
      </Callout>

      <h2>Error Events</h2>
      <CodeBlock language="typescript">
{`// Subscribe to all Comit failure events
api.query.system.events((events) => {
  events.forEach(({ event }) => {
    if (event.section === 'atlasKernel' && event.method === 'ComitFailed') {
      const [comitId, reason, evmGasUsed, svmComputeUsed] = event.data;
      
      console.error('Comit failed:', {
        comitId: comitId.toHex(),
        reason: reason.toNumber(),
        evmGasUsed: evmGasUsed.toNumber(),
        svmComputeUsed: svmComputeUsed.toNumber(),
      });
      
      // Alert monitoring system
      alertMonitoring('comit_failed', { comitId, reason });
    }
  });
});`}
      </CodeBlock>
    </DocLayout>
  );
}
