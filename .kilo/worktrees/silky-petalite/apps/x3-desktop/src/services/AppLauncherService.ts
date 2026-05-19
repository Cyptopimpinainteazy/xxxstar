/**
 * App Launcher Service
 * 
 * Handles launching and managing third-party apps from the X3 Desktop.
 * Uses Tauri shell plugin to execute external applications with treasury integration.
 */

import { Command } from "@tauri-apps/plugin-shell";
import { appDataDir, join } from "@tauri-apps/api/path";
// NOTE: do NOT statically import @tauri-apps/api/fs — use a guarded dynamic import at runtime so
// the browser dev server doesn't attempt to resolve Tauri APIs (they're only present in the Tauri runtime)
import type { AppStoreApp } from "../config/appstore.config";

export interface LaunchResult {
  success: boolean;
  message: string;
  pid?: number;
  error?: string;
}

export class AppLauncherService {
  private runningApps: Map<string, { pid: number; command: any }> = new Map();

  private getErrorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    if (typeof error === "string") return error;
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }

  /**
   * Launch an app with treasury integration enabled
   */
  async launchApp(app: AppStoreApp): Promise<LaunchResult> {
    console.log(`[AppLauncher] Launching ${app.name}...`);

    if (!app.launchCommand) {
      return {
        success: false,
        message: "No launch command configured for this app",
      };
    }

    try {
      // Check if app is already running
      if (this.runningApps.has(app.id)) {
        return {
          success: false,
          message: `${app.name} is already running`,
        };
      }

      // Construct app path
      const appPath = await join(await appDataDir(), "app-store", this.getAppDirectory(app));

      // Ensure the app directory exists (use guarded Tauri fs in runtime)
      const appDirExists = await this.tauriFsExists(appPath);
      if (!appDirExists) {
        return {
          success: false,
          message: `App directory not found: ${appPath}. Run 'apps/x3-desktop/app-store/setup-treasury-integration.sh' to install/copy apps into Tauri appDataDir.`,
        };
      }

      // Determine command(s) to try
      const attempts: Array<{ program: string; args: string[] }> = [];

      if (app.launchCommand && app.launchCommand.length > 0) {
        const parts = this.parseCommand(app.launchCommand);
        const prog = parts[0];
        const rest = parts.slice(1);

        // If the requested command is a shell script (./something.sh), run it via bash to avoid permission issues
        if (prog.startsWith("./") || prog.endsWith('.sh')) {
          attempts.push({ program: 'bash', args: [prog, ...rest] });
        } else {
          attempts.push({ program: prog, args: rest });
        }
      }

      // Fallback heuristics when no explicit launchCommand provided or first attempt fails
      // Prefer local start scripts commonly used by projects
      attempts.push({ program: 'bash', args: ['./start.sh'] });
      attempts.push({ program: 'npm', args: ['start'] });
      attempts.push({ program: 'pnpm', args: ['start'] });
      attempts.push({ program: 'yarn', args: ['start'] });

      // Set up environment variables for treasury integration
      const env = {
        X3_TREASURY_ENABLED: "true",
        X3_TREASURY_ADDRESS: (typeof process !== 'undefined' && process.env && process.env.X3_TREASURY_ADDRESS) || (typeof import.meta !== 'undefined' && (import.meta as any).env?.VITE_X3_TREASURY_ADDRESS) || "X3Treasury_DefaultAddress",
        X3_TREASURY_SHARE: "50",
        APP_STORE_PATH: await join(await appDataDir(), "app-store"),
      };

      // Try each candidate until one spawns successfully
      let lastError: any = null;
      for (const attempt of attempts) {
        try {
          console.log(`[AppLauncher] Trying to execute: ${attempt.program} ${attempt.args.join(' ')} (cwd=${appPath})`);

          // If we're about to run a local shell script, ensure it is executable first
          if (attempt.program === 'bash' && attempt.args.length > 0 && attempt.args[0].startsWith('./')) {
            try {
              const chmod = Command.create('chmod', ['+x', attempt.args[0]], { cwd: appPath });
              await chmod.execute();
              console.log('[AppLauncher] Ensured script is executable');
            } catch (chmodErr) {
              // non-fatal — we'll still try to run via bash
              console.warn('[AppLauncher] chmod failed (ignored):', this.getErrorMessage(chmodErr));
            }
          }

          // If a per-app virtualenv exists, prefer using its python binary for python invocations
          let programToUse = attempt.program;
          if (
            (programToUse === 'python' || programToUse === 'python3') &&
            await this.tauriFsExists(`${appPath}/.venv/bin/python`)
          ) {
            programToUse = `${appPath}/.venv/bin/python`;
            console.log('[AppLauncher] Using app .venv python:', programToUse);
          }

          const command = Command.create(programToUse, attempt.args, {
            cwd: appPath,
            env,
          });

          const child = await command.spawn();

          if (!child || !child.pid) {
            throw new Error('Failed to spawn process');
          }

          console.log(`[AppLauncher] ✅ ${app.name} launched successfully (PID: ${child.pid})`);

          // Store running app info
          this.runningApps.set(app.id, { pid: child.pid, command });

          // Log launch event
          this.logAppLaunch(app);

          return { success: true, message: `${app.name} launched successfully`, pid: child.pid };
        } catch (err) {
          lastError = err;
          console.warn(
            `[AppLauncher] Attempt failed: ${attempt.program} ${attempt.args.join(' ')} -> ${this.getErrorMessage(err)}`
          );
          // try next candidate
        }
      }

      // If none of the attempts succeeded
      console.error(`[AppLauncher] ❌ All launch attempts failed for ${app.name}`);
      return {
        success: false,
        message: `Failed to launch ${app.name}`,
        error: this.getErrorMessage(lastError),
      };
    } catch (error: any) {
      console.error(`[AppLauncher] ❌ Failed to launch ${app.name}:`, error);

      return {
        success: false,
        message: `Failed to launch ${app.name}`,
        error: error.message,
      };
    }
  }

  /**
   * Stop a running app
   */
  async stopApp(appId: string): Promise<boolean> {
    const runningApp = this.runningApps.get(appId);

    if (!runningApp) {
      console.log(`[AppLauncher] App ${appId} is not running`);
      return false;
    }

    try {
      // Kill the process (Tauri doesn't provide a direct way to kill spawned processes)
      // We need to use system commands
      const killCommand = Command.create("kill", [runningApp.pid.toString()]);
      await killCommand.execute();

      this.runningApps.delete(appId);
      console.log(`[AppLauncher] ✅ Stopped app ${appId}`);

      return true;
    } catch (error) {
      console.error(`[AppLauncher] ❌ Failed to stop app ${appId}:`, error);
      return false;
    }
  }

  /**
   * Check if an app is running
   */
  isAppRunning(appId: string): boolean {
    return this.runningApps.has(appId);
  }

  /**
   * Get all running apps
   */
  getRunningApps(): string[] {
    return Array.from(this.runningApps.keys());
  }

  /**
   * Stop all running apps
   */
  async stopAllApps(): Promise<void> {
    console.log("[AppLauncher] Stopping all running apps...");

    const stopPromises = Array.from(this.runningApps.keys()).map((appId) => this.stopApp(appId));

    await Promise.all(stopPromises);

    console.log("[AppLauncher] All apps stopped");
  }

  /**
   * Install or repair a single app using the repository setup script
   * (runs app-store/setup-treasury-integration.sh <appName>)
   */
  async installApp(app: AppStoreApp): Promise<LaunchResult> {
    try {
      const homeEnv = (typeof process !== 'undefined' && process.env && process.env.HOME) || (typeof import.meta !== 'undefined' && (import.meta as any).env?.HOME);
      const repoRoot = homeEnv ? `${homeEnv}/Desktop/x3-chain-master` : '/workspace';
      const setupScript = `${repoRoot}/apps/x3-desktop/app-store/setup-treasury-integration.sh`;

      console.log(`[AppLauncher] Installing/repairing ${app.name} via ${setupScript}`);

      const command = Command.create('bash', [setupScript, this.getAppDirectory(app)], {
        cwd: await appDataDir(),
      });

      // execute synchronously and capture result
      const output = await command.execute();

      if (output.stderr) {
        console.warn(`[AppLauncher] installApp stderr:`, output.stderr);
      }

      console.log(`[AppLauncher] ✅ installApp finished for ${app.name}`);

      return { success: true, message: `Installed/repaired ${app.name}` };
    } catch (error: any) {
      console.error(`[AppLauncher] installApp failed for ${app.name}:`, error);
      return { success: false, message: `Install failed for ${app.name}`, error: error?.message };
    }
  }

  /**
   * Parse launch command into program and arguments
   */
  private parseCommand(command: string): string[] {
    // Simple command parser (can be enhanced)
    // Handles: "python script.py --arg1 --arg2"
    // Handles: "npm run dev"
    // Handles: "./executable --flag"

    const parts = command.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
    return parts.map((part) => part.replace(/"/g, ""));
  }

  /**
   * Get app directory name from app config
   */
  private getAppDirectory(app: AppStoreApp): string {
    // Extract directory from repository URL
    // e.g., "https://github.com/user/repo" -> "repo"
    const urlParts = app.repositoryUrl.split("/");
    const repoName = urlParts[urlParts.length - 1].replace(".git", "");
    return repoName;
  }

  /**
   * Guarded helper that uses Tauri's fs.exists when available.
   * Returns false when not running inside Tauri (avoids browser import resolution).
   */
  private async tauriFsExists(p: string): Promise<boolean> {
    if (typeof window === 'undefined' || (!(window as any).__TAURI_INTERNALS__ && !(window as any).__TAURI__)) {
      // Browser/dev mode — pretend path does not exist (prevents Tauri-only resolution)
      return false;
    }

    try {
      const modPath = '@tauri-apps/api/fs';
      // prevent Vite from statically analyzing this runtime-only import
      const mod = await import(/* @vite-ignore */ modPath as any);
      return await mod.exists(p as any);
    } catch (err) {
      console.warn('[AppLauncher] tauriFsExists dynamic import failed:', err);
      return false;
    }
  }

  /**
   * Log app launch event
   */
  private logAppLaunch(app: AppStoreApp): void {
    const logEntry = {
      timestamp: new Date().toISOString(),
      event: "app_launched",
      appId: app.id,
      appName: app.name,
      treasuryIntegrated: app.treasuryIntegrated,
      treasuryShare: app.treasuryIntegrated ? 50 : 0,
    };

    try {
      const logs = JSON.parse(localStorage.getItem("x3-desktop:app-launches") || "[]");
      logs.push(logEntry);
      if (logs.length > 100) logs.splice(0, logs.length - 100);
      localStorage.setItem("x3-desktop:app-launches", JSON.stringify(logs));
    } catch (error) {
      console.error("[AppLauncher] Failed to log launch event:", error);
    }
  }
}

// Export singleton instance
export const appLauncher = new AppLauncherService();
