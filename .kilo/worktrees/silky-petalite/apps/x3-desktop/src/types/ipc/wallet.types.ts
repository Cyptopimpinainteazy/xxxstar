export interface IntentDraft {
  id: string; // UUID or deterministic hash
  parties: string[];
  assets: AssetRequirement[];
  feeCaps: FeeCap[];
  expiryTimestampMs: number;
}

export interface AssetRequirement {
  chain: 'EVM' | 'SVM' | 'BTC';
  chainId: string;
  tokenAddress: string;
  amount: string; // BigInt string
  slippageBasisPoints: number;
}

export interface FeeCap {
  chain: string;
  maxFee: string;
}

export interface Attestation {
  intentId: string;
  expiry: number;
  riskFlags: string[];
  signature: string; // Validator sig
}

export interface VerifiedIntent {
  intent: IntentDraft;
  attestation: Attestation;
}

export interface SignerCaps {
  chains: string[];
  maxTxValue: string;
  requiresHardware: boolean;
}
