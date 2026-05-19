import * as THREE from 'three';
import { SceneManager } from '@/lib/three/SceneManager';

export interface TerminalCommand {
  command: string;
  execute: (args: string[]) => Promise<string | object>;
  help: string;
}

export const terminalCommands: Record<string, TerminalCommand> = {
  'wallet:balance': {
    command: 'wallet:balance',
    help: 'Get current wallet balance',
    execute: async (args: string[]) => {
      return {
        wallet: args[0] || 'default',
        balance: Math.floor(Math.random() * 100000) + 1000,
        currency: 'X3',
        timestamp: new Date().toISOString(),
      };
    },
  },
  'wallet:transfer': {
    command: 'wallet:transfer',
    help: 'Transfer tokens - wallet:transfer <recipient> <amount>',
    execute: async (args: string[]) => {
      if (args.length < 2) throw new Error('Usage: wallet:transfer <recipient> <amount>');
      return {
        status: 'success',
        txHash: `0x${Math.random().toString(16).slice(2)}`,
        from: 'default',
        to: args[0],
        amount: args[1],
        confirmed: true,
      };
    },
  },
  'governance:proposals': {
    command: 'governance:proposals',
    help: 'List governance proposals',
    execute: async () => {
      return {
        proposals: [
          { id: 1, title: 'Increase fee cap', votes: 45230, status: 'active' },
          { id: 2, title: 'Add new validator', votes: 32100, status: 'active' },
          { id: 3, title: 'Protocol upgrade v2.1', votes: 89500, status: 'voting' },
        ],
      };
    },
  },
  'operator:metrics': {
    command: 'operator:metrics',
    help: 'Get operator performance metrics',
    execute: async () => {
      return {
        uptime: 99.97,
        activeValidators: 2847,
        totalStaked: 12500000,
        apr: 8.5,
        lastUpdate: new Date().toISOString(),
      };
    },
  },
  'blockchain:status': {
    command: 'blockchain:status',
    help: 'Get current blockchain status',
    execute: async () => {
      return {
        blockHeight: Math.floor(Math.random() * 5000000) + 1000000,
        blockTime: '6.5s',
        transactionsPerSecond: 450,
        networkSync: '100%',
        activeNodes: 1847,
      };
    },
  },
  'quantum:simulate': {
    command: 'quantum:simulate',
    help: 'Run quantum computation simulation',
    execute: async (args: string[]) => {
      const iterations = parseInt(args[0]) || 1000;
      return {
        simulation: 'completed',
        iterations,
        result: Math.random() * 100,
        quantumGain: `${((Math.random() * 40 + 60) | 0)}%`,
        executionTime: `${(Math.random() * 500 + 100) | 0}ms`,
      };
    },
  },
  'exchange:prices': {
    command: 'exchange:prices',
    help: 'Get current market prices',
    execute: async () => {
      return {
        X3: { usd: 2450.5, eur: 2100.3, change24h: '+5.2%' },
        ETH: { usd: 3200.0, eur: 2745.5, change24h: '+3.1%' },
        BTC: { usd: 65450.0, eur: 56100.0, change24h: '+2.8%' },
        timestamp: new Date().toISOString(),
      };
    },
  },
  'storage:usage': {
    command: 'storage:usage',
    help: 'Get storage capacity information',
    execute: async () => {
      const used = Math.floor(Math.random() * 800) + 200;
      return {
        total: 1024,
        used,
        available: 1024 - used,
        percentage: Math.floor((used / 1024) * 100),
        diskIO: `${(Math.random() * 500) | 0}MB/s`,
      };
    },
  },
  'help': {
    command: 'help',
    help: 'Show all available commands',
    execute: async () => {
      return {
        commands: Object.values(terminalCommands).map((cmd) => ({
          command: cmd.command,
          help: cmd.help,
        })),
      };
    },
  },
  'clear': {
    command: 'clear',
    help: 'Clear terminal history',
    execute: async () => {
      return { status: 'cleared' };
    },
  },
};

export async function executeCommand(cmd: string): Promise<{ output: string | object; error?: string }> {
  const [command, ...args] = cmd.trim().split(/\s+/);
  const terminalCmd = terminalCommands[command];

  if (!terminalCmd) {
    return { output: '', error: `Command not found: ${command}. Type 'help' for available commands.` };
  }

  try {
    const result = await terminalCmd.execute(args);
    return { output: JSON.stringify(result, null, 2) };
  } catch (error) {
    return { output: '', error: String(error) };
  }
}

export function formatTerminalOutput(output: string | object): string {
  if (typeof output === 'string') return output;
  return JSON.stringify(output, null, 2);
}
