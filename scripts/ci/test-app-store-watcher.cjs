#!/usr/bin/env node
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { createRequire } = require('node:module');
const { once } = require('node:events');
const http = require('node:http');
const test = require('node:test');
const root = path.resolve(__dirname, '../../x3-app-store/frontend');
const projectRequire = createRequire(path.join(root, 'package.json'));
const live = projectRequire('live-server');
const { Client: WebSocketClient } = projectRequire('faye-websocket');

function get(url) {
  return new Promise((resolve, reject) => {
    http.get(url, (response) => {
      let body = '';
      response.setEncoding('utf8');
      response.on('data', (part) => { body += part; });
      response.on('end', () => resolve({ status: response.statusCode, body }));
      response.on('error', reject);
    }).on('error', reject);
  });
}

test('real app-store server serves source and reloads after a watched change', { timeout: 15000 }, async (t) => {
  const proof = path.join(root, `x3-watcher-proof-${process.pid}.txt`);
  const server = live.start({ host: '127.0.0.1', port: 0, open: false, root, logLevel: 0 });
  const ready = once(live.watcher, 'ready', { signal: t.signal });
  let ws;
  try {
    await once(server, 'listening', { signal: t.signal });
    await ready;
    const address = server.address();
    const base = `http://127.0.0.1:${address.port}`;
    const index = await get(`${base}/index.html`);
    assert.equal(index.status, 200);
    assert.match(index.body, /<title>X3 App Store<\/title>/);
    const app = await get(`${base}/src/App.tsx`);
    assert.equal(app.status, 200);
    assert.match(app.body, /function App\(/);
    assert.equal((await get(`${base}/missing-proof-asset`)).status, 404);
    ws = new WebSocketClient(`ws://127.0.0.1:${address.port}/ws`);
    const connected = once(ws, 'message', { signal: t.signal });
    await once(ws, 'open', { signal: t.signal });
    assert.equal((await connected)[0].data, 'connected');
    const message = once(ws, 'message', { signal: t.signal });
    await fs.writeFile(proof, 'actual file-system change\n');
    const [event] = await message;
    assert.equal(event.data, 'reload');
  } finally {
    if (ws) ws.close();
    await live.watcher.close();
    await fs.rm(proof, { force: true });
    await new Promise((resolve) => server.close(resolve));
  }
});
