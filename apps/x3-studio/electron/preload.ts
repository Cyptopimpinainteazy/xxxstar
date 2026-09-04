import { contextBridge, ipcRenderer } from 'electron';

export interface X3StudioAPI {
  fs: {
    readFile: (path: string) => Promise<string>;
    writeFile: (path: string, content: string) => Promise<boolean>;
    readDir: (path: string) => Promise<{ name: string; isDirectory: boolean; isFile: boolean; path: string }[]>;
    deleteFile: (path: string) => Promise<boolean>;
    rename: (oldPath: string, newPath: string) => Promise<boolean>;
    createFile: (path: string) => Promise<boolean>;
    createDirectory: (path: string) => Promise<boolean>;
    exists: (path: string) => Promise<boolean>;
    stat: (path: string) => Promise<{ size: number; mtimeMs: number; isDirectory: boolean; isFile: boolean }>;
    glob: (dir: string, pattern: string) => Promise<string[]>;
  };
  dialog: { openDirectory: () => Promise<string | null> };
  shell: { exec: (command: string, cwd?: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>; openExternal: (url: string) => Promise<void> };
  terminal: {
    create: (id: string, cwd: string) => Promise<boolean>;
    write: (id: string, data: string) => Promise<boolean>;
    resize: (id: string, cols: number, rows: number) => Promise<boolean>;
    kill: (id: string) => Promise<boolean>;
    onData: (callback: (id: string, data: string) => void) => void;
    onExit: (callback: (id: string, code: number | null) => void) => void;
    onError: (callback: (id: string, msg: string) => void) => void;
  };
  git: {
    status: (repoPath: string) => Promise<{ status: string; file: string }[]>;
    branch: (repoPath: string) => Promise<string>;
    log: (repoPath: string, count?: number) => Promise<{ hash: string; message: string }[]>;
    diff: (repoPath: string) => Promise<string>;
    commit: (repoPath: string, message: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
    stash: (repoPath: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
    checkout: (repoPath: string, branch: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
    diffFile: (repoPath: string, file: string) => Promise<string>;
  };
  scanner: { scanFiles: (dir: string, patterns: string[]) => Promise<{ file: string; line: number; matched: string; content: string }[]> };
  app: { getEnv: () => Promise<any> };
  debugger: {
    start: (target: string, cwd: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
    step: (id: string) => Promise<{ line: number; file: string | null; variables: any[]; callStack: any[] }>;
    continue: (id: string) => Promise<{ line: number | null; file: string | null; variables: any[]; callStack: any[] }>;
    setBreakpoint: (id: string, file: string, line: number) => Promise<boolean>;
    removeBreakpoint: (id: string, file: string, line: number) => Promise<boolean>;
    getVariables: (id: string) => Promise<{ name: string; value: string; type: string }[]>;
    stop: (id: string) => Promise<boolean>;
  };
  extensions: {
    scanDirectory: (dir: string) => Promise<any[]>;
    installExtension: (sourcePath: string, name: string) => Promise<boolean>;
    uninstallExtension: (name: string) => Promise<boolean>;
    listInstalled: () => Promise<any[]>;
  };
  window: {
    create: (url: string, options?: any) => Promise<string>;
    close: (id: string) => Promise<boolean>;
  };
  permissions: {
    request: (channel: string, args: any[]) => Promise<boolean>;
    getPermissions: () => Promise<any[]>;
    setPermission: (channel: string, allowed: boolean) => Promise<boolean>;
  };
  registry: {
    search: (query: string) => Promise<any[]>;
    installPackage: (name: string, version?: string) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
  };
  windowState: {
    save: () => Promise<boolean>;
    load: () => Promise<any>;
  };
  solidity: {
    compile: (inputJson: string, solcVersion?: string) => Promise<any>;
  };
  wasm: {
    inspect: (wasmPath: string) => Promise<any>;
  };
  chain: {
    rpcCall: (rpcUrl: string, method: string, params: any[]) => Promise<any>;
    monitorBlock: (rpcUrl: string) => Promise<{ blockNumber: number; txCount: number; tps: number; timestamp: number }>;
    syncConfigs: (rpcUrl: string) => Promise<{ configs: any[]; error: string | null }>;
  };
  collab: {
    createSession: (room: string, host: string) => Promise<any>;
    joinSession: (url: string) => Promise<{ connected: boolean; error: string | null }>;
  };
  on: (channel: string, callback: (...args: any[]) => void) => void;
  removeAllListeners: (channel: string) => void;
}

const api: X3StudioAPI = {
  fs: {
    readFile: (path) => ipcRenderer.invoke('fs:readFile', path),
    writeFile: (path, content) => ipcRenderer.invoke('fs:writeFile', path, content),
    readDir: (path) => ipcRenderer.invoke('fs:readDir', path),
    deleteFile: (path) => ipcRenderer.invoke('fs:deleteFile', path),
    rename: (oldPath, newPath) => ipcRenderer.invoke('fs:rename', oldPath, newPath),
    createFile: (path) => ipcRenderer.invoke('fs:createFile', path),
    createDirectory: (path) => ipcRenderer.invoke('fs:createDirectory', path),
    exists: (path) => ipcRenderer.invoke('fs:exists', path),
    stat: (path) => ipcRenderer.invoke('fs:stat', path),
    glob: (dir, pattern) => ipcRenderer.invoke('fs:glob', dir, pattern),
  },
  dialog: { openDirectory: () => ipcRenderer.invoke('dialog:openDirectory') },
  shell: { exec: (cmd, cwd) => ipcRenderer.invoke('shell:exec', cmd, cwd), openExternal: (url) => ipcRenderer.invoke('shell:openExternal', url) },
  terminal: {
    create: (id, cwd) => ipcRenderer.invoke('terminal:create', id, cwd),
    write: (id, data) => ipcRenderer.invoke('terminal:write', id, data),
    resize: (id, cols, rows) => ipcRenderer.invoke('terminal:resize', id, cols, rows),
    kill: (id) => ipcRenderer.invoke('terminal:kill', id),
    onData: (callback) => ipcRenderer.on('terminal:data', (_, id, data) => callback(id, data)),
    onExit: (callback) => ipcRenderer.on('terminal:exit', (_, id, code) => callback(id, code)),
    onError: (callback) => ipcRenderer.on('terminal:error', (_, id, msg) => callback(id, msg)),
  },
  git: {
    status: (repoPath) => ipcRenderer.invoke('git:status', repoPath),
    branch: (repoPath) => ipcRenderer.invoke('git:branch', repoPath),
    log: (repoPath, count) => ipcRenderer.invoke('git:log', repoPath, count),
    diff: (repoPath) => ipcRenderer.invoke('git:diff', repoPath),
    commit: (repoPath, message) => ipcRenderer.invoke('git:commit', repoPath, message),
    stash: (repoPath) => ipcRenderer.invoke('git:stash', repoPath),
    checkout: (repoPath, branch) => ipcRenderer.invoke('git:checkout', repoPath, branch),
    diffFile: (repoPath, file) => ipcRenderer.invoke('git:diffFile', repoPath, file),
  },
  scanner: { scanFiles: (dir, patterns) => ipcRenderer.invoke('scanner:scanFiles', dir, patterns) },
  app: { getEnv: () => ipcRenderer.invoke('app:getEnv') },
  debugger: {
    start: (target, cwd) => ipcRenderer.invoke('debugger:start', target, cwd),
    step: (id) => ipcRenderer.invoke('debugger:step', id),
    continue: (id) => ipcRenderer.invoke('debugger:continue', id),
    setBreakpoint: (id, file, line) => ipcRenderer.invoke('debugger:setBreakpoint', id, file, line),
    removeBreakpoint: (id, file, line) => ipcRenderer.invoke('debugger:removeBreakpoint', id, file, line),
    getVariables: (id) => ipcRenderer.invoke('debugger:getVariables', id),
    stop: (id) => ipcRenderer.invoke('debugger:stop', id),
  },
  extensions: {
    scanDirectory: (dir) => ipcRenderer.invoke('extensions:scanDirectory', dir),
    installExtension: (sourcePath, name) => ipcRenderer.invoke('extensions:installExtension', sourcePath, name),
    uninstallExtension: (name) => ipcRenderer.invoke('extensions:uninstallExtension', name),
    listInstalled: () => ipcRenderer.invoke('extensions:listInstalled'),
  },
  window: {
    create: (url, options) => ipcRenderer.invoke('window:create', url, options),
    close: (id) => ipcRenderer.invoke('window:close', id),
  },
  permissions: {
    request: (channel, args) => ipcRenderer.invoke('permissions:request', channel, args),
    getPermissions: () => ipcRenderer.invoke('permissions:getPermissions'),
    setPermission: (channel, allowed) => ipcRenderer.invoke('permissions:setPermission', channel, allowed),
  },
  registry: {
    search: (query) => ipcRenderer.invoke('registry:search', query),
    installPackage: (name, version) => ipcRenderer.invoke('registry:installPackage', name, version),
  },
  windowState: {
    save: () => ipcRenderer.invoke('window:saveState'),
    load: () => ipcRenderer.invoke('window:loadState'),
  },
  solidity: {
    compile: (inputJson, solcVersion) => ipcRenderer.invoke('solidity:compile', inputJson, solcVersion),
  },
  wasm: {
    inspect: (wasmPath) => ipcRenderer.invoke('wasm:inspect', wasmPath),
  },
  chain: {
    rpcCall: (rpcUrl, method, params) => ipcRenderer.invoke('chain:rpcCall', rpcUrl, method, params),
    monitorBlock: (rpcUrl) => ipcRenderer.invoke('chain:monitorBlock', rpcUrl),
    syncConfigs: (rpcUrl) => ipcRenderer.invoke('chain:syncConfigs', rpcUrl),
  },
  collab: {
    createSession: (room, host) => ipcRenderer.invoke('collab:createSession', room, host),
    joinSession: (url) => ipcRenderer.invoke('collab:joinSession', url),
  },
  on: (channel, callback) => { ipcRenderer.on(channel, (_, ...args) => callback(...args)); },
  removeAllListeners: (channel) => { ipcRenderer.removeAllListeners(channel); },
};

contextBridge.exposeInMainWorld('x3studio', api);
