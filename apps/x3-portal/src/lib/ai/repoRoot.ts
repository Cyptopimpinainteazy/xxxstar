import fs from 'node:fs/promises';
import path from 'node:path';

async function exists(filePath: string) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

export async function findRepoRoot(startDir: string) {
  let current = path.resolve(startDir);

  for (let i = 0; i < 8; i++) {
    const cargo = path.join(current, 'Cargo.toml');
    const appsExplorer = path.join(current, 'apps', 'explorer');

    // Heuristic: this repo is a Rust workspace + apps/ tree
    if ((await exists(cargo)) && (await exists(appsExplorer))) return current;

    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }

  // Fallback: use startDir if we can't find it
  return path.resolve(startDir);
}
