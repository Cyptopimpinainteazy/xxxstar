/**
 * Validation utilities for form inputs and API data
 * Used across authentication, registration, and admin forms
 */

/**
 * Email validation
 */
export const validateEmail = (email: string): { valid: boolean; error?: string } => {
  if (!email) {
    return { valid: false, error: 'Email is required' };
  }

  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(email)) {
    return { valid: false, error: 'Invalid email format' };
  }

  if (email.length > 254) {
    return { valid: false, error: 'Email is too long' };
  }

  return { valid: true };
};

/**
 * Password validation (minimum 8 characters, at least one uppercase, lowercase, and number)
 */
export const validatePassword = (
  password: string,
  minLength: number = 8,
): { valid: boolean; error?: string } => {
  if (!password) {
    return { valid: false, error: 'Password is required' };
  }

  if (password.length < minLength) {
    return { valid: false, error: `Password must be at least ${minLength} characters` };
  }

  if (!/[A-Z]/.test(password)) {
    return { valid: false, error: 'Password must contain at least one uppercase letter' };
  }

  if (!/[a-z]/.test(password)) {
    return { valid: false, error: 'Password must contain at least one lowercase letter' };
  }

  if (!/[0-9]/.test(password)) {
    return { valid: false, error: 'Password must contain at least one number' };
  }

  return { valid: true };
};

/**
 * Blockchain address validation (Solana)
 */
export const validateSolanaAddress = (address: string): { valid: boolean; error?: string } => {
  if (!address) {
    return { valid: false, error: 'Address is required' };
  }

  // Solana addresses are base58 encoded, 32-88 characters
  const solanaAddressRegex = /^[1-9A-HJ-NP-Z]{32,88}$/;
  if (!solanaAddressRegex.test(address)) {
    return { valid: false, error: 'Invalid Solana address format' };
  }

  return { valid: true };
};

/**
 * Ethereum address validation
 */
export const validateEthereumAddress = (address: string): { valid: boolean; error?: string } => {
  if (!address) {
    return { valid: false, error: 'Address is required' };
  }

  // Ethereum addresses are 42 characters (0x + 40 hex chars)
  const ethAddressRegex = /^0x[a-fA-F0-9]{40}$/;
  if (!ethAddressRegex.test(address)) {
    return { valid: false, error: 'Invalid Ethereum address format' };
  }

  return { valid: true };
};

/**
 * Generic blockchain address validation
 */
export const validateBlockchainAddress = (
  address: string,
  chain: string,
): { valid: boolean; error?: string } => {
  const chainLower = chain.toLowerCase();

  if (chainLower === 'solana') {
    return validateSolanaAddress(address);
  } else if (chainLower === 'ethereum') {
    return validateEthereumAddress(address);
  }

  // Generic validation if chain not recognized
  if (!address || address.length < 20) {
    return { valid: false, error: 'Invalid address format' };
  }

  return { valid: true };
};

/**
 * API key format validation (typically base64 or hex string)
 */
export const validateApiKey = (apiKey: string): { valid: boolean; error?: string } => {
  if (!apiKey) {
    return { valid: false, error: 'API key is required' };
  }

  if (apiKey.length < 32) {
    return { valid: false, error: 'API key too short' };
  }

  if (apiKey.length > 256) {
    return { valid: false, error: 'API key too long' };
  }

  // Basic check for valid characters (alphanumeric, hyphens, underscores)
  if (!/^[a-zA-Z0-9\-_]+$/.test(apiKey)) {
    return { valid: false, error: 'API key contains invalid characters' };
  }

  return { valid: true };
};

/**
 * API secret validation (similar to API key)
 */
export const validateApiSecret = (apiSecret: string): { valid: boolean; error?: string } => {
  if (!apiSecret) {
    return { valid: false, error: 'API secret is required' };
  }

  if (apiSecret.length < 32) {
    return { valid: false, error: 'API secret too short' };
  }

  if (apiSecret.length > 512) {
    return { valid: false, error: 'API secret too long' };
  }

  return { valid: true };
};

/**
 * TPS (Transactions Per Second) number validation
 */
export const validateTPS = (tps: string | number): { valid: boolean; error?: string } => {
  const tpsNum = typeof tps === 'string' ? parseInt(tps, 10) : tps;

  if (Number.isNaN(tpsNum)) {
    return { valid: false, error: 'TPS must be a number' };
  }

  if (tpsNum < 1) {
    return { valid: false, error: 'TPS must be at least 1' };
  }

  if (tpsNum > 1000000) {
    return { valid: false, error: 'TPS exceeds maximum allowed value' };
  }

  return { valid: true };
};

/**
 * Port number validation
 */
export const validatePort = (port: string | number): { valid: boolean; error?: string } => {
  const portNum = typeof port === 'string' ? parseInt(port, 10) : port;

  if (Number.isNaN(portNum)) {
    return { valid: false, error: 'Port must be a number' };
  }

  if (portNum < 1 || portNum > 65535) {
    return { valid: false, error: 'Port must be between 1 and 65535' };
  }

  return { valid: true };
};

/**
 * URL validation
 */
export const validateUrl = (url: string): { valid: boolean; error?: string } => {
  if (!url) {
    return { valid: false, error: 'URL is required' };
  }

  try {
    new URL(url);
    return { valid: true };
  } catch {
    return { valid: false, error: 'Invalid URL format' };
  }
};

/**
 * RPC endpoint validation
 */
export const validateRpcEndpoint = (endpoint: string): { valid: boolean; error?: string } => {
  const urlValidation = validateUrl(endpoint);
  if (!urlValidation.valid) {
    return urlValidation;
  }

  // Check that it's http or https
  if (!endpoint.startsWith('http://') && !endpoint.startsWith('https://')) {
    return { valid: false, error: 'RPC endpoint must use http or https' };
  }

  return { valid: true };
};

/**
 * Percentage validation (0-100)
 */
export const validatePercentage = (
  value: string | number,
): { valid: boolean; error?: string } => {
  const num = typeof value === 'string' ? parseFloat(value) : value;

  if (Number.isNaN(num)) {
    return { valid: false, error: 'Must be a number' };
  }

  if (num < 0 || num > 100) {
    return { valid: false, error: 'Percentage must be between 0 and 100' };
  }

  return { valid: true };
};

/**
 * Latency validation (milliseconds)
 */
export const validateLatency = (latency: string | number): { valid: boolean; error?: string } => {
  const num = typeof latency === 'string' ? parseInt(latency, 10) : latency;

  if (Number.isNaN(num)) {
    return { valid: false, error: 'Latency must be a number' };
  }

  if (num < 0) {
    return { valid: false, error: 'Latency cannot be negative' };
  }

  if (num > 60000) {
    return { valid: false, error: 'Latency exceeds reasonable maximum (60000ms)' };
  }

  return { valid: true };
};

/**
 * Chain name validation
 */
export const validateChain = (chain: string): { valid: boolean; error?: string } => {
  if (!chain) {
    return { valid: false, error: 'Chain is required' };
  }

  const validChains = ['solana', 'ethereum', 'polygon', 'arbitrum', 'base'];
  if (!validChains.includes(chain.toLowerCase())) {
    return { valid: false, error: `Chain must be one of: ${validChains.join(', ')}` };
  }

  return { valid: true };
};

/**
 * SLA tier validation
 */
export const validateSlaTier = (tier: string): { valid: boolean; error?: string } => {
  if (!tier) {
    return { valid: false, error: 'SLA tier is required' };
  }

  const validTiers = ['free', 'standard', 'pro', 'enterprise'];
  if (!validTiers.includes(tier.toLowerCase())) {
    return { valid: false, error: `SLA tier must be one of: ${validTiers.join(', ')}` };
  }

  return { valid: true };
};

/**
 * Admin action parameter validation
 */
export const validateAdminActionParams = (
  action: string,
  params: Record<string, any>,
): { valid: boolean; error?: string } => {
  if (!action) {
    return { valid: false, error: 'Action is required' };
  }

  // Validate specific actions
  switch (action) {
    case 'faucet_config': {
      if (params.amount !== undefined && typeof params.amount !== 'number') {
        return { valid: false, error: 'Faucet amount must be a number' };
      }
      if (params.rate_limit !== undefined && typeof params.rate_limit !== 'number') {
        return { valid: false, error: 'Rate limit must be a number' };
      }
      break;
    }

    case 'emergency_pause': {
      if (params.duration !== undefined && typeof params.duration !== 'number') {
        return { valid: false, error: 'Pause duration must be a number' };
      }
      if (params.reason !== undefined && typeof params.reason !== 'string') {
        return { valid: false, error: 'Pause reason must be a string' };
      }
      break;
    }

    default:
      // Generic validation for unknown actions
      if (Object.keys(params).length === 0) {
        return { valid: false, error: 'Action parameters required' };
      }
  }

  return { valid: true };
};

/**
 * Validate multiple fields at once
 */
export const validateForm = (
  data: Record<string, any>,
  schema: Record<string, (value: any) => { valid: boolean; error?: string }>,
): {
  valid: boolean;
  errors: Record<string, string>;
} => {
  const errors: Record<string, string> = {};

  Object.entries(schema).forEach(([field, validator]) => {
    const result = validator(data[field]);
    if (!result.valid && result.error) {
      errors[field] = result.error;
    }
  });

  return {
    valid: Object.keys(errors).length === 0,
    errors,
  };
};
