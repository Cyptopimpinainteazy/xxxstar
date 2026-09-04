/**
 * Application-wide constants
 * 
 * This file centralizes all magic numbers and configuration values
 * to improve maintainability and allow easy tuning of behavior.
 */

// ============================================================================
// TPS HISTORY & DATA MANAGEMENT
// ============================================================================

/** Maximum number of TPS data points to keep in history */
export const MAX_TPS_HISTORY_POINTS = 300;

/** Time interval in milliseconds between TPS data collection (2 seconds) */
export const TPS_COLLECTION_INTERVAL_MS = 2_000;

/** Duration of maximum TPS history retention (10 minutes at 2s intervals = 300 points) */
export const TPS_HISTORY_DURATION_MS = 10 * 60 * 1_000;

/** Time ranges available in the TPS chart filter */
export const TIME_RANGE_MS = {
  '1m': 1 * 60 * 1_000,
  '5m': 5 * 60 * 1_000,
  '15m': 15 * 60 * 1_000,
  '30m': 30 * 60 * 1_000,
  '1h': 60 * 60 * 1_000,
} as const;

// ============================================================================
// TOKEN & AUTHENTICATION
// ============================================================================

/** Buffer time before JWT expiry to trigger refresh (5 minutes) */
export const TOKEN_REFRESH_BUFFER_MS = 5 * 60 * 1_000;

/** Minimum time to wait before next refresh attempt (1 second) */
export const TOKEN_REFRESH_MIN_INTERVAL_MS = 1_000;

// ============================================================================
// TIME CONVERSION CONSTANTS
// ============================================================================

/** Seconds per hour */
export const SECONDS_PER_HOUR = 3_600;

/** Seconds per minute */
export const SECONDS_PER_MINUTE = 60;

/** Milliseconds per second */
export const MS_PER_SECOND = 1_000;

// ============================================================================
// ADMIN CONTROLS
// ============================================================================

/** Default faucet rate limit (tokens per minute) */
export const FAUCET_DEFAULT_RATE_LIMIT = '1000';

/** Default maximum tokens per address */
export const FAUCET_DEFAULT_MAX_PER_ADDRESS = '100';

/** Default faucet cooldown period in hours */
export const FAUCET_DEFAULT_COOLDOWN_HOURS = '24';

/** Metric history duration in seconds (1 hour) */
export const ADMIN_METRICS_HISTORY_SECONDS = 3_600;

// ============================================================================
// ACCESSIBILITY & UI
// ============================================================================

/** Toast notification auto-dismiss timeout in milliseconds (3 seconds) */
export const TOAST_AUTO_DISMISS_MS = 3_000;

/** Chart animation transition duration in milliseconds */
export const CHART_ANIMATION_DURATION_MS = 300;

/** Loading indicator animation duration */
export const LOADING_ANIMATION_DURATION_MS = 800;

// ============================================================================
// DATA FORMATTING
// ============================================================================

/** Threshold for displaying numbers in thousands (K format) */
export const LARGE_NUMBER_THRESHOLD = 1_000;

/** Decimal places for TPS display */
export const TPS_DECIMAL_PLACES = 2;

/** Decimal places for percentages */
export const PERCENTAGE_DECIMAL_PLACES = 1;

// ============================================================================
// ERROR HANDLING
// ============================================================================

/** Timeout for API requests in milliseconds (30 seconds) */
export const API_REQUEST_TIMEOUT_MS = 30_000;

/** Maximum number of retry attempts for failed operations */
export const MAX_RETRY_ATTEMPTS = 3;

/** Delay multiplier for exponential backoff retry logic */
export const RETRY_BACKOFF_MULTIPLIER = 2;
