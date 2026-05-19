/**
 * X3 VM service — bytecode submission, job management, dispute
 *
 * Maps to pallet: x3-verifier
 * Extrinsics: register_executor, submit_job, submit_receipt,
 *             dispute_receipt, toggle_verification, slash_executor
 *
 * This enables:
 *   - Submit compiled X3 bytecode (from x3-lang compiler) as on-chain jobs
 *   - Register as an X3 executor node
 *   - Submit execution receipts with gas + state root proofs
 *   - Dispute incorrect receipts (triggers x3-court replay)
 */
import { ApiPromise } from '@polkadot/api';

function getApi(): ApiPromise {
  return (window as any).api;
}

/* ─── Queries ─── */

async function getExecutorInfo(executor: string) {
  const api = getApi();
  return (api.rpc as any).x3Verifier.getExecutorInfo(executor);
}

async function getJobStatus(jobId: number) {
  const api = getApi();
  return (api.rpc as any).x3Verifier.getJobStatus(jobId);
}

async function getReceipt(jobId: number) {
  const api = getApi();
  return (api.rpc as any).x3Verifier.getReceipt(jobId);
}

async function getAllJobs(account: string) {
  const api = getApi();
  const entries = await api.query.x3Verifier.jobs.entries();
  return entries
    .map(([key, val]: [any, any]) => ({
      id: key.args[0].toString(),
      ...val.toJSON(),
    }))
    .filter((j: any) => j.submitter === account || j.executor === account);
}

/* ─── Extrinsics ─── */

/**
 * Register as an X3 executor (validator/compute node).
 */
function registerExecutor(stake: string) {
  const api = getApi();
  return api.tx.x3Verifier.registerExecutor(stake);
}

/**
 * Submit X3 bytecode as a job for execution.
 * @param bytecodeHash - H256 hash of the compiled X3BC module
 * @param bytecode - Raw bytecode bytes
 * @param gasLimit - Maximum gas units
 * @param input - Input parameters for the X3 program
 */
function submitJob(bytecodeHash: string, bytecode: string, gasLimit: number, input: string) {
  const api = getApi();
  return api.tx.x3Verifier.submitJob(bytecodeHash, bytecode, gasLimit, input);
}

/**
 * Submit an execution receipt (called by executor after running the job).
 */
function submitReceipt(
  jobId: number,
  gasUsed: number,
  returnData: string,
  stateRoot: string,
  logs: string[]
) {
  const api = getApi();
  return api.tx.x3Verifier.submitReceipt(jobId, gasUsed, returnData, stateRoot, logs);
}

/**
 * Dispute an execution receipt — triggers deterministic replay in x3-court.
 */
function disputeReceipt(jobId: number, reason: string) {
  const api = getApi();
  return api.tx.x3Verifier.disputeReceipt(jobId, reason);
}

/**
 * Toggle verification mode for an executor.
 */
function toggleVerification(enabled: boolean) {
  const api = getApi();
  return api.tx.x3Verifier.toggleVerification(enabled);
}

/**
 * Subscribe to job status changes.
 */
async function subscribeJob(jobId: number, msgChannel: string) {
  const api = getApi();
  return api.query.x3Verifier.jobs(jobId, (job: any) => {
    (window as any).send(msgChannel, {
      jobId,
      ...job.toJSON(),
    });
  });
}

export default {
  getExecutorInfo,
  getJobStatus,
  getReceipt,
  getAllJobs,
  registerExecutor,
  submitJob,
  submitReceipt,
  disputeReceipt,
  toggleVerification,
  subscribeJob,
};
