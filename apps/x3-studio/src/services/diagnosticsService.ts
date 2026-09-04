import type { DiagnosticEntry } from '../types';

const ERROR_PATTERNS: { regex: RegExp; source: DiagnosticEntry['source']; fileGroup: number; lineGroup: number; colGroup: number; msgGroup: number }[] = [
  { regex: /^error\[(E\d+)\]: (.+)$/gm, source: 'tsc', fileGroup: -1, lineGroup: -1, colGroup: -1, msgGroup: 2 },
  { regex: /^(?:--> )?(.+?\.[rsolxtyjst]{1,5}):(\d+):(\d+):\s*(.+)$/gm, source: 'cargo', fileGroup: 1, lineGroup: 2, colGroup: 3, msgGroup: 4 },
  { regex: /^(?:Error|error):\s*(.+)$/gm, source: 'cargo', fileGroup: -1, lineGroup: -1, colGroup: -1, msgGroup: 1 },
  { regex: /^\s+--> (.+?\.[rsolxty]+):(\d+):(\d+)$/gm, source: 'cargo', fileGroup: 1, lineGroup: 2, colGroup: 3, msgGroup: -1 },
  { regex: /^(.*\.sol):(\d+):(\d+):\s*(Error|Warning|Info):\s*(.+)$/gm, source: 'forge', fileGroup: 1, lineGroup: 2, colGroup: 3, msgGroup: 5 },
];

export function parseDiagnostics(output: string, baseDir: string): DiagnosticEntry[] {
  const entries: DiagnosticEntry[] = [];
  for (const pattern of ERROR_PATTERNS) {
    let match: RegExpExecArray | null;
    while ((match = pattern.regex.exec(output)) !== null) {
      let file = pattern.fileGroup >= 0 ? match[pattern.fileGroup] : baseDir;
      let line = pattern.lineGroup >= 0 ? parseInt(match[pattern.lineGroup]) || 1 : 1;
      let col = pattern.colGroup >= 0 ? parseInt(match[pattern.colGroup]) || 1 : 1;
      let message = pattern.msgGroup >= 0 ? match[pattern.msgGroup] : match[0];

      if (!file.startsWith('/')) {
        file = baseDir + '/' + file;
      }

      const severity: DiagnosticEntry['severity'] =
        match[0].includes('Error') || match[0].startsWith('error') ? 'error' :
        match[0].includes('Warning') ? 'warning' : 'info';

      entries.push({ file, line, column: col, message, severity, source: pattern.source });
    }
  }
  return entries;
}

export function clearDiagnostics() {
  const { useDiagnosticsStore } = require('../store');
  useDiagnosticsStore.getState().clear();
}
