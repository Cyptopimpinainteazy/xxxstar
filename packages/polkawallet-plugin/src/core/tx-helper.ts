/**
 * Transaction Helper — wraps extrinsic signing & submission
 * Works with both KeyringPair (node) and Polkawallet signer (mobile)
 */

import type { ApiPromise, SubmittableResult } from '@polkadot/api';
import type { SubmittableExtrinsic } from '@polkadot/api/types';
import type { KeyringPair } from '@polkadot/keyring/types';
import type { TxStatus, TxStatusCallback, ComitEvent } from '../types/interfaces';

export type SignerAccount = string | KeyringPair;

/**
 * Sign and send an extrinsic, returning a promise that resolves on finalization.
 */
export async function signAndSend(
  tx: SubmittableExtrinsic<'promise'>,
  account: SignerAccount,
  statusCallback?: TxStatusCallback,
): Promise<{ blockHash: string; blockNumber: number; txHash: string; events: ComitEvent[] }> {
  return new Promise((resolve, reject) => {
    const unsubPromise = tx.signAndSend(account, (result: SubmittableResult) => {
      const status: TxStatus = { status: 'pending' };

      if (result.status.isInBlock) {
        status.status = 'inBlock';
        status.blockHash = result.status.asInBlock.toHex();
        status.txHash = result.txHash.toHex();
        statusCallback?.(status);
      }

      if (result.status.isFinalized) {
        const blockHash = result.status.asFinalized.toHex();
        const events: ComitEvent[] = result.events.map((record) => ({
          type: `${record.event.section}.${record.event.method}`,
          data: record.event.data.toJSON() as Record<string, unknown>,
        }));

        // Check for dispatch error
        const dispatchError = result.events.find(
          ({ event }) => event.section === 'system' && event.method === 'ExtrinsicFailed',
        );

        if (dispatchError) {
          const errorStatus: TxStatus = {
            status: 'error',
            blockHash,
            txHash: result.txHash.toHex(),
            error: 'ExtrinsicFailed',
            events,
          };
          statusCallback?.(errorStatus);
          reject(new Error(`Extrinsic failed in block ${blockHash}`));
          return;
        }

        const finalStatus: TxStatus = {
          status: 'finalized',
          blockHash,
          txHash: result.txHash.toHex(),
          events,
        };
        statusCallback?.(finalStatus);

        resolve({
          blockHash,
          blockNumber: 0, // populated by caller if needed
          txHash: result.txHash.toHex(),
          events,
        });
      }

      if (result.isError) {
        const errorStatus: TxStatus = {
          status: 'error',
          error: 'Transaction error',
        };
        statusCallback?.(errorStatus);
        reject(new Error('Transaction error'));
      }
    });

    // Handle signing errors
    unsubPromise.catch((err: Error) => {
      statusCallback?.({ status: 'error', error: err.message });
      reject(err);
    });
  });
}

/**
 * Estimate fee for a transaction
 */
export async function estimateFee(
  api: ApiPromise,
  tx: SubmittableExtrinsic<'promise'>,
  account: string,
): Promise<bigint> {
  const info = await tx.paymentInfo(account);
  return info.partialFee.toBigInt();
}

/**
 * Batch multiple extrinsics into a single call
 */
export function batchTx(
  api: ApiPromise,
  txs: SubmittableExtrinsic<'promise'>[],
): SubmittableExtrinsic<'promise'> {
  return api.tx.utility.batchAll(txs);
}
