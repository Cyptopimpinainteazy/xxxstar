const fs = require("fs/promises");
const path = require("path");

async function ensureStore(storePath, seedPath) {
  try {
    await fs.access(storePath);
  } catch {
    await fs.mkdir(path.dirname(storePath), { recursive: true });
    const seed = await fs.readFile(seedPath, "utf8");
    await fs.writeFile(storePath, seed, "utf8");
  }
}

async function readStore(storePath, seedPath) {
  await ensureStore(storePath, seedPath);
  const raw = await fs.readFile(storePath, "utf8");
  return JSON.parse(raw);
}

async function writeStore(storePath, store) {
  await fs.writeFile(storePath, `${JSON.stringify(store, null, 2)}\n`, "utf8");
}

async function updateStore(storePath, seedPath, updater) {
  const current = await readStore(storePath, seedPath);
  const next = await updater(current);
  await writeStore(storePath, next);
  return next;
}

module.exports = {
  ensureStore,
  readStore,
  writeStore,
  updateStore,
};
