import { watch } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { generate } from '../../../scripts/transparency/generate.mjs'

const appRoot = resolve(fileURLToPath(new URL('..', import.meta.url)))
const repositoryRoot = resolve(appRoot, '../..')
const options = {
  ledgerPath: resolve(appRoot, 'data/treasury-ledger.json'),
  readinessRoot: resolve(repositoryRoot, 'audit-artifacts/mainnet-readiness/live'),
  outputPath: resolve(appRoot, 'public/data/portal.json'),
}

let running = false
let queued = false

async function refresh() {
  if (running) { queued = true; return }
  running = true
  try {
    const portal = await generate(options)
    process.stdout.write(`Updated transparency portal from ${portal.source.snapshotPath}\n`)
  } catch (error) {
    process.stderr.write(`Transparency refresh failed: ${error instanceof Error ? error.message : String(error)}\n`)
  } finally {
    running = false
    if (queued) { queued = false; void refresh() }
  }
}

await refresh()
watch(options.readinessRoot, { persistent: true }, (_event, filename) => {
  if (filename === 'current.json') void refresh()
})
watch(options.ledgerPath, { persistent: true }, () => { void refresh() })
process.stdout.write('Watching readiness pointer and treasury ledger for portal updates.\n')
