/**
 * X3 Settlement Engine service — intent-based settlement, escrow, bonds, BTC proofs
 *
 * Maps to pallet: x3-settlement-engine
 * Extrinsics: create_intent, lock_escrow, submit_proof, claim_settlement,
 *             refund_settlement, deposit_bond, submit_btc_proof, submit_btc_header,
 *             report_violation
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getIntent(intentId: number) {
  const api = getApi();
  return api.query.x3SettlementEngine.intents(intentId);
}

async function getAllIntents(account: string) {
  const api = getApi();
  const entries = await api.query.x3SettlementEngine.intents.entries();
  return entries
    .map(([key, val]: [any, any]) => ({
      id: key.args[0].toString(),
      ...val.toJSON(),
    }))
    .filter((i: any) => i.creator === account || i.counterparty === account);
}

async function getBondState(account: string) {
  const api = getApi();
  return api.query.x3SettlementEngine.bonds(account);
}

async function getEscrowBalance(intentId: number) {
  const api = getApi();
  return api.query.x3SettlementEngine.escrows(intentId);
}

/* ─── Extrinsics ─── */

/**
 * Create a settlement intent.
 */
function createIntent(
  chainKind: { Evm: number } | 'Svm' | 'X3' | 'Bitcoin',
  amount: string,
  asset: number,
  counterparty: string | null,
  deadline: number
) {
  const api = getApi();
  return api.tx.x3SettlementEngine.createIntent(
    chainKind,
    amount,
    asset,
    counterparty,
    deadline
  );
}

/**
 * Lock escrow for an intent.
 */
function lockEscrow(intentId: number, amount: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.lockEscrow(intentId, amount);
}

/**
 * Submit proof of execution (e.g., tx hash on target chain).
 */
function submitProof(intentId: number, proofData: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.submitProof(intentId, proofData);
}

/**
 * Claim settlement after proof is verified.
 */
function claimSettlement(intentId: number) {
  const api = getApi();
  return api.tx.x3SettlementEngine.claimSettlement(intentId);
}

/**
 * Refund if settlement deadline expired without proof.
 */
function refundSettlement(intentId: number) {
  const api = getApi();
  return api.tx.x3SettlementEngine.refundSettlement(intentId);
}

/**
 * Deposit bond (collateral for executors/settlers).
 */
function depositBond(amount: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.depositBond(amount);
}

/**
 * Submit a BTC proof for Bitcoin cross-chain settlement.
 */
function submitBtcProof(intentId: number, btcTxHash: string, merkleProof: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.submitBtcProof(intentId, btcTxHash, merkleProof);
}

/**
 * Submit a BTC block header for light client verification.
 */
function submitBtcHeader(headerData: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.submitBtcHeader(headerData);
}

/**
 * Report a violation for slashing.
 */
function reportViolation(target: string, evidence: string) {
  const api = getApi();
  return api.tx.x3SettlementEngine.reportViolation(target, evidence);
}

/**
 * Subscribe to intent status changes.
 */
async function subscribeIntent(intentId: number, msgChannel: string) {
  const api = getApi();
  return api.query.x3SettlementEngine.intents(intentId, (intent: any) => {
    (window as any).send(msgChannel, {
      intentId,
      ...intent.toJSON(),
    });
  });
}

export default {
  getIntent,
  getAllIntents,
  getBondState,
  getEscrowBalance,
  createIntent,
  lockEscrow,
  submitProof,
  claimSettlement,
  refundSettlement,
  depositBond,
  submitBtcProof,
  submitBtcHeader,
  reportViolation,
  subscribeIntent,
};
