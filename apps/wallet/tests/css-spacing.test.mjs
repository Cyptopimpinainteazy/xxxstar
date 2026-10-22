import test from 'node:test';
import assert from 'node:assert/strict';
import { readdir, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';

test('compiled wallet spacing utilities survive the global reset', async (t) => {
  const cssDir = resolve('.next/static/css');
  const styles = (await readdir(cssDir, { recursive: true }))
    .filter(file => file.endsWith('.css'))
    .map(file => `<link rel="stylesheet" href="${pathToFileURL(join(cssDir, file))}">`).join('\n');
  assert.ok(styles, 'run npm run build before checking the emitted styles');
  const dir = await mkdtemp(join(tmpdir(), 'x3-wallet-css-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const html = join(dir, 'spacing.html');
  await writeFile(html, `<!doctype html><html><head>${styles}</head><body>
    <div id="spacing" class="p-6 mt-4">X3</div>
    <script>const computed = getComputedStyle(document.getElementById('spacing'));
    document.body.dataset.padding = computed.paddingTop;
    document.body.dataset.margin = computed.marginTop;</script></body></html>`);
  const browser = process.env.CHROMIUM_BIN || 'chromium';
  const run = spawnSync(browser, ['--headless', '--no-sandbox', '--disable-gpu',
    '--allow-file-access-from-files', '--disable-background-networking', '--disable-component-update',
    '--disable-sync', '--no-first-run', '--no-default-browser-check', '--virtual-time-budget=1000', '--dump-dom', pathToFileURL(html).href],
    { encoding: 'utf8', timeout: 20000 });
  assert.ifError(run.error);
  assert.equal(run.status, 0, run.stderr);
  assert.match(run.stdout, /data-padding="24px"/);
  assert.match(run.stdout, /data-margin="16px"/);
});
