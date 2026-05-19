/**
 * Treasury Configuration
 * 
 * All apps integrated into X3 Desktop must route 50% of mining rewards,
 * transaction fees, or earned tokens to the X3 Treasury wallet.
 */

export interface TreasuryConfig {
  enabled: boolean;
  treasuryAddress: string;
  treasuryShare: number; // Percentage (0-100)
  chainId?: string;
  network?: string;
}

/**
 * X3 Treasury Configuration
 * All integrated apps will automatically route 50% of earnings here
 */
export const X3_TREASURY_CONFIG: TreasuryConfig = {
  enabled: true,
  treasuryAddress: import.meta.env.VITE_X3_TREASURY_ADDRESS || "X3Treasury_DefaultAddress_REPLACE_IN_PRODUCTION",
  treasuryShare: 50, // 50% of all earnings go to X3 Treasury
  chainId: "x3-chain",
  network: "mainnet",
};

/**
 * Multi-chain treasury addresses for cross-chain integrations
 */
export const MULTI_CHAIN_TREASURY = {
  ethereum: {
    address: import.meta.env.VITE_ETH_TREASURY_ADDRESS || "0x_X3_TREASURY_ETH",
    share: 50,
  },
  solana: {
    address: import.meta.env.VITE_SOL_TREASURY_ADDRESS || "X3Treasury_SOL_ADDRESS",
    share: 50,
  },
  binance: {
    address: import.meta.env.VITE_BSC_TREASURY_ADDRESS || "0x_X3_TREASURY_BSC",
    share: 50,
  },
  polygon: {
    address: import.meta.env.VITE_POLYGON_TREASURY_ADDRESS || "0x_X3_TREASURY_POLYGON",
    share: 50,
  },
  arbitrum: {
    address: import.meta.env.VITE_ARB_TREASURY_ADDRESS || "0x_X3_TREASURY_ARB",
    share: 50,
  },
  avalanche: {
    address: import.meta.env.VITE_AVAX_TREASURY_ADDRESS || "0x_X3_TREASURY_AVAX",
    share: 50,
  },
};

/**
 * Calculate treasury split for a given amount
 */
export function calculateTreasurySplit(amount: number, treasuryShare: number = 50): {
  treasury: number;
  user: number;
} {
  const treasury = (amount * treasuryShare) / 100;
  const user = amount - treasury;
  return { treasury, user };
}

/**
 * Validate treasury configuration
 */
export function validateTreasuryConfig(config: TreasuryConfig): boolean {
  if (!config.enabled) return true;
  if (!config.treasuryAddress || config.treasuryAddress.includes("REPLACE")) {
    console.error("[Treasury] Invalid treasury address - must be configured in production");
    return false;
  }
  if (config.treasuryShare < 0 || config.treasuryShare > 100) {
    console.error("[Treasury] Invalid treasury share - must be 0-100%");
    return false;
  }
  return true;
}

/**
 * Get treasury address for a specific chain
 */
export function getTreasuryAddress(chain: keyof typeof MULTI_CHAIN_TREASURY): string {
  return MULTI_CHAIN_TREASURY[chain]?.address || X3_TREASURY_CONFIG.treasuryAddress;
}

/**
 * Treasury event logging for monitoring
 */
export interface TreasuryTransaction {
  timestamp: string;
  appId: string;
  chain: string;
  totalAmount: number;
  treasuryAmount: number;
  userAmount: number;
  txHash?: string;
  status: "pending" | "confirmed" | "failed";
}

export function logTreasuryTransaction(tx: TreasuryTransaction): void {
  const key = "x3-desktop:treasury-log";
  try {
    const log = JSON.parse(localStorage.getItem(key) || "[]");
    log.push(tx);
    // Keep only last 1000 transactions
    if (log.length > 1000) log.splice(0, log.length - 1000);
    localStorage.setItem(key, JSON.stringify(log));
    console.log(`[Treasury] Logged transaction: ${tx.appId} - ${tx.treasuryAmount} to treasury`);
  } catch (error) {
    console.error("[Treasury] Failed to log transaction:", error);
  }
}
