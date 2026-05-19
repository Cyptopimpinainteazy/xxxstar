/**
 * X3 Chain SDK Environment Configuration
 *
 * Provides environment variable support for configuring SDK connections
 * to live node endpoints (mainnet, testnet, or custom).
 *
 * Environment Variables:
 *   X3_RPC_ENDPOINT   - Custom WebSocket endpoint (overrides network selection)
 *   X3_NETWORK        - Network to connect to: 'mainnet' | 'testnet' | 'local' (default: 'local')
 *   X3_AUTO_RECONNECT - Enable auto-reconnect (default: 'true')
 *   X3_RECONNECT_MAX  - Maximum reconnect attempts (default: '5')
 *   X3_RECONNECT_DELAY - Reconnect delay in ms (default: '1000')
 *   X3_TIMEOUT        - Request timeout in ms (default: '30000')
 */

// Network configuration
export type X3Network = 'mainnet' | 'testnet' | 'local';

// Environment variable parser
function getEnv(name: string, defaultValue: string): string {
  if (typeof process !== 'undefined' && process.env) {
    return process.env[name] || defaultValue;
  }
  return defaultValue;
}

function getEnvNumber(name: string, defaultValue: number): number {
  const value = getEnv(name, defaultValue.toString());
  const parsed = parseInt(value, 10);
  return isNaN(parsed) ? defaultValue : parsed;
}

function getEnvBoolean(name: string, defaultValue: boolean): boolean {
  const value = getEnv(name, defaultValue.toString());
  return value.toLowerCase() === 'true' || value === '1';
}

// Network endpoints configuration
export const NETWORK_ENDPOINTS: Record<X3Network, string> = {
  mainnet: getEnv('X3_RPC_ENDPOINT', 'wss://rpc.x3chain.io:9944'),
  testnet: getEnv('X3_RPC_ENDPOINT', 'wss://testnet.x3chain.io:9944'),
  local: getEnv('X3_RPC_ENDPOINT', 'ws://127.0.0.1:9944'),
};

// SDK configuration interface
export interface X3SdkConfig {
  /** Network to connect to */
  network: X3Network;
  /** Custom endpoint (overrides network) */
  endpoint?: string;
  /** Enable auto-reconnect */
  autoReconnect: boolean;
  /** Maximum reconnect attempts */
  reconnectMaxAttempts: number;
  /** Reconnect delay in ms */
  reconnectDelay: number;
  /** Request timeout in ms */
  timeout: number;
  /** Enable debug logging */
  debug: boolean;
}

// Get SDK configuration from environment
export function getSdkConfig(): X3SdkConfig {
  const networkEnv = getEnv('X3_NETWORK', 'local').toLowerCase() as X3Network;
  const network = (['mainnet', 'testnet', 'local'].includes(networkEnv) 
    ? networkEnv 
    : 'local') as X3Network;

  return {
    network,
    endpoint: getEnv('X3_RPC_ENDPOINT', undefined),
    autoReconnect: getEnvBoolean('X3_AUTO_RECONNECT', true),
    reconnectMaxAttempts: getEnvNumber('X3_RECONNECT_MAX', 5),
    reconnectDelay: getEnvNumber('X3_RECONNECT_DELAY', 1000),
    timeout: getEnvNumber('X3_TIMEOUT', 30000),
    debug: getEnvBoolean('X3_DEBUG', false),
  };
}

// Get endpoint for a specific network
export function getEndpoint(network: X3Network): string {
  return NETWORK_ENDPOINTS[network];
}

// Get current network from environment
export function getCurrentNetwork(): X3Network {
  return getSdkConfig().network;
}

// Get current endpoint (custom or network-based)
export function getCurrentEndpoint(): string {
  const config = getSdkConfig();
  return config.endpoint || NETWORK_ENDPOINTS[config.network];
}

// Export default configuration
export default getSdkConfig;
