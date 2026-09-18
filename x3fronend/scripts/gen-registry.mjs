#!/usr/bin/env node
// Generates data/registry.json directly from the repo's FEATURE_REGISTRY.toml
// so the site's status numbers can never drift from the file CI actually
// validates (scripts/check-readiness-consistency.sh checks the same source).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "smol-toml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..");
const SRC = resolve(REPO_ROOT, "FEATURE_REGISTRY.toml");
const OUT = resolve(__dirname, "..", "data", "registry.json");

const raw = readFileSync(SRC, "utf8");
const parsed = parse(raw);

const features = Object.entries(parsed).map(([id, f]) => ({
  id,
  mode: f.mode,
  crate_or_service: f.crate_or_service,
  readiness_score: f.readiness_score,
  blockers: f.blockers ?? [],
  required_tests: f.required_tests ?? [],
}));

const average =
  features.reduce((sum, f) => sum + f.readiness_score, 0) / features.length;

const out = {
  generatedFrom: "FEATURE_REGISTRY.toml",
  generatedAt: new Date().toISOString().slice(0, 10),
  featureCount: features.length,
  averageReadiness: Math.round(average * 10) / 10,
  features: features.sort((a, b) => b.readiness_score - a.readiness_score),
};

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, JSON.stringify(out, null, 2) + "\n");
console.log(
  `[gen-registry] wrote ${features.length} features, avg readiness ${out.averageReadiness}% -> data/registry.json`
);
