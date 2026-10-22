import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, copyFile, writeFile, symlink, rm, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';

// Exercise the real server and sockets in a disposable document root.
test('static server serves public files but refuses symlinks outside its root', async (t) => {
  const dir = await mkdtemp(join(tmpdir(), 'x3-static-server-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const root = join(dir, 'public');
  await mkdir(root);
  await copyFile(fileURLToPath(new URL('../serve.mjs', import.meta.url)), join(root, 'serve.mjs'));
  await writeFile(join(root, 'index.html'), '<h1>X3</h1>');
  await writeFile(join(dir, 'private.txt'), 'outside-private-fixture');
  await symlink(join(dir, 'private.txt'), join(root, 'leak.txt'));
  await symlink(dir, join(root, 'outside'));
  const reservation = createServer().listen(0, '127.0.0.1');
  await once(reservation, 'listening');
  const port = reservation.address().port;
  await new Promise((resolve, reject) => reservation.close(error => error ? reject(error) : resolve()));
  const child = spawn(process.execPath, [join(root, 'serve.mjs'), '--port', String(port), '--host', '127.0.0.1']);
  t.after(async () => {
    if (child.exitCode === null) { const exited = once(child, 'exit'); child.kill(); await exited; }
  });
  let errors = '';
  child.stderr.on('data', data => { errors += data; });
  await Promise.race([
    once(child.stdout, 'data'),
    once(child, 'exit').then(([code]) => { throw new Error(`server exited ${code}: ${errors}`); }),
    new Promise((_, reject) => { const timer = setTimeout(() => reject(new Error('server startup timed out')), 5000); timer.unref(); }),
  ]);
  const origin = `http://127.0.0.1:${port}`;
  const index = await fetch(origin);
  assert.equal(index.status, 200);
  assert.equal(await index.text(), '<h1>X3</h1>');
  assert.equal((await fetch(`${origin}/missing`)).status, 404);
  for (const path of ['/leak.txt', '/outside/private.txt']) {
    const response = await fetch(origin + path);
    assert.equal(response.status, 403, path);
    assert.equal((await response.text()).includes('outside-private-fixture'), false);
  }
});
