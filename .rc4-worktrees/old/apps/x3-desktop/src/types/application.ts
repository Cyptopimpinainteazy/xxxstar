/**
 * Application manifest — defines a launchable application in the desktop environment.
 *
 * @example
 * ```ts
 * const app: Application = {
 *   id: "explorer",
 *   name: "Block Explorer",
 *   category: "blockchain",
 *   icon: { type: "placeholder", category: "blockchain", color: "#ff6b35" },
 *   launchCommand: { type: "tauri", target: "launch_explorer" },
 * };
 * ```
 */
export interface Application {
  /** Unique identifier */
  id: string;
  /** Human-readable display name */
  name: string;
  /** Short description shown in tooltips */
  description?: string;
  /** Functional category for grouping */
  category: ApplicationCategory;
  /** Icon configuration */
  icon: ApplicationIcon;
  /** How to launch this application */
  launchCommand: LaunchCommand;
  /** Mark this app as preinstalled / locked into the desktop (optional) */
  preinstalled?: boolean;
  /** System requirements for launch validation */
  systemRequirements?: SystemRequirements;
  /** Lifecycle management configuration */
  lifecycle?: LifecycleConfig;
  /** Arbitrary metadata */
  metadata?: Record<string, unknown>;
}

export type ApplicationCategory =
  | "blockchain"
  | "analysis"
  | "utility"
  | "service"
  | "defi"
  | "security"
  | "development"
  | "other";

export interface ApplicationIcon {
  /** Icon source type — 'file' for actual asset, 'placeholder' for auto-generated */
  type: "file" | "placeholder";
  /** Filesystem or URL path to icon asset (SVG/PNG) */
  path?: string;
  /** Category label displayed on placeholder icons */
  category?: string;
  /** Accent color for placeholder icon background */
  color?: string;
}

export interface LaunchCommand {
  /** Execution method */
  type: "tauri" | "process" | "url" | "internal";
  /** Command name, binary path, or URL */
  target: string;
  /** CLI arguments */
  args?: string[];
  /** Environment variable overrides */
  env?: Record<string, string>;
}

export interface SystemRequirements {
  /** Minimum memory in MB */
  minMemory?: number;
  /** Minimum disk space in MB */
  minDiskSpace?: number;
  /** Service IDs that must be running first */
  dependencies?: string[];
}

export interface LifecycleConfig {
  /** Startup timeout in milliseconds (default 10000) */
  timeout: number;
  /** Optional health-check endpoint */
  healthcheck?: { endpoint: string; interval: number };
  /** Auto-restart on crash */
  autoRestart: boolean;
}

/**
 * Runtime information about a running application process.
 */
export interface ProcessInfo {
  /** Corresponding application ID */
  appId: string;
  /** OS process ID (if available) */
  pid?: number;
  /** Process lifecycle state */
  status: ProcessStatus;
  /** ISO-8601 timestamp of launch */
  startedAt: string;
  /** Last heartbeat ISO-8601 timestamp */
  lastHeartbeat?: string;
  /** Memory usage in bytes */
  memoryUsage?: number;
  /** Most recent error message */
  lastError?: string;
}

export type ProcessStatus =
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "crashed"
  | "unreachable";

/** Category icon/color mapping for placeholder generation */
export const CATEGORY_COLORS: Record<ApplicationCategory, string> = {
  blockchain: "#1a9fb5",
  analysis: "#42a5f5",
  utility: "#66bb6a",
  service: "#ab47bc",
  defi: "#2ab4cc",
  security: "#ef5350",
  development: "#26c6da",
  other: "#78909c",
};

/** Category labels for placeholder icons */
export const CATEGORY_LABELS: Record<ApplicationCategory, string> = {
  blockchain: "⛓",
  analysis: "📊",
  utility: "🔧",
  service: "⚙",
  defi: "💰",
  security: "🔒",
  development: "🛠",
  other: "📦",
};
