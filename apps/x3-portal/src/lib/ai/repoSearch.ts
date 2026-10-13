import fs from 'node:fs/promises';
import path from 'node:path';

export type RepoSnippet = {
  path: string; // repo-relative
  startLine: number;
  endLine: number;
  text: string;
  score: number;
};

type SearchOptions = {
  repoRoot: string;
  question: string;
  maxFiles?: number;
  maxFileBytes?: number;
  maxSnippets?: number;
  windowLines?: number;
};

const DEFAULT_DIRS = [
  'apps/explorer/src',
  'apps/shared',
  'packages',
  'pallets',
  'runtime',
  'node',
  'crates',
  'openspec',
  'docs',
];

const ALLOWED_EXT = new Set([
  '.ts', '.tsx', '.js', '.jsx',
  '.rs',
  '.md', '.txt',
  '.toml', '.yaml', '.yml', '.json',
]);

const SKIP_DIR = new Set([
  '.git', 'node_modules', 'target', '.next', '.turbo', 'dist', 'build', '.cache',
]);

function tokenize(question: string) {
  const stop = new Set([
    'the','a','an','and','or','to','of','in','on','for','with','is','are','was','were','be','been','being',
    'this','that','these','those','it','its','as','at','by','from','about','into','over','under','than',
    'how','what','why','where','when','who','which','can','could','should','would','do','does','did',
  ]);

  const tokens = question
    .toLowerCase()
    .replace(/[^a-z0-9_\-/\s]/g, ' ')
    .split(/\s+/g)
    .filter(Boolean)
    .filter(t => t.length >= 3)
    .filter(t => !stop.has(t));

  // De-dupe while preserving order
  const seen = new Set<string>();
  const deduped: string[] = [];
  for (const t of tokens) {
    if (!seen.has(t)) {
      seen.add(t);
      deduped.push(t);
    }
  }
  return deduped.slice(0, 12);
}

async function* walkFiles(rootAbs: string, relBase: string, maxFiles: number): AsyncGenerator<string> {
  let yielded = 0;

  async function* walk(dirAbs: string, dirRel: string): AsyncGenerator<string> {
    if (yielded >= maxFiles) return;

    let entries: Array<import('node:fs').Dirent>;
    try {
      entries = await fs.readdir(dirAbs, { withFileTypes: true });
    } catch {
      return;
    }

    for (const entry of entries) {
      if (yielded >= maxFiles) return;
      if (entry.name.startsWith('.DS_Store')) continue;

      const nextRel = dirRel ? path.posix.join(dirRel, entry.name) : entry.name;
      const nextAbs = path.join(dirAbs, entry.name);

      if (entry.isDirectory()) {
        if (SKIP_DIR.has(entry.name)) continue;
        yield* walk(nextAbs, nextRel);
        continue;
      }

      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name).toLowerCase();
      if (!ALLOWED_EXT.has(ext)) continue;

      // Skip obviously sensitive / huge binary-ish names
      if (entry.name.includes('keys') || entry.name.endsWith('.enc') || entry.name.endsWith('.gpg')) continue;

      yielded++;
      yield path.posix.join(relBase, nextRel);
    }
  }

  yield* walk(rootAbs, '');
}

function scoreText(text: string, relPath: string, tokens: string[]) {
  let score = 0;
  const lower = text.toLowerCase();

  for (const t of tokens) {
    const re = new RegExp(`\\b${t.replace(/[.*+?^${}()|[\\]\\]/g, '\\$&')}\\b`, 'g');
    const matches = lower.match(re);
    if (matches) score += Math.min(matches.length, 20);

    if (relPath.toLowerCase().includes(t)) score += 3;
  }

  // Prefer spec/docs when question is conceptual
  if (relPath.endsWith('.md')) score += 2;
  if (relPath.includes('openspec/')) score += 2;
  if (relPath.includes('pallets/') || relPath.includes('runtime/')) score += 1;

  return score;
}

function extractWindows(fileText: string, relPath: string, tokens: string[], windowLines: number) {
  const lines = fileText.split(/\r?\n/);
  const lowerLines = lines.map(l => l.toLowerCase());

  const hitLines: number[] = [];
  for (let i = 0; i < lowerLines.length; i++) {
    const l = lowerLines[i];
    for (const t of tokens) {
      if (l.includes(t)) {
        hitLines.push(i);
        break;
      }
    }
  }

  // If no direct hits, skip snippet extraction
  if (hitLines.length === 0) return [];

  // Cluster hits into windows
  const windows: Array<{ start: number; end: number }> = [];
  const half = Math.max(2, Math.floor(windowLines / 2));

  hitLines.sort((a, b) => a - b);
  for (const hit of hitLines) {
    const start = Math.max(0, hit - half);
    const end = Math.min(lines.length - 1, hit + half);
    const prev = windows[windows.length - 1];
    if (prev && start <= prev.end + 2) {
      prev.end = Math.max(prev.end, end);
    } else {
      windows.push({ start, end });
    }
    if (windows.length >= 3) break;
  }

  return windows.map(w => {
    const snippetText = lines.slice(w.start, w.end + 1).join('\n');
    return {
      path: relPath,
      startLine: w.start + 1,
      endLine: w.end + 1,
      text: snippetText,
      score: scoreText(snippetText, relPath, tokens),
    };
  });
}

export async function searchRepository(options: SearchOptions): Promise<{ snippets: RepoSnippet[]; tokens: string[] }> {
  const {
    repoRoot,
    question,
    maxFiles = 1600,
    maxFileBytes = 1_000_000,
    maxSnippets = 10,
    windowLines = 18,
  } = options;

  const tokens = tokenize(question);
  if (tokens.length === 0) return { snippets: [], tokens };

  const candidates: RepoSnippet[] = [];

  for (const relDir of DEFAULT_DIRS) {
    const dirAbs = path.join(repoRoot, relDir);

    for await (const relPath of walkFiles(dirAbs, relDir, maxFiles)) {
      const absPath = path.join(repoRoot, relPath);

      let buf: Buffer;
      try {
        buf = await fs.readFile(absPath);
      } catch {
        continue;
      }

      if (buf.byteLength > maxFileBytes) continue;
      const text = buf.toString('utf8');

      const fileScore = scoreText(text, relPath, tokens);
      if (fileScore <= 0) continue;

      const windows = extractWindows(text, relPath.replace(/\\/g, '/'), tokens, windowLines);
      for (const w of windows) {
        candidates.push(w);
      }
    }
  }

  candidates.sort((a, b) => b.score - a.score);

  // De-dupe by (path,start,end)
  const seen = new Set<string>();
  const picked: RepoSnippet[] = [];
  for (const c of candidates) {
    const key = `${c.path}::${c.startLine}::${c.endLine}`;
    if (seen.has(key)) continue;
    seen.add(key);
    picked.push(c);
    if (picked.length >= maxSnippets) break;
  }

  return { snippets: picked, tokens };
}
