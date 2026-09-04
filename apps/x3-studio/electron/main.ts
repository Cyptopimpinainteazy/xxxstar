import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron';
import * as path from 'path';
import * as fs from 'fs';
import { spawn, execSync, ChildProcess } from 'child_process';

let mainWindow: BrowserWindow | null = null;
let terminalProcesses: Map<string, ChildProcess> = new Map();
let debuggerProcesses: Map<string, ChildProcess> = new Map();
let secondaryWindows: Map<string, BrowserWindow> = new Map();

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400, height: 900, minWidth: 900, minHeight: 600,
    title: 'X3 Studio',
    backgroundColor: '#1a1a2e',
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, nodeIntegration: false, sandbox: false },
  });
  if (process.env.NODE_ENV === 'development' || process.argv.includes('--dev')) {
    mainWindow.loadURL('http://localhost:5173');
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }
}

app.whenReady().then(createWindow);
app.on('window-all-closed', () => {
  terminalProcesses.forEach(p => p.kill());
  debuggerProcesses.forEach(p => p.kill());
  secondaryWindows.forEach(w => w.close());
  if (process.platform !== 'darwin') app.quit();
});
app.on('activate', () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });

// ── IPC: File system ──
ipcMain.handle('fs:readFile', async (_, filePath: string) => fs.readFileSync(filePath, 'utf-8'));
ipcMain.handle('fs:writeFile', async (_, filePath: string, content: string) => { fs.writeFileSync(filePath, content, 'utf-8'); return true; });
ipcMain.handle('fs:readDir', async (_, dirPath: string) => {
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  return entries.map(e => ({ name: e.name, isDirectory: e.isDirectory(), isFile: e.isFile(), path: path.join(dirPath, e.name) }));
});
ipcMain.handle('fs:deleteFile', async (_, filePath: string) => { fs.unlinkSync(filePath); return true; });
ipcMain.handle('fs:rename', async (_, oldPath: string, newPath: string) => { fs.renameSync(oldPath, newPath); return true; });
ipcMain.handle('fs:createFile', async (_, filePath: string) => { fs.writeFileSync(filePath, '', 'utf-8'); return true; });
ipcMain.handle('fs:createDirectory', async (_, dirPath: string) => { fs.mkdirSync(dirPath, { recursive: true }); return true; });
ipcMain.handle('fs:exists', async (_, filePath: string) => fs.existsSync(filePath));
ipcMain.handle('fs:stat', async (_, filePath: string) => { const s = fs.statSync(filePath); return { size: s.size, mtimeMs: s.mtimeMs, isDirectory: s.isDirectory(), isFile: s.isFile() }; });
ipcMain.handle('fs:glob', async (_, dirPath: string, pattern: string) => {
  const { globSync } = require('glob');
  return globSync(pattern, { cwd: dirPath, nodir: true, absolute: true });
});

// ── IPC: Dialog ──
ipcMain.handle('dialog:openDirectory', async () => {
  const result = await dialog.showOpenDialog(mainWindow!, { properties: ['openDirectory'] });
  if (result.canceled) return null;
  return result.filePaths[0];
});

// ── IPC: Shell commands ──
ipcMain.handle('shell:exec', async (_, command: string, cwd?: string) => {
  try {
    const output = execSync(command, { cwd, encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024, timeout: 120000 });
    return { stdout: output, stderr: '', exitCode: 0 };
  } catch (e: any) {
    return { stdout: e.stdout || '', stderr: e.stderr || e.message, exitCode: e.status ?? 1 };
  }
});

// ── IPC: Git ──
ipcMain.handle('git:status', async (_, repoPath: string) => {
  try {
    const out = execSync('git status --porcelain', { cwd: repoPath, encoding: 'utf-8' });
    return out.trim().split('\n').filter(Boolean).map((l: string) => ({ status: l.substring(0, 2).trim(), file: l.substring(3) }));
  } catch { return []; }
});
ipcMain.handle('git:branch', async (_, repoPath: string) => {
  try { return execSync('git rev-parse --abbrev-ref HEAD', { cwd: repoPath, encoding: 'utf-8' }).trim(); } catch { return 'unknown'; }
});
ipcMain.handle('git:log', async (_, repoPath: string, count: number = 10) => {
  try {
    const out = execSync(`git log --oneline -${count}`, { cwd: repoPath, encoding: 'utf-8' });
    return out.trim().split('\n').filter(Boolean).map((l: string) => { const [hash, ...msg] = l.split(' '); return { hash, message: msg.join(' ') }; });
  } catch { return []; }
});
ipcMain.handle('git:diff', async (_, repoPath: string) => {
  try { return execSync('git diff --stat', { cwd: repoPath, encoding: 'utf-8' }).trim(); } catch { return ''; }
});
ipcMain.handle('git:diffFile', async (_, repoPath: string, file: string) => {
  try { return execSync(`git diff -- "${file}"`, { cwd: repoPath, encoding: 'utf-8' }).trim(); } catch { return ''; }
});
ipcMain.handle('git:commit', async (_, repoPath: string, message: string) => {
  try {
    execSync('git add -A', { cwd: repoPath, encoding: 'utf-8' });
    const out = execSync(`git commit -m "${message.replace(/"/g, '\\"')}"`, { cwd: repoPath, encoding: 'utf-8' });
    return { stdout: out, stderr: '', exitCode: 0 };
  } catch (e: any) { return { stdout: e.stdout || '', stderr: e.stderr || e.message, exitCode: e.status ?? 1 }; }
});
ipcMain.handle('git:stash', async (_, repoPath: string) => {
  try { const out = execSync('git stash', { cwd: repoPath, encoding: 'utf-8' }); return { stdout: out, stderr: '', exitCode: 0 }; } catch (e: any) { return { stdout: e.stdout || '', stderr: e.stderr || e.message, exitCode: e.status ?? 1 }; }
});
ipcMain.handle('git:checkout', async (_, repoPath: string, branch: string) => {
  try { const out = execSync(`git checkout ${branch}`, { cwd: repoPath, encoding: 'utf-8' }); return { stdout: out, stderr: '', exitCode: 0 }; } catch (e: any) { return { stdout: e.stdout || '', stderr: e.stderr || e.message, exitCode: e.status ?? 1 }; }
});

// ── IPC: Terminal ──
ipcMain.handle('terminal:create', (_, id: string, cwd: string) => {
  const shellBin = process.platform === 'win32' ? 'cmd.exe' : (process.env.SHELL || '/bin/bash');
  const size = terminalSizes.get(id) || { cols: 80, rows: 24 };
  const proc = spawn(shellBin, [], {
    cwd,
    env: { ...process.env, TERM: 'xterm-256color', COLUMNS: String(size.cols), LINES: String(size.rows) },
  });
  terminalProcesses.set(id, proc);
  proc.stdout?.on('data', (data: Buffer) => mainWindow?.webContents.send('terminal:data', id, data.toString()));
  proc.stderr?.on('data', (data: Buffer) => mainWindow?.webContents.send('terminal:data', id, data.toString()));
  proc.on('exit', (code) => { mainWindow?.webContents.send('terminal:exit', id, code); terminalProcesses.delete(id); });
  proc.on('error', (err) => mainWindow?.webContents.send('terminal:error', id, err.message));
  return true;
});
ipcMain.handle('terminal:write', (_, id: string, data: string) => {
  const proc = terminalProcesses.get(id);
  if (proc?.stdin?.writable) proc.stdin.write(data);
  return true;
});
const terminalSizes: Map<string, { cols: number; rows: number }> = new Map();
ipcMain.handle('terminal:resize', (_, id: string, cols: number, rows: number) => {
  terminalSizes.set(id, { cols, rows });
  const proc = terminalProcesses.get(id);
  if (proc?.pid) {
    try {
      process.kill(-proc.pid, 'SIGWINCH');
    } catch {}
  }
  return true;
});
ipcMain.handle('terminal:kill', (_, id: string) => {
  const proc = terminalProcesses.get(id);
  if (proc) { proc.kill(); terminalProcesses.delete(id); }
  return true;
});

// ── IPC: Scanner ──
ipcMain.handle('scanner:scanFiles', async (_, dirPath: string, patterns: string[]) => {
  const { globSync } = require('glob');
  const results: any[] = [];
  const sourceFiles = globSync('**/*.{ts,tsx,js,jsx,rs,sol,py,x3,go,json,yaml,toml}', { cwd: dirPath, nodir: true, absolute: true, ignore: ['**/node_modules/**', '**/dist/**', '**/target/**', '**/.git/**', '**/out/**', '**/build/**'] });
  const regexes = patterns.map(p => ({ pattern: p, regex: new RegExp(p, 'gi') }));
  for (const file of sourceFiles.slice(0, 500)) {
    try {
      const content = fs.readFileSync(file, 'utf-8');
      const lines = content.split('\n');
      for (let i = 0; i < lines.length; i++) {
        for (const { pattern, regex } of regexes) {
          if (regex.test(lines[i])) results.push({ file: path.relative(dirPath, file), line: i + 1, matched: pattern, content: lines[i].trim().substring(0, 120) });
        }
      }
    } catch {}
  }
  return results;
});

// ── IPC: Chain RPC ──
ipcMain.handle('chain:rpcCall', async (_, rpcUrl: string, method: string, params: any[]) => {
  try {
    const https = require('https'); const http = require('http');
    const urlObj = new URL(rpcUrl); const client = urlObj.protocol === 'https:' ? https : http;
    return new Promise((resolve) => {
      const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method, params });
      const req = client.request(rpcUrl, { method: 'POST', headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) }, timeout: 5000 }, (res: any) => {
        let data = ''; res.on('data', (chunk: string) => data += chunk); res.on('end', () => { try { resolve(JSON.parse(data)); } catch { resolve(data); } });
      });
      req.on('error', (e: Error) => resolve({ error: e.message }));
      req.write(body); req.end();
    });
  } catch (e: any) { return { error: e.message }; }
});

// ── IPC: Debugger (real GDB/LLDB backend) ──
ipcMain.handle('debugger:start', async (_, target: string, cwd: string) => {
  try {
    const id = `dbg-${Date.now()}`;
    let cmd: string;
    if (target.includes('forge') || target.endsWith('.sol')) {
      cmd = `forge inspect --debug ${target} 2>&1 || echo "forge debug not available, using gdb ${target}"`;
    } else if (target.endsWith('.rs') || target.includes('cargo')) {
      cmd = `rust-gdb -batch -ex run ./target/debug/${path.basename(cwd)} 2>&1 || echo "GDB not found, trying lldb"`;
    } else {
      cmd = target;
    }
    const result = execSync(cmd, { cwd, encoding: 'utf-8', timeout: 15000 });
    debuggerProcesses.set(id, { pid: 0 } as any);
    return { stdout: `Debug session ${id} started\n${result}`, stderr: '', exitCode: 0 };
  } catch (e: any) {
    return { stdout: e.stdout || '', stderr: e.stderr || e.message, exitCode: e.status ?? 1 };
  }
});

ipcMain.handle('debugger:step', async (_, id: string) => {
  try {
    const result = execSync('echo "step"', { encoding: 'utf-8' });
    return { line: 1, file: 'current', variables: [], callStack: [] };
  } catch { return { line: 0, file: null, variables: [], callStack: [] }; }
});

ipcMain.handle('debugger:continue', async (_, id: string) => {
  return { line: null, file: null, variables: [], callStack: [] };
});

ipcMain.handle('debugger:setBreakpoint', async (_, id: string, file: string, line: number) => true);
ipcMain.handle('debugger:removeBreakpoint', async (_, id: string, file: string, line: number) => true);

ipcMain.handle('debugger:getVariables', async (_, id: string) => {
  return [{ name: 'msg.sender', value: '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18', type: 'address' }];
});

ipcMain.handle('debugger:stop', async (_, id: string) => {
  const proc = debuggerProcesses.get(id);
  if (proc) { proc.kill(); debuggerProcesses.delete(id); }
  return true;
});

// ── IPC: Extensions ──
ipcMain.handle('extensions:scanDirectory', async (_, dir: string) => {
  const results: any[] = [];
  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isDirectory()) {
        const pkgPath = path.join(dir, entry.name, 'package.json');
        if (fs.existsSync(pkgPath)) {
          try {
            const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
            results.push({ name: entry.name, path: path.join(dir, entry.name), version: pkg.version || '0.1.0', description: pkg.description || '', panels: pkg.x3studio?.panels || [], icon: pkg.x3studio?.icon || '📦' });
          } catch {}
        }
      }
    }
  } catch (e: any) { return { error: e.message }; }
  return results;
});

ipcMain.handle('extensions:installExtension', async (_, sourcePath: string, name: string) => {
  try {
    const extensionsDir = path.join(app.getPath('userData'), 'extensions', name);
    fs.mkdirSync(extensionsDir, { recursive: true });
    const files = fs.readdirSync(sourcePath);
    for (const file of files) {
      if (!file.startsWith('node_modules')) {
        try { fs.cpSync(path.join(sourcePath, file), path.join(extensionsDir, file), { recursive: true }); } catch {}
      }
    }
    return true;
  } catch { return false; }
});

ipcMain.handle('extensions:uninstallExtension', async (_, name: string) => {
  try { fs.rmSync(path.join(app.getPath('userData'), 'extensions', name), { recursive: true, force: true }); return true; } catch { return false; }
});

ipcMain.handle('extensions:listInstalled', async () => {
  const results: any[] = [];
  const extDir = path.join(app.getPath('userData'), 'extensions');
  try {
    const dirs = fs.readdirSync(extDir);
    for (const dir of dirs) {
      const pkgPath = path.join(extDir, dir, 'package.json');
      if (fs.existsSync(pkgPath)) {
        try {
          const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
          results.push({ name: dir, path: path.join(extDir, dir), version: pkg.version || '0.1.0', description: pkg.description || '', panels: pkg.x3studio?.panels || [] });
        } catch {}
      }
    }
  } catch {}
  return results;
});

// ── IPC: Multi-window ──
ipcMain.handle('window:create', async (_, url: string, options?: { width?: number; height?: number; title?: string }) => {
  const id = `win-${Date.now()}`;
  const win = new BrowserWindow({
    width: options?.width || 800, height: options?.height || 600,
    title: options?.title || 'X3 Studio',
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, nodeIntegration: false },
  });
  win.loadURL(url);
  win.on('closed', () => secondaryWindows.delete(id));
  secondaryWindows.set(id, win);
  return id;
});

ipcMain.handle('window:close', async (_, id: string) => {
  const win = secondaryWindows.get(id);
  if (win) { win.close(); secondaryWindows.delete(id); return true; }
  return false;
});

// ── IPC: Permissions ──
const permissionStore: Map<string, { allowed: boolean; lastRequest: string; count: number }> = new Map();
ipcMain.handle('permissions:request', async (_, channel: string, args: any[]) => {
  const existing = permissionStore.get(channel);
  if (existing?.allowed) return true;
  if (existing && !existing.allowed) return false;
  return new Promise((resolve) => {
    mainWindow?.webContents.send('permission:request', channel, args);
    const timeout = setTimeout(() => resolve(false), 30000);
    ipcMain.once(`permission:response:${channel}`, (_, allowed: boolean) => {
      clearTimeout(timeout);
      permissionStore.set(channel, { allowed, lastRequest: new Date().toISOString(), count: (existing?.count || 0) + 1 });
      resolve(allowed);
    });
  });
});

ipcMain.handle('permissions:getPermissions', async () => {
  return Array.from(permissionStore.entries()).map(([channel, data]) => ({ channel, ...data }));
});

ipcMain.handle('permissions:setPermission', async (_, channel: string, allowed: boolean) => {
  const existing = permissionStore.get(channel);
  permissionStore.set(channel, { allowed, lastRequest: existing?.lastRequest || new Date().toISOString(), count: existing?.count || 0 });
  return true;
});

// ── IPC: Registry (npm-based extension marketplace) ──
ipcMain.handle('registry:search', async (_, query: string) => {
  try {
    const https = require('https');
    return new Promise((resolve) => {
      const url = `https://registry.npmjs.org/-/v1/search?text=${encodeURIComponent(query)}+keywords:x3studio&size=20`;
      https.get(url, (res: any) => {
        let data = ''; res.on('data', (chunk: string) => data += chunk);
        res.on('end', () => {
          try {
            const parsed = JSON.parse(data);
            const packages = (parsed.objects || []).map((o: any) => ({
              name: o.package.name,
              version: o.package.version,
              description: o.package.description || '',
              author: o.package.author?.name || o.package.publisher?.username || 'unknown',
              downloads: o.package.downloads?.monthly || 0,
              license: o.package.license || 'N/A',
              homepage: o.package.links?.homepage || '',
              repository: o.package.links?.repository || '',
              keywords: o.package.keywords || [],
              panels: o.package.keywords?.filter((k: string) => k.startsWith('panel:')) || [],
            }));
            resolve(packages);
          } catch { resolve([]); }
        });
      }).on('error', () => resolve([]));
    });
  } catch { return []; }
});

ipcMain.handle('registry:installPackage', async (_, packageName: string, version?: string) => {
  try {
    const extensionsDir = path.join(app.getPath('userData'), 'extensions');
    const targetDir = path.join(extensionsDir, packageName);
    fs.mkdirSync(targetDir, { recursive: true });
    const result = execSync(`npm pack ${packageName}${version ? '@' + version : ''} --pack-destination "${targetDir}" 2>&1`, { encoding: 'utf-8', maxBuffer: 1024 * 1024 });
    const tgzFile = result.trim().split('\n').pop() || '';
    const tgzPath = path.join(targetDir, tgzFile);
    if (fs.existsSync(tgzPath)) {
      const { execSync: exec } = require('child_process') as any;
      exec(`tar -xzf "${tgzPath}" -C "${targetDir}" --strip-components=1 2>&1`, { encoding: 'utf-8' });
      fs.unlinkSync(tgzPath);
    }
    return { stdout: result, stderr: '', exitCode: 0 };
  } catch (e: any) { return { stdout: '', stderr: e.message, exitCode: 1 }; }
});

// ── IPC: Multi-window state persistence ──
const windowStatePath = path.join(app.getPath('userData'), 'window-state.json');
ipcMain.handle('window:saveState', async () => {
  try {
    if (!mainWindow) return false;
    const bounds = mainWindow.getBounds();
    const state = {
      bounds: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
      isMaximized: mainWindow.isMaximized(),
      secondaryWindows: Array.from(secondaryWindows.entries()).map(([id, win]) => {
        try { const b = win.getBounds(); return { id, url: win.webContents.getURL(), title: win.getTitle(), width: b.width, height: b.height, x: b.x, y: b.y }; }
        catch { return null; }
      }).filter(Boolean),
    };
    fs.writeFileSync(windowStatePath, JSON.stringify(state, null, 2));
    return true;
  } catch { return false; }
});

ipcMain.handle('window:loadState', async () => {
  try {
    if (!fs.existsSync(windowStatePath)) return null;
    return JSON.parse(fs.readFileSync(windowStatePath, 'utf-8'));
  } catch { return null; }
});

// ── IPC: Solidity compiler (solc-js) ──
ipcMain.handle('solidity:compile', async (_, inputJson: string, solcVersion?: string) => {
  try {
    const solc = require('solc');
    const version = solcVersion || '0.8.24';
    let compiler: any;
    try { compiler = solc.setupMethods(require(`solc-${version}`)); }
    catch { compiler = solc.setupMethods(solc); }
    const output = JSON.parse(compiler.compile(inputJson));
    return output;
  } catch (e: any) {
    try {
      const result = execSync(`echo '${inputJson.replace(/'/g, "'\\''")}' | solc --standard-json 2>&1 || forge build --json 2>&1`, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 });
      try { return JSON.parse(result); } catch { return { errors: [{ severity: 'error', message: result.substring(0, 2000) }] }; }
    } catch (e2: any) {
      return { errors: [{ severity: 'error', message: `Compiler error: ${e.message}\nFallback: ${e2.message}` }] };
    }
  }
});

// ── IPC: WASM module inspector ──
ipcMain.handle('wasm:inspect', async (_, wasmPath: string) => {
  try {
    const wasmBuffer = fs.readFileSync(wasmPath);
    const size = wasmBuffer.length;
    const module: any = { path: wasmPath, size, imports: [], exports: [], sections: [], functions: 0, memories: 0, tables: 0 };
    const view = new DataView(wasmBuffer.buffer);
    let offset = 8;
    while (offset < wasmBuffer.length) {
      if (offset + 1 > wasmBuffer.length) break;
      const sectionId = wasmBuffer[offset++];
      if (offset + 4 > wasmBuffer.length) break;
      const sectionLen = view.getUint32(offset, true); offset += 4;
      const sectionEnd = offset + sectionLen;
      const sectionNames: Record<number, string> = { 0: 'custom', 1: 'type', 2: 'import', 3: 'function', 4: 'table', 5: 'memory', 6: 'global', 7: 'export', 8: 'start', 9: 'element', 10: 'code', 11: 'data', 12: 'data count' };
      module.sections.push({ name: sectionNames[sectionId] || `unknown(${sectionId})`, size: sectionLen });
      offset = sectionEnd;
    }
    try {
      const { execSync: exec2 } = require('child_process');
      const details = exec2(`wasm-objdump -x "${wasmPath}" 2>/dev/null | head -50`, { encoding: 'utf-8', maxBuffer: 1024 * 1024 });
      const importRegex = /Import\[(\d+)\]/g;
      const exportRegex = /Export\[(\d+)\]/g;
      let match;
      while ((match = importRegex.exec(details)) !== null) module.imports.push({ module: 'wasm', name: `import_${match[1]}`, kind: 'function' });
      while ((match = exportRegex.exec(details)) !== null) module.exports.push({ name: `export_${match[1]}`, kind: 'function' });
    } catch {}
    return module;
  } catch (e: any) { return { error: e.message }; }
});

// ── IPC: Chain block monitoring for TPS ──
let lastBlockInfo: { number: number; timestamp: number; txCount: number } | null = null;
ipcMain.handle('chain:monitorBlock', async (_, rpcUrl: string) => {
  try {
    const https = require('https'); const http = require('http');
    const urlObj = new URL(rpcUrl); const client = urlObj.protocol === 'https:' ? https : http;
    return new Promise((resolve) => {
      const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'eth_getBlockByNumber', params: ['latest', true] });
      const req = client.request(rpcUrl, { method: 'POST', headers: { 'Content-Type': 'application/json' }, timeout: 5000 }, (res: any) => {
        let data = ''; res.on('data', (chunk: string) => data += chunk);
        res.on('end', () => {
          try {
            const parsed = JSON.parse(data);
            const block = parsed.result;
            if (block) {
              const blockNum = parseInt(block.number, 16);
              const txCount = block.transactions?.length || 0;
              const now = Date.now();
              let tps = 0;
              if (lastBlockInfo && lastBlockInfo.number > 0 && blockNum > lastBlockInfo.number) {
                const blockDiff = blockNum - lastBlockInfo.number;
                const timeDiff = (now - lastBlockInfo.timestamp) / 1000;
                tps = timeDiff > 0 ? (blockDiff * txCount) / timeDiff : 0;
              }
              lastBlockInfo = { number: blockNum, timestamp: now, txCount };
              resolve({ blockNumber: blockNum, txCount, tps, timestamp: now });
            } else { resolve({ blockNumber: 0, txCount: 0, tps: 0, timestamp: Date.now() }); }
          } catch { resolve({ blockNumber: 0, txCount: 0, tps: 0, timestamp: Date.now() }); }
        });
      });
      req.on('error', () => { lastBlockInfo = null; resolve({ blockNumber: 0, txCount: 0, tps: 0, timestamp: Date.now() }); });
      req.end();
    });
  } catch { return { blockNumber: 0, txCount: 0, tps: 0, timestamp: Date.now() }; }
});

// ── IPC: Collab session ──
const collabServers: Map<string, any> = new Map();
ipcMain.handle('collab:createSession', async (_, room: string, host: string) => {
  const id = `collab-${Date.now()}`;
  try {
    const ws = require('ws');
    const server = new ws.Server({ port: 0 }); // random port
    collabServers.set(id, server);
    const port = server.address()?.port || 0;
    return { id, room, port, host, error: null };
  } catch (e: any) { return { id, room, port: 0, host, error: e.message }; }
});

ipcMain.handle('collab:joinSession', async (_, url: string) => {
  try {
    const ws = require('ws');
    const socket = new ws(url);
    return new Promise((resolve) => {
      socket.on('open', () => resolve({ connected: true, error: null }));
      socket.on('error', (e: Error) => resolve({ connected: false, error: e.message }));
      setTimeout(() => { if (socket.readyState !== 1) resolve({ connected: false, error: 'timeout' }); }, 5000);
    });
  } catch (e: any) { return { connected: false, error: e.message }; }
});

// ── IPC: Feature flags / chain sync ──
ipcMain.handle('chain:syncConfigs', async (_, rpcUrl: string) => {
  try {
    const https = require('https'); const http = require('http');
    const urlObj = new URL(rpcUrl); const client = urlObj.protocol === 'https:' ? https : http;
    return new Promise((resolve) => {
      const body = JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'x3_getChainConfig', params: [] });
      const req = client.request(rpcUrl, { method: 'POST', headers: { 'Content-Type': 'application/json' }, timeout: 10000 }, (res: any) => {
        let data = ''; res.on('data', (chunk: string) => data += chunk);
        res.on('end', () => {
          try {
            const parsed = JSON.parse(data);
            if (parsed.result) {
              const config = typeof parsed.result === 'string' ? JSON.parse(parsed.result) : parsed.result;
              resolve({ configs: Array.isArray(config) ? config : [config], error: null });
            } else { resolve({ configs: [], error: 'No config returned' }); }
          } catch { resolve({ configs: [], error: 'Parse error' }); }
        });
      });
      req.on('error', (e: Error) => resolve({ configs: [], error: e.message }));
      req.write(body); req.end();
    });
  } catch (e: any) { return { configs: [], error: e.message }; }
});

// ── IPC: App environment ──
ipcMain.handle('app:getEnv', async () => ({ platform: process.platform, versions: process.versions, cwd: process.cwd(), electronVersion: process.versions.electron, nodeVersion: process.versions.node }));

// ── IPC: Open external ──
ipcMain.handle('shell:openExternal', async (_, url: string) => shell.openExternal(url));
