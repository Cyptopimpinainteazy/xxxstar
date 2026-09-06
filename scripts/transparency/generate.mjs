import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { buildPortalData } from '../../apps/x3-transparency/scripts/transparency-data.mjs'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..')

function argument(name, fallback) {
  const index = process.argv.indexOf(name)
  return index === -1 ? fallback : process.argv[index + 1]
}

function requireArgument(name, value) {
  if (!value || value.startsWith('--')) throw new Error(`${name} needs a value`)
  return value
}

function safeSnapshotPath(value) {
  if (typeof value !== 'string' || !/^snapshots\/[A-Za-z0-9-]+$/.test(value)) throw new Error('readiness snapshot path is invalid')
  return value
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

const evidenceAssetNames = ['readiness.svg', 'subsystems.svg', 'findings.svg', 'tasks.svg', 'checks.svg']

export async function generate({ ledgerPath, readinessRoot, outputPath }) {
  const ledger = await readJson(ledgerPath)
  const pointer = await readJson(resolve(readinessRoot, 'current.json'))
  const snapshotPath = safeSnapshotPath(pointer.snapshot)
  const readiness = await readJson(resolve(readinessRoot, snapshotPath, 'summary.json'))
  const portal = buildPortalData({ ledger, readiness, snapshotPath })
  await mkdir(dirname(outputPath), { recursive: true })
  const assetsDirectory = resolve(dirname(outputPath), 'evidence-assets')
  await mkdir(assetsDirectory, { recursive: true })
  await Promise.all(evidenceAssetNames.map((name) => copyFile(
    resolve(readinessRoot, snapshotPath, 'assets', name),
    resolve(assetsDirectory, name),
  )))
  await writeFile(outputPath, JSON.stringify(portal, null, 2) + '\n')
  return portal
}

const directExecution = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)
if (directExecution) {
  const ledgerPath = resolve(requireArgument('--ledger', argument('--ledger', resolve(repositoryRoot, 'apps/x3-transparency/data/treasury-ledger.json'))))
  const readinessRoot = resolve(requireArgument('--readiness-root', argument('--readiness-root', resolve(repositoryRoot, 'audit-artifacts/mainnet-readiness/live'))))
  const outputPath = resolve(requireArgument('--output', argument('--output', resolve(repositoryRoot, 'apps/x3-transparency/public/data/portal.json'))))
  generate({ ledgerPath, readinessRoot, outputPath }).then((portal) => {
    process.stdout.write(`Generated transparency data for ${portal.source.snapshotPath}\n`)
  }).catch((error) => {
    process.stderr.write(`Transparency generation failed: ${error.message}\n`)
    process.exitCode = 1
  })
}
