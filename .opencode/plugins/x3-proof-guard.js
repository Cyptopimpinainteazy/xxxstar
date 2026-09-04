import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";

const BANNED_PRODUCTION_MARKERS = [
  /\bTODO\b/i, /\bFIXME\b/i, /\bHACK\b/i,
  /\bstub\b/i, /\bmock\b/i, /\bfake\b/i,
  /\bplaceholder\b/i, /\bdummy\b/i,
  /unimplemented!\s*\(/i, /todo!\s*\(/i,
  /panic!\s*\(\s*["']not implemented/i,
  /return\s+Ok\s*\(\s*\(\s*\)\s*\)\s*;/i,
  /return\s+true\s*;/i, /return\s+false\s*;/i
];

const CLAIM_WORDS = [
  "implemented", "complete", "completed", "done",
  "finished", "verified", "working", "production-ready",
  "mainnet-ready", "testnet-ready"
];

const PROOF_COMMANDS = [
  "./scripts/x3-hard-gate.sh", "./scripts/x3-proof.sh",
  "cargo check", "cargo test", "cargo clippy",
  "pnpm test", "pnpm build", "npm test",
  "python3 -m pytest", "python -m pytest"
];

function safeJson(value) {
  try { return JSON.stringify(value ?? {}); }
  catch { return String(value ?? ""); }
}

function repoRoot(ctx) {
  return ctx.worktree || ctx.directory || process.cwd();
}

function rel(root, filePath) {
  if (!filePath) return "";
  const absolute = path.isAbsolute(filePath) ? filePath : path.join(root, filePath);
  return path.relative(root, absolute).replaceAll("\\", "/");
}

function isDocsPath(filePath) {
  const p = filePath.replaceAll("\\", "/");
  return p.startsWith("docs/") || p.endsWith(".md") || p.endsWith(".txt") || p === "README" || p === "README.md";
}

function isTestPath(filePath) {
  const p = filePath.replaceAll("\\", "/").toLowerCase();
  return p.includes("/test/") || p.includes("/tests/") || p.includes("__tests__") ||
    p.endsWith(".test.ts") || p.endsWith(".test.tsx") || p.endsWith(".spec.ts") ||
    p.endsWith(".spec.tsx") || p.endsWith("_test.rs") || p.endsWith("_test.py") ||
    p.endsWith(".test.js") || p.endsWith(".spec.js");
}

function isIgnoredPath(filePath) {
  const p = filePath.replaceAll("\\", "/");
  return p.includes("/.git/") || p.includes("/target/") || p.includes("/node_modules/") ||
    p.includes("/.venv/") || p.includes("/dist/") || p.includes("/build/");
}

function isProductionCodePath(filePath) {
  if (!filePath) return false;
  if (isIgnoredPath(filePath)) return false;
  if (isDocsPath(filePath)) return false;
  if (isTestPath(filePath)) return false;
  const ext = path.extname(filePath).toLowerCase();
  return [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".sol", ".move",
    ".c", ".cpp", ".h", ".hpp", ".toml", ".json", ".yaml", ".yml", ".x3"].includes(ext);
}

function extractFilePath(input, output) {
  const blob = { input: input ?? {}, output: output ?? {} };
  const candidates = [
    blob.output?.args?.filePath, blob.output?.args?.path, blob.output?.args?.file,
    blob.output?.args?.target, blob.output?.args?.source,
    blob.input?.args?.filePath, blob.input?.args?.path, blob.input?.args?.file,
    blob.input?.args?.target, blob.input?.args?.source,
    blob.input?.filePath, blob.input?.path, blob.input?.file,
    blob.output?.filePath, blob.output?.path, blob.output?.file
  ];
  for (const item of candidates) {
    if (typeof item === "string" && item.trim()) return item.trim();
  }
  return "";
}

function extractCommand(input, output) {
  const candidates = [
    output?.args?.command, output?.args?.cmd,
    input?.args?.command, input?.args?.cmd,
    input?.command, output?.command
  ];
  for (const item of candidates) {
    if (typeof item === "string" && item.trim()) return item.trim();
  }
  return "";
}

function gitChangedFiles(root) {
  try {
    const out = execFileSync("git", ["diff", "--name-only"], {
      cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"]
    });
    return out.split("\n").map(x => x.trim()).filter(Boolean);
  } catch { return []; }
}

function scanProductionFile(root, filePath) {
  const relative = rel(root, filePath);
  const absolute = path.isAbsolute(filePath) ? filePath : path.join(root, filePath);
  if (!isProductionCodePath(relative)) return [];
  if (!existsSync(absolute)) return [];
  const text = readFileSync(absolute, "utf8");
  const hits = [];
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    for (const pattern of BANNED_PRODUCTION_MARKERS) {
      if (pattern.test(lines[i])) {
        hits.push(`${relative}:${i + 1}: ${lines[i].trim().slice(0, 220)}`);
        break;
      }
    }
  }
  return hits;
}

function commandHasProof(command) {
  return PROOF_COMMANDS.some(p => command.toLowerCase().includes(p.toLowerCase()));
}

function looksLikeCompletionClaim(text) {
  return CLAIM_WORDS.some(w => text.toLowerCase().includes(w));
}

function dangerousCommandReason(command) {
  const c = command.toLowerCase();
  if (c.includes("--no-verify")) return "blocked use of --no-verify";
  if (/(rm|mv|truncate|sed\s+-i|perl\s+-pi|python3?\s+-c|cat\s*>)/i.test(c) &&
    /scripts\/x3-(hard-gate|proof)\.sh/i.test(c))
    return "blocked modification/removal of X3 proof gate scripts";
  if (/(rm|mv|truncate|sed\s+-i|perl\s+-pi)/i.test(c) &&
    /\.github\/workflows\/x3-hard-gate\.yml/i.test(c))
    return "blocked modification/removal of X3 hard-gate CI workflow";
  if (/(rm\s+-rf|rm\s+-r|rm\s+-f)/i.test(c) && /(tests?|specs?)/i.test(c))
    return "blocked removal of tests";
  if (/cargo\s+test/i.test(c) && /(ignore|--ignored|--no-run)/i.test(c))
    return "blocked suspicious cargo test weakening";
  if (/(cargo|npm|pnpm|python3?|pytest)/i.test(c) && /(true\s*$|\|\|\s*true|;\s*true)/i.test(c))
    return "blocked command that hides test/build failure with true";
  return "";
}

async function log(client, level, message, extra = {}) {
  try {
    if (client?.app?.log) {
      await client.app.log({ body: { service: "x3-proof-guard", level, message, extra } });
      return;
    }
  } catch {}
  console.log(`[X3 ${level.toUpperCase()}]`, message, Object.keys(extra).length ? extra : "");
}

export const X3ProofGuard = async (ctx) => {
  const root = repoRoot(ctx);
  await log(ctx.client, "info", "X3 Proof Guard loaded", { root });

  return {
    "tool.execute.before": async (input, output) => {
      const toolName = String(input?.tool || output?.tool || "").toLowerCase();
      const filePath = extractFilePath(input, output);
      const relativeFile = rel(root, filePath);
      const command = extractCommand(input, output);
      const blob = safeJson({ input, output });

      if (relativeFile.includes(".env") || relativeFile.endsWith("secrets.json") ||
          relativeFile.endsWith("wallet.json") || relativeFile.endsWith("id_rsa") ||
          relativeFile.endsWith("id_ed25519")) {
        throw new Error(`X3 Proof Guard blocked sensitive file: ${relativeFile}`);
      }

      if (toolName.includes("bash") || command) {
        const reason = dangerousCommandReason(command);
        if (reason) throw new Error(`X3 Proof Guard ${reason}: ${command}`);

        if (looksLikeCompletionClaim(command) && !commandHasProof(command)) {
          await log(ctx.client, "warn", "Completion claim without proof", { command });
        }
      }

      if (isDocsPath(relativeFile) && looksLikeCompletionClaim(blob) && !commandHasProof(blob)) {
        const changed = gitChangedFiles(root);
        const nonDocs = changed.filter(p => !isDocsPath(p));
        if (nonDocs.length === 0) {
          throw new Error(`X3 Proof Guard blocked docs-only completion claim in ${relativeFile}.`);
        }
      }

      if (isProductionCodePath(relativeFile) &&
          BANNED_PRODUCTION_MARKERS.some(p => p.test(blob))) {
        throw new Error(`X3 Proof Guard blocked stub/mock/fake in production edit: ${relativeFile}`);
      }
    },

    "file.edited": async (input) => {
      const filePath = input?.filePath || input?.path || input?.file || input?.target || "";
      const relativeFile = rel(root, filePath);
      const hits = scanProductionFile(root, filePath);
      if (hits.length > 0) {
        throw new Error([
          `X3 Proof Guard found forbidden markers in ${relativeFile}:`,
          ...hits.slice(0, 25),
          hits.length > 25 ? `...and ${hits.length - 25} more` : ""
        ].filter(Boolean).join("\n"));
      }
      if (isDocsPath(relativeFile)) {
        const changed = gitChangedFiles(root);
        const nonDocs = changed.filter(p => !isDocsPath(p));
        if (changed.length > 0 && nonDocs.length === 0) {
          throw new Error(`X3 Proof Guard blocked docs-only change: ${relativeFile}.`);
        }
      }
    },

    "session.idle": async () => {
      await log(ctx.client, "warn", "Session idle. Run ./scripts/x3-hard-gate.sh before claiming completion.");
    },

    "experimental.session.compacting": async (_input, output) => {
      output.context.push(`
## X3 Proof Guard Context

Agent must not claim completion without command output proof.

Required: ./scripts/x3-hard-gate.sh, cargo check --workspace, cargo test --workspace, cargo clippy -- -D warnings

Forbidden: docs-only completion, fake adapters, fake relayers, fake proofs, no-op paths, placeholder code, production mocks/stubs, deleting tests to pass, weakening proof gates.
`);
    }
  };
};

export default X3ProofGuard;
