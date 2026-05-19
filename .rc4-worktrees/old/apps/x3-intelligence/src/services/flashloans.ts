// Flashloan provider estimation helpers (simple simulator + scaffolding for real integration)

export type ProviderEstimate = {
  provider: string;
  feePct: number; // e.g., 0.0003 == 0.03%
  maxLiquidity: number; // in USDC
};

const DEFAULT_PROVIDERS = ["Aave", "DyDx", "Balancer", "UniswapV3", "BProtocol"];

// Basic local simulation
export function getProviders(): string[] {
  return DEFAULT_PROVIDERS;
}

export async function estimateFlashloan(provider: string, amount: number): Promise<ProviderEstimate> {
  // If environment has provider endpoints, we could call them here. For now simulate.
  const baseFeeMap: Record<string, number> = {
    Aave: 0.0003,
    DyDx: 0.0004,
    Balancer: 0.0005,
    UniswapV3: 0.0006,
    BProtocol: 0.00035,
  };

  const liquidityMap: Record<string, number> = {
    Aave: 5_000_000,
    DyDx: 2_500_000,
    Balancer: 1_500_000,
    UniswapV3: 3_000_000,
    BProtocol: 500_000,
  };

  // Simulate a network call
  await new Promise((r) => setTimeout(r, 120));

  const feePct = baseFeeMap[provider] ?? 0.0005;
  const maxLiquidity = liquidityMap[provider] ?? 250_000;

  return { provider, feePct, maxLiquidity };
}
